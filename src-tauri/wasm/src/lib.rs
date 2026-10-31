//! The editlicious engine in the browser (prompt 011): the same `ed-core` session as the app,
//! commands and frames as JSON – the demo page only renders, like the app's editor view.

use ed_core::document::Document;
use ed_core::editor::model::Command;
use ed_core::editor::playback::{Step, Tempo, timeline};
use ed_core::editor::script;
use ed_core::editor::{Clipboard, Cursor, MemoryClipboard, Session};
use ed_core::error::FsError;
use std::sync::atomic::AtomicBool;
use wasm_bindgen::prelude::*;

/// An error for JavaScript: the `FsError` as JSON (`{ code, message, path }`, design 025 E2).
fn js_err(e: FsError) -> String {
    serde_json::to_string(&e).unwrap_or_else(|_| format!("{:?}", e.code))
}

fn invalid(message: impl Into<String>) -> String {
    js_err(FsError::new(
        ed_core::error::FsErrorCode::InvalidRequest,
        message.into(),
        None,
    ))
}

/// The clipboard of a page: text and – after „copy as HTML“ – the HTML of the last copy; the
/// page puts both on the system clipboard (design 025 E6).
#[derive(Debug, Default)]
pub struct WebClipboard {
    text: Option<String>,
    html: Option<String>,
}

impl Clipboard for WebClipboard {
    fn get(&mut self) -> Option<String> {
        self.text.clone()
    }
    fn set(&mut self, text: String) -> bool {
        self.text = Some(text);
        self.html = None;
        true
    }
    fn set_html(&mut self, html: String, alt: String) -> bool {
        self.text = Some(alt);
        self.html = Some(html);
        true
    }
}

/// What a language setting means (design 025 E3): a grammar, or a (virtual) file name whose
/// extension picks the grammar and the word lexicon. Both `None` = plain text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LanguageSpec {
    pub grammar: Option<String>,
    pub file_name: Option<String>,
}

/// `"Rust"`, `"rust"` → the grammar; `"TypeScript"`, `"Kotlin"` (no grammar of their own) and
/// `"app.ts"`, `".kt"` → a file name; empty or unknown → plain text.
pub fn resolve_language(spec: &str) -> LanguageSpec {
    let spec = spec.trim();
    if spec.is_empty() {
        return LanguageSpec::default();
    }
    if let Some(g) = ed_core::highlight::languages()
        .into_iter()
        .find(|g| g.eq_ignore_ascii_case(spec))
    {
        return LanguageSpec {
            grammar: Some(g),
            file_name: None,
        };
    }
    let file = match spec.to_lowercase().as_str() {
        "typescript" | "ts" | "angular" => Some("editor.ts".to_owned()),
        "kotlin" | "kt" => Some("editor.kt".to_owned()),
        "javascript" | "js" => Some("editor.js".to_owned()),
        _ if spec.starts_with('.') && spec.len() > 1 => Some(format!("editor{spec}")),
        _ if spec.contains('.') => Some(spec.to_owned()),
        _ => None,
    };
    match file {
        Some(f) => LanguageSpec {
            grammar: ed_core::highlight::detect(Some(&f), ""),
            file_name: Some(f),
        },
        None => LanguageSpec::default(),
    }
}

#[wasm_bindgen]
pub struct Editor {
    session: Session,
    clip: WebClipboard,
}

