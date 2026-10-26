//! The document model (design 002): the text of an open file as a rope, the view on a part of
//! it, edits with undo/redo, load and save. Pure Rust without Tauri – the commands are thin.

pub mod history;
pub mod load;
pub mod model;
pub mod registry;
pub mod save;

#[cfg(test)]
mod tests;

use crate::error::{FsError, FsErrorCode};
use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE};
use history::{History, RawChange, Step};
use model::{
    Change, DocInfo, EditKind, EditRequest, LineEnding, Pos, SaveRequest, Sel, View, ViewLine,
    ViewRequest,
};
use ropey::Rope;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use web_time::Instant;

/// Lines up to this many characters are sent whole; longer ones only as a window (design 002 E3).
pub const FULL_LINE_LIMIT: usize = 10_000;
/// Largest window of a long line per request.
pub const MAX_WINDOW: usize = 64_000;
/// Most lines per view request.
pub const MAX_VIEW_LINES: usize = 2_000;
/// Largest text `get_text` hands out (design 002 E11), in bytes of UTF-8.
pub const MAX_TEXT_BYTES: usize = 64 << 20;
/// From this many changes in one edit the rope is rebuilt instead of edited in place.
const REBUILD_FROM: usize = 256;

#[derive(Debug)]
pub struct Document {
    rope: Rope,
    pub path: Option<PathBuf>,
    pub encoding: &'static Encoding,
    pub bom: bool,
    pub line_ending: LineEnding,
    pub mixed_line_endings: bool,
    pub decode_errors: bool,
    pub file_size: u64,
    /// mtime of the file when loaded/saved (changes from outside, prompt 004).
    pub disk_mtime: Option<SystemTime>,
    /// Changes with every modification (also undo) – the view drops older answers.
    revision: u64,
    /// Identity of the current text state – equal to `saved_state` means "not dirty".
    state: u64,
    next_state: u64,
    saved_state: u64,
    history: History,
    /// First line changed since the highlighter last looked (design 008 E2).
    changed_from: Option<usize>,
}

impl Document {
    /// A new, empty, never saved document.
    pub fn new_empty(line_ending: LineEnding) -> Self {
        Self {
            rope: Rope::new(),
            path: None,
            encoding: UTF_8,
            bom: false,
            line_ending,
            mixed_line_endings: false,
            decode_errors: false,
            file_size: 0,
            disk_mtime: None,
            revision: 0,
            state: 0,
            next_state: 1,
            saved_state: 0,
            history: History::default(),
            changed_from: None,
        }
    }

    /// A document from text (tests, "new from clipboard").
    pub fn from_text(text: &str) -> Self {
        let mut counter = load::LineEndingCounter::default();
        counter.feed(text);
        let stats = counter.finish();
        let mut d = Self::new_empty(stats.dominant().unwrap_or(LineEnding::Lf));
        d.mixed_line_endings = stats.mixed();
        d.rope = Rope::from_str(text);
        d
    }

