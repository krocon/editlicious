//! Recordings of editor commands with time stamps (design 015 E1–E4): what a session did, in a
//! form that plays back the same on any window size – `*.edrec.jsonl`.

use super::model::{Command, Motion};
use crate::document::Document;
use crate::document::model::Sel;
use crate::error::{FsError, FsErrorCode};
use serde::{Deserialize, Serialize};
use web_time::Instant;

/// Version of the file format.
pub const FORMAT_VERSION: u32 = 1;
/// Clipboard texts up to this size are kept in the recording (design 015 E3).
pub const MAX_CLIP_BYTES: usize = 1 << 20;
/// Steps a recording keeps at most; later ones are dropped (and counted).
pub const MAX_STEPS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub tab_width: usize,
    pub insert_spaces: bool,
    pub wrap: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocStamp {
    pub name: String,
    pub lines: usize,
    /// FNV-1a 64 of the text as hex.
    pub hash: String,
}

/// First line of a recording file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub app: String,
    pub doc: DocStamp,
    /// Selections at the start (primary last).
    pub start: Vec<Sel>,
    pub settings: Settings,
}

impl Header {
    pub fn new(app: &str, doc: &Document, start: Vec<Sel>, settings: Settings) -> Self {
        Self {
            format: "edrec".into(),
            version: FORMAT_VERSION,
            app: app.into(),
            doc: stamp(doc),
            start,
            settings,
        }
    }
}

/// Name, lines and hash of a document – compared before a recording plays.
pub fn stamp(doc: &Document) -> DocStamp {
    DocStamp {
        name: doc.name(),
        lines: doc.line_count(),
        hash: format!("{:016x}", doc.text_hash()),
    }
}

/// One recorded command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecStep {
    /// Milliseconds since the start of the recording.
    pub t: u64,
    pub cmd: Command,
    /// The text a `paste` inserted (design 015 E3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<String>,
    /// A pasted text too large to keep: only its length in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip_ref: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recording {
    pub header: Header,
    pub steps: Vec<RecStep>,
    /// Steps beyond [`MAX_STEPS`] that were not kept.
    pub dropped: usize,
    /// The text when the recording started (a cheap rope copy; not in the file) – the video
    /// export starts from it (design 017).
    pub start_text: Option<ropey::Rope>,
}

impl Recording {
    pub fn duration_ms(&self) -> u64 {
        self.steps.last().map_or(0, |s| s.t)
    }

    /// The `*.edrec.jsonl` text: header line, then one step per line.
    pub fn to_jsonl(&self) -> String {
        let mut out = serde_json::to_string(&self.header).unwrap_or_default();
        out.push('\n');
        for s in &self.steps {
            out.push_str(&serde_json::to_string(s).unwrap_or_default());
            out.push('\n');
        }
        out
    }

    /// Reads a `*.edrec.jsonl` text; errors name the line (1-based).
    pub fn from_jsonl(text: &str) -> Result<Self, FsError> {
        let mut lines = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let (_, first) = lines.next().ok_or_else(|| invalid(1, "empty"))?;
        let header: Header =
            serde_json::from_str(first).map_err(|e| invalid(1, &format!("header: {e}")))?;
        if header.format != "edrec" {
            return Err(invalid(1, "not an editlicious recording"));
        }
        if header.version > FORMAT_VERSION {
            return Err(invalid(1, "newer format version"));
        }
        let mut steps = Vec::new();
        for (i, line) in lines {
            if steps.len() >= MAX_STEPS {
                return Err(invalid(i + 1, "too many steps"));
            }
            let step: RecStep =
                serde_json::from_str(line).map_err(|e| invalid(i + 1, &e.to_string()))?;
            steps.push(step);
        }
        Ok(Self {
            header,
            steps,
            dropped: 0,
            start_text: None,
        })
    }
}

pub(crate) fn invalid(line: usize, what: &str) -> FsError {
    FsError::new(
        FsErrorCode::RecordingInvalid,
        format!("line {line}: {what}"),
        None,
    )
}

/// A recording in progress.
#[derive(Debug)]
pub struct Recorder {
    started: Instant,
    rec: Recording,
}

impl Recorder {
    pub fn new(header: Header) -> Self {
        Self {
            started: Instant::now(),
            rec: Recording {
                header,
                steps: Vec::new(),
                dropped: 0,
                start_text: None,
            },
        }
    }

    /// Keeps the text the recording starts on (for the video export).
    pub fn with_start(mut self, text: ropey::Rope) -> Self {
        self.rec.start_text = Some(text);
        self
    }

    /// Keeps a command that ran; `pasted` = the clipboard text of a `paste`, `sels` = the
    /// selections after it (primary last).
    pub fn keep(&mut self, cmd: Command, pasted: Option<String>, sels: Vec<Sel>, wrap: bool) {
        let t = self.started.elapsed().as_millis() as u64;
        self.keep_at(t, cmd, pasted, sels, wrap);
    }

