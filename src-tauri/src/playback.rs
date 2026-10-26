//! Recordings and scripts in the app (design 015 E9/E10): reading them from disk and playing
//! them back in real time on a thread – every step under the session lock, the frame to the
//! UI over a channel. The timeline itself lives in `ed-core` (`editor::playback`).

use crate::document::registry::{Handle, lock_doc};
use crate::editor::Session;
use crate::editor::model::Frame;
use crate::editor::playback::{self, Item, Playback, Tempo};
use crate::editor::recording::{self, Recording};
use crate::editor::script;
use crate::error::{FsError, FsErrorCode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// Largest recording or script file read.
pub const MAX_FILE_BYTES: u64 = 64 << 20;

/// What to play: a file or the last recording of the document.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Source {
    Path { path: String },
    Last,
}

/// What `check_playback` tells the UI before playing.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCheck {
    /// `recording` or `script`.
    pub kind: String,
    pub steps: usize,
    /// At the default tempo.
    pub duration_ms: u64,
    /// A recording made on another text (name/lines/hash differ) – ask before playing.
    pub other_text: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    pub steps: usize,
    pub duration_ms: u64,
    pub dropped: usize,
}

impl RecordingInfo {
    pub fn of(r: &Recording) -> Self {
        Self {
            steps: r.steps.len(),
            duration_ms: r.duration_ms(),
            dropped: r.dropped,
        }
    }
}

/// Events of a running playback.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PlaybackEvent {
    Frame {
        frame: Box<Frame>,
    },
    Progress {
        done: usize,
        total: usize,
        paused: bool,
    },
    /// `reason`: `finished`, `stopped` or an error code.
    Done {
        reason: String,
    },
}

/// A loaded source: its items, the recording (for the text check) and its kind.
pub struct Loaded {
    pub items: Vec<Item>,
    /// Compared with the document before playing (files only, not a macro).
    pub recording: Option<Recording>,
    pub default_tempo: Tempo,
    /// `recording` or `script`.
    pub kind: &'static str,
}

fn read_text(path: &Path) -> Result<String, FsError> {
    let meta = std::fs::metadata(path).map_err(|e| FsError::from_io(&e, path))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(FsError::new(
            FsErrorCode::FileTooLarge,
            "too large",
            Some(path),
        ));
    }
    std::fs::read_to_string(path).map_err(|e| FsError::from_io(&e, path))
}

/// Is this a script (by extension or because it is no JSON)?
fn is_script(path: &Path, text: &str) -> bool {
    path.extension().is_some_and(|e| e == "edscript") || !text.trim_start().starts_with('{')
}

/// Reads a recording (`*.edrec.jsonl`) or a script (`*.edscript`; `type @file` relative to it).
pub fn load_path(path: &Path) -> Result<Loaded, FsError> {
    let text = read_text(path)?;
    if is_script(path, &text) {
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut read = |f: &str| {
            let p = dir.join(f);
            read_text(&p)
        };
        let items = script::parse(&text, &mut read).map_err(|mut e| {
            e.path = Some(path.to_string_lossy().into_owned());
            e
        })?;
        Ok(Loaded {
            items,
            recording: None,
            default_tempo: Tempo::Original,
            kind: "script",
        })
    } else {
        let rec = Recording::from_jsonl(&text)?;
        Ok(Loaded {
            items: playback::items_of(&rec, true)?,
            recording: Some(rec),
            default_tempo: Tempo::Original,
            kind: "recording",
        })
    }
}

pub fn load(source: &Source, session: &Session) -> Result<Loaded, FsError> {
    match source {
        Source::Path { path } => load_path(&PathBuf::from(path)),
        Source::Last => {
            let rec = session
                .last_recording()
                .ok_or_else(|| FsError::new(FsErrorCode::NoRecording, "no recording", None))?
                .clone();
            Ok(Loaded {
                items: playback::items_of(&rec, false)?,
                // a macro: the text check does not apply
                recording: None,
                default_tempo: Tempo::Instant,
                kind: "recording",
            })
        }
    }
}

