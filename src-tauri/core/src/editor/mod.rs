//! The editor session (design 003): a document plus cursors, scroll position, viewport and
//! settings. It runs every command of the UI and answers with a [`model::Frame`] – the UI only
//! renders (Marc, 26.10.2026; design 002 E13).

pub mod layout;
pub mod model;
mod words;

#[cfg(test)]
mod tests;

use crate::document::Document;
use crate::document::model::{Change, EditKind, EditRequest, Pos, Sel};
use crate::error::FsError;
use layout::{LineLayout, char_width, next_grapheme, prev_grapheme};
use model::{CaretPos, Command, Frame, FrameLine, Motion, Run, RunKind};
use words::CharClass;

/// The system clipboard as the session sees it; the app provides the real one, tests a fake.
pub trait Clipboard {
    fn get(&mut self) -> Option<String>;
    fn set(&mut self, text: String) -> bool;
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
}

impl Session {
    pub fn new(doc: Document, id: u64) -> Self {
        let (spaces, tab) = words::detect_indent(&doc);
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
        }
    }

    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn set_cursors(&mut self, cursors: Vec<Cursor>) {
        self.cursors = cursors;
        self.normalize();
    }

    pub fn scroll(&self) -> (f64, f64) {
        (self.top, self.left)
    }

    /// Runs one command and returns the frame to draw.
    pub fn run(&mut self, cmd: Command, clip: &mut dyn Clipboard) -> Result<Frame, FsError> {
        self.notice = None;
        let mut reveal = Reveal::Margin;
        match cmd {
            Command::Frame => reveal = Reveal::No,
            Command::Resize { lines, cols } => {
                self.view_lines = lines.max(1.0);
                self.view_cols = cols.max(1.0);
                reveal = Reveal::No;
            }
            Command::Configure {
                tab_width,
                insert_spaces,
            } => {
                if let Some(t) = tab_width {
                    self.tab = t.clamp(1, 16);
                }
                if let Some(s) = insert_spaces {
                    self.spaces = s;
                }
                reveal = Reveal::No;
            }
            Command::Scroll { lines, cells } => {
                self.top += lines;
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
            Command::Click {
                line,
                cell,
                count,
                extend,
            } => {
                self.click(line, cell, count, extend);
                reveal = Reveal::Exact;
            }
            Command::Drag { line, cell } => {
                self.drag_to(line, cell);
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
                Some(text) => self.insert(&text, EditKind::Other)?,
                None => self.notice = Some("clipboard_empty".into()),
            },
        }
        self.clamp_scroll();
        match reveal {
            Reveal::No => {}
            Reveal::Margin => self.reveal(H_MARGIN),
            Reveal::Exact => self.reveal(0.0),
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

    fn pos_at(&self, line: f64, cell: f64) -> Pos {
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

    fn move_cursors(&mut self, to: Motion, select: bool) {
        if matches!(to, Motion::PageUp | Motion::PageDown) {
            let n = self.page() as f64;
            self.top += if to == Motion::PageDown { n } else { -n };
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
            let anchor = self.cursors[0].anchor;
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

    fn drag_to(&mut self, line: f64, cell: f64) {
        let p = self.pos_at(line, cell);
        let origin = self.drag.unwrap_or(DragOrigin {
            count: 1,
            from: self.cursors[0].anchor,
            to: self.cursors[0].anchor,
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

    // ------------------------------------------------------------------ cursors & scrolling

    /// Sorts cursors and merges overlapping ones (the first stays primary when nothing merges).
    fn normalize(&mut self) {
        for c in &mut self.cursors {
            c.anchor = self.doc.clamp(c.anchor);
            c.head = self.doc.clamp(c.head);
        }
        if self.cursors.len() < 2 {
            return;
        }
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
        self.cursors = out;
    }

    fn clamp_scroll(&mut self) {
        let max_top = self.doc.line_count().saturating_sub(1) as f64;
        self.top = self.top.clamp(0.0, max_top);
        let max_left = (self.max_width as f64 - self.view_cols + 2.0).max(0.0);
        // the caret may sit right of everything seen so far (end of a long line)
        let caret = self.cell_of(self.cursors[0].head) as f64;
        self.left = self.left.clamp(0.0, max_left.max(caret));
    }

    /// Scrolls so the primary caret is visible.
    fn reveal(&mut self, margin: f64) {
        let head = self.cursors[0].head;
        let line = head.line as f64;
        let rows = self.view_lines.floor().max(1.0);
        if line < self.top {
            self.top = line;
        } else if line + 1.0 > self.top + rows {
            self.top = line + 1.0 - rows;
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
        let end = (first + rows).min(count);
        let wa = (self.left.floor() as usize).saturating_sub(WINDOW_SLACK);
        let wb = (self.left + self.view_cols).ceil() as usize + WINDOW_SLACK;
        let mut lines = Vec::with_capacity(end - first);
        for line in first..end {
            let fl = self.frame_line(line, wa, wb);
            self.max_width = self.max_width.max(fl.width);
            lines.push(fl);
        }
        let primary = self.cursors[0];
        let carets = self
            .cursors
            .iter()
            .map(|c| CaretPos {
                line: c.head.line,
                cell: self.cell_of(c.head),
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
            caret_line: primary.head.line,
            caret_ch: primary.head.ch,
            sel_chars,
            sel_lines,
            overwrite: self.overwrite,
            tab_width: self.tab,
            insert_spaces: self.spaces,
            notice: self.notice.clone(),
        }
    }

    fn frame_line(&self, line: usize, wa: usize, wb: usize) -> FrameLine {
        let len = self.doc.line_len(line);
        let long = self.doc.is_long(line);
        let (runs, width, cell_of): (Vec<Run>, usize, CellOf) = if long {
            let from = wa.min(len);
            let text = self.doc.line_slice(line, from, wb);
            (
                runs_of(text.chars(), from, |_, _| 1),
                len,
                Box::new(|ch| ch),
            )
        } else {
            let lay = LineLayout::new(&self.doc.line_string(line), self.tab);
            let width = lay.width();
            // characters touching the window
            let first = lay.cells.partition_point(|&c| c < wa).saturating_sub(1);
            let last = lay.cells.partition_point(|&c| c <= wb).min(lay.len());
            let start_cell = lay.cells[first.min(lay.len())];
            let chars = lay.chars[first.min(lay.len())..last.max(first).min(lay.len())]
                .iter()
                .copied();
            let tab = self.tab;
            let runs = runs_of(chars, start_cell, |c, col| char_width(c, col, tab));
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
        FrameLine {
            line,
            runs,
            sel,
            sel_eol,
            width,
            long,
        }
    }
}

/// Character index in a line → cell.
type CellOf = Box<dyn Fn(usize) -> usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reveal {
    No,
    Margin,
    Exact,
}

/// Groups characters into runs of one kind; `width(c, col)` gives the cells of a character.
fn runs_of(
    chars: impl Iterator<Item = char>,
    start_cell: usize,
    width: impl Fn(char, usize) -> usize,
) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut col = start_cell;
    for c in chars {
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
            Some(last) if last.kind == kind && kind != RunKind::Tab => {
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
