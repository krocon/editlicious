//! Undo/redo (design 002 E7): a step is the list of changes forward and backward plus the
//! selections around it. Consecutive typing or deleting becomes one step.

use super::model::{EditKind, LineEnding, Sel};
use std::collections::VecDeque;
use std::time::Duration;
use web_time::Instant;

/// A change in character offsets of the rope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawChange {
    pub from: usize,
    pub to: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Step {
    /// Changes in coordinates of the state before the step.
    pub forward: Vec<RawChange>,
    /// Changes that undo the step, in coordinates of the state after it.
    pub inverse: Vec<RawChange>,
    pub sel_before: Vec<Sel>,
    pub sel_after: Vec<Sel>,
    pub kind: EditKind,
    pub at: Instant,
    pub state_before: u64,
    pub state_after: u64,
    /// "Convert line endings" (design 013 E3): instead of millions of changes, the kind of
    /// every break before (1 byte each); `forward`/`inverse` stay empty.
    pub breaks: Option<Breaks>,
    /// A group of steps undone and redone together (a playback, design 015 E8); `forward` and
    /// `inverse` stay empty.
    pub group: Option<Vec<Step>>,
}

/// The line breaks before a conversion to `after`.
#[derive(Debug, Clone)]
pub struct Breaks {
    /// Kind of every line break in order: 0 = LF, 1 = CRLF, 2 = CR.
    pub before: Vec<u8>,
    pub after: LineEnding,
    pub line_ending_before: LineEnding,
    pub mixed_before: bool,
}

/// Book-keeping per change beyond its text (offsets, string header, allocation).
const CHANGE_OVERHEAD: usize = 48;

impl Step {
    fn bytes(&self) -> usize {
        self.forward.iter().map(|c| c.text.len()).sum::<usize>()
            + self.inverse.iter().map(|c| c.text.len()).sum::<usize>()
            + (self.forward.len() + self.inverse.len()) * CHANGE_OVERHEAD
            + self.breaks.as_ref().map_or(0, |b| b.before.len())
            + self
                .group
                .as_ref()
                .map_or(0, |g| g.iter().map(Step::bytes).sum())
            + 64
    }
}

/// At most this many steps …
pub const MAX_STEPS: usize = 10_000;
/// … and this much text in the history (oldest steps fall out).
pub const MAX_BYTES: usize = 256 << 20;
/// Keystrokes closer than this merge into one step.
pub const MERGE_WINDOW: Duration = Duration::from_millis(1000);

#[derive(Debug, Default)]
pub struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    bytes: usize,
    /// No merging into the last step (after a save or an undo/redo).
    sealed: bool,
    /// Length of the undo stack when a group began (design 015 E8).
    group_mark: Option<usize>,
}

impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The next keystroke starts a new step.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Starts a group: every step from here to [`History::end_group`] becomes one undo step.
    pub fn begin_group(&mut self) {
        self.sealed = true;
        self.group_mark = Some(self.undo.len());
    }

    /// Ends the group: two or more steps since `begin_group` become one group step.
    pub fn end_group(&mut self) {
        let Some(mark) = self.group_mark.take() else {
            return;
        };
        self.sealed = true;
        let mark = mark.min(self.undo.len());
        if self.undo.len() - mark < 2 {
            return;
        }
        let children: Vec<Step> = self.undo.drain(mark..).collect();
        let (first, last) = (&children[0], &children[children.len() - 1]);
        let step = Step {
            forward: Vec::new(),
            inverse: Vec::new(),
            sel_before: first.sel_before.clone(),
            sel_after: last.sel_after.clone(),
            kind: EditKind::Other,
            at: last.at,
            state_before: first.state_before,
            state_after: last.state_after,
            breaks: None,
            group: Some(children),
        };
        self.undo.push_back(step);
    }

    pub fn in_group(&self) -> bool {
        self.group_mark.is_some()
    }

    /// Records a new step (clears redo). Merges typing/deleting into the previous step.
    pub fn push(&mut self, step: Step) {
        for s in self.redo.drain(..) {
            self.bytes = self.bytes.saturating_sub(s.bytes());
        }
        if !self.sealed
            && let Some(last) = self.undo.back_mut()
            && let Some(merged) = merge(last, &step)
        {
            self.bytes = self.bytes.saturating_sub(last.bytes());
            self.bytes += merged.bytes();
            *last = merged;
            return;
        }
        self.sealed = false;
        self.bytes += step.bytes();
        self.undo.push_back(step);
        while self.undo.len() > MAX_STEPS || (self.bytes > MAX_BYTES && self.undo.len() > 1) {
            if let Some(old) = self.undo.pop_front() {
                self.bytes = self.bytes.saturating_sub(old.bytes());
                if let Some(m) = self.group_mark.as_mut() {
                    *m = m.saturating_sub(1);
                }
            }
        }
    }

    pub fn pop_undo(&mut self) -> Option<Step> {
        self.sealed = true;
        let s = self.undo.pop_back()?;
        self.bytes = self.bytes.saturating_sub(s.bytes());
        let len = self.undo.len();
        if let Some(m) = self.group_mark.as_mut() {
            *m = (*m).min(len);
        }
        Some(s)
    }

    pub fn push_redo(&mut self, s: Step) {
        self.bytes += s.bytes();
        self.redo.push(s);
    }

    pub fn pop_redo(&mut self) -> Option<Step> {
        self.sealed = true;
        let s = self.redo.pop()?;
        self.bytes = self.bytes.saturating_sub(s.bytes());
        Some(s)
    }

    /// After a redo: back on the undo stack without clearing the rest of redo.
    pub fn push_undo_keep_redo(&mut self, s: Step) {
        self.bytes += s.bytes();
        self.undo.push_back(s);
    }
}

fn has_break(text: &str) -> bool {
    text.contains(['\n', '\r'])
}

/// Merges `next` into `last` for the three cases of design 002 E7, `None` otherwise.
fn merge(last: &Step, next: &Step) -> Option<Step> {
    if last.kind != next.kind
        || next.kind == EditKind::Other
        || last.breaks.is_some()
        || next.breaks.is_some()
        || last.group.is_some()
        || last.forward.len() != 1
        || next.forward.len() != 1
        || next.at.duration_since(last.at) > MERGE_WINDOW
        || last.state_after != next.state_before
    {
        return None;
    }
    let (lf, li) = (&last.forward[0], &last.inverse[0]);
    let (nf, ni) = (&next.forward[0], &next.inverse[0]);
    let merged = match next.kind {
        EditKind::Type => {
            // typing right behind the text typed so far, no line break
            if nf.from != nf.to || has_break(&nf.text) || has_break(&lf.text) || nf.from != li.to {
                return None;
            }
            let inserted = nf.text.chars().count();
            (
                RawChange {
                    from: lf.from,
                    to: lf.to,
                    text: format!("{}{}", lf.text, nf.text),
                },
                RawChange {
                    from: li.from,
                    to: li.to + inserted,
                    text: li.text.clone(),
                },
            )
        }
        EditKind::Delete => {
            if !nf.text.is_empty() || !lf.text.is_empty() || has_break(&ni.text) {
                return None;
            }
            if nf.to == li.from {
                // backspace: the new deletion ends where the last one started
                (
                    RawChange {
                        from: nf.from,
                        to: lf.to,
                        text: String::new(),
                    },
                    RawChange {
                        from: nf.from,
                        to: nf.from,
                        text: format!("{}{}", ni.text, li.text),
                    },
                )
            } else if nf.from == li.from {
                // forward delete at the same place
                let n = nf.to - nf.from;
                (
                    RawChange {
                        from: lf.from,
                        to: lf.to + n,
                        text: String::new(),
                    },
                    RawChange {
                        from: li.from,
                        to: li.from,
                        text: format!("{}{}", li.text, ni.text),
                    },
                )
            } else {
                return None;
            }
        }
        EditKind::Other => return None,
    };
    Some(Step {
        forward: vec![merged.0],
        inverse: vec![merged.1],
        sel_before: last.sel_before.clone(),
        sel_after: next.sel_after.clone(),
        kind: next.kind,
        at: next.at,
        state_before: last.state_before,
        state_after: next.state_after,
        breaks: None,
        group: None,
    })
}
