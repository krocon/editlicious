//! Commands and frames of the editor session (design 003). Mirror:
//! `src/app/core/editor/editor.models.ts`.

pub use super::ops::{CaseChange, SortMode, SortOptions};
use crate::document::model::{DocInfo, LineEnding, Sel};
use crate::search::Query;
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
    /// Settings (design 009): `detect` = indentation like the file (spaces and width when the
    /// file is indented with spaces, else `tab_width` and `insert_spaces`).
    Configure {
        tab_width: Option<usize>,
        insert_spaces: Option<bool>,
        #[serde(default)]
        detect: bool,
        /// Soft wrap at the view width (design 010 E4); `None` = unchanged.
        #[serde(default)]
        wrap: Option<bool>,
        /// Colour swatches in the gutter (design 018); `None` = unchanged.
        #[serde(default)]
        colors: Option<bool>,
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
        /// Alt-click: one more caret (design 007 E5).
        #[serde(default)]
        add: bool,
    },
    Drag {
        line: f64,
        cell: f64,
        /// Alt-drag: column selection from the click (design 007 E4).
        #[serde(default)]
        column: bool,
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
    /// Selects the next (previous) match and highlights all visible ones (design 006).
    /// `incremental`: search from the start of the selection (typing in the find field).
    Find {
        query: Query,
        #[serde(default)]
        backwards: bool,
        #[serde(default)]
        incremental: bool,
    },
    ClearSearch,
    /// Replaces the selected match (if it is one) and selects the next.
    Replace {
        query: Query,
        replacement: String,
    },
    /// Replaces every match as one undo step.
    ReplaceAll {
        query: Query,
        replacement: String,
    },
    // ---- design 007
    SortLines {
        options: SortOptions,
    },
    ChangeCase {
        to: CaseChange,
    },
    DuplicateLines,
    DeleteLines,
    MoveLines {
        up: bool,
    },
    JoinLines,
    TrimTrailingWhitespace,
    ConvertIndentation {
        spaces: bool,
    },
    /// Alt+Shift+arrows: grow a column selection by cells/lines.
    BlockMove {
        dx: isize,
        dy: isize,
    },
    AddNextOccurrence,
    SingleCursor,
    ToggleMarker,
    ClearMarkers,
    /// Language by name (`None` = plain text), design 008.
    /// Converts every line break (design 010 E2).
    SetLineEnding {
        line_ending: LineEnding,
    },
    /// Copies the selection (or the whole line) as HTML with the colours of the view
    /// (design 010 E5). `styles[i]` = CSS declarations of style class i, `base` = of the text.
    CopyHtml {
        line_numbers: bool,
        styles: Vec<String>,
        base: String,
        gutter: String,
    },
    SetLanguage {
        name: Option<String>,
    },
    /// Sets the selections – one cursor per entry, the last one primary (recordings, scripts,
    /// design 015 E2).
    Select {
        sels: Vec<Sel>,
    },
    /// The colour value under the caret into its next notation (design 018).
    ConvertColor,
}

impl Command {
    /// Only changes the view (size, scroll, settings, colours) – not recorded (design 015 E1),
    /// allowed while a playback runs (E10).
    pub fn is_view_only(&self) -> bool {
        matches!(
            self,
            Command::Frame
                | Command::Resize { .. }
                | Command::Scroll { .. }
                | Command::ScrollTo { .. }
                | Command::Configure { .. }
                | Command::CopyHtml { .. }
                | Command::Copy
        )
    }
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
    /// Style class (`highlight::CLASSES`), 0 = plain (design 008).
    pub style: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Search matches `[from, to, kind]` in cells; kind 1 = the selected one (design 006).
    pub marks: Vec<[usize; 3]>,
    /// Marked words `[from, to, colour 0…9]` in cells (design 007 E6).
    pub markers: Vec<[usize; 3]>,
    /// Width of the leading whitespace in cells (indentation guides, design 009); blank lines
    /// take the indentation of the line above.
    pub indent: usize,
    /// Rows from the top of the view to this line (negative: partly scrolled away).
    pub row: f64,
    /// Soft wrap: first cells of the rows after the first (runs never cross them).
    pub wraps: Vec<usize>,
    /// Colour values `[from, to, rgba]`: characters, colour as 0xRRGGBBAA (design 018).
    pub colors: Vec<[u32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CaretPos {
    pub line: usize,
    /// Absolute cell in the line.
    pub cell: usize,
}

/// Where a caret is drawn: rows from the top of the view, cell within its row.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CaretXY {
    pub row: f64,
    pub x: usize,
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
    /// Carets in document order.
    pub carets: Vec<CaretPos>,
    /// Where each caret is drawn (same order as `carets`).
    pub caret_xy: Vec<CaretXY>,
    /// Index of the primary caret (IME window, status bar).
    pub primary: usize,
    /// Primary caret as line/character (0-based) for the status bar.
    pub caret_line: usize,
    pub caret_ch: usize,
    pub sel_chars: usize,
    pub sel_lines: usize,
    pub overwrite: bool,
    pub tab_width: usize,
    pub insert_spaces: bool,
    /// A hint for the UI: `clipboard_empty`, `no_match`, `replaced:N`, `invalid_search` …
    pub notice: Option<String>,
    /// A search is active (matches are highlighted).
    pub search: bool,
    /// More than one cursor (Escape goes back to one).
    pub multi: bool,
    /// Language of the highlighting, `None` = plain text.
    pub language: Option<String>,
    /// The highlighting of the view started fresh (far jump in a big file, design 008 E3).
    pub approx_highlight: bool,
    /// The primary selection when it is short and on one line (to prefill the find field).
    pub selection_text: Option<String>,
    /// Soft wrap is on (no horizontal scrolling except for long lines).
    pub wrap: bool,
    /// Changes when search or markers change (the overview asks again, design 010 E3).
    pub deco: u64,
    /// The gutter has a column of colour swatches: once a colour showed up in this document it
    /// stays, so the gutter does not jump while scrolling (design 018).
    pub color_column: bool,
    /// The colour value at the primary caret (`#rrggbb[aa]`) for the status bar.
    pub caret_color: Option<String>,
}
