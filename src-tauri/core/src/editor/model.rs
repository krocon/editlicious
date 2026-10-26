//! Commands and frames of the editor session (design 003). Mirror:
//! `src/app/core/editor/editor.models.ts`.

use crate::document::model::DocInfo;
use serde::{Deserialize, Serialize};

/// What the UI asks the session to do. Positions from the mouse come as line + cell (float:
/// the click may be anywhere in a cell).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    /// Only render (after opening, after a theme change …).
    Frame,
    Resize {
        lines: f64,
        cols: f64,
    },
    Configure {
        tab_width: Option<usize>,
        insert_spaces: Option<bool>,
    },
    Scroll {
        lines: f64,
        cells: f64,
    },
    ScrollTo {
        top: Option<f64>,
        left: Option<f64>,
    },
    Move {
        to: Motion,
        #[serde(default)]
        select: bool,
    },
    SelectAll,
    /// Puts the caret at a position (session restore, "go to line").
    GoTo {
        line: usize,
        ch: usize,
        #[serde(default)]
        select: bool,
    },
    Click {
        line: f64,
        cell: f64,
        /// 1 = place caret, 2 = word, 3 = line.
        #[serde(default = "one")]
        count: u8,
        /// Shift-click: extend the selection.
        #[serde(default)]
        extend: bool,
    },
    Drag {
        line: f64,
        cell: f64,
    },
    /// Typed text (one keystroke or a finished IME composition).
    Type {
        text: String,
    },
    /// Text to insert as one step (paste from the UI, drop).
    Insert {
        text: String,
    },
    Newline,
    Backspace,
    Delete,
    DeleteWordLeft,
    DeleteWordRight,
    Indent,
    Outdent,
    ToggleOverwrite,
    Undo,
    Redo,
    Copy,
    Cut,
    /// Paste from the system clipboard (read in Rust – nothing big crosses the bridge).
    Paste,
}

fn one() -> u8 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    /// Smart: first non-blank, then column 0.
    LineStart,
    LineEnd,
    PageUp,
    PageDown,
    DocStart,
    DocEnd,
}

/// Kind of a run of a frame line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RunKind {
    /// Characters of one cell each (combining marks included).
    Text,
    /// Characters of two cells each.
    Wide,
    /// A tab (`text` is empty, `width` cells).
    Tab,
    /// Control characters, drawn as control pictures (`text` holds the pictures).
    Ctrl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Run {
    /// First cell (absolute in the line).
    pub cell: usize,
    pub width: usize,
    pub kind: RunKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameLine {
    pub line: usize,
    pub runs: Vec<Run>,
    /// Selected cell ranges `[from, to)` of this line (absolute cells).
    pub sel: Vec<[usize; 2]>,
    /// The selection goes on past the end of the line (draw the line break selected).
    pub sel_eol: bool,
    /// Width of the line in cells.
    pub width: usize,
    /// Long line (design 002 E3): one cell per character, shown as a window.
    pub long: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CaretPos {
    pub line: usize,
    pub cell: usize,
}

/// Everything the UI needs to draw the editor (design 003 E1).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub seq: u64,
    pub info: DocInfo,
    /// Scroll position: first visible line (fraction = part of a line scrolled away).
    pub top: f64,
    /// Scroll position in cells.
    pub left: f64,
    pub first_line: usize,
    pub lines: Vec<FrameLine>,
    /// Widest line seen so far (horizontal scroll range).
    pub max_width: usize,
    /// Carets; the first one is the primary (status bar, IME).
    pub carets: Vec<CaretPos>,
    /// Primary caret as line/character (0-based) for the status bar.
    pub caret_line: usize,
    pub caret_ch: usize,
    pub sel_chars: usize,
    pub sel_lines: usize,
    pub overwrite: bool,
    pub tab_width: usize,
    pub insert_spaces: bool,
    /// The last command changed the clipboard (UI can show a hint) – or why it could not.
    pub notice: Option<String>,
}
