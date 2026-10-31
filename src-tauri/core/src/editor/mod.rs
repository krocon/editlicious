//! The editor session (design 003): a document plus cursors, scroll position, viewport and
//! settings. It runs every command of the UI and answers with a [`model::Frame`] – the UI only
//! renders (Marc, 26.10.2026; design 002 E13).

pub mod colors;
pub mod complete;
pub mod expand;
pub mod hints;
pub mod layout;
pub mod model;
mod ops;
pub mod playback;
pub mod recording;
pub mod script;
mod words;

#[cfg(test)]
mod tests;

use crate::document::Document;
use crate::document::model::{Change, EditKind, EditRequest, Pos, Sel};
use crate::error::FsError;
use crate::highlight::{self, Highlighter};
use crate::search::{self, Query};
use layout::{LineLayout, char_width, next_grapheme, prev_grapheme, wrap_points};
use model::{CaretPos, CaretXY, Command, Frame, FrameLine, Motion, Run, RunKind};
use regex::Regex;
use words::CharClass;

/// The system clipboard as the session sees it; the app provides the real one, tests a fake.
pub trait Clipboard {
    fn get(&mut self) -> Option<String>;
    fn set(&mut self, text: String) -> bool;
    /// Rich text with a plain alternative; without HTML support only the text.
    fn set_html(&mut self, _html: String, alt: String) -> bool {
        self.set(alt)
    }
}

/// A clipboard that only lives in memory (tests, WebAssembly demo).
#[derive(Debug, Default)]
pub struct MemoryClipboard(pub Option<String>);

impl Clipboard for MemoryClipboard {
    fn get(&mut self) -> Option<String> {
        self.0.clone()
    }
    fn set(&mut self, text: String) -> bool {
        self.0 = Some(text);
        true
    }
}

/// A selection with the cell the caret wants to keep on vertical moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    pub anchor: Pos,
    pub head: Pos,
    pub goal: Option<usize>,
}

