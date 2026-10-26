//! DTOs of the document model (design 002). Mirror: `src/app/core/document/document.models.ts`.

use serde::{Deserialize, Serialize};

/// A position: line (0-based) and `ch` = Unicode scalar values into the line, without the line
/// break (design 002 E2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Pos {
    pub line: usize,
    pub ch: usize,
}

impl Pos {
    pub fn new(line: usize, ch: usize) -> Self {
        Self { line, ch }
    }
}

/// A selection; `anchor == head` is a plain caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Sel {
    pub anchor: Pos,
    pub head: Pos,
}

impl Sel {
    pub fn caret(p: Pos) -> Self {
        Self { anchor: p, head: p }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineEnding {
    Lf,
    Crlf,
    Cr,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }
}

/// What the frontend knows about an open document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocInfo {
    pub id: u64,
    /// `None` for a new, never saved document.
    pub path: Option<String>,
    /// File name (or "Untitled") for tabs and the title.
    pub name: String,
    /// Name of the encoding as in the Encoding Standard (`UTF-8`, `UTF-16LE`, `windows-1252` …).
    pub encoding: String,
    pub bom: bool,
    pub line_ending: LineEnding,
    pub mixed_line_endings: bool,
    /// Characters were replaced by U+FFFD while decoding – saving would not give the old bytes back.
    pub decode_errors: bool,
    pub line_count: usize,
    pub char_count: usize,
    /// Size of the file on disk when loaded/saved (0 for new documents).
    pub file_size: u64,
    pub revision: u64,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// Which part of the document the view wants (design 002 E3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewRequest {
    pub first_line: usize,
    pub line_count: usize,
    /// Only for lines longer than `FULL_LINE_LIMIT`: first character of the window.
    #[serde(default)]
    pub window_from: usize,
    /// Only for long lines: number of characters of the window.
    #[serde(default)]
    pub window_count: usize,
}

/// One line of the view: `text` starts at character `from` of a line with `len` characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewLine {
    pub text: String,
    pub from: usize,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub revision: u64,
    pub line_count: usize,
    pub first_line: usize,
    pub lines: Vec<ViewLine>,
}

/// One change in coordinates **before** the whole edit (design 002 E6).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Change {
    pub from: Pos,
    pub to: Pos,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditKind {
    /// Typing – consecutive keystrokes become one undo step (design 002 E7).
    Type,
    /// Backspace/Delete – consecutive ones become one undo step.
    Delete,
    /// Everything else: paste, commands, … – always its own step.
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequest {
    pub changes: Vec<Change>,
    #[serde(default)]
    pub kind: EditKind,
    #[serde(default)]
    pub sel_before: Vec<Sel>,
    /// The selections after the edit; computed (caret behind each insert) when missing.
    #[serde(default)]
    pub sel_after: Option<Vec<Sel>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditResult {
    pub info: DocInfo,
    pub selections: Vec<Sel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SaveRequest {
    /// Save under this path ("Save as"); `None` = the document's own path.
    #[serde(default)]
    pub path: Option<String>,
    /// Save in this encoding (e.g. after "unmappable character": UTF-8).
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub bom: Option<bool>,
    /// Save even though the file changed on disk (the user said so).
    #[serde(default)]
    pub force: bool,
}

/// Progress of `open_document` (design 002 E4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LoadEvent {
    Progress { loaded: u64, total: u64 },
}
