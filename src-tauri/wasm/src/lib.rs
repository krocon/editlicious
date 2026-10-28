//! The editlicious engine in the browser (prompt 011): the same `ed-core` session as the app,
//! commands and frames as JSON – the demo page only renders, like the app's editor view.

use ed_core::document::Document;
use ed_core::editor::model::Command;
use ed_core::editor::playback::{Step, Tempo, timeline};
use ed_core::editor::script;
use ed_core::editor::{Clipboard, MemoryClipboard, Session};
use ed_core::error::FsError;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Editor {
    session: Session,
    clip: MemoryClipboard,
}

#[wasm_bindgen]
impl Editor {
    /// A session on `text`; `language` = name of a grammar (`None`: plain text).
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str, language: Option<String>) -> Editor {
        let mut session = Session::new(Document::from_text(text), 1);
        let mut clip = MemoryClipboard::default();
        let _ = session.run(Command::SetLanguage { name: language }, &mut clip);
        Editor { session, clip }
    }

    /// Runs one command (JSON, like `editor_command` in the app) and returns the frame (JSON).
    pub fn command(&mut self, cmd: &str) -> Result<String, String> {
        let cmd: Command = serde_json::from_str(cmd).map_err(|e| e.to_string())?;
        let frame = self
            .session
            .run(cmd, &mut self.clip)
            .map_err(|e| format!("{:?}", e.code))?;
        serde_json::to_string(&frame).map_err(|e| e.to_string())
    }

    /// What the last copy/cut put on the (in-memory) clipboard.
    pub fn clipboard(&self) -> Option<String> {
        self.clip.0.clone()
    }

    /// Text the page pasted from the system clipboard.
    pub fn set_clipboard(&mut self, text: String) {
        self.clip.0 = Some(text);
    }

    /// The whole text (the demo is small).
    pub fn text(&self) -> String {
        self.session.doc.rope().to_string()
    }
}

/// Steps of a typing script (`*.edscript`, docs/scripts.md) – `type @file` is not available here.
fn script_steps(text: &str) -> Result<Vec<Step>, String> {
    let mut no_files = |f: &str| -> Result<String, FsError> {
        Err(FsError::new(
            ed_core::error::FsErrorCode::RecordingInvalid,
            format!("files are not available here: {f}"),
            None,
        ))
    };
    let items = script::parse(text, &mut no_files).map_err(|e| e.message)?;
    Ok(timeline(&items, Tempo::Original))
}

/// How long a typing script takes (ms) – to place it so that it ends on a spoken word.
#[wasm_bindgen(js_name = scriptDuration)]
pub fn script_duration(text: &str) -> Result<f64, String> {
    Ok(script_steps(text)?.last().map_or(0, |s| s.at) as f64)
}

/// A typing script on a virtual clock (screen videos, e.g. a HyperFrames composition): the page
/// asks for the frame at any time and gets the same picture every time – the engine replays the
/// steps up to that moment (backwards: from the start text). Several scripts can follow each
/// other, each from its own start time.
#[wasm_bindgen]
pub struct Player {
    start: String,
    language: Option<String>,
    /// Commands run after every reset (view size, settings).
    setup: Vec<Command>,
    steps: Vec<Step>,
    session: Session,
    clip: MemoryClipboard,
    /// Steps already run on `session`.
    next: usize,
}

