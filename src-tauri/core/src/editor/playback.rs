//! Playing recordings and scripts back (design 015 E6–E9): a deterministic timeline of engine
//! commands. The app runs it in real time, the video export (017) on a virtual clock, the MCP
//! server (016) types through it.

use super::model::{Command, Frame};
use super::recording::Recording;
use super::{Clipboard, MemoryClipboard, Session};
use crate::error::{FsError, FsErrorCode};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// How fast a playback types (design 015 E7).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Tempo {
    /// The times of the recording (scripts: [`DEFAULT_CPM`] per character).
    Original,
    /// Everything at once (a macro).
    Instant,
    /// Faster (> 1) or slower than the original.
    Factor { factor: f64 },
    /// The same time before every step.
    Even { ms: u64 },
    /// Like a person: characters per minute, spread, longer after punctuation; `seed` makes it
    /// repeatable, `typos` = chance of a typo (and its correction) per letter.
    Human { cpm: f64, seed: u64, typos: f64 },
}

impl Tempo {
    pub fn human() -> Self {
        Tempo::Human {
            cpm: DEFAULT_CPM,
            seed: 1,
            typos: 0.0,
        }
    }
}

/// Characters per minute when nothing else says.
pub const DEFAULT_CPM: f64 = 330.0;
/// Time before a command (not typing) when nothing else says.
pub const DEFAULT_ACTION_MS: u64 = 250;

/// What a recording or script consists of.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Text to type character by character (design 015 E6).
    Type(String),
    /// One engine command; `gap` = time since the step before (recordings), `clip` = the text
    /// a `paste` brought.
    Cmd {
        cmd: Command,
        clip: Option<String>,
        gap: Option<u64>,
    },
    Pause(u64),
    /// From here on another tempo (scripts).
    Tempo(Tempo),
    /// Shows keys or a caption in a video (design 017); nothing happens in the editor.
    Keys(String),
}

/// One step of the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// Milliseconds since the start.
    pub at: u64,
    pub cmd: Command,
    pub clip: Option<String>,
    /// A caption for the video export (`keys "Ctrl+D"`); the command is then `frame`.
    pub overlay: Option<String>,
}

/// The items of a recording; a pasted text that was too large to keep cannot play.
/// `from_start`: first put the cursors where the recording started (a file played on its
/// text); a macro (`false`) works where the cursors are now.
pub fn items_of(rec: &Recording, from_start: bool) -> Result<Vec<Item>, FsError> {
    let mut out = Vec::with_capacity(rec.steps.len() + 1);
    if let Some(start) =
        (from_start && !rec.header.start.is_empty()).then(|| rec.header.start.clone())
    {
        out.push(Item::Cmd {
            cmd: Command::Select { sels: start },
            clip: None,
            gap: Some(0),
        });
    }
    let mut last = 0;
    for s in &rec.steps {
        if s.clip_ref.is_some() {
            return Err(FsError::new(
                FsErrorCode::RecordingUnsupported,
                "a pasted text was too large to record",
                None,
            ));
        }
        out.push(Item::Cmd {
            cmd: s.cmd.clone(),
            clip: s.clip.clone(),
            gap: Some(s.t.saturating_sub(last)),
        });
        last = s.t;
    }
    Ok(out)
}

/// SplitMix64: small, fast and the same everywhere (no dependency, WebAssembly too).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// In `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Neighbouring keys on a QWERTY keyboard – the typo for a letter.
fn neighbour(c: char, rng: &mut Rng) -> Option<char> {
    const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
    let lower = c.to_ascii_lowercase();
    for row in ROWS {
        if let Some(i) = row.find(lower) {
            let bytes = row.as_bytes();
            let mut options = Vec::new();
            if i > 0 {
                options.push(bytes[i - 1] as char);
            }
            if i + 1 < bytes.len() {
                options.push(bytes[i + 1] as char);
            }
            let pick = options[(rng.next_u64() % options.len() as u64) as usize];
            return Some(if c.is_ascii_uppercase() {
                pick.to_ascii_uppercase()
            } else {
                pick
            });
        }
    }
    None
}

struct Clock<'a> {
    tempo: Tempo,
    rng: Rng,
    at: u64,
    out: &'a mut Vec<Step>,
}