#[wasm_bindgen]
impl Editor {
    /// A session on `text`; `language` = grammar name, alias or file name (design 025 E3),
    /// `None`: plain text.
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str, language: Option<String>) -> Editor {
        let session = Session::new(Document::from_text(text), 1);
        let mut e = Editor {
            session,
            clip: WebClipboard::default(),
        };
        e.set_language(language.as_deref().unwrap_or(""));
        e
    }

    /// Runs one command (JSON, like `editor_command` in the app) and returns the frame (JSON);
    /// errors are `FsError` JSON.
    pub fn command(&mut self, cmd: &str) -> Result<String, String> {
        let cmd: Command = serde_json::from_str(cmd).map_err(|e| invalid(e.to_string()))?;
        let frame = self.session.run(cmd, &mut self.clip).map_err(js_err)?;
        serde_json::to_string(&frame).map_err(|e| e.to_string())
    }

    /// The language by grammar name, alias or (virtual) file name (design 025 E3).
    #[wasm_bindgen(js_name = setLanguage)]
    pub fn set_language(&mut self, spec: &str) {
        let s = resolve_language(spec);
        self.session.set_file_name(s.file_name.as_deref());
        if s.file_name.is_none() {
            let _ = self.session.run(
                Command::SetLanguage { name: s.grammar },
                &mut MemoryClipboard::default(),
            );
        }
    }

    /// The language shown (`undefined` = plain text).
    pub fn language(&self) -> Option<String> {
        self.session.language().map(str::to_owned)
    }

    /// The document info (JSON `DocInfo`).
    pub fn info(&self) -> String {
        serde_json::to_string(&self.session.doc.info(self.session.id)).unwrap_or_default()
    }

    /// Replaces the whole text (a new document: clean, no undo history); language, settings and
    /// read-only stay, the caret goes to the start.
    #[wasm_bindgen(js_name = setText)]
    pub fn set_text(&mut self, text: &str) {
        let mut doc = Document::from_text(text);
        doc.path = self.session.doc.path.clone();
        self.session.replace_document(doc);
        self.session.set_cursors(vec![Cursor::default()]);
        let _ = self.session.run(
            Command::ScrollTo {
                top: Some(0.0),
                left: Some(0.0),
            },
            &mut MemoryClipboard::default(),
        );
    }

    /// What the last copy/cut put on the clipboard (text).
    pub fn clipboard(&self) -> Option<String> {
        self.clip.text.clone()
    }

    /// The HTML of the last „copy as HTML“ (`undefined` after a plain copy).
    #[wasm_bindgen(js_name = clipboardHtml)]
    pub fn clipboard_html(&self) -> Option<String> {
        self.clip.html.clone()
    }

    /// Text the page pasted from the system clipboard.
    #[wasm_bindgen(js_name = setClipboard)]
    pub fn set_clipboard(&mut self, text: String) {
        self.clip.text = Some(text);
        self.clip.html = None;
    }

    /// The whole text – only on request (`getValue()`), never per keystroke.
    pub fn text(&self) -> String {
        self.session.doc.rope().to_string()
    }

    /// Lines `from..to` (0-based, end exclusive) with their line breaks.
    pub fn slice(&self, from: u32, to: u32) -> String {
        let rope = self.session.doc.rope();
        let n = rope.len_lines();
        let a = (from as usize).min(n);
        let b = (to as usize).clamp(a, n);
        rope.slice(rope.line_to_char(a)..rope.line_to_char(b))
            .to_string()
    }

    /// All matches of a query (JSON `Query`) and the number of the selected one (JSON
    /// `MatchCount`), counted like the app's `count_matches`.
    #[wasm_bindgen(js_name = countMatches)]
    pub fn count_matches(&self, query: &str) -> Result<String, String> {
        let query: ed_core::search::Query =
            serde_json::from_str(query).map_err(|e| invalid(e.to_string()))?;
        let re = ed_core::search::compile(&query).map_err(js_err)?;
        let (rope, start, end) = self.session.search_snapshot();
        let c = ed_core::search::match_count(&rope, &re, start, end, &AtomicBool::new(false))
            .unwrap_or(ed_core::search::MatchCount {
                total: 0,
                current: 0,
            });
        serde_json::to_string(&c).map_err(|e| e.to_string())
    }

    /// How often the marked word at the caret occurs (`undefined` = no marked word there).
    #[wasm_bindgen(js_name = countMarker)]
    pub fn count_marker(&self) -> Option<u32> {
        let (rope, re) = self.session.marker_snapshot()?;
        ed_core::search::count(&rope, &re, 0, &AtomicBool::new(false)).map(|(n, _)| n as u32)
    }

    /// Matches (bit 1) and markers (bit 2) per slice of the lines (design 010 E3).
    pub fn overview(&self, buckets: u32) -> Vec<u8> {
        let (rope, layers) = self.session.overview_snapshot();
        if layers.is_empty() {
            return Vec::new();
        }
        ed_core::search::overview(&rope, &layers, buckets as usize, &AtomicBool::new(false))
            .unwrap_or_default()
    }
}