#[wasm_bindgen]
impl Player {
    /// A player on `text`; `language` = name of a grammar (`None`: plain text).
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str, language: Option<String>) -> Player {
        let mut p = Player {
            start: text.to_string(),
            language,
            setup: Vec::new(),
            steps: Vec::new(),
            session: Session::new(Document::from_text(""), 1),
            clip: MemoryClipboard::default(),
            next: 0,
        };
        p.reset();
        p
    }

    fn reset(&mut self) {
        self.session = Session::new(Document::from_text(&self.start), 1);
        self.clip = MemoryClipboard::default();
        let _ = self.session.run(
            Command::SetLanguage {
                name: self.language.clone(),
            },
            &mut self.clip,
        );
        for c in self.setup.clone() {
            let _ = self.session.run(c, &mut self.clip);
        }
        self.next = 0;
    }

    /// A command that also holds after every jump back (JSON, e.g. `resize`, `configure`).
    pub fn setup(&mut self, cmd: &str) -> Result<(), String> {
        let cmd: Command = serde_json::from_str(cmd).map_err(|e| e.to_string())?;
        self.setup.push(cmd);
        self.reset();
        Ok(())
    }

    /// Adds a script that starts at `start_ms` (not before the end of the one before);
    /// returns when it ends (ms).
    pub fn add(&mut self, text: &str, start_ms: f64) -> Result<f64, String> {
        let steps = script_steps(text)?;
        let from = (start_ms.max(0.0).round() as u64).max(self.duration() as u64);
        self.steps.extend(steps.into_iter().map(|mut s| {
            s.at += from;
            s
        }));
        self.reset();
        Ok(self.duration())
    }

    /// End of the last step (ms).
    pub fn duration(&self) -> f64 {
        self.steps.last().map_or(0, |s| s.at) as f64
    }

    /// The frame at `ms` (JSON) – deterministic, whatever was asked before.
    pub fn seek(&mut self, ms: f64) -> Result<String, String> {
        let ms = ms.max(0.0) as u64;
        if self.next > 0 && self.steps[self.next - 1].at > ms {
            self.reset();
        }
        while let Some(s) = self.steps.get(self.next)
            && s.at <= ms
        {
            let s = s.clone();
            self.next += 1;
            if let Some(c) = s.clip {
                self.clip.set(c);
            }
            // a step that does not apply (e.g. nothing to undo) must not stop the video
            let _ = self.session.run(s.cmd, &mut self.clip);
        }
        let frame = self
            .session
            .run(Command::Frame, &mut self.clip)
            .map_err(|e| format!("{:?}", e.code))?;
        serde_json::to_string(&frame).map_err(|e| e.to_string())
    }

    /// The whole text at the moment of the last `seek`.
    pub fn text(&self) -> String {
        self.session.doc.rope().to_string()
    }

    /// The text after all scripts (to compare with the expected file).
    #[wasm_bindgen(js_name = finalText)]
    pub fn final_text(&mut self) -> String {
        let _ = self.seek(f64::MAX);
        self.text()
    }

    /// The steps as `[{ "at": ms, "kind": "char" | "space" | "enter" | "delete" | "move" | "other",
    /// "keys": caption? }]` – for key sounds and captions in a video.
    pub fn events(&self) -> String {
        let ev: Vec<serde_json::Value> = self
            .steps
            .iter()
            .map(|s| {
                let kind = match &s.cmd {
                    Command::Type { text } if text == "\n" => "enter",
                    Command::Type { text } if text.trim().is_empty() => "space",
                    Command::Type { .. } | Command::Insert { .. } => "char",
                    Command::Backspace
                    | Command::Delete
                    | Command::DeleteWordLeft
                    | Command::DeleteWordRight => "delete",
                    Command::Move { .. } | Command::GoTo { .. } | Command::Select { .. } => "move",
                    Command::Frame if s.overlay.is_some() => "keys",
                    _ => "other",
                };
                serde_json::json!({ "at": s.at, "kind": kind, "keys": s.overlay })
            })
            .collect();
        serde_json::Value::Array(ev).to_string()
    }
}

/// Names of all grammars (JSON array).
#[wasm_bindgen]
pub fn languages() -> String {
    serde_json::to_string(&ed_core::highlight::languages()).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_in_frames_out() {
        let mut e = Editor::new("fn main() {}\n", Some("Rust".into()));
        let f = e
            .command(r#"{"type":"resize","lines":10,"cols":40}"#)
            .unwrap();
        assert!(f.contains("\"language\":\"Rust\""));
        e.command(r#"{"type":"type","text":"// hi\n"}"#).unwrap();
        assert!(e.text().starts_with("// hi\n"));
        e.command(r#"{"type":"selectAll"}"#).unwrap();
        e.command(r#"{"type":"copy"}"#).unwrap();
        assert!(e.clipboard().unwrap().contains("fn main"));
        assert!(e.command("nonsense").is_err());
        assert!(languages().contains("Rust"));
    }

    #[test]
    fn player_is_deterministic_on_a_virtual_clock() {
        let script =
            "tempo human cpm=300 seed=7 typos=0.05\ngoto 2:1\ntype \"  white-space: nowrap;\\n\"";
        let d = script_duration(script).unwrap();
        assert!(d > 1000.0, "{d}");
        let mut p = Player::new(".card-title {\n}\n", Some("CSS".into()));
        p.setup(r#"{"type":"resize","lines":12,"cols":40}"#)
            .unwrap();
        let end = p.add(script, 2000.0).unwrap();
        assert_eq!(end, 2000.0 + d);
        assert_eq!(p.final_text(), ".card-title {\n  white-space: nowrap;\n}\n");
        // forwards and backwards give the same picture
        let mid = p.seek(2000.0 + d / 2.0).unwrap();
        p.seek(end).unwrap();
        p.seek(0.0).unwrap();
        assert_eq!(p.text(), ".card-title {\n}\n");
        assert_eq!(
            p.seek(2000.0 + d / 2.0)
                .unwrap()
                .replace(char::is_numeric, ""),
            mid.replace(char::is_numeric, "")
        );
        // a second script never starts before the first ends
        let end2 = p.add("type \"x\"", 0.0).unwrap();
        assert!(end2 > end);
        let ev: serde_json::Value = serde_json::from_str(&p.events()).unwrap();
        assert!(ev.as_array().unwrap().iter().any(|e| e["kind"] == "move"));
        assert!(ev.as_array().unwrap().iter().any(|e| e["kind"] == "enter"));
        assert!(script_duration("nonsense").is_err());
    }
}
