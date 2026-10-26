//! The editlicious engine in the browser (prompt 011): the same `ed-core` session as the app,
//! commands and frames as JSON – the demo page only renders, like the app's editor view.

use ed_core::document::Document;
use ed_core::editor::model::Command;
use ed_core::editor::{MemoryClipboard, Session};
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
}
