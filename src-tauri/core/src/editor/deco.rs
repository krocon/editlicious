//! Markers from outside (design 029): the page underlines errors, puts a symbol in the gutter,
//! tints a line – by owner (`"sql-error"`, `"running"` …). The ranges are character offsets
//! that follow the text ([`super::track`]); a frame only carries the ones in view.

use super::track::{Bias, EditMap, remap_lines};
use crate::document::Document;
use crate::document::model::Pos;
use ropey::Rope;
use serde::{Deserialize, Serialize};

/// What becomes of a marker when the text inside it changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OnEdit {
    /// It follows the text: grows and shrinks with it (gone when nothing of it is left).
    #[default]
    Track,
    /// Any change inside it removes it (a diagnosis that is no longer true).
    Remove,
}

/// One marker as the page gives it (positions 0-based, like every engine position).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecoSpec {
    pub from: Pos,
    pub to: Pos,
    /// `error`, `warning`, `info`, `hint` (a wavy underline) – or a class of the page's own
    /// (a tinted background, colour `--ed-mark-<class>`).
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    /// A symbol in the gutter of its first line: empty = a dot, else this text.
    #[serde(default)]
    pub gutter: Option<String>,
    /// Tint the whole lines it covers.
    #[serde(default)]
    pub line: bool,
    #[serde(default)]
    pub on_edit: OnEdit,
}

#[derive(Debug, Clone)]
struct Deco {
    start: usize,
    end: usize,
    owner: u32,
    /// Index in the list the owner set (the page finds its own data by it).
    index: u32,
    severity: Box<str>,
    message: Option<Box<str>>,
    gutter: Option<Box<str>>,
    line: bool,
    on_edit: OnEdit,
}

/// A marker as a frame or a query hands it out.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecoInfo {
    pub owner: String,
    pub index: u32,
    pub severity: String,
    pub message: Option<String>,
    pub gutter: Option<String>,
    pub line: bool,
    pub from: Pos,
    pub to: Pos,
}

/// Bits of the overview bar (design 010 E3 has 1 = match, 2 = word marker).
pub const OVERVIEW_ERROR: u8 = 4;
pub const OVERVIEW_WARNING: u8 = 8;
pub const OVERVIEW_OTHER: u8 = 16;

/// All markers of a session, sorted by their start.
#[derive(Debug, Default)]
pub struct Decos {
    owners: Vec<String>,
    items: Vec<Deco>,
    /// Longest marker in characters (how far before a window one may start).
    max_len: usize,
    /// Changes with every set/clear (the overview asks again).
    generation: u64,
    /// Markers with a gutter symbol (the gutter keeps a column for them).
    symbols: usize,
}

impl Decos {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn has_symbols(&self) -> bool {
        self.symbols > 0
    }

    fn owner_id(&mut self, owner: &str) -> u32 {
        match self.owners.iter().position(|o| o == owner) {
            Some(i) => i as u32,
            None => {
                self.owners.push(owner.to_owned());
                (self.owners.len() - 1) as u32
            }
        }
    }

    fn recount(&mut self) {
        self.max_len = self
            .items
            .iter()
            .map(|d| d.end - d.start)
            .max()
            .unwrap_or(0);
        self.symbols = self.items.iter().filter(|d| d.gutter.is_some()).count();
        self.generation += 1;
    }

    /// Replaces all markers of `owner`.
    pub fn set(&mut self, owner: &str, specs: Vec<DecoSpec>, doc: &Document) {
        let id = self.owner_id(owner);
        self.items.retain(|d| d.owner != id);
        self.items.reserve(specs.len());
        for (index, s) in specs.into_iter().enumerate() {
            let (a, b) = (doc.pos_to_char(s.from), doc.pos_to_char(s.to));
            self.items.push(Deco {
                start: a.min(b),
                end: a.max(b),
                owner: id,
                index: index as u32,
                severity: s.severity.unwrap_or_else(|| "info".into()).into(),
                message: s.message.map(Into::into),
                gutter: s.gutter.map(Into::into),
                line: s.line,
                on_edit: s.on_edit,
            });
        }
        self.items.sort_by_key(|d| (d.start, d.end));
        self.recount();
    }

    /// Removes the markers of `owner`, or all of them.
    pub fn clear(&mut self, owner: Option<&str>) {
        match owner {
            None => self.items.clear(),
            Some(o) => {
                if let Some(id) = self.owners.iter().position(|x| x == o) {
                    self.items.retain(|d| d.owner != id as u32);
                }
            }
        }
        self.recount();
    }

