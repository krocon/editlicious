//! Layout of a line in cells (design 003 E2): tabs to tab stops, wide characters 2 cells,
//! combining marks 0 (they stick to the character before), control characters 1 cell (drawn as
//! control pictures). Long lines (design 002 E3) are 1 cell per character – no layout needed.

use unicode_segmentation::GraphemeCursor;
use unicode_width::UnicodeWidthChar;

/// Width of one character at cell `col` (tab depends on the column).
pub fn char_width(c: char, col: usize, tab: usize) -> usize {
    match c {
        '\t' => tab - (col % tab),
        c if (c as u32) < 0x20 || c == '\u{7f}' => 1,
        c => c.width().unwrap_or(1),
    }
}

/// Cell offsets of a normal line: `cells[i]` = first cell of character `i`; `cells[len]` = width.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineLayout {
    pub chars: Vec<char>,
    pub cells: Vec<usize>,
}

impl LineLayout {
    pub fn new(text: &str, tab: usize) -> Self {
        let tab = tab.max(1);
        let chars: Vec<char> = text.chars().collect();
        let mut cells = Vec::with_capacity(chars.len() + 1);
        let mut col = 0;
        for &c in &chars {
            cells.push(col);
            col += char_width(c, col, tab);
        }
        cells.push(col);
        Self { chars, cells }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn width(&self) -> usize {
        *self.cells.last().unwrap_or(&0)
    }

    /// Cell of the caret before character `ch`.
    pub fn cell_of(&self, ch: usize) -> usize {
        self.cells[ch.min(self.len())]
    }

    /// The character boundary nearest to `cell` (clicks): the half of a character decides,
    /// combining marks are skipped (never between a base and its mark).
    pub fn ch_at(&self, cell: f64) -> usize {
        if cell <= 0.0 {
            return 0;
        }
        for i in 0..self.len() {
            let (a, b) = (self.cells[i] as f64, self.cells[i + 1] as f64);
            if b > a && cell < b {
                let ch = if cell < (a + b) / 2.0 { i } else { i + 1 };
                return self.snap(ch);
            }
        }
        self.len()
    }

    /// The character boundary at or before `cell` (vertical moves keep the goal cell).
    pub fn ch_at_or_before(&self, cell: usize) -> usize {
        let mut best = 0;
        for i in 0..=self.len() {
            if self.cells[i] <= cell {
                best = i;
            } else {
                break;
            }
        }
        self.snap(best)
    }

    /// Moves `ch` forward to a grapheme boundary.
    pub fn snap(&self, ch: usize) -> usize {
        let mut ch = ch.min(self.len());
        while ch < self.len()
            && ch > 0
            && self.cells[ch] == self.cells[ch + 1]
            && self.chars[ch] != '\t'
        {
            // zero width: a combining mark or similar belongs to the character before
            ch += 1;
        }
        ch
    }
}

/// Next grapheme boundary after char `ch` in `text` (`None` at the end).
pub fn next_grapheme(text: &str, ch: usize) -> Option<usize> {
    let byte = char_to_byte(text, ch)?;
    if byte >= text.len() {
        return None;
    }
    let mut cur = GraphemeCursor::new(byte, text.len(), true);
    let next = cur.next_boundary(text, 0).ok().flatten()?;
    Some(ch + text[byte..next].chars().count())
}

/// Previous grapheme boundary before char `ch` in `text` (`None` at the start).
pub fn prev_grapheme(text: &str, ch: usize) -> Option<usize> {
    if ch == 0 {
        return None;
    }
    let byte = char_to_byte(text, ch)?;
    let mut cur = GraphemeCursor::new(byte, text.len(), true);
    let prev = cur.prev_boundary(text, 0).ok().flatten()?;
    Some(ch - text[prev..byte].chars().count())
}

fn char_to_byte(text: &str, ch: usize) -> Option<usize> {
    if ch == 0 {
        return Some(0);
    }
    text.char_indices()
        .nth(ch)
        .map(|(b, _)| b)
        .or_else(|| (text.chars().count() == ch).then_some(text.len()))
}

/// Cells where the rows of a wrapped line start, without the first row (design 010 E4).
/// A row breaks after the last space that fits, else at the last character that fits (at
/// least one grapheme per row).
pub fn wrap_points(lay: &LineLayout, width: usize) -> Vec<usize> {
    let width = width.max(1);
    let mut out = Vec::new();
    let mut start = 0; // first character of the row
    let n = lay.len();
    while start < n {
        let base = lay.cells[start];
        // characters [start, end) fit into the row
        let mut end = start;
        while end < n && lay.cells[end + 1] - base <= width {
            end += 1;
        }
        if end >= n {
            break;
        }
        let mut brk = end;
        // after the last space of the row (not at its start)
        if let Some(k) = (start + 1..=end)
            .rev()
            .find(|&k| lay.chars[k - 1] == ' ' || lay.chars[k - 1] == '\t')
        {
            brk = k;
        }
        if brk <= start {
            brk = start + 1;
        }
        let brk = lay.snap(brk).max(start + 1);
        if brk >= n {
            break;
        }
        out.push(lay.cells[brk]);
        start = brk;
    }
    out
}

#[cfg(test)]
mod wrap_tests {
    use super::*;

    #[test]
    fn wraps_after_spaces_else_hard() {
        let lay = LineLayout::new("aaa bbb ccc", 4);
        assert_eq!(wrap_points(&lay, 5), vec![4, 8]);
        let lay = LineLayout::new("abcdefghij", 4);
        assert_eq!(wrap_points(&lay, 4), vec![4, 8]);
        // fits: no break
        assert!(wrap_points(&LineLayout::new("abc", 4), 3).is_empty());
        // wide characters never split
        let lay = LineLayout::new("日本語です", 4);
        assert_eq!(wrap_points(&lay, 5), vec![4, 8]);
        // a combining mark stays with its base
        let lay = LineLayout::new("abe\u{301}fg", 4);
        assert_eq!(wrap_points(&lay, 3), vec![3]);
        // narrower than one wide character: still progress
        assert_eq!(wrap_points(&LineLayout::new("日本", 4), 1), vec![2]);
    }
}