pub fn check(loaded: &Loaded, session: &Session) -> PlaybackCheck {
    let steps = playback::timeline(&loaded.items, loaded.default_tempo);
    let other_text = loaded.recording.as_ref().is_some_and(|r| {
        let now = recording::stamp(&session.doc);
        now.hash != r.header.doc.hash || now.lines != r.header.doc.lines
    });
    PlaybackCheck {
        kind: loaded.kind.into(),
        steps: steps.len(),
        duration_ms: steps.last().map_or(0, |s| s.at),
        other_text,
    }
}

/// Saves the last recording as `*.edrec.jsonl` or (`script`) as `*.edscript`.
pub fn save(session: &Session, path: &Path, as_script: bool) -> Result<(), FsError> {
    let rec = session
        .last_recording()
        .ok_or_else(|| FsError::new(FsErrorCode::NoRecording, "no recording", None))?;
    let text = if as_script {
        script::from_recording(rec)
    } else {
        rec.to_jsonl()
    };
    crate::fs_atomic::write_atomic(path, text.as_bytes()).map_err(|e| FsError::from_io(&e, path))
}

/// Playing, paused or stopped – set by `playback_control`.
#[derive(Debug, Default)]
pub struct Control {
    state: AtomicU8,
    step: AtomicBool,
}

const RUN: u8 = 0;
const PAUSE: u8 = 1;
const STOP: u8 = 2;

impl Control {
    pub fn pause(&self) {
        self.state.store(PAUSE, Ordering::SeqCst);
    }
    pub fn resume(&self) {
        self.state.store(RUN, Ordering::SeqCst);
    }
    pub fn stop(&self) {
        self.state.store(STOP, Ordering::SeqCst);
    }
    /// One step while paused.
    pub fn step(&self) {
        self.pause();
        self.step.store(true, Ordering::SeqCst);
    }
}