    /// Follows a batch of changes (design 029 E3): text before a marker moves it, text inside it
    /// grows or shrinks it – or removes it, when it asked for that or nothing of it is left.
    pub fn follow(&mut self, map: &EditMap) {
        let mut max_len = 0;
        self.items.retain_mut(|d| {
            let was_empty = d.start == d.end;
            if d.on_edit == OnEdit::Remove && map.touches(d.start, d.end) {
                return false;
            }
            let start = map.map(d.start, Bias::Right);
            let end = map.map(d.end, Bias::Left).max(start);
            if end == start && !was_empty {
                return false;
            }
            (d.start, d.end) = (start, end);
            max_len = max_len.max(end - start);
            true
        });
        // changes keep the order of the starts, except where markers collapsed onto each other
        if !self.items.is_sorted_by_key(|d| d.start) {
            self.items.sort_by_key(|d| (d.start, d.end));
        }
        self.max_len = max_len;
        self.symbols = self.items.iter().filter(|d| d.gutter.is_some()).count();
    }

    /// Every line break was rewritten: lines and columns stay.
    pub fn rewrite(&mut self, old: &Rope, new: &Rope) {
        for d in &mut self.items {
            d.start = remap_lines(old, new, d.start);
            d.end = remap_lines(old, new, d.end).max(d.start);
        }
        self.max_len = self
            .items
            .iter()
            .map(|d| d.end - d.start)
            .max()
            .unwrap_or(0);
    }

    fn info(&self, d: &Deco, doc: &Document) -> DecoInfo {
        DecoInfo {
            owner: self.owners[d.owner as usize].clone(),
            index: d.index,
            severity: d.severity.to_string(),
            message: d.message.as_deref().map(str::to_owned),
            gutter: d.gutter.as_deref().map(str::to_owned),
            line: d.line,
            from: doc.char_to_pos(d.start),
            to: doc.char_to_pos(d.end),
        }
    }

    /// The markers that touch the characters `[a, b]`, in order, with their offsets.
    pub fn in_range(&self, a: usize, b: usize, doc: &Document) -> Vec<(usize, usize, DecoInfo)> {
        let first = self
            .items
            .partition_point(|d| d.start < a.saturating_sub(self.max_len));
        self.items[first..]
            .iter()
            .take_while(|d| d.start <= b)
            .filter(|d| d.end >= a)
            .map(|d| (d.start, d.end, self.info(d, doc)))
            .collect()
    }

    /// All markers (of one owner) where they are now.
    pub fn all(&self, owner: Option<&str>, doc: &Document) -> Vec<DecoInfo> {
        let id = owner.map(|o| self.owners.iter().position(|x| x == o));
        self.items
            .iter()
            .filter(|d| match id {
                None => true,
                Some(Some(i)) => d.owner == i as u32,
                Some(None) => false,
            })
            .map(|d| self.info(d, doc))
            .collect()
    }

    /// Start of the next (previous) marker from character `at`, around the ends of the text.
    pub fn step(&self, at: usize, backwards: bool, owner: Option<&str>) -> Option<usize> {
        let id = match owner {
            None => None,
            Some(o) => Some(self.owners.iter().position(|x| x == o)? as u32),
        };
        let fits = |d: &&Deco| id.is_none_or(|i| d.owner == i);
        if backwards {
            let i = self.items.partition_point(|d| d.start < at);
            self.items[..i]
                .iter()
                .rev()
                .find(fits)
                .or_else(|| self.items.iter().rev().find(fits))
                .map(|d| d.start)
        } else {
            let i = self.items.partition_point(|d| d.start <= at);
            self.items[i..]
                .iter()
                .find(fits)
                .or_else(|| self.items.iter().find(fits))
                .map(|d| d.start)
        }
    }

    /// The markers per slice of the lines for the overview bar: errors, warnings, the rest.
    pub fn overview(&self, rope: &Rope, buckets: usize) -> Vec<u8> {
        let buckets = buckets.clamp(1, 4096);
        let mut out = vec![0u8; buckets];
        let lines = rope.len_lines().max(1);
        for d in &self.items {
            let line = rope.char_to_line(d.start.min(rope.len_chars()));
            let k = (line * buckets / lines).min(buckets - 1);
            out[k] |= match &*d.severity {
                "error" => OVERVIEW_ERROR,
                "warning" => OVERVIEW_WARNING,
                _ => OVERVIEW_OTHER,
            };
        }
        out
    }
}
