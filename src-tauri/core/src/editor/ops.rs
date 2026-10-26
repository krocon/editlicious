//! Line and text operations of the session (design 007): sort, case, duplicate/move/delete/join
//! lines, whitespace, column selection, more cursors and markers. Each edit is one undo step.

use super::{Clipboard, Cursor, Session};
use crate::document::model::{Change, EditKind, EditRequest, LineEnding, Pos, Sel};
use crate::error::FsError;
use crate::search::{self, Query};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// At most this many lines in a column selection (one cursor each).
pub const MAX_BLOCK_LINES: usize = 10_000;
/// Marker colours (legacy: "up to 10 words").
pub const MAX_MARKERS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub anchor_line: usize,
    pub anchor_cell: usize,
    pub head_line: usize,
    pub head_cell: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortMode {
    /// Text with numbers by value: `a2 < a10` (legacy wish).
    #[default]
    Natural,
    /// Plain text order.
    Text,
    /// By the first number in the line; lines without one go last.
    Numeric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SortOptions {
    pub descending: bool,
    pub mode: SortMode,
    pub ignore_case: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CaseChange {
    Upper,
    Lower,
    Title,
    /// Legacy Ctrl+U: upper unless it already is, then lower.
    Toggle,
}

// ---------------------------------------------------------------- pure helpers

fn digits_cmp(a: &str, b: &str) -> Ordering {
    let ta = a.trim_start_matches('0');
    let tb = b.trim_start_matches('0');
    ta.len()
        .cmp(&tb.len())
        .then_with(|| ta.cmp(tb))
        .then_with(|| a.len().cmp(&b.len()))
}

/// Runs of digits and of other characters, without allocating (sorting compares millions of
/// times, design 013 E5).
struct Chunks<'a> {
    s: &'a str,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = (bool, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let first = self.s.as_bytes().first()?;
        let digit = first.is_ascii_digit();
        // digits are ASCII: a byte scan finds the end of the run on a char boundary
        let end = self
            .s
            .bytes()
            .position(|b| b.is_ascii_digit() != digit)
            .unwrap_or(self.s.len());
        let (run, rest) = self.s.split_at(end);
        self.s = rest;
        Some((digit, run))
    }
}

fn chunks(s: &str) -> Chunks<'_> {
    Chunks { s }
}

fn text_cmp(a: &str, b: &str, ignore_case: bool) -> Ordering {
    if ignore_case {
        lower_cmp(a, b).then_with(|| a.cmp(b))
    } else {
        a.cmp(b)
    }
}

/// Like comparing the lower-case strings, without building them.
fn lower_cmp(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
}