impl Cursor {
    pub fn caret(p: Pos) -> Self {
        Self {
            anchor: p,
            head: p,
            goal: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn start(&self) -> Pos {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> Pos {
        self.anchor.max(self.head)
    }

    fn sel(&self) -> Sel {
        Sel {
            anchor: self.anchor,
            head: self.head,
        }
    }
}

/// How a mouse drag extends the selection: from the click's character, word or line.
#[derive(Debug, Clone, Copy)]
struct DragOrigin {
    count: u8,
    from: Pos,
    to: Pos,
}

/// Cells kept between the caret and the edge when the view scrolls horizontally.
const H_MARGIN: f64 = 4.0;
/// Largest view the UI may ask for (rows, cells) and the largest single scroll step.
const MAX_VIEW_ROWS: f64 = 2_000.0;
const MAX_VIEW_COLS: f64 = 4_000.0;
const MAX_SCROLL: f64 = 100_000.0;
/// Extra cells requested left and right of the visible window.
const WINDOW_SLACK: usize = 16;

#[derive(Debug)]
pub struct Session {
    pub doc: Document,
    pub id: u64,
    cursors: Vec<Cursor>,
    top: f64,
    left: f64,
    view_lines: f64,
    view_cols: f64,
    tab: usize,
    spaces: bool,
    overwrite: bool,
    seq: u64,
    max_width: usize,
    drag: Option<DragOrigin>,
    notice: Option<String>,
    search: Option<(Query, Regex)>,
    /// Index of the primary cursor in `cursors` (design 007 E5).
    primary: usize,
    /// Column (block) selection: anchor and head as line + cell (design 007 E4).
    block: Option<ops::Block>,
    /// Marked words, up to ten colours (legacy „Mark Text“, design 007 E6).
    markers: Vec<Option<(String, Regex)>>,
    /// Line and cell of the last click – the corner of an Alt-drag block.
    block_origin: Option<(usize, usize)>,
    highlighter: Option<Highlighter>,
    /// Soft wrap at the view width (design 010 E4).
    wrap: bool,
    /// Repeated double clicks (prompt 018a): the steps at the last double click, where the
    /// selection is, at which text revision.
    expansion: Option<Expansion>,
    weights: expand::Weights,
    /// Arithmetic hints and the open suggestion list (prompt 020).
    math: hints::MathOptions,
    completion: Option<OpenCompletion>,
    /// Word completion (prompt 023).
    words: complete::WordOptions,
    /// Colour swatches on (design 018) and whether one ever showed (the gutter column stays).
    colors: bool,
    color_column: bool,
    /// The running recording (design 015) …
    recorder: Option<recording::Recorder>,
    /// … and the last finished one (save it, play it again as a macro).
    last_recording: Option<recording::Recording>,
    /// Edits are refused (design 025 E1).
    read_only: bool,
}

/// Suggestions shown for a line (prompt 020): arithmetic or words (prompt 023).
#[derive(Debug, Clone)]
struct OpenCompletion {
    line: usize,
    items: Vec<hints::Suggestion>,
    revision: u64,
    /// Word suggestions: typing filters them (design 023 E8).
    words: bool,
}

/// The steps of a progressive selection and the current one (prompt 018a).
#[derive(Debug, Clone)]
struct Expansion {
    steps: Vec<(Pos, Pos)>,
    at: usize,
    revision: u64,
}

/// A place in the view: document line and row within the line (fraction = part of a row).
type VPos = (usize, f64);

impl Session {
    pub fn new(doc: Document, id: u64) -> Self {
        let (spaces, tab) = words::detect_indent(&doc);
        let highlighter = Self::detect_highlighter(&doc);
        Self {
            doc,
            id,
            cursors: vec![Cursor::default()],
            top: 0.0,
            left: 0.0,
            view_lines: 40.0,
            view_cols: 120.0,
            tab,
            spaces,
            overwrite: false,
            seq: 0,
            max_width: 0,
            drag: None,
            notice: None,
            search: None,
            primary: 0,
            block: None,
            markers: Vec::new(),
            block_origin: None,
            highlighter,
            wrap: false,
            colors: true,
            color_column: false,
            expansion: None,
            weights: expand::Weights::default(),
            math: hints::MathOptions::default(),
            completion: None,
            words: complete::WordOptions::default(),
            recorder: None,
            last_recording: None,
            read_only: false,
        }
    }

    fn detect_highlighter(doc: &Document) -> Option<Highlighter> {
        let first = if doc.is_long(0) {
            doc.line_slice(0, 0, 200)
        } else {
            doc.line_string(0)
        };
        let name = doc
            .path
            .as_deref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        highlight::detect(name.as_deref(), &first).and_then(|l| Highlighter::new(&l))
    }

    /// A file name for a text without a file (embedded editor, design 025 E3): the language
    /// follows its extension, the word completion its lexicon (`app.ts` → TypeScript words).
    /// `None` = no name (language: plain text unless set otherwise).
    pub fn set_file_name(&mut self, name: Option<&str>) {
        self.doc.path = name.map(std::path::PathBuf::from);
        self.highlighter = Self::detect_highlighter(&self.doc);
    }

    /// The language shown (`None` = plain text).
    pub fn language(&self) -> Option<&str> {
        self.highlighter.as_ref().map(|h| h.language.as_str())
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// The cursor that scrolls into view and feeds the status bar (the one added last).
    pub fn primary(&self) -> Cursor {
        self.cursors[self.primary.min(self.cursors.len() - 1)]
    }

    pub fn set_cursors(&mut self, cursors: Vec<Cursor>) {
        self.cursors = cursors;
        self.normalize();
    }

    pub fn scroll(&self) -> (f64, f64) {
        (self.top, self.left)
    }

    /// Replaces the text with a fresh load of the file (changed on disk, design 004 E6);
    /// cursors and scroll position stay where they were, as far as the new text allows.
    pub fn replace_document(&mut self, doc: Document) {
        let lang = self.highlighter.as_ref().map(|h| h.language.clone());
        self.doc = doc;
        self.highlighter = lang.and_then(|l| Highlighter::new(&l));
        self.max_width = 0;
        self.drag = None;
        self.normalize();
        self.clamp_scroll();
    }

    /// Runs one command and returns the frame to draw; a running recording keeps it
    /// (design 015).
    pub fn run(&mut self, cmd: Command, clip: &mut dyn Clipboard) -> Result<Frame, FsError> {
        if self.recorder.is_none() || cmd.is_view_only() {
            return self.run_inner(cmd, clip);
        }
        let pasted = matches!(cmd, Command::Paste).then(|| clip.get()).flatten();
        // arithmetic depends on settings and the open list: kept as the edit it makes (prompt 020)
        let math = matches!(
            cmd,
            Command::Complete
                | Command::CloseCompletion
                | Command::ApplyCompletion { .. }
                | Command::Evaluate
        );
        let math_edit = if math {
            self.planned_math_edit(&cmd)
        } else {
            None
        };
        let kept = cmd.clone();
        let frame = self.run_inner(cmd, clip)?;
        let sels = self.sels_primary_last();
        let wrap = self.wrap;
        if let Some(r) = self.recorder.as_mut() {
            if let Some((line, from, to, text)) = math_edit {
                let range = vec![Sel {
                    anchor: Pos::new(line, from),
                    head: Pos::new(line, to),
                }];
                r.keep(
                    Command::Select {
                        sels: range.clone(),
                    },
                    None,
                    range,
                    wrap,
                );
                r.keep(Command::Insert { text }, None, sels, wrap);
            } else if !math {
                r.keep(kept, pasted, sels, wrap);
            }
        }
        Ok(frame)
    }

    /// The change `ApplyCompletion`/`Evaluate` (or `Complete` with a single word, design 023
    /// E7) will make: line, characters `from..to`, text.
    fn planned_math_edit(&self, cmd: &Command) -> Option<(usize, usize, usize, String)> {
        match cmd {
            Command::Complete => {
                if self.math_suggestions().is_some() {
                    return None;
                }
                let (line, items) = self.word_completion(1)?;
                let [s] = items.as_slice() else {
                    return None;
                };
                Some((line, s.from, s.to, s.text.clone()))
            }
            Command::ApplyCompletion { index } => {
                let c = self.completion.as_ref()?;
                let s = c.items.get(*index)?;
                (c.revision == self.doc.revision()).then(|| (c.line, s.from, s.to, s.text.clone()))
            }
            Command::Evaluate => {
                let (line, f) = self.math_at_caret()?;
                f.value.is_finite().then(|| {
                    let to = if f.eq {
                        self.primary().head.ch.max(f.to)
                    } else {
                        f.to
                    };
                    (line, f.from, to, hints::result_text(f.value, &self.math))
                })
            }
            _ => None,
        }
    }

    /// The selections with the primary one last (as [`Command::Select`] wants them).
    pub fn sels_primary_last(&self) -> Vec<Sel> {
        let p = self.primary.min(self.cursors.len() - 1);
        let mut out: Vec<Sel> = (0..self.cursors.len())
            .filter(|&i| i != p)
            .map(|i| self.cursors[i].sel())
            .collect();
        out.push(self.cursors[p].sel());
        out
    }

    /// Starts recording the commands of this session (design 015); a running one starts over.
    pub fn start_recording(&mut self, app: &str) {
        let sels = self.sels_primary_last();
        let header = recording::Header::new(
            app,
            &self.doc,
            sels,
            recording::Settings {
                tab_width: self.tab,
                insert_spaces: self.spaces,
                wrap: self.wrap,
            },
        );
        self.recorder = Some(recording::Recorder::new(header).with_start(self.doc.rope().clone()));
    }

    /// Stops the recording; it becomes the last one. `None` when none was running.
    pub fn stop_recording(&mut self) -> Option<&recording::Recording> {
        let r = self.recorder.take()?;
        self.last_recording = Some(r.finish());
        self.last_recording.as_ref()
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.is_some()
    }

    pub fn last_recording(&self) -> Option<&recording::Recording> {
        self.last_recording.as_ref()
    }

    /// Settings of a recording before it plays (tab width, spaces, wrap stay as the view has
    /// them – only indentation matters for the text).
    pub fn indentation(&self) -> (usize, bool) {
        (self.tab, self.spaces)
    }

    fn run_inner(&mut self, cmd: Command, clip: &mut dyn Clipboard) -> Result<Frame, FsError> {
        if self.read_only && cmd.is_edit() {
            return Err(FsError::new(
                crate::error::FsErrorCode::ReadOnly,
                "the document is read-only",
                None,
            ));
        }
        self.notice = None;
        let mut reveal = Reveal::Margin;
        let mut keep_block = false;
        let mut keep_expansion = cmd.is_view_only();
        let mut keep_completion =
            cmd.is_view_only() || matches!(cmd, Command::Complete | Command::CloseCompletion);
        let typed_eq = matches!(&cmd, Command::Type { text } if text.ends_with('='));
        // word suggestions follow typing (design 023 E8, E9)
        let was_words = self.completion.as_ref().is_some_and(|c| c.words);
        let extra = self.word_extra();
        let typed_word = matches!(&cmd, Command::Type { text }
            if !text.is_empty() && text.chars().all(|c| complete::is_word(c, &extra)));
        let backspace = matches!(cmd, Command::Backspace);
        match cmd {
            Command::Frame => reveal = Reveal::No,
            Command::SetReadOnly { read_only } => {
                self.read_only = read_only;
                if read_only {
                    self.completion = None;
                }
                reveal = Reveal::No;
            }
            Command::Resize { lines, cols } => {
                // bounded: a frame never holds more than this (design 013 review #13)
                self.view_lines = lines.clamp(1.0, MAX_VIEW_ROWS);
                self.view_cols = cols.clamp(1.0, MAX_VIEW_COLS);
                reveal = Reveal::No;
            }
            Command::Configure {
                tab_width,
                insert_spaces,
                detect,
                wrap,
                colors,
                expand_weights,
                math,
                words,
            } => {
                if let Some(w) = words {
                    self.words = complete::WordOptions {
                        auto: w.auto,
                        keywords: w.keywords,
                    };
                }
                if let Some(m) = math {
                    self.math = hints::MathOptions {
                        hints: m.hints,
                        suggest: m.suggest,
                        decimals: m.decimals.map(|d| d.min(12)),
                        comma: m.comma,
                    };
                }
                if let Some(w) = expand_weights {
                    self.weights = expand::Weights::with(&w);
                }
                if let Some(c) = colors {
                    self.colors = c;
                    if !c {
                        self.color_column = false;
                    }
                }
                if let Some(w) = wrap
                    && w != self.wrap
                {
                    self.wrap = w;
                    self.max_width = 0;
                    if w {
                        self.left = 0.0;
                    }
                }
                if let Some(t) = tab_width {
                    self.tab = t.clamp(1, 16);
                }
                if let Some(s) = insert_spaces {
                    self.spaces = s;
                }
                if detect {
                    match words::file_indent(&self.doc) {
                        Some((true, w)) => {
                            self.spaces = true;
                            self.tab = w;
                        }
                        Some((false, _)) => self.spaces = false,
                        None => {}
                    }
                }
                reveal = Reveal::No;
            }
            Command::Scroll { lines, cells } => {
                let lines = lines.clamp(-MAX_SCROLL, MAX_SCROLL);
                let cells = cells.clamp(-MAX_SCROLL, MAX_SCROLL);
                if self.wrap {
                    let v = self.advance(self.top_vpos(), lines);
                    self.set_top_vpos(v);
                } else {
                    self.top += lines;
                }
                self.left += cells;
                reveal = Reveal::No;
            }
            Command::ScrollTo { top, left } => {
                if let Some(t) = top {
                    self.top = t;
                }
                if let Some(l) = left {
                    self.left = l;
                }
                reveal = Reveal::No;
            }
            Command::Move { to, select } => self.move_cursors(to, select),
            Command::SelectAll => {
                let end = self.doc.char_to_pos(self.doc.rope().len_chars());
                self.cursors = vec![Cursor {
                    anchor: Pos::default(),
                    head: end,
                    goal: None,
                }];
                reveal = Reveal::No;
            }
            Command::GoTo { line, ch, select } => {
                let p = self.doc.clamp(Pos::new(line, ch));
                let anchor = if select { self.primary().anchor } else { p };
                self.cursors = vec![Cursor {
                    anchor,
                    head: p,
                    goal: None,
                }];
            }
            Command::Click {
                line,
                cell,
                count,
                extend,
                add,
            } if count == 2 && !extend && !add => {
                let p = self.pos_at(line, cell);
                self.double_click(p);
                self.block_origin = Some((p.line, cell.max(0.0).round() as usize));
                keep_expansion = true;
                reveal = Reveal::Exact;
            }
            Command::Click {
                line,
                cell,
                count,
                extend,
                add,
            } => {
                // a single click inside the expanded selection keeps the sequence: the second
                // click of the next double click lands here first
                if count == 1 && !extend && !add {
                    let p = self.pos_at(line, cell);
                    keep_expansion = self.expansion.as_ref().is_some_and(|e| {
                        let (a, b) = e.steps[e.at];
                        e.revision == self.doc.revision() && a <= p && p <= b
                    });
                }
                if add && count <= 1 && !extend {
                    let p = self.pos_at(line, cell);
                    self.add_cursor(p);
                    self.drag = Some(DragOrigin {
                        count: 1,
                        from: p,
                        to: p,
                    });
                    self.block_origin = Some((p.line, cell.max(0.0).round() as usize));
                } else {
                    self.click(line, cell, count, extend);
                    self.block_origin =
                        Some((self.primary().head.line, cell.max(0.0).round() as usize));
                }
                reveal = Reveal::Exact;
            }
            Command::Drag { line, cell, column } => {
                match (column, self.block_origin) {
                    (true, Some(o)) => self.block_drag(o, line, cell),
                    _ => self.drag_to(line, cell),
                }
                keep_block = column;
                reveal = Reveal::Exact;
            }
            Command::Type { text } => self.type_text(&text)?,
            Command::Insert { text } => self.insert(&text, EditKind::Other)?,
            Command::Newline => self.newline()?,
            Command::Backspace => self.delete_back(false)?,
            Command::Delete => self.delete_forward(false)?,
            Command::DeleteWordLeft => self.delete_back(true)?,
            Command::DeleteWordRight => self.delete_forward(true)?,
            Command::Indent => self.indent()?,
            Command::Outdent => self.outdent()?,
            Command::ToggleOverwrite => {
                self.overwrite = !self.overwrite;
                reveal = Reveal::No;
            }
            Command::Undo => {
                if let Some(sels) = self.doc.undo() {
                    self.cursors_from(sels);
                }
            }
            Command::Redo => {
                if let Some(sels) = self.doc.redo() {
                    self.cursors_from(sels);
                }
            }
            Command::Copy => {
                self.copy(clip, false)?;
                reveal = Reveal::No;
            }
            Command::Cut => self.copy(clip, true)?,
            Command::Paste => match clip.get() {
                Some(text) => {
                    if !self.paste_distributed(&text)? {
                        self.insert(&text, EditKind::Other)?;
                    }
                }
                None => self.notice = Some("clipboard_empty".into()),
            },
            Command::Find {
                query,
                backwards,
                incremental,
            } => {
                self.find(query, backwards, incremental);
                reveal = Reveal::Center;
            }
            Command::ClearSearch => {
                self.search = None;
                reveal = Reveal::No;
            }
            Command::Replace { query, replacement } => {
                self.replace(query, &replacement)?;
                reveal = Reveal::Center;
            }
            Command::ReplaceAll { query, replacement } => self.replace_all(query, &replacement)?,
            Command::SortLines { options } => self.sort(options)?,
            Command::ChangeCase { to } => self.change_case(to)?,
            Command::DuplicateLines => self.duplicate_lines()?,
            Command::DeleteLines => self.delete_lines()?,
            Command::MoveLines { up } => self.move_lines(up)?,
            Command::JoinLines => self.join_lines()?,
            Command::TrimTrailingWhitespace => {
                let n = self.trim_trailing()?;
                self.notice = Some(format!("trimmed:{n}"));
            }
            Command::ConvertIndentation { spaces } => self.convert_indent(spaces)?,
            Command::BlockMove { dx, dy } => {
                self.block_move(dx, dy);
                keep_block = true;
            }
            Command::AddNextOccurrence => self.add_next_occurrence(),
            Command::SingleCursor => self.single_cursor(),
            Command::ToggleMarker => {
                self.toggle_marker();
                reveal = Reveal::No;
            }
            Command::ClearMarkers => {
                self.clear_markers();
                reveal = Reveal::No;
            }
            Command::SetLineEnding { line_ending } => self.convert_line_endings(line_ending)?,
            Command::CopyHtml {
                line_numbers,
                styles,
                base,
                gutter,
                markers,
                background,
            } => {
                let look = ops::HtmlLook {
                    styles: &styles,
                    base: &base,
                    gutter: &gutter,
                    markers: &markers,
                    background: &background,
                };
                self.copy_html(clip, line_numbers, &look)?;
                reveal = Reveal::No;
            }
            Command::SetMarker { colour } => {
                self.set_marker(colour);
                reveal = Reveal::No;
            }
            Command::MarkerStep { backwards } => {
                self.marker_step(backwards);
                reveal = Reveal::Center;
            }
            Command::SetMarkers { words } => {
                self.set_markers(words);
                reveal = Reveal::No;
            }
            Command::SetLanguage { name } => {
                self.highlighter = name.as_deref().and_then(Highlighter::new);
                reveal = Reveal::No;
            }
            Command::ConvertColor => self.convert_color()?,
            Command::Complete => {
                if !self.complete_manually()? {
                    reveal = Reveal::No;
                }
            }
            Command::ApplyCompletion { index } => self.apply_completion(index)?,
            Command::CloseCompletion => {
                self.completion = None;
                reveal = Reveal::No;
            }
            Command::Evaluate => {
                if let Some((line, f)) = self.math_at_caret()
                    && f.value.is_finite()
                {
                    let caret = self.primary().head.ch;
                    let to = if f.eq { caret.max(f.to) } else { f.to };
                    let text = hints::result_text(f.value, &self.math);
                    self.replace_in_line(line, f.from, to, &text)?;
                } else {
                    self.notice = Some("no_expression".into());
                }
            }
            Command::CopyResult => {
                match self.math_at_caret().filter(|(_, f)| f.value.is_finite()) {
                    Some((_, f)) => {
                        if !clip.set(hints::result_text(f.value, &self.math)) {
                            self.notice = Some("clipboard_failed".into());
                        } else {
                            self.notice = Some("result_copied".into());
                        }
                    }
                    None => self.notice = Some("no_expression".into()),
                }
                reveal = Reveal::No;
            }
            Command::Select { sels } => {
                if !sels.is_empty() {
                    let n = sels.len();
                    self.cursors = sels
                        .into_iter()
                        .map(|s| Cursor {
                            anchor: self.doc.clamp(s.anchor),
                            head: self.doc.clamp(s.head),
                            goal: None,
                        })
                        .collect();
                    self.primary = n - 1;
                    self.normalize();
                }
            }
        }
        if !keep_block && !matches!(reveal, Reveal::No) {
            self.block = None;
        }
        if !keep_expansion {
            // moved, typed, dragged …: the next double click starts with the word again
            self.expansion = None;
        }
        if typed_eq && self.math.suggest {
            // "255/8 =" – the result is offered right away (prompt 020)
            self.open_completion(false);
            keep_completion = self.completion.is_some();
        } else if (was_words && (typed_word || backspace)) || (typed_word && self.words.auto) {
            // filter the open list, or open it from two characters on
            let min = if was_words { 1 } else { complete::AUTO_MIN };
            self.completion = self
                .word_completion(min)
                .filter(|(_, items)| !items.is_empty())
                .map(|(line, items)| OpenCompletion {
                    line,
                    items,
                    revision: self.doc.revision(),
                    words: true,
                });
            keep_completion = self.completion.is_some();
        }
        if !keep_completion {
            self.completion = None;
        }
        self.clamp_scroll();
        match reveal {
            Reveal::No => {}
            Reveal::Margin => self.reveal(H_MARGIN),
            Reveal::Exact => self.reveal(0.0),
            Reveal::Center => {
                let rows = self.view_lines.floor().max(1.0);
                if self.wrap {
                    let c = self.caret_vpos(self.primary().head);
                    let top = self.top_vpos();
                    let bottom = self.advance(c, -(rows - 1.0));
                    if vless(c, top) || vless(top, bottom) {
                        let v = self.advance(c, -(rows / 2.0).floor());
                        self.set_top_vpos(v);
                    }
                } else {
                    let line = self.primary().head.line as f64;
                    if line < self.top || line + 1.0 > self.top + rows {
                        self.top = (line - (rows / 2.0).floor()).max(0.0);
                    }
                }
                self.reveal(H_MARGIN);
            }
        }
        Ok(self.frame())
    }

    // ------------------------------------------------------------------ positions

    fn layout(&self, line: usize) -> Option<LineLayout> {
        (!self.doc.is_long(line)).then(|| LineLayout::new(&self.doc.line_string(line), self.tab))
    }

    /// Cell of a position (long lines: one cell per character).
    pub fn cell_of(&self, p: Pos) -> usize {
        match self.layout(p.line) {
            Some(l) => l.cell_of(p.ch),
            None => p.ch,
        }
    }

    fn ch_for_goal(&self, line: usize, cell: usize) -> usize {
        match self.layout(line) {
            Some(l) => l.ch_at_or_before(cell),
            None => cell.min(self.doc.line_len(line)),
        }
    }

    /// The position at a point of the view: `line` = top + rows down (soft wrap: rows, not
    /// lines), `cell` = cell in the row.
    fn pos_at(&self, line: f64, cell: f64) -> Pos {
        if self.wrap {
            let max_line = self.doc.line_count().saturating_sub(1);
            if line < 0.0 {
                return Pos::new(0, 0);
            }
            let (l, r) = self.advance(self.top_vpos(), line - self.top);
            let breaks = self.breaks(l);
            let row = (r.max(0.0).floor() as usize).min(breaks.len());
            if l == max_line && line - self.top > 0.0 && r >= self.rows(l) as f64 {
                return Pos::new(max_line, self.doc.line_len(max_line));
            }
            let start = if row == 0 { 0 } else { breaks[row - 1] };
            let next = breaks.get(row).copied();
            let mut abs = start as f64 + cell.max(0.0);
            if let Some(n) = next {
                // a click right of a row's end lands before the row break
                abs = abs.min(n as f64 - 0.5);
            }
            let ch = match self.layout(l) {
                Some(lay) => lay.ch_at(abs),
                None => (abs.round() as usize).min(self.doc.line_len(l)),
            };
            return Pos::new(l, ch);
        }
        let max_line = self.doc.line_count().saturating_sub(1);
        if line < 0.0 {
            return Pos::new(0, 0);
        }
        let l = (line.floor() as usize).min(max_line);
        if line.floor() as usize > max_line {
            return Pos::new(max_line, self.doc.line_len(max_line));
        }
        let ch = match self.layout(l) {
            Some(lay) => lay.ch_at(cell),
            None => (cell.max(0.0).round() as usize).min(self.doc.line_len(l)),
        };
        Pos::new(l, ch)
    }

    fn char_at(&self, idx: usize) -> Option<char> {
        let rope = self.doc.rope();
        (idx < rope.len_chars()).then(|| rope.char(idx))
    }

    // ------------------------------------------------------------------ motion

    fn step_left(&self, p: Pos) -> Pos {
        if p.ch > 0 {
            let ch = if self.doc.is_long(p.line) {
                p.ch - 1
            } else {
                prev_grapheme(&self.doc.line_string(p.line), p.ch).unwrap_or(p.ch - 1)
            };
            Pos::new(p.line, ch)
        } else if p.line > 0 {
            Pos::new(p.line - 1, self.doc.line_len(p.line - 1))
        } else {
            p
        }
    }

    fn step_right(&self, p: Pos) -> Pos {
        let len = self.doc.line_len(p.line);
        if p.ch < len {
            let ch = if self.doc.is_long(p.line) {
                p.ch + 1
            } else {
                next_grapheme(&self.doc.line_string(p.line), p.ch).unwrap_or(p.ch + 1)
            };
            Pos::new(p.line, ch)
        } else if p.line + 1 < self.doc.line_count() {
            Pos::new(p.line + 1, 0)
        } else {
            p
        }
    }

    /// End of the next word (or start of the next line at a line end).
    fn word_right(&self, p: Pos) -> Pos {
        let len = self.doc.line_len(p.line);
        if p.ch >= len {
            return self.step_right(p);
        }
        let line_start = self.doc.rope().line_to_char(p.line);
        let mut chars = self.doc.rope().chars_at(line_start + p.ch);
        let mut ch = p.ch;
        let mut class = None;
        while ch < len {
            let Some(c) = chars.next() else { break };
            let k = CharClass::of(c);
            match class {
                None if k == CharClass::Space => {}
                None => class = Some(k),
                Some(cl) if cl != k => break,
                Some(_) => {}
            }
            ch += 1;
        }
        Pos::new(p.line, ch)
    }

    /// Start of the previous word (or end of the previous line at a line start).
    fn word_left(&self, p: Pos) -> Pos {
        if p.ch == 0 {
            return self.step_left(p);
        }
        let line_start = self.doc.rope().line_to_char(p.line);
        let mut chars = self.doc.rope().chars_at(line_start + p.ch);
        let mut ch = p.ch;
        let mut class = None;
        while ch > 0 {
            let Some(c) = chars.prev() else { break };
            let k = CharClass::of(c);
            match class {
                None if k == CharClass::Space => {}
                None => class = Some(k),
                Some(cl) if cl != k => break,
                Some(_) => {}
            }
            ch -= 1;
        }
        Pos::new(p.line, ch)
    }

    /// The word (or run of spaces/punctuation) around a position, for double clicks.
    fn word_at(&self, p: Pos) -> (Pos, Pos) {
        let len = self.doc.line_len(p.line);
        let line_start = self.doc.rope().line_to_char(p.line);
        let at = |ch: usize| self.char_at(line_start + ch).map(CharClass::of);
        let class = if p.ch < len {
            at(p.ch)
        } else if p.ch > 0 {
            at(p.ch - 1)
        } else {
            None
        };
        let Some(class) = class else {
            return (p, p);
        };
        let (mut a, mut b) = (p.ch, p.ch);
        while a > 0 && at(a - 1) == Some(class) {
            a -= 1;
        }
        while b < len && at(b) == Some(class) {
            b += 1;
        }
        (Pos::new(p.line, a), Pos::new(p.line, b))
    }

    fn line_range(&self, line: usize) -> (Pos, Pos) {
        let last = self.doc.line_count().saturating_sub(1);
        let end = if line < last {
            Pos::new(line + 1, 0)
        } else {
            Pos::new(line, self.doc.line_len(line))
        };
        (Pos::new(line, 0), end)
    }

    fn first_non_blank(&self, line: usize) -> usize {
        let len = self.doc.line_len(line);
        let start = self.doc.rope().line_to_char(line);
        let mut i = 0;
        while i < len && matches!(self.char_at(start + i), Some(' ' | '\t')) {
            i += 1;
        }
        i
    }

    fn page(&self) -> usize {
        (self.view_lines.floor() as usize).saturating_sub(1).max(1)
    }

    fn vertical(&self, c: &Cursor, down: bool, n: usize) -> (Pos, usize) {
        if self.wrap {
            return self.vertical_wrapped(c, down, n);
        }
        let goal = c.goal.unwrap_or_else(|| self.cell_of(c.head));
        let last = self.doc.line_count().saturating_sub(1);
        let p = c.head;
        if !down && p.line == 0 {
            return (Pos::new(0, 0), goal);
        }
        if down && p.line >= last {
            return (Pos::new(last, self.doc.line_len(last)), goal);
        }
        let line = if down {
            (p.line + n).min(last)
        } else {
            p.line.saturating_sub(n)
        };
        (Pos::new(line, self.ch_for_goal(line, goal)), goal)
    }

    /// Up/down by rows of wrapped lines; the goal is the cell within the row.
    fn vertical_wrapped(&self, c: &Cursor, down: bool, n: usize) -> (Pos, usize) {
        let p = c.head;
        let breaks = self.breaks(p.line);
        let cell = self.cell_of(p);
        let row = breaks.partition_point(|&b| b <= cell);
        let row_start = if row == 0 { 0 } else { breaks[row - 1] };
        let goal = c.goal.unwrap_or(cell - row_start);
        let last = self.doc.line_count().saturating_sub(1);
        let (mut line, mut row) = (p.line, row);
        let mut rows_here = breaks.len() + 1;
        for _ in 0..n {
            if down {
                if row + 1 < rows_here {
                    row += 1;
                } else if line < last {
                    line += 1;
                    row = 0;
                    rows_here = self.rows(line);
                } else {
                    return (Pos::new(last, self.doc.line_len(last)), goal);
                }
            } else if row > 0 {
                row -= 1;
            } else if line > 0 {
                line -= 1;
                rows_here = self.rows(line);
                row = rows_here - 1;
            } else {
                return (Pos::new(0, 0), goal);
            }
        }
        let b = self.breaks(line);
        let start = if row == 0 { 0 } else { b[row - 1] };
        let mut target = start + goal;
        if let Some(&next) = b.get(row) {
            target = target.min(next.saturating_sub(1).max(start));
        }
        let mut ch = self.ch_for_goal(line, target);
        // never before the row (a wide character at its start)
        if self.cell_of(Pos::new(line, ch)) < start {
            ch = self.ch_for_goal(line, start + 1).max(ch);
        }
        (Pos::new(line, ch), goal)
    }

    fn move_cursors(&mut self, to: Motion, select: bool) {
        if matches!(to, Motion::PageUp | Motion::PageDown) {
            let n = self.page() as f64;
            let d = if to == Motion::PageDown { n } else { -n };
            if self.wrap {
                let v = self.advance(self.top_vpos(), d);
                self.set_top_vpos(v);
            } else {
                self.top += d;
            }
        }
        let cursors = self.cursors.clone();
        self.cursors = cursors
            .iter()
            .map(|c| {
                let mut goal = None;
                let head = match to {
                    Motion::Left if !select && !c.is_empty() => c.start(),
                    Motion::Right if !select && !c.is_empty() => c.end(),
                    Motion::Left => self.step_left(c.head),
                    Motion::Right => self.step_right(c.head),
                    Motion::Up | Motion::Down | Motion::PageUp | Motion::PageDown => {
                        let down = matches!(to, Motion::Down | Motion::PageDown);
                        let n = if matches!(to, Motion::Up | Motion::Down) {
                            1
                        } else {
                            self.page()
                        };
                        let (p, g) = self.vertical(c, down, n);
                        goal = Some(g);
                        p
                    }
                    Motion::WordLeft => self.word_left(c.head),
                    Motion::WordRight => self.word_right(c.head),
                    Motion::LineStart => {
                        let fnb = self.first_non_blank(c.head.line);
                        Pos::new(c.head.line, if c.head.ch == fnb { 0 } else { fnb })
                    }
                    Motion::LineEnd => Pos::new(c.head.line, self.doc.line_len(c.head.line)),
                    Motion::DocStart => Pos::new(0, 0),
                    Motion::DocEnd => self.doc.char_to_pos(self.doc.rope().len_chars()),
                };
                Cursor {
                    anchor: if select { c.anchor } else { head },
                    head,
                    goal,
                }
            })
            .collect();
        self.normalize();
    }

    // ------------------------------------------------------------------ mouse

    fn click(&mut self, line: f64, cell: f64, count: u8, extend: bool) {
        let p = self.pos_at(line, cell);
        let (from, to) = match count {
            2 => self.word_at(p),
            n if n >= 3 => self.line_range(p.line),
            _ => (p, p),
        };
        if extend && count <= 1 {
            let anchor = self.primary().anchor;
            self.cursors = vec![Cursor {
                anchor,
                head: p,
                goal: None,
            }];
            self.drag = Some(DragOrigin {
                count: 1,
                from: anchor,
                to: anchor,
            });
            return;
        }
        self.cursors = vec![Cursor {
            anchor: from,
            head: to,
            goal: None,
        }];
        self.drag = Some(DragOrigin {
            count: count.clamp(1, 3),
            from,
            to,
        });
    }

    /// A double click (prompt 018a): the word, or – again inside the selection of the last one,
    /// on the same text – the next larger step; after the largest the word again.
    fn double_click(&mut self, p: Pos) {
        let rev = self.doc.revision();
        let again = self.expansion.as_ref().is_some_and(|e| {
            let (a, b) = e.steps[e.at];
            e.revision == rev && a <= p && p <= b
        });
        if again {
            if let Some(e) = self.expansion.as_mut() {
                e.at = (e.at + 1) % e.steps.len();
            }
        } else {
            let steps = expand::steps(&self.doc, p, &self.weights);
            self.expansion = Some(Expansion {
                steps,
                at: 0,
                revision: rev,
            });
        }
        let Some((from, to)) = self.expansion.as_ref().map(|e| e.steps[e.at]) else {
            return;
        };
        self.cursors = vec![Cursor {
            anchor: from,
            head: to,
            goal: None,
        }];
        self.primary = 0;
        self.drag = Some(DragOrigin { count: 2, from, to });
    }

    fn drag_to(&mut self, line: f64, cell: f64) {
        let p = self.pos_at(line, cell);
        let origin = self.drag.unwrap_or(DragOrigin {
            count: 1,
            from: self.primary().anchor,
            to: self.primary().anchor,
        });
        let (from, to) = match origin.count {
            2 => self.word_at(p),
            3 => self.line_range(p.line),
            _ => (p, p),
        };
        let c = if p < origin.from {
            Cursor {
                anchor: origin.to,
                head: from,
                goal: None,
            }
        } else {
            Cursor {
                anchor: origin.from,
                head: to.max(origin.to),
                goal: None,
            }
        };
        self.cursors = vec![c];
    }

    // ------------------------------------------------------------------ editing

    fn cursors_from(&mut self, sels: Vec<Sel>) {
        if sels.is_empty() {
            return;
        }
        self.cursors = sels
            .into_iter()
            .map(|s| Cursor {
                anchor: s.anchor,
                head: s.head,
                goal: None,
            })
            .collect();
        self.normalize();
    }

    fn sels(&self) -> Vec<Sel> {
        self.cursors.iter().map(Cursor::sel).collect()
    }

    /// Applies one change per cursor; carets land behind the inserted text.
    fn edit_each(
        &mut self,
        kind: EditKind,
        f: impl Fn(&Self, &Cursor) -> Option<Change>,
    ) -> Result<(), FsError> {
        let changes: Vec<Change> = self.cursors.iter().filter_map(|c| f(self, c)).collect();
        if changes.is_empty() {
            return Ok(());
        }
        let sels = self.doc.apply(EditRequest {
            changes,
            kind,
            sel_before: self.sels(),
            sel_after: None,
        })?;
        self.cursors_from(sels);
        Ok(())
    }

    fn insert(&mut self, text: &str, kind: EditKind) -> Result<(), FsError> {
        self.edit_each(kind, |_, c| {
            Some(Change {
                from: c.start(),
                to: c.end(),
                text: text.to_owned(),
            })
        })
    }

    fn type_text(&mut self, text: &str) -> Result<(), FsError> {
        let single = text.chars().count() == 1 && !text.contains(['\n', '\r']);
        let kind = if single {
            EditKind::Type
        } else {
            EditKind::Other
        };
        if self.overwrite && !text.contains(['\n', '\r']) {
            let n = text.chars().count();
            return self.edit_each(kind, |s, c| {
                let to = if c.is_empty() {
                    let len = s.doc.line_len(c.head.line);
                    Pos::new(c.head.line, (c.head.ch + n).min(len))
                } else {
                    c.end()
                };
                Some(Change {
                    from: c.start(),
                    to,
                    text: text.to_owned(),
                })
            });
        }
        self.insert(text, kind)
    }

    fn newline(&mut self) -> Result<(), FsError> {
        let le = self.doc.line_ending.as_str().to_owned();
        self.edit_each(EditKind::Other, |s, c| {
            let start = c.start();
            let indent = s.first_non_blank(start.line).min(start.ch);
            let prefix = s.doc.line_slice(start.line, 0, indent);
            Some(Change {
                from: start,
                to: c.end(),
                text: format!("{le}{prefix}"),
            })
        })
    }

    fn delete_back(&mut self, word: bool) -> Result<(), FsError> {
        let (spaces, tab) = (self.spaces, self.tab);
        self.edit_each(EditKind::Delete, |s, c| {
            if !c.is_empty() {
                return Some(Change {
                    from: c.start(),
                    to: c.end(),
                    text: String::new(),
                });
            }
            let p = c.head;
            if p == Pos::default() {
                return None;
            }
            let from = if word {
                s.word_left(p)
            } else if spaces && p.ch > 0 && s.first_non_blank(p.line) >= p.ch {
                // soft tabs: back to the previous tab stop
                let line = s.doc.line_slice(p.line, 0, p.ch);
                if line.chars().all(|c| c == ' ') {
                    let col = p.ch;
                    let back = match col % tab {
                        0 => tab,
                        r => r,
                    };
                    Pos::new(p.line, col - back.min(col))
                } else {
                    s.step_left(p)
                }
            } else {
                s.step_left(p)
            };
            Some(Change {
                from,
                to: p,
                text: String::new(),
            })
        })
    }

    fn delete_forward(&mut self, word: bool) -> Result<(), FsError> {
        self.edit_each(EditKind::Delete, |s, c| {
            if !c.is_empty() {
                return Some(Change {
                    from: c.start(),
                    to: c.end(),
                    text: String::new(),
                });
            }
            let to = if word {
                s.word_right(c.head)
            } else {
                s.step_right(c.head)
            };
            (to != c.head).then(|| Change {
                from: c.head,
                to,
                text: String::new(),
            })
        })
    }

    /// Lines a cursor covers for indent/outdent (a selection ending at column 0 leaves that line).
    fn lines_of(c: &Cursor) -> (usize, usize) {
        let (s, e) = (c.start(), c.end());
        let last = if e.ch == 0 && e.line > s.line {
            e.line - 1
        } else {
            e.line
        };
        (s.line, last)
    }

    fn indent_unit(&self) -> String {
        if self.spaces {
            " ".repeat(self.tab)
        } else {
            "\t".to_owned()
        }
    }

    fn indent(&mut self) -> Result<(), FsError> {
        let multi = self.cursors.iter().any(|c| c.start().line != c.end().line);
        if !multi {
            if self.spaces {
                // spaces up to the next tab stop
                let tab = self.tab;
                return self.edit_each(EditKind::Other, |s, c| {
                    let cell = s.cell_of(c.start());
                    let n = tab - (cell % tab);
                    Some(Change {
                        from: c.start(),
                        to: c.end(),
                        text: " ".repeat(n),
                    })
                });
            }
            return self.insert("\t", EditKind::Other);
        }
        let unit = self.indent_unit();
        let n = unit.chars().count();
        let mut lines: Vec<usize> = self
            .cursors
            .iter()
            .flat_map(|c| {
                let (a, b) = Self::lines_of(c);
                a..=b
            })
            .collect();
        lines.sort_unstable();
        lines.dedup();
        let changes = lines
            .iter()
            .filter(|&&l| self.doc.line_len(l) > 0)
            .map(|&l| Change {
                from: Pos::new(l, 0),
                to: Pos::new(l, 0),
                text: unit.clone(),
            })
            .collect();
        let shift = |p: Pos, doc: &Document| {
            if lines.binary_search(&p.line).is_ok() && doc.line_len(p.line) > 0 && p.ch > 0 {
                Pos::new(p.line, p.ch + n)
            } else {
                p
            }
        };
        let after: Vec<Sel> = self
            .cursors
            .iter()
            .map(|c| Sel {
                anchor: shift(c.anchor, &self.doc),
                head: shift(c.head, &self.doc),
            })
            .collect();
        self.apply_lines(changes, after)
    }

    fn outdent(&mut self) -> Result<(), FsError> {
        let mut lines: Vec<usize> = self
            .cursors
            .iter()
            .flat_map(|c| {
                let (a, b) = Self::lines_of(c);
                a..=b
            })
            .collect();
        lines.sort_unstable();
        lines.dedup();
        let mut removed = std::collections::HashMap::new();
        let mut changes = Vec::new();
        for &l in &lines {
            let text = self.doc.line_slice(l, 0, self.tab.max(1));
            let n = if text.starts_with('\t') {
                1
            } else {
                text.chars().take_while(|&c| c == ' ').count()
            };
            if n > 0 {
                removed.insert(l, n);
                changes.push(Change {
                    from: Pos::new(l, 0),
                    to: Pos::new(l, n),
                    text: String::new(),
                });
            }
        }
        let shift = |p: Pos| match removed.get(&p.line) {
            Some(&n) => Pos::new(p.line, p.ch.saturating_sub(n)),
            None => p,
        };
        let after = self
            .cursors
            .iter()
            .map(|c| Sel {
                anchor: shift(c.anchor),
                head: shift(c.head),
            })
            .collect();
        self.apply_lines(changes, after)
    }

    fn apply_lines(&mut self, changes: Vec<Change>, after: Vec<Sel>) -> Result<(), FsError> {
        if changes.is_empty() {
            return Ok(());
        }
        let sels = self.doc.apply(EditRequest {
            changes,
            kind: EditKind::Other,
            sel_before: self.sels(),
            sel_after: Some(after),
        })?;
        self.cursors_from(sels);
        Ok(())
    }

    /// Copy/cut; without a selection the whole line (with its break) – like most editors.
    fn copy(&mut self, clip: &mut dyn Clipboard, cut: bool) -> Result<(), FsError> {
        let le = self.doc.line_ending.as_str();
        let whole_lines = self.cursors.iter().all(Cursor::is_empty);
        let mut parts = Vec::new();
        let mut ranges = Vec::new();
        for c in &self.cursors {
            let (a, b) = if whole_lines {
                self.line_range(c.head.line)
            } else {
                (c.start(), c.end())
            };
            let mut text = self.doc.text(a, b)?;
            if whole_lines && b.line == a.line {
                // the last line has no break of its own
                text.push_str(le);
            }
            parts.push(text);
            ranges.push((a, b));
        }
        let text = parts.join(if whole_lines { "" } else { le });
        if !clip.set(text) {
            self.notice = Some("clipboard_failed".into());
            return Ok(());
        }
        if cut {
            let mut ranges = ranges;
            ranges.dedup();
            let changes = ranges
                .into_iter()
                .map(|(from, to)| Change {
                    from,
                    to,
                    text: String::new(),
                })
                .collect();
            let sels = self.doc.apply(EditRequest {
                changes,
                kind: EditKind::Other,
                sel_before: self.sels(),
                sel_after: None,
            })?;
            self.cursors_from(sels);
        }
        Ok(())
    }

    // ------------------------------------------------------------------ search (design 006)

    /// Compiles and keeps the query (highlights); `false` = bad pattern (notice set).
    fn set_search(&mut self, query: Query) -> bool {
        match search::compile(&query) {
            Ok(re) => {
                self.search = Some((query, re));
                true
            }
            Err(_) => {
                self.search = None;
                self.notice = Some("invalid_search".into());
                false
            }
        }
    }

    fn select_chars(&mut self, (s, e): (usize, usize)) {
        self.cursors = vec![Cursor {
            anchor: self.doc.char_to_pos(s),
            head: self.doc.char_to_pos(e),
            goal: None,
        }];
    }

    /// The primary selection as characters.
    fn primary_chars(&self) -> (usize, usize) {
        let c = self.primary();
        (
            self.doc.pos_to_char(c.start()),
            self.doc.pos_to_char(c.end()),
        )
    }

    fn find(&mut self, query: Query, backwards: bool, incremental: bool) {
        if !self.set_search(query) {
            return;
        }
        let Some((_, re)) = &self.search else { return };
        let (s, e) = self.primary_chars();
        let rope = self.doc.rope();
        let found = if backwards {
            search::find_prev(rope, re, s)
        } else {
            search::find_next(rope, re, if incremental { s } else { e })
        };
        match found {
            Some(m) => self.select_chars(m),
            None => self.notice = Some("no_match".into()),
        }
    }

    /// Is the primary selection exactly a match?
    fn selection_is_match(&self, re: &Regex) -> bool {
        let (s, e) = self.primary_chars();
        e > s && search::find_next(self.doc.rope(), re, s) == Some((s, e))
    }

    fn replace(&mut self, query: Query, replacement: &str) -> Result<(), FsError> {
        if !self.set_search(query) {
            return Ok(());
        }
        let Some((q, re)) = self.search.clone() else {
            return Ok(());
        };
        if self.selection_is_match(&re) {
            let (s, e) = self.primary_chars();
            // groups in context: the lines around the match (lookaround-free regexes still
            // depend on \b, ^, $ …)
            let rope = self.doc.rope();
            let ctx_from = rope.line_to_char(rope.char_to_line(s));
            let ctx_to = rope
                .line_to_char((rope.char_to_line(e) + 2).min(rope.len_lines()))
                .max(e);
            let ctx = rope.slice(ctx_from..ctx_to).to_string();
            let at = rope.slice(ctx_from..s).len_bytes();
            let text = search::expand_at(&re, &ctx, at, replacement, q.regex);
            self.insert(&text, EditKind::Other)?;
        }
        self.find(q, false, false);
        Ok(())
    }

    fn replace_all(&mut self, query: Query, replacement: &str) -> Result<(), FsError> {
        if !self.set_search(query) {
            return Ok(());
        }
        let Some((q, re)) = self.search.clone() else {
            return Ok(());
        };
        let never = std::sync::atomic::AtomicBool::new(false);
        let found = search::replacements(self.doc.rope(), &re, replacement, q.regex, &never)
            .unwrap_or_default();
        if found.is_empty() {
            self.notice = Some("no_match".into());
            return Ok(());
        }
        let n = found.len();
        let first = self.primary();
        // character offsets: a match may start or end inside a CRLF
        self.doc.apply_chars(
            found,
            self.sels(),
            Some(vec![Sel::caret(first.start())]),
            EditKind::Other,
        )?;
        self.cursors = vec![Cursor::caret(self.doc.clamp(first.start()))];
        self.notice = Some(format!("replaced:{n}"));
        Ok(())
    }

    /// Snapshot for the overview (design 010 E3): text, search (bit 1) and markers (bit 2).
    pub fn overview_snapshot(&self) -> (ropey::Rope, Vec<(Regex, u8)>) {
        let mut layers = Vec::new();
        if let Some((_, re)) = &self.search {
            layers.push((re.clone(), 1));
        }
        for m in self.markers.iter().flatten() {
            layers.push((m.1.clone(), 2));
        }
        (self.doc.rope().clone(), layers)
    }

    /// Snapshot for counting matches outside the session lock (a rope clone is O(1)):
    /// text, query and the primary selection in characters.
    pub fn search_snapshot(&self) -> (ropey::Rope, usize, usize) {
        let (s, e) = self.primary_chars();
        (self.doc.rope().clone(), s, e)
    }

    // ------------------------------------------------------------------ cursors & scrolling

    /// Sorts cursors and merges overlapping ones (the first stays primary when nothing merges).
    fn normalize(&mut self) {
        for c in &mut self.cursors {
            c.anchor = self.doc.clamp(c.anchor);
            c.head = self.doc.clamp(c.head);
        }
        if self.cursors.len() < 2 {
            self.primary = 0;
            return;
        }
        let key = self.primary().head;
        let mut cs = self.cursors.clone();
        cs.sort_by_key(|c| c.start());
        let mut out: Vec<Cursor> = Vec::with_capacity(cs.len());
        for c in cs {
            if let Some(last) = out.last_mut()
                && (c.start() < last.end() || c.start() == last.start())
            {
                let end = last.end().max(c.end());
                let start = last.start();
                *last = if last.head >= last.anchor {
                    Cursor {
                        anchor: start,
                        head: end,
                        goal: None,
                    }
                } else {
                    Cursor {
                        anchor: end,
                        head: start,
                        goal: None,
                    }
                };
                continue;
            }
            out.push(c);
        }
        self.primary = out
            .iter()
            .position(|c| c.start() <= key && key <= c.end())
            .unwrap_or(0);
        self.cursors = out;
    }

    fn clamp_scroll(&mut self) {
        let max_top = self.doc.line_count().saturating_sub(1) as f64;
        self.top = self.top.clamp(0.0, max_top);
        let max_left = (self.max_width as f64 - self.view_cols + 2.0).max(0.0);
        if self.wrap && !self.doc.is_long(self.primary().head.line) {
            self.left = 0.0;
            return;
        }
        // the caret may sit right of everything seen so far (end of a long line)
        let caret = self.cell_of(self.primary().head) as f64;
        self.left = self.left.clamp(0.0, max_left.max(caret));
    }

    /// Scrolls so the primary caret is visible.
    fn reveal(&mut self, margin: f64) {
        let head = self.primary().head;
        let line = head.line as f64;
        let rows = self.view_lines.floor().max(1.0);
        if self.wrap {
            let c = self.caret_vpos(head);
            let top = self.top_vpos();
            if vless(c, top) {
                self.set_top_vpos(c);
            } else {
                let bottom = self.advance(c, -(rows - 1.0));
                if vless(top, bottom) {
                    self.set_top_vpos(bottom);
                }
            }
        } else if line < self.top {
            self.top = line;
        } else if line + 1.0 > self.top + rows {
            self.top = line + 1.0 - rows;
        }
        if self.wrap && !self.doc.is_long(head.line) {
            self.left = 0.0;
            return;
        }
        let cell = self.cell_of(head) as f64;
        let cols = self.view_cols.floor().max(1.0);
        let m = margin.min((cols / 4.0).floor());
        if cell < self.left + m {
            self.left = (cell - m).max(0.0);
        } else if cell > self.left + cols - 1.0 - m {
            self.left = cell - cols + 1.0 + m;
        }
        self.top = self.top.max(0.0);
        self.left = self.left.max(0.0);
    }

    // ------------------------------------------------------------------ frame

    /// The frame for the current state (design 003 E1).
    pub fn frame(&mut self) -> Frame {
        self.seq += 1;
        let count = self.doc.line_count();
        let first = (self.top.floor() as usize).min(count.saturating_sub(1));
        let rows = self.view_lines.ceil() as usize + 1;
        // soft wrap: lines until the rows are filled (each line at least one row)
        let (first_row, end) = if self.wrap {
            let (_, r) = self.top_vpos();
            let mut filled = -r;
            let mut l = first;
            while l < count && filled < rows as f64 {
                filled += self.rows(l) as f64;
                l += 1;
            }
            (-r, l)
        } else {
            (first as f64 - self.top, (first + rows).min(count))
        };
        let wa = (self.left.floor() as usize).saturating_sub(WINDOW_SLACK);
        let wb = (self.left + self.view_cols).ceil() as usize + WINDOW_SLACK;
        let mut lines = Vec::with_capacity(end - first);
        let changed = self.doc.take_changed_from();
        let (styles, approx) = match &mut self.highlighter {
            Some(h) => {
                if let Some(l) = changed {
                    h.invalidate_from(l);
                }
                h.styles(&self.doc, first, end)
            }
            None => (Vec::new(), false),
        };
        let mut indent_above = 0;
        let mut row = first_row;
        if self.wrap {
            // wrapped: only the lines in view count (long ones scroll sideways)
            self.max_width = 0;
        }
        for line in first..end {
            let st = styles.get(line - first).cloned().flatten();
            let breaks = self.breaks(line);
            let (a, b) = if breaks.is_empty() && !self.wrap || self.doc.is_long(line) {
                (wa, wb)
            } else {
                (0, usize::MAX)
            };
            let mut fl = self.frame_line(line, a, b, st.as_deref(), &breaks);
            fl.row = row;
            row += (breaks.len() + 1) as f64;
            if self.wrap && !fl.long {
                // wrapped lines never scroll sideways
                fl.width = fl.width.min(self.wrap_width());
            }
            fl.wraps = breaks;
            if !fl.colors.is_empty() {
                self.color_column = true;
            }
            if fl.indent == usize::MAX {
                fl.indent = indent_above;
            } else {
                indent_above = fl.indent;
            }
            self.max_width = self.max_width.max(fl.width);
            lines.push(fl);
        }
        let primary = self.primary();
        let carets: Vec<CaretPos> = self
            .cursors
            .iter()
            .map(|c| CaretPos {
                line: c.head.line,
                cell: self.cell_of(c.head),
            })
            .collect();
        let caret_xy = carets
            .iter()
            .map(|c| match lines.iter().find(|l| l.line == c.line) {
                Some(l) => {
                    let r = l.wraps.partition_point(|&b| b <= c.cell);
                    let start = if r == 0 { 0 } else { l.wraps[r - 1] };
                    CaretXY {
                        row: l.row + r as f64,
                        x: c.cell - start,
                    }
                }
                None => CaretXY {
                    row: if self.wrap {
                        if c.line < first {
                            -2.0
                        } else {
                            rows as f64 + 2.0
                        }
                    } else {
                        c.line as f64 - self.top
                    },
                    x: c.cell,
                },
            })
            .collect();
        let sel_chars = self
            .cursors
            .iter()
            .map(|c| self.doc.pos_to_char(c.end()) - self.doc.pos_to_char(c.start()))
            .sum();
        let sel_lines = if primary.is_empty() {
            0
        } else {
            primary.end().line - primary.start().line + 1
        };
        Frame {
            seq: self.seq,
            info: self.doc.info(self.id),
            top: self.top,
            left: self.left,
            first_line: first,
            lines,
            max_width: self.max_width,
            carets,
            caret_xy,
            primary: self.primary.min(self.cursors.len() - 1),
            caret_line: primary.head.line,
            caret_ch: primary.head.ch,
            sel_chars,
            sel_lines,
            overwrite: self.overwrite,
            tab_width: self.tab,
            insert_spaces: self.spaces,
            notice: self.notice.clone(),
            search: self.search.is_some(),
            multi: self.cursors.len() > 1,
            language: self.highlighter.as_ref().map(|h| h.language.clone()),
            approx_highlight: approx,
            selection_text: self.selection_text(),
            wrap: self.wrap,
            deco: self.deco(),
            color_column: self.color_column,
            caret_color: self.color_at_caret().map(|h| colors::to_hash(h.rgba)),
            marker_words: self
                .markers
                .iter()
                .map(|m| m.as_ref().map(|(t, _)| t.clone()))
                .collect(),
            caret_marker: self.marker_at_caret().map(|(i, _, _)| i),
            hint: self.math_hint(),
            completion: self.completion.as_ref().map(|c| model::Completion {
                line: c.line,
                // words: below the start of the word, steady while typing (design 023 E14)
                cell: match c.items.first() {
                    Some(s) if c.words => self.cell_of(Pos::new(c.line, s.from)),
                    _ => self.cell_of(self.primary().head),
                },
                items: c
                    .items
                    .iter()
                    .map(|s| model::CompletionItem {
                        kind: s.kind.into(),
                        label: s.label.clone(),
                    })
                    .collect(),
            }),
            read_only: self.read_only,
        }
    }

    /// The expression at the primary caret (one caret, no long line).
    fn math_at_caret(&self) -> Option<(usize, hints::Found)> {
        if self.cursors.len() != 1 {
            return None;
        }
        let c = self.primary();
        let line = c.head.line;
        if self.doc.is_long(line) || (!c.is_empty() && c.start().line != c.end().line) {
            return None;
        }
        let text = self.doc.line_string(line);
        let sel = (!c.is_empty()).then(|| (c.start().ch, c.end().ch));
        // variables: `name = …` in the lines above
        let from = line.saturating_sub(200);
        let above: Vec<String> = (from..line)
            .map(|l| self.doc.line_slice(l, 0, 200))
            .filter(|t| t.contains('='))
            .collect();
        let vars = hints::vars_of(above.iter().map(String::as_str));
        hints::find(&text, c.head.ch, sel, &vars).map(|f| (line, f))
    }

    fn math_hint(&self) -> Option<model::MathHint> {
        if !self.math.hints || self.completion.is_some() {
            return None;
        }
        let (line, f) = self.math_at_caret()?;
        Some(model::MathHint {
            line,
            cell: self.cell_of(Pos::new(line, f.from)),
            expr: f.shown,
            result: (!f.value.is_nan()).then(|| hints::result_text(f.value, &self.math)),
            hex: hints::hex(f.value),
        })
    }

    /// The suggestions for the expression at the caret, `None` = no expression or none.
    fn math_suggestions(&self) -> Option<(usize, Vec<hints::Suggestion>)> {
        let (line, f) = self.math_at_caret()?;
        let items = hints::suggestions(&f, self.primary().head.ch, &self.math);
        (!items.is_empty()).then_some((line, items))
    }

    /// Opens the suggestions for the expression at the caret (after a typed `=`).
    fn open_completion(&mut self, notice: bool) {
        self.completion = self.math_suggestions().map(|(line, items)| OpenCompletion {
            line,
            items,
            revision: self.doc.revision(),
            words: false,
        });
        if self.completion.is_none() && notice {
            self.notice = Some("no_expression".into());
        }
    }

    /// "Show suggestions": the arithmetic at the caret first (design 023 E2), else the words; a
    /// single word goes in right away (E7). `true` = the text changed.
    fn complete_manually(&mut self) -> Result<bool, FsError> {
        self.open_completion(false);
        if self.completion.is_some() {
            return Ok(false);
        }
        match self.word_completion(1) {
            Some((line, items)) if items.len() == 1 => {
                let s = &items[0];
                self.replace_in_line(line, s.from, s.to, &s.text)?;
                Ok(true)
            }
            Some((line, items)) if !items.is_empty() => {
                self.completion = Some(OpenCompletion {
                    line,
                    items,
                    revision: self.doc.revision(),
                    words: true,
                });
                Ok(false)
            }
            _ => {
                self.notice = Some("no_suggestions".into());
                Ok(false)
            }
        }
    }

    /// The lexicon of the document: by its extension and language (design 023 E6), with the
    /// user's own lists (E13).
    fn word_lexicon(&self) -> Option<std::sync::Arc<complete::Lexicon>> {
        let name = self
            .doc
            .path
            .as_deref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let language = self.highlighter.as_ref().map(|h| h.language.as_str());
        complete::lexicon(&complete::key_for(name.as_deref(), language))
    }

    /// Characters that also belong to words in this language (CSS: `-`).
    fn word_extra(&self) -> String {
        self.word_lexicon()
            .map(|l| l.extra.clone())
            .unwrap_or_default()
    }

    /// The words for the prefix at the primary caret (at least `min` characters): line and
    /// suggestions; `None` = no prefix (one caret without selection, no long line).
    fn word_completion(&self, min: usize) -> Option<(usize, Vec<hints::Suggestion>)> {
        if self.cursors.len() != 1 || !self.primary().is_empty() {
            return None;
        }
        let p = self.primary().head;
        if self.doc.is_long(p.line) {
            return None;
        }
        let lex = self.word_lexicon();
        let extra = lex.as_ref().map(|l| l.extra.clone()).unwrap_or_default();
        let extra = extra.as_str();
        let text = self.doc.line_string(p.line);
        let chars: Vec<char> = text.chars().collect();
        let caret = p.ch.min(chars.len());
        let start = complete::prefix_start(&chars, caret, extra)?;
        if caret - start < min {
            return None;
        }
        let prefix: String = chars[start..caret].iter().collect();
        let mut end = caret;
        while end < chars.len() && complete::is_word(chars[end], extra) {
            end += 1;
        }
        // the lines around the caret, nearest first, bounded (design 023 E4)
        let mut budget = complete::BUDGET_CHARS;
        let lines: Vec<(String, Option<(usize, usize)>)> =
            complete::nearest_lines(p.line, self.doc.line_count())
                .map_while(|l| {
                    if budget == 0 {
                        return None;
                    }
                    let (t, skip) = if l == p.line {
                        (text.clone(), Some((start, end)))
                    } else {
                        (self.doc.line_slice(l, 0, complete::LINE_CHARS), None)
                    };
                    budget = budget.saturating_sub(t.len().max(1));
                    Some((t, skip))
                })
                .collect();
        let items = complete::candidates(
            &prefix,
            lines.iter().map(|(t, s)| (t.as_str(), *s)),
            lex.as_deref().filter(|_| self.words.keywords),
            extra,
        )
        .into_iter()
        .map(|c| hints::Suggestion {
            kind: if c.keyword { "keyword" } else { "word" },
            label: c.word.clone(),
            from: start,
            to: caret,
            text: c.word,
        })
        .collect();
        Some((p.line, items))
    }

    fn apply_completion(&mut self, index: usize) -> Result<(), FsError> {
        let Some(c) = self.completion.take() else {
            return Ok(());
        };
        if c.revision != self.doc.revision() {
            return Ok(());
        }
        let Some(s) = c.items.get(index) else {
            return Ok(());
        };
        self.replace_in_line(c.line, s.from, s.to, &s.text.clone())
    }

    /// Replaces characters `from..to` of a line, caret behind the new text, one undo step.
    fn replace_in_line(
        &mut self,
        line: usize,
        from: usize,
        to: usize,
        text: &str,
    ) -> Result<(), FsError> {
        let end = Pos::new(line, from + text.chars().count());
        let sels = self.doc.apply(EditRequest {
            changes: vec![Change {
                from: Pos::new(line, from),
                to: Pos::new(line, to),
                text: text.to_owned(),
            }],
            kind: EditKind::Other,
            sel_before: self.sels(),
            sel_after: Some(vec![Sel::caret(end)]),
        })?;
        self.cursors_from(sels);
        Ok(())
    }

    /// The regex of the marked word at the primary caret and a snapshot of the text – counted
    /// off the session lock (status bar, design 019).
    pub fn marker_snapshot(&self) -> Option<(ropey::Rope, Regex)> {
        let (i, _, _) = self.marker_at_caret()?;
        let (_, re) = self.markers.get(i)?.clone()?;
        Some((self.doc.rope().clone(), re))
    }

    /// The colour value the primary caret is in or touches (design 018).
    fn color_at_caret(&self) -> Option<colors::ColorHit> {
        if !self.colors {
            return None;
        }
        let p = self.primary().head;
        let text = self.doc.line_slice(p.line, 0, colors::SCAN_CHARS);
        colors::find(&text)
            .into_iter()
            .find(|h| h.from <= p.ch && p.ch <= h.to)
    }

    /// "Convert colour value": the next notation, as one undo step; selects the new value.
    fn convert_color(&mut self) -> Result<(), FsError> {
        let Some(hit) = self.color_at_caret() else {
            self.notice = Some("no_color".into());
            return Ok(());
        };
        let line = self.primary().head.line;
        let text = colors::convert(&hit);
        let n = text.chars().count();
        let (from, to) = (Pos::new(line, hit.from), Pos::new(line, hit.to));
        let sels = self.doc.apply(EditRequest {
            changes: vec![Change { from, to, text }],
            kind: EditKind::Other,
            sel_before: self.sels(),
            sel_after: Some(vec![Sel {
                anchor: from,
                head: Pos::new(line, hit.from + n),
            }]),
        })?;
        self.cursors_from(sels);
        Ok(())
    }

    /// Short single-line selections go to the UI (find field).
    fn selection_text(&self) -> Option<String> {
        let c = self.primary();
        if c.is_empty() || c.start().line != c.end().line || c.end().ch - c.start().ch > 1_000 {
            return None;
        }
        self.doc.text(c.start(), c.end()).ok()
    }

    fn frame_line(
        &self,
        line: usize,
        wa: usize,
        wb: usize,
        styles: Option<&[(usize, u8)]>,
        breaks: &[usize],
    ) -> FrameLine {
        let len = self.doc.line_len(line);
        let long = self.doc.is_long(line);
        let mut found: Vec<(usize, usize)> = Vec::new();
        let mut marked: Vec<(usize, usize, usize)> = Vec::new();
        let mut indent = 0;
        let (runs, width, cell_of): (Vec<Run>, usize, CellOf) = if long {
            let from = wa.min(len);
            let text = self.doc.line_slice(line, from, wb);
            if !self.markers.is_empty() {
                marked = self.markers_in(&text, from);
            }
            if let Some((_, re)) = &self.search {
                found = search::matches_in(&text, re)
                    .into_iter()
                    .map(|(a, b)| (a + from, b + from))
                    .collect();
            }
            (
                runs_of(text.chars(), from, |_, _| 1, |_| 0, &[]),
                len,
                Box::new(|ch| ch),
            )
        } else {
            let line_text = self.doc.line_string(line);
            if let Some((_, re)) = &self.search {
                found = search::matches_in(&line_text, re);
            }
            if !self.markers.is_empty() {
                marked = self.markers_in(&line_text, 0);
            }
            let lay = LineLayout::new(&line_text, self.tab);
            let width = lay.width();
            let lead = line_text
                .chars()
                .take_while(|&c| c == ' ' || c == '\t')
                .count();
            indent = if lead == lay.len() {
                usize::MAX
            } else {
                lay.cell_of(lead)
            };
            // characters touching the window
            let first = lay.cells.partition_point(|&c| c < wa).saturating_sub(1);
            let last = lay.cells.partition_point(|&c| c <= wb).min(lay.len());
            let start_cell = lay.cells[first.min(lay.len())];
            let chars = lay.chars[first.min(lay.len())..last.max(first).min(lay.len())]
                .iter()
                .copied();
            let tab = self.tab;
            let style = char_styles(
                &line_text,
                styles,
                first.min(lay.len()),
                last.max(first).min(lay.len()),
            );
            let runs = runs_of(
                chars,
                start_cell,
                |c, col| char_width(c, col, tab),
                |i| style.get(i).copied().unwrap_or(0),
                breaks,
            );
            (runs, width, Box::new(move |ch| lay.cell_of(ch)))
        };
        let mut sel = Vec::new();
        let mut sel_eol = false;
        for c in &self.cursors {
            if c.is_empty() {
                continue;
            }
            let (s, e) = (c.start(), c.end());
            if line < s.line || line > e.line {
                continue;
            }
            let a = if line == s.line { s.ch } else { 0 };
            let b = if line == e.line { e.ch } else { len };
            let (ca, cb) = (cell_of(a), cell_of(b));
            if cb > ca {
                sel.push([ca, cb]);
            }
            if line < e.line {
                sel_eol = true;
            }
        }
        let primary = self.primary();
        let marks = found
            .into_iter()
            .map(|(a, b)| {
                let current =
                    primary.start() == Pos::new(line, a) && primary.end() == Pos::new(line, b);
                [cell_of(a), cell_of(b), usize::from(current)]
            })
            .collect();
        let markers = marked
            .into_iter()
            .map(|(a, b, c)| [cell_of(a), cell_of(b), c])
            .collect();
        FrameLine {
            line,
            runs,
            sel,
            sel_eol,
            width,
            long,
            marks,
            markers,
            indent,
            row: 0.0,
            wraps: Vec::new(),
            colors: if self.colors {
                let text = if long {
                    self.doc.line_slice(line, 0, colors::SCAN_CHARS)
                } else {
                    self.doc.line_string(line)
                };
                colors::find(&text)
                    .into_iter()
                    .map(|h| [h.from as u32, h.to as u32, h.rgba])
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    // ------------------------------------------------------------------ soft wrap (design 010 E4)

    /// Width of a wrapped row in cells.
    fn wrap_width(&self) -> usize {
        (self.view_cols.floor() as usize).saturating_sub(1).max(8)
    }

    /// Row starts of a line after the first; empty without wrap and for long lines.
    fn breaks(&self, line: usize) -> Vec<usize> {
        if !self.wrap || line >= self.doc.line_count() {
            return Vec::new();
        }
        match self.layout(line) {
            Some(lay) if lay.width() > self.wrap_width() => wrap_points(&lay, self.wrap_width()),
            _ => Vec::new(),
        }
    }

    fn rows(&self, line: usize) -> usize {
        self.breaks(line).len() + 1
    }

    fn top_vpos(&self) -> VPos {
        let line = (self.top.floor() as usize).min(self.doc.line_count().saturating_sub(1));
        let frac = (self.top - line as f64).clamp(0.0, 0.999_999);
        let r = frac * self.rows(line) as f64;
        // the fraction loses a little: whole rows stay whole
        (
            line,
            if (r - r.round()).abs() < 1e-6 {
                r.round()
            } else {
                r
            },
        )
    }

    fn set_top_vpos(&mut self, (line, row): VPos) {
        let rows = self.rows(line) as f64;
        self.top = line as f64 + (row / rows).clamp(0.0, 0.999_999);
        if row <= 0.0 {
            self.top = line as f64;
        }
    }

    fn caret_vpos(&self, p: Pos) -> VPos {
        let cell = self.cell_of(p);
        let row = self.breaks(p.line).partition_point(|&b| b <= cell);
        (p.line, row as f64)
    }

    /// `d` rows down (negative: up) from `v`, within the document.
    fn advance(&self, (mut line, mut row): VPos, d: f64) -> VPos {
        let last = self.doc.line_count().saturating_sub(1);
        if d >= 0.0 {
            let mut d = d;
            loop {
                let r = self.rows(line) as f64;
                if row + d < r {
                    return (line, row + d);
                }
                if line >= last {
                    return (last, (r - 1.0).max(0.0));
                }
                d -= r - row;
                line += 1;
                row = 0.0;
            }
        } else {
            let mut d = -d;
            loop {
                if row - d >= 0.0 {
                    return (line, row - d);
                }
                if line == 0 {
                    return (0, 0.0);
                }
                d -= row;
                line -= 1;
                row = self.rows(line) as f64;
            }
        }
    }

    /// Identity of search and markers (the overview asks again when it changes).
    fn deco(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        if self.search.is_none() && self.markers.iter().all(Option::is_none) {
            return 0;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        if let Some((_, re)) = &self.search {
            re.as_str().hash(&mut h);
        }
        for m in self.markers.iter().flatten() {
            m.1.as_str().hash(&mut h);
        }
        h.finish()
    }
}

/// `a` comes before `b` in the view.
fn vless(a: VPos, b: VPos) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 < b.1 - 1e-9)
}

/// Character index in a line → cell.
type CellOf = Box<dyn Fn(usize) -> usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reveal {
    No,
    Margin,
    Exact,
    /// Jumps far away (find): the target line lands in the middle of the view.
    Center,
}

/// Groups characters into runs of one kind; `width(c, col)` gives the cells of a character.
/// Styles of the characters `[from, to)` of a line from its byte-offset spans.
fn char_styles(text: &str, spans: Option<&[(usize, u8)]>, from: usize, to: usize) -> Vec<u8> {
    let Some(spans) = spans else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(to.saturating_sub(from));
    let mut si = 0;
    let mut cur = 0u8;
    for (i, (b, _)) in text.char_indices().enumerate() {
        if i >= to {
            break;
        }
        while si < spans.len() && spans[si].0 <= b {
            cur = spans[si].1;
            si += 1;
        }
        if i >= from {
            out.push(cur);
        }
    }
    out
}

/// Groups characters into runs of one kind and style; `width(c, col)` gives the cells of a
/// character, `style(i)` its style class (i = index in `chars`).
fn runs_of(
    chars: impl Iterator<Item = char>,
    start_cell: usize,
    width: impl Fn(char, usize) -> usize,
    style: impl Fn(usize) -> u8,
    breaks: &[usize],
) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut col = start_cell;
    for (i, c) in chars.enumerate() {
        let st = style(i);
        let row_start = breaks.binary_search(&col).is_ok();
        let w = width(c, col);
        let kind = match c {
            '\t' => RunKind::Tab,
            c if (c as u32) < 0x20 || c == '\u{7f}' => RunKind::Ctrl,
            _ if w == 2 => RunKind::Wide,
            _ => RunKind::Text,
        };
        let glyph = match kind {
            RunKind::Tab => None,
            RunKind::Ctrl => Some(control_picture(c)),
            _ => Some(c),
        };
        // zero-width characters stick to the run before
        if w == 0
            && let Some(last) = runs.last_mut()
        {
            last.text.push(c);
            continue;
        }
        match runs.last_mut() {
            Some(last)
                if last.kind == kind && kind != RunKind::Tab && last.style == st && !row_start =>
            {
                if let Some(g) = glyph {
                    last.text.push(g);
                }
                last.width += w;
            }
            _ => runs.push(Run {
                cell: col,
                width: w,
                kind,
                text: glyph.map(String::from).unwrap_or_default(),
                style: st,
            }),
        }
        col += w;
    }
    runs
}

/// U+2400… pictures for C0 controls, ␡ for DEL.
fn control_picture(c: char) -> char {
    match c as u32 {
        0x7f => '\u{2421}',
        n if n < 0x20 => char::from_u32(0x2400 + n).unwrap_or('?'),
        _ => c,
    }
}