    pub(crate) fn keep_at(
        &mut self,
        t: u64,
        cmd: Command,
        pasted: Option<String>,
        sels: Vec<Sel>,
        wrap: bool,
    ) {
        if cmd.is_view_only() {
            return;
        }
        if self.rec.steps.len() >= MAX_STEPS {
            self.rec.dropped += 1;
            return;
        }
        let cmd = if depends_on_view(&cmd, wrap) {
            Command::Select { sels }
        } else {
            cmd
        };
        // two selections in a row (a drag): the last one is enough
        if let (Command::Select { .. }, Some(last)) = (&cmd, self.rec.steps.last_mut())
            && matches!(last.cmd, Command::Select { .. })
        {
            last.cmd = cmd;
            last.t = t;
            return;
        }
        let (clip, clip_ref) = match pasted {
            Some(p) if p.len() > MAX_CLIP_BYTES => (None, Some(p.len())),
            p => (p, None),
        };
        self.rec.steps.push(RecStep {
            t,
            cmd,
            clip,
            clip_ref,
        });
    }

    pub fn finish(self) -> Recording {
        self.rec
    }
}

/// Commands whose effect depends on window size or scroll position (design 015 E2).
fn depends_on_view(cmd: &Command, wrap: bool) -> bool {
    match cmd {
        Command::Click { .. } | Command::Drag { .. } => true,
        Command::Move { to, .. } => {
            matches!(to, Motion::PageUp | Motion::PageDown)
                || (wrap && matches!(to, Motion::Up | Motion::Down))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::model::Pos;

    fn header() -> Header {
        Header::new(
            "0.1.0",
            &Document::from_text("a\nb"),
            vec![Sel::default()],
            Settings {
                tab_width: 4,
                insert_spaces: true,
                wrap: false,
            },
        )
    }

    #[test]
    fn jsonl_round_trip() {
        let mut r = Recorder::new(header());
        r.keep_at(0, Command::Type { text: "x".into() }, None, vec![], false);
        r.keep_at(10, Command::Paste, Some("äö🙂".into()), vec![], false);
        r.keep_at(20, Command::Frame, None, vec![], false);
        let rec = r.finish();
        assert_eq!(rec.steps.len(), 2);
        let text = rec.to_jsonl();
        assert_eq!(text.lines().count(), 3);
        assert_eq!(Recording::from_jsonl(&text).unwrap(), rec);
        assert!(rec.start_text.is_none());
    }

    #[test]
    fn clicks_become_selections_and_drags_collapse() {
        let sel = |l| vec![Sel::caret(Pos::new(l, 0))];
        let mut r = Recorder::new(header());
        let click = Command::Click {
            line: 1.0,
            cell: 0.0,
            count: 1,
            extend: false,
            add: false,
        };
        r.keep_at(0, click, None, sel(1), false);
        let drag = Command::Drag {
            line: 2.0,
            cell: 0.0,
            column: false,
        };
        r.keep_at(5, drag.clone(), None, sel(2), false);
        r.keep_at(9, drag, None, sel(3), false);
        let rec = r.finish();
        assert_eq!(rec.steps.len(), 1);
        assert_eq!(rec.steps[0].cmd, Command::Select { sels: sel(3) });
        assert_eq!(rec.steps[0].t, 9);
    }

    #[test]
    fn page_moves_and_wrapped_vertical_moves_depend_on_the_view() {
        let page = Command::Move {
            to: Motion::PageDown,
            select: false,
        };
        let up = Command::Move {
            to: Motion::Up,
            select: false,
        };
        assert!(depends_on_view(&page, false));
        assert!(!depends_on_view(&up, false));
        assert!(depends_on_view(&up, true));
    }

    #[test]
    fn big_clipboard_texts_are_only_referenced() {
        let mut r = Recorder::new(header());
        let big = "x".repeat(MAX_CLIP_BYTES + 1);
        r.keep_at(0, Command::Paste, Some(big), vec![], false);
        let s = &r.finish().steps[0];
        assert_eq!(
            (s.clip.as_deref(), s.clip_ref),
            (None, Some(MAX_CLIP_BYTES + 1))
        );
    }

    #[test]
    fn bad_files_name_the_line() {
        let h = serde_json::to_string(&header()).unwrap();
        let e = Recording::from_jsonl(&format!("{h}\n{{\"t\":1,\"cmd\":{{\"type\":\"nope\"}}}}"))
            .unwrap_err();
        assert_eq!(e.code, FsErrorCode::RecordingInvalid);
        assert!(e.message.starts_with("line 2:"), "{}", e.message);
        assert!(Recording::from_jsonl("").is_err());
        assert!(Recording::from_jsonl("{\"format\":\"x\"}").is_err());
    }
}