impl Clock<'_> {
    fn set_tempo(&mut self, t: Tempo) {
        if let Tempo::Human { seed, .. } = t {
            self.rng = Rng::new(seed);
        }
        self.tempo = t;
    }

    fn push(&mut self, delay: u64, cmd: Command, clip: Option<String>) {
        self.at += delay;
        self.out.push(Step {
            at: self.at,
            cmd,
            clip,
            overlay: None,
        });
    }

    /// Time before a typed grapheme `g` that follows `prev`.
    fn char_delay(&mut self, g: &str, prev: &str) -> u64 {
        let base = |cpm: f64| 60_000.0 / cpm;
        match self.tempo {
            Tempo::Instant => 0,
            Tempo::Original => base(DEFAULT_CPM).round() as u64,
            Tempo::Factor { factor: f } => (base(DEFAULT_CPM) / f).round() as u64,
            Tempo::Even { ms } => ms,
            Tempo::Human { cpm, .. } => {
                let mut d = base(cpm) * (0.55 + 1.05 * self.rng.unit());
                if prev.ends_with(['.', '!', '?', ';', ':', ',']) {
                    d *= 3.0;
                } else if prev == "\n" {
                    d *= 4.0;
                } else if prev == " " && g != " " {
                    d *= 1.3;
                }
                d.round() as u64
            }
        }
    }

    /// Time before a command: `gap` from a recording, else a human or fixed delay.
    fn action_delay(&mut self, gap: Option<u64>) -> u64 {
        match self.tempo {
            Tempo::Instant => 0,
            Tempo::Original => gap.unwrap_or(DEFAULT_ACTION_MS),
            Tempo::Factor { factor: f } => {
                (gap.unwrap_or(DEFAULT_ACTION_MS) as f64 / f).round() as u64
            }
            Tempo::Even { ms } => ms,
            Tempo::Human { .. } => {
                (DEFAULT_ACTION_MS as f64 * (0.6 + 1.2 * self.rng.unit())).round() as u64
            }
        }
    }

    fn pause(&mut self, ms: u64) {
        self.at += match self.tempo {
            Tempo::Instant => 0,
            Tempo::Factor { factor: f } => (ms as f64 / f).round() as u64,
            _ => ms,
        };
    }

    /// Types `text`: a grapheme per step, a line break as `type "\n"` and the indentation of
    /// the next line in one step (design 015 E6).
    fn type_text(&mut self, text: &str) {
        let mut prev = String::new();
        let lines: Vec<&str> = text.split('\n').collect();
        for (n, line) in lines.iter().enumerate() {
            let mut rest: &str = line;
            if n > 0 {
                let d = self.char_delay("\n", &prev);
                self.push(d, Command::Type { text: "\n".into() }, None);
                prev = "\n".into();
                let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
                if indent_len > 0 {
                    let d = self.char_delay(" ", &prev) / 2;
                    self.push(
                        d,
                        Command::Insert {
                            text: line[..indent_len].into(),
                        },
                        None,
                    );
                    rest = &line[indent_len..];
                    prev = " ".into();
                }
            }
            for g in rest.graphemes(true) {
                let d = self.char_delay(g, &prev);
                if let Tempo::Human { typos, .. } = self.tempo
                    && typos > 0.0
                    && g.len() == 1
                    && g.chars().all(|c| c.is_ascii_alphabetic())
                    && self.rng.unit() < typos
                    && let Some(wrong) = neighbour(g.chars().next().unwrap_or('a'), &mut self.rng)
                {
                    self.push(d, Command::Type { text: wrong.into() }, None);
                    let notice = 180 + (self.rng.unit() * 320.0) as u64;
                    self.push(notice, Command::Backspace, None);
                    let again = self.char_delay(g, " ");
                    self.push(again, Command::Type { text: g.into() }, None);
                } else {
                    self.push(d, Command::Type { text: g.into() }, None);
                }
                prev = g.into();
            }
        }
    }
}