    pub fn from_loaded(l: load::Loaded, path: &Path) -> Self {
        let mut d = Self::new_empty(l.stats.dominant().unwrap_or(LineEnding::Lf));
        d.rope = l.rope;
        d.path = Some(path.to_path_buf());
        d.encoding = l.encoding;
        d.bom = l.bom;
        d.mixed_line_endings = l.stats.mixed();
        d.decode_errors = l.decode_errors;
        d.file_size = l.file_size;
        d.disk_mtime = l.mtime;
        d
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// The first line changed since the last call (syntax highlighting starts over there).
    pub fn take_changed_from(&mut self) -> Option<usize> {
        self.changed_from.take()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn dirty(&self) -> bool {
        self.state != self.saved_state
    }

    pub fn name(&self) -> String {
        self.path
            .as_deref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned())
    }

    pub fn info(&self, id: u64) -> DocInfo {
        DocInfo {
            id,
            path: self.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            name: self.name(),
            encoding: self.encoding.name().to_owned(),
            bom: self.bom,
            line_ending: self.line_ending,
            mixed_line_endings: self.mixed_line_endings,
            decode_errors: self.decode_errors,
            line_count: self.line_count(),
            char_count: self.rope.len_chars(),
            file_size: self.file_size,
            revision: self.revision,
            dirty: self.dirty(),
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Characters of a line without its line break.
    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.line_count() {
            return 0;
        }
        let slice = self.rope.line(line);
        let n = slice.len_chars();
        if n == 0 {
            return 0;
        }
        match slice.char(n - 1) {
            '\n' if n >= 2 && slice.char(n - 2) == '\r' => n - 2,
            '\n' | '\r' => n - 1,
            _ => n,
        }
    }

    /// A long line (design 002 E3): one cell per character, only windows go to the UI.
    pub fn is_long(&self, line: usize) -> bool {
        self.line_len(line) > FULL_LINE_LIMIT
    }

    /// The whole text of a line without its break – callers use it for normal lines only.
    pub fn line_string(&self, line: usize) -> String {
        if line >= self.line_count() {
            return String::new();
        }
        let start = self.rope.line_to_char(line);
        self.rope
            .slice(start..start + self.line_len(line))
            .to_string()
    }

    /// Characters `[from, to)` of a line (clamped).
    pub fn line_slice(&self, line: usize, from: usize, to: usize) -> String {
        let len = self.line_len(line);
        let (from, to) = (from.min(len), to.min(len));
        if from >= to {
            return String::new();
        }
        let start = self.rope.line_to_char(line);
        self.rope.slice(start + from..start + to).to_string()
    }

    /// A position clamped into the document.
    pub fn clamp(&self, p: Pos) -> Pos {
        let line = p.line.min(self.line_count().saturating_sub(1));
        Pos {
            line,
            ch: p.ch.min(self.line_len(line)),
        }
    }

    pub fn pos_to_char(&self, p: Pos) -> usize {
        let p = self.clamp(p);
        self.rope.line_to_char(p.line) + p.ch
    }

    pub fn char_to_pos(&self, c: usize) -> Pos {
        let c = c.min(self.rope.len_chars());
        let line = self.rope.char_to_line(c);
        let ch = c - self.rope.line_to_char(line);
        Pos {
            line,
            ch: ch.min(self.line_len(line)),
        }
    }

    /// The lines (and windows of long lines) the view asked for (design 002 E3).
    pub fn view(&self, req: &ViewRequest) -> View {
        let count = self.line_count();
        let first = req.first_line.min(count.saturating_sub(1));
        let end = first
            .saturating_add(req.line_count.min(MAX_VIEW_LINES))
            .min(count);
        let window = req.window_count.min(MAX_WINDOW);
        let lines = (first..end)
            .map(|line| {
                let len = self.line_len(line);
                let start = self.rope.line_to_char(line);
                if len <= FULL_LINE_LIMIT {
                    ViewLine {
                        text: self.rope.slice(start..start + len).to_string(),
                        from: 0,
                        len,
                    }
                } else {
                    let from = req.window_from.min(len);
                    let to = from.saturating_add(window).min(len);
                    ViewLine {
                        text: self.rope.slice(start + from..start + to).to_string(),
                        from,
                        len,
                    }
                }
            })
            .collect();
        View {
            revision: self.revision,
            line_count: count,
            first_line: first,
            lines,
        }
    }

    /// The text between two positions (design 002 E11).
    pub fn text(&self, from: Pos, to: Pos) -> Result<String, FsError> {
        let (a, b) = ordered(self.pos_to_char(from), self.pos_to_char(to));
        let slice = self.rope.slice(a..b);
        if slice.len_bytes() > MAX_TEXT_BYTES {
            return Err(FsError::new(
                FsErrorCode::SelectionTooLarge,
                format!("{} bytes", slice.len_bytes()),
                None,
            ));
        }
        Ok(slice.to_string())
    }

    /// Line breaks of inserted text become the document's line ending (design 002 E6).
    fn normalize(&self, text: &str) -> String {
        if !text.contains(['\r', '\n']) {
            return text.to_owned();
        }
        let le = self.line_ending.as_str();
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    out.push_str(le);
                }
                '\n' => out.push_str(le),
                c => out.push(c),
            }
        }
        out
    }

    /// Applies changes given in coordinates before the edit (design 002 E6) and records the step.
    pub fn apply(&mut self, req: EditRequest) -> Result<Vec<Sel>, FsError> {
        let mut raw: Vec<RawChange> = req
            .changes
            .iter()
            .map(|c: &Change| {
                let (from, to) = ordered(self.pos_to_char(c.from), self.pos_to_char(c.to));
                RawChange {
                    from,
                    to,
                    text: self.normalize(&c.text),
                }
            })
            .filter(|c| c.from != c.to || !c.text.is_empty())
            .collect();
        raw.sort_by_key(|c| (c.from, c.to));
        for w in raw.windows(2) {
            if w[1].from < w[0].to {
                return Err(FsError::new(
                    FsErrorCode::InvalidEdit,
                    "overlapping changes",
                    None,
                ));
            }
        }
        Ok(self.record(raw, req.sel_before, req.sel_after, req.kind))
    }

    /// Every line break becomes `le` (design 010 E2) as one undo step; new lines get `le`.
    /// Scans the text once – a million lines take well under a second.
    pub fn convert_line_endings(&mut self, le: LineEnding, sels: Vec<Sel>) -> Vec<Sel> {
        let mut raw = Vec::new();
        let mut idx = 0usize;
        let mut cr_at: Option<usize> = None;
        let push = |from: usize, to: usize, is: LineEnding, raw: &mut Vec<RawChange>| {
            if is != le {
                raw.push(RawChange {
                    from,
                    to,
                    text: le.as_str().to_owned(),
                });
            }
        };
        for chunk in self.rope.chunks() {
            for c in chunk.chars() {
                match c {
                    '\n' => match cr_at.take() {
                        Some(i) if i + 1 == idx => push(i, idx + 1, LineEnding::Crlf, &mut raw),
                        Some(i) => {
                            push(i, i + 1, LineEnding::Cr, &mut raw);
                            push(idx, idx + 1, LineEnding::Lf, &mut raw);
                        }
                        None => push(idx, idx + 1, LineEnding::Lf, &mut raw),
                    },
                    '\r' => {
                        if let Some(i) = cr_at.replace(idx) {
                            push(i, i + 1, LineEnding::Cr, &mut raw);
                        }
                    }
                    _ => {
                        if let Some(i) = cr_at.take() {
                            push(i, i + 1, LineEnding::Cr, &mut raw);
                        }
                    }
                }
                idx += 1;
            }
        }
        if let Some(i) = cr_at {
            push(i, i + 1, LineEnding::Cr, &mut raw);
        }
        self.line_ending = le;
        self.mixed_line_endings = false;
        self.record(raw, sels.clone(), Some(sels), EditKind::Other)
    }

    /// Applies sorted, non-overlapping changes (char offsets) and records the undo step.
    fn record(
        &mut self,
        raw: Vec<RawChange>,
        sel_before: Vec<Sel>,
        sel_after: Option<Vec<Sel>>,
        kind: EditKind,
    ) -> Vec<Sel> {
        let req_sel_after = sel_after;
        if raw.is_empty() {
            return req_sel_after.unwrap_or(sel_before);
        }
        let inverse = self.apply_raw(&raw);
        let sel_after: Vec<Sel> = match req_sel_after {
            Some(s) => s.into_iter().map(|s| self.clamp_sel(s)).collect(),
            None => inverse
                .iter()
                .map(|c| Sel::caret(self.char_to_pos(c.to)))
                .collect(),
        };
        let state_before = self.state;
        self.state = self.next_state;
        self.next_state += 1;
        self.history.push(Step {
            forward: raw,
            inverse,
            sel_before,
            sel_after: sel_after.clone(),
            kind,
            at: Instant::now(),
            state_before,
            state_after: self.state,
        });
        sel_after
    }

    fn clamp_sel(&self, s: Sel) -> Sel {
        Sel {
            anchor: self.clamp(s.anchor),
            head: self.clamp(s.head),
        }
    }

    /// Applies sorted, non-overlapping changes; returns the inverse changes (ascending, in the
    /// coordinates after the edit).
    fn apply_raw(&mut self, changes: &[RawChange]) -> Vec<RawChange> {
        if let Some(first) = changes.iter().map(|c| c.from).min() {
            let line = self.rope.char_to_line(first.min(self.rope.len_chars()));
            self.changed_from = Some(self.changed_from.map_or(line, |l| l.min(line)));
        }
        let mut inverse = Vec::with_capacity(changes.len());
        let mut delta: isize = 0;
        for c in changes {
            let removed = self.rope.slice(c.from..c.to).to_string();
            let inserted = c.text.chars().count();
            let new_from = (c.from as isize + delta) as usize;
            inverse.push(RawChange {
                from: new_from,
                to: new_from + inserted,
                text: removed,
            });
            delta += inserted as isize - (c.to - c.from) as isize;
        }
        if changes.len() > REBUILD_FROM {
            // many changes (replace all, line endings): one pass building a new rope is far
            // faster than editing in place
            let mut b = ropey::RopeBuilder::new();
            let mut pos = 0;
            for c in changes {
                for chunk in self.rope.slice(pos..c.from).chunks() {
                    b.append(chunk);
                }
                b.append(&c.text);
                pos = c.to;
            }
            for chunk in self.rope.slice(pos..).chunks() {
                b.append(chunk);
            }
            self.rope = b.finish();
        } else {
            // apply back to front so earlier offsets stay valid
            for c in changes.iter().rev() {
                if c.to > c.from {
                    self.rope.remove(c.from..c.to);
                }
                if !c.text.is_empty() {
                    self.rope.insert(c.from, &c.text);
                }
            }
        }
        self.revision += 1;
        inverse
    }

    /// Undoes the last step; returns the selections before it.
    pub fn undo(&mut self) -> Option<Vec<Sel>> {
        let step = self.history.pop_undo()?;
        let mut inv = step.inverse.clone();
        inv.sort_by_key(|c| (c.from, c.to));
        self.apply_raw(&inv);
        self.state = step.state_before;
        let sel = step.sel_before.clone();
        self.history.push_redo(step);
        Some(sel)
    }

    /// Redoes the last undone step; returns the selections after it.
    pub fn redo(&mut self) -> Option<Vec<Sel>> {
        let step = self.history.pop_redo()?;
        let mut fwd = step.forward.clone();
        fwd.sort_by_key(|c| (c.from, c.to));
        self.apply_raw(&fwd);
        self.state = step.state_after;
        let sel = step.sel_after.clone();
        self.history.push_undo_keep_redo(step);
        Some(sel)
    }

    /// After a successful save: the current state is the clean one; no merging across it.
    pub fn mark_saved(&mut self, path: &Path, size: u64, mtime: Option<SystemTime>) {
        self.path = Some(path.to_path_buf());
        self.saved_state = self.state;
        self.file_size = size;
        self.disk_mtime = mtime;
        self.decode_errors = false;
        self.history.seal();
    }

    /// Has the file changed on disk since it was loaded/saved? (design 004 E6)
    pub fn disk_state(&self) -> DiskState {
        let Some(path) = &self.path else {
            return DiskState::default();
        };
        match std::fs::metadata(path) {
            Err(_) => DiskState {
                changed: false,
                missing: true,
            },
            Ok(m) => DiskState {
                changed: m.len() != self.file_size || m.modified().ok() != self.disk_mtime,
                missing: false,
            },
        }
    }

    /// Saves to `req.path` or the own path, in `req.encoding` or the own one (design 002 E8).
    pub fn save(&mut self, req: &SaveRequest) -> Result<(), FsError> {
        let path = req
            .path
            .as_deref()
            .map(PathBuf::from)
            .or_else(|| self.path.clone())
            .ok_or_else(|| FsError::new(FsErrorCode::InvalidPath, "no path", None))?;
        if !path.is_absolute() {
            return Err(FsError::new(
                FsErrorCode::InvalidPath,
                "not absolute",
                Some(&path),
            ));
        }
        let encoding = match &req.encoding {
            Some(label) => encoding_for_label(label)?,
            None => self.encoding,
        };
        let bom = req.bom.unwrap_or(if encoding == self.encoding {
            self.bom
        } else {
            encoding == UTF_16LE || encoding == UTF_16BE
        });
        let rope = &self.rope;
        save::write_file(&path, &|w| save::encode(rope, encoding, bom, w)).map_err(
            |e| match e {
                save::SaveError::Io(e) => FsError::from_io(&e, &path),
                save::SaveError::Unmappable(i) => {
                    let p = self.char_to_pos(i);
                    FsError::new(
                        FsErrorCode::UnmappableCharacter,
                        format!("{}:{}", p.line, p.ch),
                        Some(&path),
                    )
                }
            },
        )?;
        let meta = std::fs::metadata(&path).ok();
        self.encoding = encoding;
        self.bom = bom;
        self.mark_saved(
            &path,
            meta.as_ref().map(|m| m.len()).unwrap_or(0),
            meta.and_then(|m| m.modified().ok()),
        );
        Ok(())
    }
}