/// Encodings the app offers (JSON array) – in the browser only for display.
#[wasm_bindgen]
pub fn encodings() -> String {
    serde_json::to_string(&ed_core::document::ENCODINGS).unwrap_or_else(|_| "[]".into())
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
    fn info_text_and_slices() {
        let mut e = Editor::new("a\nb\nc\n", None);
        let info: serde_json::Value = serde_json::from_str(&e.info()).unwrap();
        assert_eq!(info["lineCount"], 4);
        assert_eq!(info["dirty"], false);
        assert_eq!(e.slice(1, 3), "b\nc\n");
        assert_eq!(e.slice(2, 99), "c\n");
        assert_eq!(e.slice(9, 1), "");
        e.command(r#"{"type":"type","text":"x"}"#).unwrap();
        e.set_text("new\ntext");
        assert_eq!(e.text(), "new\ntext");
        let info: serde_json::Value = serde_json::from_str(&e.info()).unwrap();
        assert_eq!(info["dirty"], false);
        assert_eq!(info["lineCount"], 2);
        // no undo back into the old text
        e.command(r#"{"type":"undo"}"#).unwrap();
        assert_eq!(e.text(), "new\ntext");
    }

    #[test]
    fn counts_and_overview() {
        let mut e = Editor::new("beta\nalpha\nbeta\n", None);
        e.command(r#"{"type":"resize","lines":10,"cols":40}"#)
            .unwrap();
        let q = r#"{"pattern":"beta","regex":false,"caseSensitive":false,"wholeWord":false}"#;
        e.command(&format!(r#"{{"type":"find","query":{q}}}"#))
            .unwrap();
        let c: serde_json::Value = serde_json::from_str(&e.count_matches(q).unwrap()).unwrap();
        assert_eq!(c["total"], 2);
        assert_eq!(c["current"], 1);
        let bad = r#"{"pattern":"(","regex":true,"caseSensitive":false,"wholeWord":false}"#;
        let err: serde_json::Value =
            serde_json::from_str(&e.count_matches(bad).unwrap_err()).unwrap();
        assert_eq!(err["code"], "invalid_search");
        let o = e.overview(3);
        assert_eq!(o.len(), 3);
        assert!(o.iter().any(|b| b & 1 == 1));
        assert_eq!(e.count_marker(), None);
        e.command(r#"{"type":"goTo","line":0,"ch":1}"#).unwrap();
        e.command(r#"{"type":"toggleMarker"}"#).unwrap();
        assert_eq!(e.count_marker(), Some(2));
        assert!(encodings().contains("UTF-8"));
    }

    #[test]
    fn copy_as_html_keeps_text_and_html() {
        let mut e = Editor::new("fn main() {}\n", Some("Rust".into()));
        e.command(r#"{"type":"selectAll"}"#).unwrap();
        e.command(r#"{"type":"copyHtml","lineNumbers":false,"styles":[],"base":"color:red","gutter":"color:gray"}"#)
            .unwrap();
        assert!(e.clipboard_html().unwrap().contains("<"));
        assert!(e.clipboard().unwrap().contains("fn main"));
        e.command(r#"{"type":"copy"}"#).unwrap();
        assert_eq!(e.clipboard_html(), None);
    }

    #[test]
    fn language_by_name_alias_or_file_name() {
        assert_eq!(resolve_language("rust").grammar.as_deref(), Some("Rust"));
        assert_eq!(resolve_language("").grammar, None);
        assert_eq!(resolve_language("Klingon"), LanguageSpec::default());
        let ts = resolve_language("TypeScript");
        assert_eq!(ts.file_name.as_deref(), Some("editor.ts"));
        assert_eq!(ts.grammar.as_deref(), Some("JavaScript"));
        assert_eq!(
            resolve_language(".kt").file_name.as_deref(),
            Some("editor.kt")
        );
        assert_eq!(
            resolve_language("app.ts").file_name.as_deref(),
            Some("app.ts")
        );
        let mut e = Editor::new("", Some("TypeScript".into()));
        assert_eq!(e.language().as_deref(), Some("JavaScript"));
        // the TypeScript lexicon (Angular words) completes
        e.command(r#"{"type":"resize","lines":10,"cols":40}"#)
            .unwrap();
        e.command(r#"{"type":"type","text":"ngOnIn"}"#).unwrap();
        e.command(r#"{"type":"complete"}"#).unwrap();
        assert_eq!(e.text(), "ngOnInit");
        e.set_language("Rust");
        assert_eq!(e.language().as_deref(), Some("Rust"));
        e.set_language("");
        assert_eq!(e.language(), None);
    }

    #[test]
    fn read_only_refuses_edits() {
        let mut e = Editor::new("abc", None);
        e.command(r#"{"type":"setReadOnly","readOnly":true}"#)
            .unwrap();
        let err: serde_json::Value =
            serde_json::from_str(&e.command(r#"{"type":"type","text":"x"}"#).unwrap_err()).unwrap();
        assert_eq!(err["code"], "read_only");
        e.command(r#"{"type":"selectAll"}"#).unwrap();
        e.command(r#"{"type":"copy"}"#).unwrap();
        assert_eq!(e.clipboard().as_deref(), Some("abc"));
        assert_eq!(e.text(), "abc");
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