/// The timeline of items under a tempo (scripts may change it with `tempo`).
pub fn timeline(items: &[Item], tempo: Tempo) -> Vec<Step> {
    let mut out = Vec::new();
    let mut clock = Clock {
        tempo,
        rng: Rng::new(1),
        at: 0,
        out: &mut out,
    };
    clock.set_tempo(tempo);
    for item in items {
        match item {
            Item::Tempo(t) => clock.set_tempo(*t),
            Item::Pause(ms) => clock.pause(*ms),
            Item::Keys(text) => clock.out.push(Step {
                at: clock.at,
                cmd: Command::Frame,
                clip: None,
                overlay: Some(text.clone()),
            }),
            Item::Type(text) => clock.type_text(text),
            Item::Cmd { cmd, clip, gap } => {
                // a recording with human tempo: typed characters get a human rhythm too
                let d = match (cmd, clock.tempo) {
                    (Command::Type { text }, Tempo::Human { .. }) => clock.char_delay(text, " "),
                    _ => clock.action_delay(*gap),
                };
                clock.push(d, cmd.clone(), clip.clone());
            }
        }
    }
    out
}

/// A playback in progress: the timeline, how far it got and its own clipboard (design 015 E3).
#[derive(Debug)]
pub struct Playback {
    pub steps: Vec<Step>,
    pub next: usize,
    clip: MemoryClipboard,
}

impl Playback {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            next: 0,
            clip: MemoryClipboard::default(),
        }
    }

    pub fn done(&self) -> bool {
        self.next >= self.steps.len()
    }

    pub fn duration_ms(&self) -> u64 {
        self.steps.last().map_or(0, |s| s.at)
    }

    /// When the next step is due (ms since the start).
    pub fn next_at(&self) -> Option<u64> {
        self.steps.get(self.next).map(|s| s.at)
    }

    /// Starts: the whole playback becomes one undo step (design 015 E8).
    pub fn begin(&self, session: &mut Session) {
        session.doc.begin_group();
    }

    /// Ends (also when stopped early).
    pub fn end(&self, session: &mut Session) {
        session.doc.end_group();
    }

    /// Runs every step due at `elapsed` ms; returns the last frame (`None` = nothing was due).
    pub fn run_due(
        &mut self,
        session: &mut Session,
        elapsed: u64,
    ) -> Result<Option<Frame>, FsError> {
        let mut frame = None;
        while let Some(s) = self.steps.get(self.next)
            && s.at <= elapsed
        {
            frame = Some(self.step(session)?);
        }
        Ok(frame)
    }

    /// Runs the next step.
    pub fn step(&mut self, session: &mut Session) -> Result<Frame, FsError> {
        let Some(s) = self.steps.get(self.next).cloned() else {
            return session.run(Command::Frame, &mut self.clip);
        };
        self.next += 1;
        if let Some(c) = s.clip {
            self.clip.set(c);
        }
        session.run(s.cmd, &mut self.clip)
    }
}

