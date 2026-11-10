//! Positions that follow the text (design 030 E2): character offsets mapped through the
//! changes a document reports ([`crate::document::EditBatch`]). Snippet placeholders (030) and
//! markers from outside (029) use it.

use crate::document::EditBatch;
use ropey::Rope;

/// Where a position goes when text is inserted exactly at it, or when it sat inside text that
/// was replaced: before the new text or behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    Left,
    Right,
}

/// One batch of sorted, non-overlapping changes as a map from old to new offsets.
#[derive(Debug)]
pub struct EditMap<'a> {
    changes: &'a [(usize, usize, usize)],
    /// Shift of everything before change `i` (sum over the changes before it).
    delta: Vec<isize>,
}

impl<'a> EditMap<'a> {
    pub fn new(changes: &'a [(usize, usize, usize)]) -> Self {
        let mut delta = Vec::with_capacity(changes.len() + 1);
        let mut d = 0isize;
        for &(from, to, inserted) in changes {
            delta.push(d);
            d += inserted as isize - (to - from) as isize;
        }
        delta.push(d);
        Self { changes, delta }
    }

    /// Did a change touch the characters `[a, b)` (or insert strictly inside them)?
    pub fn touches(&self, a: usize, b: usize) -> bool {
        let i = self.changes.partition_point(|c| c.1 <= a);
        self.changes.get(i).is_some_and(|&(from, to, _)| {
            if from == to {
                from > a && from < b
            } else {
                from < b
            }
        })
    }

    /// The new offset of `p`.
    pub fn map(&self, p: usize, bias: Bias) -> usize {
        // changes that end before `p` only shift it
        let i = self.changes.partition_point(|c| c.1 < p);
        let shift = |v: usize, d: isize| (v as isize + d).max(0) as usize;
        let Some(&(from, to, inserted)) = self.changes.get(i) else {
            return shift(p, self.delta[i]);
        };
        if from > p {
            return shift(p, self.delta[i]);
        }
        let start = shift(from, self.delta[i]);
        let behind = match (from == to, p == from, p == to) {
            // a pure insertion at `p`
            (true, _, _) => bias == Bias::Right,
            // at the start of replaced text: stays before it; at its end: behind the new text
            (false, true, _) => false,
            (false, _, true) => true,
            // inside replaced text
            _ => bias == Bias::Right,
        };
        if behind { start + inserted } else { start }
    }
}

/// Maps the offsets `[(start, end)]` of ranges through a line-break rewrite: line and column of
/// every position stay (the text before is `old`, after it `new`).
pub fn remap_lines(old: &Rope, new: &Rope, p: usize) -> usize {
    let p = p.min(old.len_chars());
    let line = old.char_to_line(p);
    let col = p - old.line_to_char(line);
    if line >= new.len_lines() {
        return new.len_chars();
    }
    let start = new.line_to_char(line);
    let next = if line + 1 < new.len_lines() {
        new.line_to_char(line + 1)
    } else {
        new.len_chars()
    };
    (start + col).min(next)
}

/// Applies `f(batch)` to every batch in order; a rewrite gets the rope after it (the next
/// rewrite's "before", or the current text).
pub fn for_each_batch(
    batches: &[EditBatch],
    current: &Rope,
    mut changes: impl FnMut(&EditMap),
    mut rewrite: impl FnMut(&Rope, &Rope),
) {
    for (i, b) in batches.iter().enumerate() {
        match b {
            EditBatch::Changes(c) => changes(&EditMap::new(c)),
            EditBatch::Rewrite(old) => {
                // the text right after a rewrite is not kept: positions are exact when nothing
                // else changed before the next look (the usual case), else clamped
                let after = batches[i + 1..]
                    .iter()
                    .find_map(|n| match n {
                        EditBatch::Rewrite(r) => Some(r),
                        EditBatch::Changes(_) => None,
                    })
                    .unwrap_or(current);
                rewrite(old, after);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_follow_insertions_deletions_and_replacements() {
        // "abcdef": insert 2 at 1, delete [3,4), replace [5,6) by 3
        let changes = [(1, 1, 2), (3, 4, 0), (5, 6, 3)];
        let m = EditMap::new(&changes);
        assert_eq!(m.map(0, Bias::Right), 0);
        assert_eq!(m.map(1, Bias::Left), 1, "before the insertion");
        assert_eq!(m.map(1, Bias::Right), 3, "behind the insertion");
        assert_eq!(m.map(2, Bias::Left), 4);
        assert_eq!(m.map(3, Bias::Right), 5, "start of the deleted text");
        assert_eq!(m.map(4, Bias::Left), 5, "end of the deleted text");
        assert_eq!(
            m.map(5, Bias::Right),
            6,
            "start of replaced text stays before it"
        );
        assert_eq!(
            m.map(6, Bias::Left),
            9,
            "end of replaced text goes behind the new text"
        );
        assert_eq!(m.map(9, Bias::Left), 12);
    }

    #[test]
    fn inside_replaced_text_the_bias_decides() {
        let changes = [(2, 6, 1)];
        let m = EditMap::new(&changes);
        assert_eq!(m.map(4, Bias::Left), 2);
        assert_eq!(m.map(4, Bias::Right), 3);
        assert!(m.touches(0, 3));
        assert!(m.touches(5, 9));
        assert!(!m.touches(6, 9));
        assert!(!m.touches(0, 2));
        // an insertion at the edge of a range does not touch it, inside it does
        let ins = [(4, 4, 1)];
        let m = EditMap::new(&ins);
        assert!(!m.touches(4, 8));
        assert!(!m.touches(0, 4));
        assert!(m.touches(3, 5));
    }

    #[test]
    fn a_line_break_rewrite_keeps_line_and_column() {
        let old = Rope::from_str("ab\ncd\nef");
        let new = Rope::from_str("ab\r\ncd\r\nef");
        assert_eq!(remap_lines(&old, &new, 4), 5);
        assert_eq!(remap_lines(&old, &new, 8), 10);
        assert_eq!(remap_lines(&new, &old, 9), 7);
    }
}