/// Encodings offered in the status bar (design 010 E2) – all of them can be read and written.
pub const ENCODINGS: &[&str] = &[
    "UTF-8",
    "UTF-16LE",
    "UTF-16BE",
    "windows-1252",
    "ISO-8859-2",
    "ISO-8859-15",
    "windows-1250",
    "windows-1251",
    "windows-1253",
    "windows-1254",
    "windows-1257",
    "KOI8-R",
    "KOI8-U",
    "IBM866",
    "macintosh",
    "Shift_JIS",
    "EUC-JP",
    "EUC-KR",
    "GBK",
    "gb18030",
    "Big5",
];

/// An encoding the app can write: UTF-8, UTF-16 or one whose encoder writes itself.
pub fn encoding_for_label(label: &str) -> Result<&'static Encoding, FsError> {
    let enc = Encoding::for_label(label.trim().as_bytes())
        .ok_or_else(|| FsError::new(FsErrorCode::UnsupportedEncoding, label.to_owned(), None))?;
    if enc != UTF_16LE && enc != UTF_16BE && enc.output_encoding() != enc {
        return Err(FsError::new(
            FsErrorCode::UnsupportedEncoding,
            label.to_owned(),
            None,
        ));
    }
    Ok(enc)
}

/// See [`Document::disk_state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct DiskState {
    pub changed: bool,
    pub missing: bool,
}

fn ordered(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}