/// Plays everything at once (a macro, tests): one undo step; returns the last frame.
pub fn run_all(session: &mut Session, steps: Vec<Step>) -> Result<Frame, FsError> {
    let mut p = Playback::new(steps);
    p.begin(session);
    let result = (|| {
        let mut f = session.run(Command::Frame, &mut MemoryClipboard::default())?;
        while !p.done() {
            f = p.step(session)?;
        }
        Ok(f)
    })();
    p.end(session);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::document::model::{Pos, Sel};
    use crate::editor::Cursor;
    use crate::editor::model::Motion;
    use crate::editor::script;

    fn session(text: &str) -> Session {
        let mut s = Session::new(Document::from_text(text), 1);
        s.run(
            Command::Resize {
                lines: 20.0,
                cols: 60.0,
            },
            &mut MemoryClipboard::default(),
        )
        .unwrap();
        s
    }

    fn text(s: &Session) -> String {
        s.doc.rope().to_string()
    }

    fn cmds(s: &mut Session, list: Vec<Command>, clip: &mut MemoryClipboard) {
        for c in list {
            s.run(c, clip).unwrap();
        }
    }

    /// Records what `list` does, plays it on a fresh copy of `start`, compares text and cursors.
    fn round_trip(start: &str, list: Vec<Command>, clip_text: Option<&str>) {
        let mut a = session(start);
        let mut clip = MemoryClipboard(clip_text.map(str::to_owned));
        a.start_recording("test");
        cmds(&mut a, list, &mut clip);
        let rec = a.stop_recording().unwrap().clone();
        let rec = Recording::from_jsonl(&rec.to_jsonl()).unwrap();
        let mut b = session(start);
        run_all(
            &mut b,
            timeline(&items_of(&rec, true).unwrap(), Tempo::Instant),
        )
        .unwrap();
        assert_eq!(text(&b), text(&a));
        assert_eq!(b.cursors(), a.cursors());
        // and through a script
        let script = script::from_recording(&rec);
        let items = script::parse(&script, &mut |_| unreachable!()).unwrap();
        let mut c = session(start);
        run_all(&mut c, timeline(&items, Tempo::Instant)).unwrap();
        assert_eq!(text(&c), text(&a), "script:\n{script}");
    }

    fn typed(t: &str) -> Vec<Command> {
        t.chars()
            .map(|c| match c {
                '\n' => Command::Newline,
                c => Command::Type { text: c.into() },
            })
            .collect()
    }

    #[test]
    fn recording_plays_back_the_same() {
        let mut list = typed("héllo 🙂 中文 e\u{301}\nnext line");
        list.extend([
            Command::Move {
                to: Motion::WordLeft,
                select: true,
            },
            Command::Cut,
            Command::Move {
                to: Motion::DocStart,
                select: false,
            },
            Command::Paste,
            Command::Backspace,
            Command::DuplicateLines,
            Command::Undo,
            Command::Redo,
        ]);
        round_trip("", list, None);
    }

    #[test]
    fn crlf_and_multiple_cursors() {
        let mut list = vec![Command::Click {
            line: 0.0,
            cell: 1.0,
            count: 1,
            extend: false,
            add: false,
        }];
        list.push(Command::Click {
            line: 1.0,
            cell: 1.0,
            count: 1,
            extend: false,
            add: true,
        });
        list.extend(typed("XY"));
        list.push(Command::Newline);
        round_trip("ab\r\ncd\r\nef", list, None);
    }

    #[test]
    fn paste_uses_the_recorded_text_not_the_clipboard() {
        round_trip("x", vec![Command::Paste, Command::Paste], Some("pasted\n"));
    }

    #[test]
    fn a_playback_is_one_undo_step_even_when_stopped() {
        let mut s = session("start");
        s.run(
            Command::Type { text: "!".into() },
            &mut MemoryClipboard::default(),
        )
        .unwrap();
        let items = script::parse(
            "move docEnd\ntype \"ab\\ncd\"\nnewline",
            &mut |_| unreachable!(),
        )
        .unwrap();
        run_all(&mut s, timeline(&items, Tempo::Instant)).unwrap();
        assert_eq!(text(&s), "!startab\ncd\n");
        s.run(Command::Undo, &mut MemoryClipboard::default())
            .unwrap();
        assert_eq!(text(&s), "!start");
        s.run(Command::Redo, &mut MemoryClipboard::default())
            .unwrap();
        assert_eq!(text(&s), "!startab\ncd\n");
        // stopped half way
        let mut p = Playback::new(timeline(&items, Tempo::Instant));
        let mut t = session("x");
        p.begin(&mut t);
        for _ in 0..3 {
            p.step(&mut t).unwrap();
        }
        p.end(&mut t);
        assert_eq!(text(&t), "xab");
        t.run(Command::Undo, &mut MemoryClipboard::default())
            .unwrap();
        assert_eq!(text(&t), "x");
    }

    #[test]
    fn script_typing_keeps_indentation_exactly() {
        let mut s = session("");
        let code = "fn main() {\n    if x {\n        y();\n    }\n}";
        let items = vec![Item::Type(code.into())];
        run_all(&mut s, timeline(&items, Tempo::human())).unwrap();
        assert_eq!(text(&s), code);
    }

    #[test]
    fn human_tempo_is_repeatable_and_typos_correct_themselves() {
        let items = vec![
            Item::Tempo(Tempo::Human {
                cpm: 300.0,
                seed: 42,
                typos: 0.3,
            }),
            Item::Type("the quick brown fox jumps over the lazy dog".into()),
        ];
        let a = timeline(&items, Tempo::Original);
        let b = timeline(&items, Tempo::Original);
        assert_eq!(a, b);
        assert!(
            a.iter().any(|s| s.cmd == Command::Backspace),
            "no typo at 30 %"
        );
        assert!(a.windows(2).all(|w| w[0].at <= w[1].at));
        let mut s = session("");
        run_all(&mut s, a.clone()).unwrap();
        assert_eq!(text(&s), "the quick brown fox jumps over the lazy dog");
        // another seed, another rhythm
        let mut other = items.clone();
        other[0] = Item::Tempo(Tempo::Human {
            cpm: 300.0,
            seed: 43,
            typos: 0.3,
        });
        assert_ne!(timeline(&other, Tempo::Original), a);
        // about 300 characters per minute
        let plain = timeline(
            &[Item::Type("x".repeat(300))],
            Tempo::Human {
                cpm: 300.0,
                seed: 1,
                typos: 0.0,
            },
        );
        let ms = plain.last().unwrap().at;
        assert!((40_000..80_000).contains(&ms), "{ms}");
    }

    #[test]
    fn tempos_scale_the_original_times() {
        let items = vec![
            Item::Cmd {
                cmd: Command::Newline,
                clip: None,
                gap: Some(1000),
            },
            Item::Pause(500),
            Item::Cmd {
                cmd: Command::Newline,
                clip: None,
                gap: Some(200),
            },
        ];
        let at = |t| -> Vec<u64> { timeline(&items, t).iter().map(|s| s.at).collect() };
        assert_eq!(at(Tempo::Original), vec![1000, 1700]);
        assert_eq!(at(Tempo::Factor { factor: 2.0 }), vec![500, 850]);
        assert_eq!(at(Tempo::Even { ms: 100 }), vec![100, 700]);
        assert_eq!(at(Tempo::Instant), vec![0, 0]);
    }

    #[test]
    fn run_due_follows_the_clock() {
        let items = vec![Item::Type("abc".into())];
        let mut p = Playback::new(timeline(&items, Tempo::Even { ms: 100 }));
        let mut s = session("");
        assert!(p.run_due(&mut s, 50).unwrap().is_none());
        p.run_due(&mut s, 250).unwrap().unwrap();
        assert_eq!(text(&s), "ab");
        assert_eq!(p.next_at(), Some(300));
        p.run_due(&mut s, 1000).unwrap();
        assert!(p.done());
        assert_eq!(text(&s), "abc");
    }

    #[test]
    fn a_macro_works_where_the_cursor_is_now() {
        let mut s = session("one\ntwo");
        let mut clip = MemoryClipboard::default();
        s.start_recording("t");
        cmds(&mut s, typed("[x]"), &mut clip);
        let rec = s.stop_recording().unwrap().clone();
        s.run(
            Command::Move {
                to: Motion::DocEnd,
                select: false,
            },
            &mut clip,
        )
        .unwrap();
        run_all(
            &mut s,
            timeline(&items_of(&rec, false).unwrap(), Tempo::Instant),
        )
        .unwrap();
        assert_eq!(text(&s), "[x]one\ntwo[x]");
    }

    #[test]
    fn select_sets_cursors_with_the_primary_last() {
        let mut s = session("abc\ndef");
        let sels = vec![
            Sel::caret(Pos::new(1, 1)),
            Sel {
                anchor: Pos::new(0, 0),
                head: Pos::new(0, 2),
            },
        ];
        s.run(Command::Select { sels }, &mut MemoryClipboard::default())
            .unwrap();
        assert_eq!(s.cursors().len(), 2);
        assert_eq!(
            s.primary(),
            Cursor {
                anchor: Pos::new(0, 0),
                head: Pos::new(0, 2),
                goal: None
            }
        );
        assert_eq!(s.sels_primary_last().last().unwrap().head, Pos::new(0, 2));
    }

    #[test]
    fn too_large_pastes_cannot_play() {
        let mut a = session("");
        a.start_recording("t");
        let mut clip = MemoryClipboard(Some(
            "x".repeat(super::super::recording::MAX_CLIP_BYTES + 1),
        ));
        a.run(Command::Paste, &mut clip).unwrap();
        let rec = a.stop_recording().unwrap();
        assert_eq!(
            items_of(rec, true).unwrap_err().code,
            FsErrorCode::RecordingUnsupported
        );
    }
}