/// Plays `steps` in real time; `send` gets frames, progress and the end. Returns when done.
pub fn run(
    handle: Handle<Session>,
    steps: Vec<playback::Step>,
    control: Arc<Control>,
    send: impl Fn(PlaybackEvent),
) {
    let total = steps.len();
    let mut p = Playback::new(steps);
    p.begin(&mut lock_doc(&handle));
    let started = Instant::now();
    let mut paused_for = Duration::ZERO;
    let mut paused_at: Option<Instant> = None;
    let mut last_progress = Instant::now() - Duration::from_secs(1);
    let mut reason = "finished".to_string();
    loop {
        let state = control.state.load(Ordering::SeqCst);
        if state == STOP {
            reason = "stopped".into();
            break;
        }
        if state == PAUSE {
            if paused_at.is_none() {
                paused_at = Some(Instant::now());
                send(PlaybackEvent::Progress {
                    done: p.next,
                    total,
                    paused: true,
                });
            }
            if control.step.swap(false, Ordering::SeqCst) && !p.done() {
                let r = p.step(&mut lock_doc(&handle));
                match r {
                    Ok(f) => send(PlaybackEvent::Frame { frame: Box::new(f) }),
                    Err(e) => {
                        reason = code_of(&e);
                        break;
                    }
                }
                send(PlaybackEvent::Progress {
                    done: p.next,
                    total,
                    paused: true,
                });
            }
            if p.done() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        if let Some(at) = paused_at.take() {
            paused_for += at.elapsed();
            // continue right where it stopped: the next step is due now
            if let Some(next) = p.next_at() {
                let elapsed = started.elapsed().saturating_sub(paused_for).as_millis() as u64;
                paused_for += Duration::from_millis(elapsed.saturating_sub(next));
            }
        }
        let Some(next) = p.next_at() else { break };
        let elapsed = started.elapsed().saturating_sub(paused_for).as_millis() as u64;
        if next > elapsed {
            std::thread::sleep(Duration::from_millis((next - elapsed).min(30)));
            continue;
        }
        let r = p.run_due(&mut lock_doc(&handle), elapsed);
        match r {
            Ok(Some(f)) => send(PlaybackEvent::Frame { frame: Box::new(f) }),
            Ok(None) => {}
            Err(e) => {
                reason = code_of(&e);
                break;
            }
        }
        if last_progress.elapsed() > Duration::from_millis(100) || p.done() {
            last_progress = Instant::now();
            send(PlaybackEvent::Progress {
                done: p.next,
                total,
                paused: false,
            });
        }
    }
    {
        let mut s = lock_doc(&handle);
        p.end(&mut s);
        if let Ok(f) = s.run(
            crate::editor::model::Command::Frame,
            &mut crate::editor::MemoryClipboard::default(),
        ) {
            send(PlaybackEvent::Frame { frame: Box::new(f) });
        }
    }
    send(PlaybackEvent::Done { reason });
}

fn code_of(e: &FsError) -> String {
    serde_json::to_value(e.code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "io".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::document::registry::Registry;
    use crate::editor::MemoryClipboard;
    use crate::editor::model::Command;
    use std::sync::Mutex;

    fn handle(text: &str) -> (Arc<Registry<Session>>, Handle<Session>) {
        let reg = Arc::new(Registry::default());
        reg.insert(1, Session::new(Document::from_text(text), 1));
        let h = reg.get(1).unwrap();
        (reg, h)
    }

    #[test]
    fn plays_a_script_file_with_a_snippet_in_real_time() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("snip.txt"), "fn x() {}").unwrap();
        let path = dir.path().join("a.edscript");
        std::fs::write(
            &path,
            "tempo even 5\ntype \"ab\"\npause 10\ntype @snip.txt\n",
        )
        .unwrap();
        let (_reg, h) = handle("");
        let loaded = load_path(&path).unwrap();
        let c = check(&loaded, &lock_doc(&h));
        assert_eq!((c.kind.as_str(), c.other_text), ("script", false));
        let steps = playback::timeline(&loaded.items, loaded.default_tempo);
        let events = Mutex::new(Vec::new());
        run(h.clone(), steps, Arc::new(Control::default()), |e| {
            events.lock().unwrap().push(e)
        });
        assert_eq!(lock_doc(&h).doc.rope().to_string(), "abfn x() {}");
        let ev = events.into_inner().unwrap();
        assert!(matches!(ev.last(), Some(PlaybackEvent::Done { reason }) if reason == "finished"));
        assert!(ev.iter().any(|e| matches!(e, PlaybackEvent::Frame { .. })));
        // one undo step
        lock_doc(&h)
            .run(Command::Undo, &mut MemoryClipboard::default())
            .unwrap();
        assert_eq!(lock_doc(&h).doc.rope().to_string(), "");
    }

    #[test]
    fn stop_ends_early_and_saved_recordings_load_again() {
        let (_reg, h) = handle("text");
        {
            let mut s = lock_doc(&h);
            s.start_recording("t");
            for c in ["a", "b", "c"] {
                s.run(
                    Command::Type { text: c.into() },
                    &mut MemoryClipboard::default(),
                )
                .unwrap();
            }
            s.stop_recording();
        }
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("r.edrec.jsonl");
        save(&lock_doc(&h), &file, false).unwrap();
        let loaded = load_path(&file).unwrap();
        // the text changed since: other_text
        assert!(check(&loaded, &lock_doc(&h)).other_text);
        let script_file = dir.path().join("r.edscript");
        save(&lock_doc(&h), &script_file, true).unwrap();
        assert!(read_text(&script_file).unwrap().contains("type \"abc\""));
        let control = Arc::new(Control::default());
        control.stop();
        let steps = playback::timeline(&loaded.items, Tempo::Even { ms: 1000 });
        let done = Mutex::new(None);
        run(h.clone(), steps, control, |e| {
            if let PlaybackEvent::Done { reason } = e {
                *done.lock().unwrap() = Some(reason);
            }
        });
        assert_eq!(done.into_inner().unwrap().as_deref(), Some("stopped"));
    }

    #[test]
    fn no_recording_is_an_error() {
        let (_reg, h) = handle("");
        let e = load(&Source::Last, &lock_doc(&h)).err().unwrap();
        assert_eq!(e.code, FsErrorCode::NoRecording);
    }
}