/// Natural order: digit runs compare by value (`file2 < file10`).
pub fn natural_cmp(a: &str, b: &str, ignore_case: bool) -> Ordering {
    let (mut ca, mut cb) = (chunks(a), chunks(b));
    loop {
        match (ca.next(), cb.next()) {
            (Some(x), Some(y)) => {
                let o = match (x.0, y.0) {
                    (true, true) => digits_cmp(x.1, y.1),
                    _ if ignore_case => lower_cmp(x.1, y.1),
                    _ => x.1.cmp(y.1),
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

/// The first number of a line (`-12.5`, `3`, `1e3` is 1).
pub fn first_number(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        if (b[i] == b'-' || b[i] == b'+') && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
            i += 1;
        }
        if b[i].is_ascii_digit() {
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < b.len() && (b[i] == b'.' || b[i] == b',') && b[i + 1].is_ascii_digit() {
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            return s[start..i].replace(',', ".").parse().ok();
        }
        i = start + 1;
    }
    None
}

/// Sorts lines (stable) as the options say.
pub fn sort_lines<S: AsRef<str>>(lines: &mut Vec<S>, o: SortOptions) {
    match o.mode {
        SortMode::Natural => {
            lines.sort_unstable_by(|a, b| natural_cmp(a.as_ref(), b.as_ref(), o.ignore_case))
        }
        SortMode::Text => {
            lines.sort_unstable_by(|a, b| text_cmp(a.as_ref(), b.as_ref(), o.ignore_case))
        }
        SortMode::Numeric => {
            let mut keyed: Vec<(Option<f64>, S)> = lines
                .drain(..)
                .map(|l| (first_number(l.as_ref()), l))
                .collect();
            keyed.sort_by(|a, b| match (a.0, b.0) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            });
            lines.extend(keyed.into_iter().map(|(_, l)| l));
        }
    }
    if o.descending {
        lines.reverse();
    }
    if o.unique {
        if o.ignore_case {
            lines.dedup_by(|a, b| lower_cmp(a.as_ref(), b.as_ref()) == Ordering::Equal);
        } else {
            lines.dedup_by(|a, b| a.as_ref() == b.as_ref());
        }
    }
}

/// The lines of a text (breaks LF, CRLF, CR); a final break ends the last line.
fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                out.push(&text[start..i]);
                i += if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = i;
            }
            b'\n' => {
                out.push(&text[start..i]);
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

pub fn change_case(text: &str, how: CaseChange) -> String {
    match how {
        CaseChange::Upper => text.to_uppercase(),
        CaseChange::Lower => text.to_lowercase(),
        CaseChange::Toggle => {
            if text.chars().any(char::is_lowercase) {
                text.to_uppercase()
            } else {
                text.to_lowercase()
            }
        }
        CaseChange::Title => {
            let mut out = String::with_capacity(text.len());
            let mut start = true;
            for c in text.chars() {
                if c.is_alphanumeric() {
                    if start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    start = false;
                } else {
                    out.push(c);
                    start = true;
                }
            }
            out
        }
    }
}

/// Text that "sort lines" takes at most (design 013 E4).
pub const MAX_SORT_BYTES: usize = 512 << 20;

/// Lines "copy as HTML" takes at most (it is meant for snippets, design 010 E5).
pub const MAX_HTML_LINES: usize = 20_000;

/// Escapes text and attribute values for HTML.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------- session operations

impl Session {
    /// Lines covered by the primary cursor; `None` = whole document wanted.
    fn primary_lines(&self) -> (usize, usize) {
        Session::lines_of(&self.primary())
    }

    /// All distinct line ranges of the cursors, merged and sorted.
    fn cursor_line_ranges(&self) -> Vec<(usize, usize)> {
        let mut r: Vec<(usize, usize)> = self.cursors.iter().map(Session::lines_of).collect();
        r.sort_unstable();
        let mut out: Vec<(usize, usize)> = Vec::new();
        for (a, b) in r {
            match out.last_mut() {
                Some(last) if a <= last.1 + 1 => last.1 = last.1.max(b),
                _ => out.push((a, b)),
            }
        }
        out
    }

    fn lines_text(&self, a: usize, b: usize) -> Vec<String> {
        (a..=b).map(|l| self.doc.line_string(l)).collect()
    }

    fn apply_one(&mut self, changes: Vec<Change>, after: Vec<Sel>) -> Result<(), FsError> {
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

    /// The primary selection (or its line) as HTML with inline styles, plain text as the
    /// alternative (design 010 E5).
    pub(super) fn copy_html(
        &mut self,
        clip: &mut dyn Clipboard,
        line_numbers: bool,
        styles: &[String],
        base: &str,
        gutter: &str,
    ) -> Result<(), FsError> {
        let c = self.primary();
        let (a, b) = if c.is_empty() {
            let l = c.head.line;
            (Pos::new(l, 0), Pos::new(l, self.doc.line_len(l)))
        } else {
            (c.start(), c.end())
        };
        // the end at the start of a line: that line is not part of it
        let last = if b.ch == 0 && b.line > a.line {
            b.line - 1
        } else {
            b.line
        };
        if last - a.line >= MAX_HTML_LINES {
            self.notice = Some("html_too_large".into());
            return Ok(());
        }
        let line_styles = match &mut self.highlighter {
            Some(h) => h.styles(&self.doc, a.line, last + 1).0,
            None => Vec::new(),
        };
        let width = (last + 1).to_string().len();
        let mut html = format!(
            "<pre style=\"{};tab-size:{};-moz-tab-size:{}\">",
            esc(base),
            self.tab,
            self.tab
        );
        let mut plain = String::new();
        for line in a.line..=last {
            let len = self.doc.line_len(line);
            let from = if line == a.line { a.ch } else { 0 };
            let to = if line == b.line { b.ch.min(len) } else { len };
            if line_numbers {
                html.push_str(&format!(
                    "<span style=\"{};user-select:none\">{:>width$} </span>",
                    esc(gutter),
                    line + 1
                ));
            }
            if self.doc.is_long(line) {
                let t = self.doc.line_slice(line, from, to);
                html.push_str(&esc(&t));
                plain.push_str(&t);
            } else {
                let text = self.doc.line_string(line);
                let spans = line_styles.get(line - a.line).cloned().flatten();
                let st = super::char_styles(&text, spans.as_deref(), from, to);
                let chars: Vec<char> = text.chars().skip(from).take(to - from).collect();
                let mut i = 0;
                while i < chars.len() {
                    let k = st.get(i).copied().unwrap_or(0);
                    let mut j = i;
                    while j < chars.len() && st.get(j).copied().unwrap_or(0) == k {
                        j += 1;
                    }
                    let piece: String = chars[i..j].iter().collect();
                    match styles.get(k as usize).filter(|s| k > 0 && !s.is_empty()) {
                        Some(css) => html.push_str(&format!(
                            "<span style=\"{}\">{}</span>",
                            esc(css),
                            esc(&piece)
                        )),
                        None => html.push_str(&esc(&piece)),
                    }
                    plain.push_str(&piece);
                    i = j;
                }
            }
            if line < last {
                html.push('\n');
                plain.push_str(self.doc.line_ending.as_str());
            }
        }
        html.push_str("</pre>");
        if !clip.set_html(html, plain) {
            self.notice = Some("clipboard_failed".into());
        }
        Ok(())
    }

    /// Every line break becomes `le` (status bar, design 010 E2) – one undo step.
    pub(super) fn convert_line_endings(&mut self, le: LineEnding) -> Result<(), FsError> {
        let sels = self.sels();
        let after = self.doc.convert_line_endings(le, sels);
        self.cursors_from(after);
        Ok(())
    }

    /// Sorts the selected lines, or the whole document without a selection (design 007 E1).
    pub(super) fn sort(&mut self, o: SortOptions) -> Result<(), FsError> {
        let c = self.primary();
        let (a, mut b) = if c.is_empty() {
            (0, self.doc.line_count().saturating_sub(1))
        } else {
            self.primary_lines()
        };
        // the empty line after a final line break stays where it is
        if b > a && self.doc.line_len(b) == 0 && b + 1 == self.doc.line_count() {
            b -= 1;
        }
        if b <= a {
            return Ok(());
        }
        // sorting holds the lines about four times (lines, result, undo both ways): a limit
        // instead of running out of memory (design 013 E4)
        let rope = self.doc.rope();
        let bytes = rope.line_to_byte((b + 1).min(rope.len_lines())) - rope.line_to_byte(a);
        if bytes > MAX_SORT_BYTES {
            self.notice = Some("sort_too_large".into());
            return Ok(());
        }
        // one copy of the block, sorted as slices (no string per line, design 013 E5)
        let block = {
            let r = self.doc.rope();
            let (ca, cb) = (r.line_to_char(a), r.line_to_char(b) + self.doc.line_len(b));
            r.slice(ca..cb).to_string()
        };
        let mut lines = split_lines(&block);
        if lines.len() < b - a + 1 {
            lines.push(""); // the last line of the block is empty
        }
        sort_lines(&mut lines, o);
        let le = self.doc.line_ending.as_str();
        let new_b = a + lines.len() - 1;
        let last_len = lines.last().map_or(0, |l| l.chars().count());
        let text = lines.join(le);
        let after = if c.is_empty() {
            vec![Sel::caret(Pos::new(a, 0))]
        } else {
            vec![Sel {
                anchor: Pos::new(a, 0),
                head: Pos::new(new_b, last_len),
            }]
        };
        self.apply_one(
            vec![Change {
                from: Pos::new(a, 0),
                to: Pos::new(b, self.doc.line_len(b)),
                text,
            }],
            after,
        )
    }

    /// Case of every selection, or of the word at a caret.
    pub(super) fn change_case(&mut self, how: CaseChange) -> Result<(), FsError> {
        let mut changes = Vec::new();
        let mut after = Vec::new();
        for c in self.cursors.clone() {
            let (a, b) = if c.is_empty() {
                self.word_at(c.head)
            } else {
                (c.start(), c.end())
            };
            let text = self.doc.text(a, b)?;
            let new = change_case(&text, how);
            if a.line == b.line && new.chars().count() == text.chars().count() {
                after.push(if c.is_empty() {
                    c.sel()
                } else {
                    Sel { anchor: a, head: b }
                });
            } else {
                after.push(Sel::caret(a));
            }
            if new != text {
                changes.push(Change {
                    from: a,
                    to: b,
                    text: new,
                });
            }
        }
        self.apply_one(changes, after)
    }

    /// Copies the lines of every cursor below them; the cursors move along to the copy.
    pub(super) fn duplicate_lines(&mut self) -> Result<(), FsError> {
        let le = self.doc.line_ending.as_str().to_owned();
        let ranges = self.cursor_line_ranges();
        let mut changes = Vec::new();
        let mut shift_at: Vec<(usize, usize, usize)> = Vec::new(); // (a, b, lines added before)
        let mut added = 0;
        for &(a, b) in &ranges {
            let block = self.lines_text(a, b).join(&le);
            changes.push(Change {
                from: Pos::new(b, self.doc.line_len(b)),
                to: Pos::new(b, self.doc.line_len(b)),
                text: format!("{le}{block}"),
            });
            shift_at.push((a, b, added));
            added += b - a + 1;
        }
        let after = self
            .cursors
            .iter()
            .map(|c| {
                let shift = |p: Pos| {
                    let (a, b, before) = shift_at
                        .iter()
                        .copied()
                        .find(|(a, b, _)| p.line >= *a && p.line <= *b)
                        .unwrap_or((0, 0, 0));
                    Pos::new(p.line + before + (b - a + 1), p.ch)
                };
                Sel {
                    anchor: shift(c.anchor),
                    head: shift(c.head),
                }
            })
            .collect();
        self.apply_one(changes, after)
    }

    /// Removes the lines of every cursor (with their breaks).
    pub(super) fn delete_lines(&mut self) -> Result<(), FsError> {
        let ranges = self.cursor_line_ranges();
        let last = self.doc.line_count() - 1;
        let changes: Vec<Change> = ranges
            .iter()
            .map(|&(a, b)| {
                if b < last {
                    Change {
                        from: Pos::new(a, 0),
                        to: Pos::new(b + 1, 0),
                        text: String::new(),
                    }
                } else if a > 0 {
                    Change {
                        from: Pos::new(a - 1, self.doc.line_len(a - 1)),
                        to: Pos::new(b, self.doc.line_len(b)),
                        text: String::new(),
                    }
                } else {
                    Change {
                        from: Pos::new(0, 0),
                        to: Pos::new(b, self.doc.line_len(b)),
                        text: String::new(),
                    }
                }
            })
            .collect();
        // carets land at the start of what follows (post-edit lines; clamped by the document)
        let mut removed = 0;
        let mut after = Vec::new();
        for &(a, b) in &ranges {
            after.push(Sel::caret(Pos::new(a.saturating_sub(removed), 0)));
            removed += b - a + 1;
        }
        self.apply_one(changes, after)
    }

    /// Swaps the primary cursor's lines with the line above/below; the selection moves along.
    pub(super) fn move_lines(&mut self, up: bool) -> Result<(), FsError> {
        let (a, b) = self.primary_lines();
        let last = self.doc.line_count() - 1;
        if (up && a == 0) || (!up && b >= last) {
            return Ok(());
        }
        let le = self.doc.line_ending.as_str();
        let (from, to, text) = if up {
            let mut lines = self.lines_text(a, b);
            lines.push(self.doc.line_string(a - 1));
            (a - 1, b, lines.join(le))
        } else {
            let mut lines = vec![self.doc.line_string(b + 1)];
            lines.extend(self.lines_text(a, b));
            (a, b + 1, lines.join(le))
        };
        let c = self.primary();
        let mv = |p: Pos| Pos::new(if up { p.line - 1 } else { p.line + 1 }, p.ch);
        self.apply_one(
            vec![Change {
                from: Pos::new(from, 0),
                to: Pos::new(to, self.doc.line_len(to)),
                text,
            }],
            vec![Sel {
                anchor: mv(c.anchor),
                head: mv(c.head),
            }],
        )
    }

    /// Joins the selected lines (or the line with the next) with one space.
    pub(super) fn join_lines(&mut self) -> Result<(), FsError> {
        let (a, mut b) = self.primary_lines();
        if a == b {
            b = a + 1;
        }
        if b >= self.doc.line_count() {
            return Ok(());
        }
        let mut changes = Vec::new();
        for l in a..b {
            let end = self.doc.line_len(l);
            let next_start = self.first_non_blank(l + 1);
            let prev_space = self
                .doc
                .line_slice(l, end.saturating_sub(1), end)
                .ends_with([' ', '\t']);
            let sep = if prev_space || next_start == self.doc.line_len(l + 1) || end == 0 {
                ""
            } else {
                " "
            };
            changes.push(Change {
                from: Pos::new(l, end),
                to: Pos::new(l + 1, next_start),
                text: sep.into(),
            });
        }
        let caret = Pos::new(a, self.doc.line_len(a));
        self.apply_one(changes, vec![Sel::caret(caret)])
    }

    /// Removes spaces and tabs at line ends (selection lines or the whole document).
    pub(super) fn trim_trailing(&mut self) -> Result<usize, FsError> {
        let c = self.primary();
        let (a, b) = if c.is_empty() {
            (0, self.doc.line_count() - 1)
        } else {
            self.primary_lines()
        };
        let rope = self.doc.rope();
        let mut changes = Vec::new();
        for l in a..=b {
            let len = self.doc.line_len(l);
            if len == 0 {
                continue;
            }
            let start = rope.line_to_char(l);
            let mut chars = rope.chars_at(start + len);
            let mut n = 0;
            while n < len {
                match chars.prev() {
                    Some(' ' | '\t') => n += 1,
                    _ => break,
                }
            }
            if n > 0 {
                changes.push(Change {
                    from: Pos::new(l, len - n),
                    to: Pos::new(l, len),
                    text: String::new(),
                });
            }
        }
        let n = changes.len();
        let after = self.cursors.iter().map(|c| c.sel()).collect();
        self.apply_one(changes, after)?;
        Ok(n)
    }

    /// Leading whitespace as spaces or as tabs (+ spaces for the rest).
    pub(super) fn convert_indent(&mut self, to_spaces: bool) -> Result<(), FsError> {
        let tab = self.tab;
        let mut changes = Vec::new();
        for l in 0..self.doc.line_count() {
            let n = self.first_non_blank(l);
            if n == 0 {
                continue;
            }
            let lead = self.doc.line_slice(l, 0, n);
            let width = lead.chars().fold(0, |w, c| {
                if c == '\t' {
                    w + tab - (w % tab)
                } else {
                    w + 1
                }
            });
            let new = if to_spaces {
                " ".repeat(width)
            } else {
                format!("{}{}", "\t".repeat(width / tab), " ".repeat(width % tab))
            };
            if new != lead {
                changes.push(Change {
                    from: Pos::new(l, 0),
                    to: Pos::new(l, n),
                    text: new,
                });
            }
        }
        self.spaces = to_spaces;
        let after = vec![Sel::caret(Pos::new(self.primary().head.line, 0))];
        self.apply_one(changes, after)
    }

    // ---------------------------------------------------------------- column selection

    /// Cursors of a block: one per line, between the two cells (shorter lines: at their end).
    pub(super) fn apply_block(&mut self, b: Block) {
        let (l0, l1) = (
            b.anchor_line.min(b.head_line),
            b.anchor_line.max(b.head_line),
        );
        let l1 = l1.min(l0 + MAX_BLOCK_LINES - 1);
        self.cursors = (l0..=l1)
            .map(|l| Cursor {
                anchor: Pos::new(l, self.ch_for_goal(l, b.anchor_cell)),
                head: Pos::new(l, self.ch_for_goal(l, b.head_cell)),
                goal: Some(b.head_cell),
            })
            .collect();
        self.primary = b.head_line.saturating_sub(l0).min(self.cursors.len() - 1);
        self.block = Some(b);
    }

    /// Alt+Shift+arrows: grow the column selection from the primary caret.
    pub(super) fn block_move(&mut self, dx: isize, dy: isize) {
        let mut b = self.block.unwrap_or_else(|| {
            let h = self.primary().head;
            let cell = self.cell_of(h);
            Block {
                anchor_line: h.line,
                anchor_cell: cell,
                head_line: h.line,
                head_cell: cell,
            }
        });
        let last = self.doc.line_count() - 1;
        b.head_line = b.head_line.saturating_add_signed(dy).min(last);
        b.head_cell = b.head_cell.saturating_add_signed(dx);
        self.apply_block(b);
    }

    /// Alt-drag: the block between the click and the mouse.
    pub(super) fn block_drag(&mut self, from: (usize, usize), line: f64, cell: f64) {
        let last = self.doc.line_count() - 1;
        let head_line = if line < 0.0 {
            0
        } else {
            (line.floor() as usize).min(last)
        };
        self.apply_block(Block {
            anchor_line: from.0,
            anchor_cell: from.1,
            head_line,
            head_cell: cell.max(0.0).round() as usize,
        });
    }

    // ---------------------------------------------------------------- more cursors

    /// Mod+D: select the word at the caret, then add the next occurrence of the selection.
    pub(super) fn add_next_occurrence(&mut self) {
        let c = self.primary();
        if c.is_empty() {
            let (a, b) = self.word_at(c.head);
            if a != b {
                let i = self.primary.min(self.cursors.len() - 1);
                self.cursors[i] = Cursor {
                    anchor: a,
                    head: b,
                    goal: None,
                };
            }
            return;
        }
        let Ok(text) = self.doc.text(c.start(), c.end()) else {
            return;
        };
        let Ok(re) = search::compile(&Query {
            pattern: text,
            case_sensitive: true,
            ..Default::default()
        }) else {
            return;
        };
        // search after the last cursor, wrapping around
        let after = self
            .cursors
            .iter()
            .map(|c| c.end())
            .max()
            .unwrap_or(c.end());
        let from = self.doc.pos_to_char(after);
        let Some((s, e)) = search::find_next(self.doc.rope(), &re, from) else {
            return;
        };
        let (a, b) = (self.doc.char_to_pos(s), self.doc.char_to_pos(e));
        if self.cursors.iter().any(|x| x.start() == a && x.end() == b) {
            self.notice = Some("no_more_matches".into());
            return;
        }
        self.cursors.push(Cursor {
            anchor: a,
            head: b,
            goal: None,
        });
        self.primary = self.cursors.len() - 1;
        self.normalize();
    }

    /// Alt+click: one more caret.
    pub(super) fn add_cursor(&mut self, p: Pos) {
        self.cursors.push(Cursor::caret(p));
        self.primary = self.cursors.len() - 1;
        self.normalize();
    }

    /// Back to one cursor (Escape).
    pub(super) fn single_cursor(&mut self) {
        let c = self.primary();
        self.cursors = vec![c];
        self.primary = 0;
        self.block = None;
    }

    /// Pastes one clipboard line per cursor when the counts match (column paste, legacy).
    pub(super) fn paste_distributed(&mut self, text: &str) -> Result<bool, FsError> {
        let n = self.cursors.len();
        if n < 2 {
            return Ok(false);
        }
        let lines: Vec<&str> = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .unwrap_or(text)
            .split(['\n'])
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .collect();
        if lines.len() != n {
            return Ok(false);
        }
        let changes = self
            .cursors
            .iter()
            .zip(lines)
            .map(|(c, l)| Change {
                from: c.start(),
                to: c.end(),
                text: l.to_owned(),
            })
            .collect();
        let sels = self.doc.apply(EditRequest {
            changes,
            kind: EditKind::Other,
            sel_before: self.sels(),
            sel_after: None,
        })?;
        self.cursors_from(sels);
        Ok(true)
    }

    // ---------------------------------------------------------------- markers

    /// Marks the word at the caret (or the selection) in the next free colour; again = unmark.
    pub(super) fn toggle_marker(&mut self) {
        let c = self.primary();
        let (a, b) = if c.is_empty() {
            self.word_at(c.head)
        } else {
            (c.start(), c.end())
        };
        if a.line != b.line || a == b {
            return;
        }
        let Ok(text) = self.doc.text(a, b) else {
            return;
        };
        if let Some(i) = self
            .markers
            .iter()
            .position(|m| m.as_ref().is_some_and(|(t, _)| *t == text))
        {
            self.markers[i] = None;
            return;
        }
        let Ok(re) = search::compile(&Query {
            pattern: text.clone(),
            case_sensitive: true,
            whole_word: true,
            ..Default::default()
        }) else {
            return;
        };
        match self.markers.iter().position(Option::is_none) {
            Some(i) => self.markers[i] = Some((text, re)),
            None if self.markers.len() < MAX_MARKERS => self.markers.push(Some((text, re))),
            None => {
                // all colours taken: the oldest goes
                self.markers.remove(0);
                self.markers.push(Some((text, re)));
            }
        }
    }

    pub(super) fn clear_markers(&mut self) {
        self.markers.clear();
    }

    /// Marker ranges `(from_ch, to_ch, colour)` in a piece of text.
    pub(super) fn markers_in(&self, text: &str, offset: usize) -> Vec<(usize, usize, usize)> {
        let mut out = Vec::new();
        for (i, m) in self.markers.iter().enumerate() {
            if let Some((_, re)) = m {
                for (a, b) in search::matches_in(text, re) {
                    out.push((a + offset, b + offset, i));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order() {
        let mut v: Vec<String> = ["a10", "a2", "A1", "b", "a02", "10", "9"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        sort_lines(
            &mut v,
            SortOptions {
                ignore_case: true,
                ..Default::default()
            },
        );
        assert_eq!(v, ["9", "10", "A1", "a2", "a02", "a10", "b"]);
    }

    #[test]
    fn numeric_descending_unique() {
        let mut v: Vec<String> = ["x 5", "100", "-3 c", "no number", "5", "5", "2,5 kg"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        sort_lines(
            &mut v,
            SortOptions {
                mode: SortMode::Numeric,
                unique: true,
                ..Default::default()
            },
        );
        assert_eq!(v, ["-3 c", "2,5 kg", "x 5", "5", "100", "no number"]);
        assert_eq!(first_number("abc"), None);
        assert_eq!(first_number("v1.25 x"), Some(1.25));
    }

    #[test]
    fn case_changes() {
        assert_eq!(change_case("hello wORLD", CaseChange::Title), "Hello World");
        assert_eq!(change_case("straße", CaseChange::Upper), "STRASSE");
        assert_eq!(change_case("ABC", CaseChange::Toggle), "abc");
        assert_eq!(change_case("AbC", CaseChange::Toggle), "ABC");
    }
}
