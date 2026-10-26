//! Syntax highlighting for the visible lines only (design 008): Sublime/TextMate grammars via
//! `syntect`, parse states cached at checkpoints (every [`CHECKPOINT_EVERY`] lines), scopes
//! mapped to a small set of style classes the colour schemes paint. Far jumps in big files start
//! from a fresh state a little above the view (approximate) instead of parsing everything.

use crate::document::Document;
use ropey::Rope;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use syntect::parsing::{ParseState, Scope, ScopeStack, ScopeStackOp, SyntaxDefinition, SyntaxSet};

/// A parse state is kept every this many lines.
pub const CHECKPOINT_EVERY: usize = 128;
/// Further than this from the last exact state, the view starts fresh (approximate) and a
/// background thread catches up (design 008 E3).
pub const MAX_CATCHUP: usize = 1_000;
/// Lines parsed above the view when starting fresh.
pub const FRESH_CONTEXT: usize = 300;
/// Lines longer than this are not highlighted (minified files) – they keep the state.
pub const MAX_HIGHLIGHT_LINE: usize = 4_000;

/// Style classes (index = value sent to the UI; mirror in `editor.models.ts` `STYLE_CLASSES`).
pub const CLASSES: [&str; 26] = [
    "",
    "comment",
    "string",
    "keyword",
    "number",
    "constant",
    "type",
    "function",
    "tag",
    "attribute",
    "variable",
    "operator",
    "punct",
    "heading",
    "bold",
    "italic",
    "code",
    "link",
    "invalid",
    "inserted",
    "deleted",
    "changed",
    "log-error",
    "log-warning",
    "log-info",
    "log-debug",
];

const LOG_SYNTAX: &str = r#"%YAML 1.2
---
name: Log
file_extensions: [log, out]
scope: text.log
contexts:
  main:
    - match: '\b(FATAL|ERROR|SEVERE|CRITICAL|ERR|EXCEPTION)\b'
      scope: log.level.error
    - match: '\b(WARN|WARNING)\b'
      scope: log.level.warning
    - match: '\b(INFO|NOTICE|CONFIG)\b'
      scope: log.level.info
    - match: '\b(DEBUG|FINE|FINER|FINEST|TRACE|VERBOSE)\b'
      scope: log.level.debug
    - match: '\b\d{4}-\d{2}-\d{2}([T ]\d{2}:\d{2}(:\d{2})?([.,]\d+)?(Z|[+-]\d{2}:?\d{2})?)?'
      scope: constant.numeric.date.log
    - match: '^\s+at\s.*$'
      scope: comment.line.stacktrace.log
    - match: '"[^"]*"'
      scope: string.quoted.double.log
    - match: '\b0x[0-9a-fA-F]+\b|\b\d+(\.\d+)?\b'
      scope: constant.numeric.log
"#;

const PROPERTIES_SYNTAX: &str = r#"%YAML 1.2
---
name: INI / Properties / TOML
file_extensions: [ini, cfg, conf, properties, toml, env, editorconfig, gitconfig]
scope: source.ini
contexts:
  main:
    - match: '^\s*[#;!].*$'
      scope: comment.line.ini
    - match: '^\s*\[+[^\]]*\]+'
      scope: entity.name.section.ini
    - match: '^\s*([^=:\s][^=:]*?)\s*(?=[=:])'
      captures:
        1: entity.other.attribute-name.ini
    - match: '"([^"\\]|\\.)*"|''[^'']*'''
      scope: string.quoted.ini
    - match: '\b(true|false|yes|no|on|off)\b'
      scope: constant.language.ini
    - match: '\b-?\d+(\.\d+)?\b'
      scope: constant.numeric.ini
"#;

/// The grammars: syntect's defaults plus Log and INI/TOML (built once).
pub fn syntax_set() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(|| {
        let mut b = SyntaxSet::load_defaults_nonewlines().into_builder();
        for (src, name) in [(LOG_SYNTAX, "Log"), (PROPERTIES_SYNTAX, "INI")] {
            if let Ok(def) = SyntaxDefinition::load_from_str(src, false, Some(name)) {
                b.add(def);
            }
        }
        b.build()
    })
}

/// Names of all languages (for "change language").
pub fn languages() -> Vec<String> {
    let mut v: Vec<String> = syntax_set()
        .syntaxes()
        .iter()
        .filter(|s| s.name != "Plain Text")
        .map(|s| s.name.clone())
        .collect();
    v.sort_by_key(|s| s.to_lowercase());
    v.dedup();
    v
}

/// The language of a file: by extension (TypeScript counts as JavaScript), then by the first
/// line (shebang, `<?xml`). `None` = plain text.
pub fn detect(file_name: Option<&str>, first_line: &str) -> Option<String> {
    let ss = syntax_set();
    let ext = file_name
        .and_then(|n| n.rsplit_once('.').map(|(_, e)| e.to_lowercase()))
        .map(|e| match e.as_str() {
            "ts" | "tsx" | "mjs" | "cjs" | "jsx" | "mts" | "cts" => "js".to_owned(),
            "jsonc" | "json5" => "json".to_owned(),
            "htm" | "vue" | "svelte" => "html".to_owned(),
            "kts" | "gradle" => "groovy".to_owned(),
            "zsh" | "fish" => "sh".to_owned(),
            "markdown" | "mdx" => "md".to_owned(),
            other => other.to_owned(),
        });
    let by_name = file_name.and_then(|n| match n.to_lowercase().as_str() {
        "makefile" | "gnumakefile" => Some("Makefile"),
        "dockerfile" => Some("Shell-Unix-Generic"),
        ".bashrc" | ".zshrc" | ".profile" => Some("Bourne Again Shell (bash)"),
        _ => None,
    });
    let syntax = by_name
        .and_then(|n| ss.find_syntax_by_name(n))
        .or_else(|| ext.as_deref().and_then(|e| ss.find_syntax_by_extension(e)))
        .or_else(|| ss.find_syntax_by_first_line(first_line))?;
    (syntax.name != "Plain Text").then(|| syntax.name.clone())
}

/// Scope atoms → class, innermost scope first; punctuation and meta scopes defer to the scope
/// around them (a comment's `//` is comment-coloured).
fn class_of_scope(name: &str) -> Option<u8> {
    const RULES: &[(&str, u8)] = &[
        ("log.level.error", 22),
        ("log.level.warning", 23),
        ("log.level.info", 24),
        ("log.level.debug", 25),
        ("comment", 1),
        ("constant.character.escape", 5),
        ("string", 2),
        ("constant.numeric", 4),
        ("constant", 5),
        ("keyword.operator", 11),
        ("keyword", 3),
        ("storage.type", 6),
        ("storage", 3),
        ("entity.name.function", 7),
        ("support.function", 7),
        ("variable.function", 7),
        ("entity.name.type", 6),
        ("entity.name.class", 6),
        ("entity.name.struct", 6),
        ("entity.name.enum", 6),
        ("entity.other.inherited-class", 6),
        ("support.type", 6),
        ("support.class", 6),
        ("entity.name.tag", 8),
        ("entity.other.attribute-name", 9),
        ("entity.name.section", 13),
        ("markup.heading", 13),
        ("markup.bold", 14),
        ("markup.italic", 15),
        ("markup.raw", 16),
        ("markup.inline.raw", 16),
        ("markup.underline.link", 17),
        ("markup.inserted", 19),
        ("markup.deleted", 20),
        ("markup.changed", 21),
        ("invalid", 18),
        ("variable.parameter", 10),
        ("variable.language", 5),
        ("variable", 10),
        ("support.constant", 5),
    ];
    RULES
        .iter()
        .find(|(prefix, _)| {
            name == *prefix || (name.starts_with(prefix) && name.as_bytes()[prefix.len()] == b'.')
        })
        .map(|(_, c)| *c)
}

/// Per line: `(byte offset in the line, class)` where the class changes; `None` = not highlighted.
pub type LineStyles = Option<Vec<(usize, u8)>>;

/// A parse state with its scope stack.
///
/// SAFETY: with Oniguruma, `ParseState` holds a capture `Region` – a heap buffer owned by this
/// state and only reached through it. The states live behind a `Mutex` or are cloned before
/// they move to the catch-up thread, so no two threads ever touch the same buffer; Oniguruma has
/// no thread affinity. That makes moving them between threads sound (design 008 E5).
#[derive(Debug, Clone)]
struct Snapshot(ParseState, ScopeStack);

unsafe impl Send for Snapshot {}

type Checkpoints = BTreeMap<usize, Snapshot>;

/// Exact states at line starts, shared with the catch-up thread.
#[derive(Debug, Default)]
struct Shared {
    checkpoints: Checkpoints,
    /// Lowest line edited while the catch-up thread runs: its states up to there stay valid
    /// (the text above an edit does not change), later ones are thrown away.
    invalid_from: usize,
    working: bool,
}

#[derive(Debug)]
pub struct Highlighter {
    pub language: String,
    initial: Snapshot,
    shared: Arc<Mutex<Shared>>,
    /// Styles of recently drawn lines, valid for `cache_revision`.
    cache: HashMap<usize, LineStyles>,
    cache_revision: u64,
    cache_approx: bool,
    classes: HashMap<Scope, Option<u8>>,
}

impl Highlighter {
    pub fn new(language: &str) -> Option<Self> {
        let syntax = syntax_set().find_syntax_by_name(language)?;
        let initial = Snapshot(ParseState::new(syntax), ScopeStack::new());
        let mut shared = Shared::default();
        shared.checkpoints.insert(0, initial.clone());
        Some(Self {
            language: language.to_owned(),
            initial,
            shared: Arc::new(Mutex::new(shared)),
            cache: HashMap::new(),
            cache_revision: u64::MAX,
            cache_approx: false,
            classes: HashMap::new(),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The text changed from `line` on: later states are wrong now.
    pub fn invalidate_from(&mut self, line: usize) {
        let initial = self.initial.clone();
        let mut s = self.lock();
        s.invalid_from = s.invalid_from.min(line);
        s.checkpoints.split_off(&(line + 1));
        if s.checkpoints.is_empty() {
            s.checkpoints.insert(0, initial);
        }
        drop(s);
        self.cache.clear();
    }

    /// Number of exact checkpoints (tests, measurements).
    pub fn checkpoint_count(&self) -> usize {
        self.lock().checkpoints.len()
    }

    /// Parses `[from, to)` of a rope snapshot in the background and leaves checkpoints
    /// (design 008 E3); stops when the text changes.
    #[cfg(not(target_arch = "wasm32"))]
    fn catch_up(&self, rope: Rope, to: usize) {
        let shared = self.shared.clone();
        let (from, snap) = {
            let mut s = self.lock();
            if s.working {
                return;
            }
            s.working = true;
            s.invalid_from = usize::MAX;
            s.checkpoints
                .range(..=to)
                .next_back()
                .map(|(l, v)| (*l, v.clone()))
                .unwrap_or((0, self.initial.clone()))
        };
        std::thread::spawn(move || {
            // capture the whole `Snapshot` (it is `Send`), not its fields
            let snap = snap;
            let ss = syntax_set();
            let Snapshot(mut state, mut stack) = snap;
            let mut found = Checkpoints::new();
            for l in from..to {
                if l % CHECKPOINT_EVERY == 0 && l > from {
                    found.insert(l, Snapshot(state.clone(), stack.clone()));
                    if l % (CHECKPOINT_EVERY * 16) == 0 {
                        let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                        let limit = s.invalid_from;
                        found.retain(|k, _| *k <= limit);
                        s.checkpoints.append(&mut found);
                        if l > limit {
                            s.working = false;
                            return;
                        }
                    }
                }
                let Some(text) = rope_line(&rope, l) else {
                    continue;
                };
                let Ok(ops) = state.parse_line(&text, ss) else {
                    break;
                };
                for op in ops {
                    if stack.apply(&op.1).is_err() {
                        break;
                    }
                }
            }
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            let limit = s.invalid_from;
            found.retain(|k, _| *k <= limit);
            s.checkpoints.append(&mut found);
            s.working = false;
        });
    }

    #[cfg(target_arch = "wasm32")]
    fn catch_up(&self, _rope: Rope, _to: usize) {}

    fn class_of(&mut self, stack: &ScopeStack) -> u8 {
        let mut punct = false;
        for s in stack.as_slice().iter().rev() {
            let c = *self.classes.entry(*s).or_insert_with(|| {
                let name = s.build_string();
                if name.starts_with("punctuation") {
                    Some(254)
                } else if name.starts_with("meta")
                    || name.starts_with("source")
                    || name.starts_with("text")
                {
                    None
                } else {
                    class_of_scope(&name)
                }
            });
            match c {
                Some(254) => punct = true,
                Some(c) => return c,
                None => {}
            }
        }
        if punct { 12 } else { 0 }
    }

    /// Styles of the lines `[first, end)`; `true` = approximate (started fresh, design 008 E3).
    pub fn styles(&mut self, doc: &Document, first: usize, end: usize) -> (Vec<LineStyles>, bool) {
        let rev = doc.revision();
        if rev != self.cache_revision {
            self.cache.clear();
            self.cache_revision = rev;
            self.cache_approx = false;
        }
        if (first..end).all(|l| self.cache.contains_key(&l)) {
            let v = (first..end).map(|l| self.cache[&l].clone()).collect();
            return (v, self.cache_approx);
        }
        let ss = syntax_set();
        let (cp_line, Snapshot(state, stack)) = self
            .lock()
            .checkpoints
            .range(..=first)
            .next_back()
            .map(|(l, s)| (*l, s.clone()))
            .unwrap_or((0, self.initial.clone()));
        let (start, mut state, mut stack, exact) = if first - cp_line > MAX_CATCHUP {
            self.catch_up(doc.rope().clone(), first);
            let s = first.saturating_sub(FRESH_CONTEXT);
            (s, self.initial.0.clone(), ScopeStack::new(), false)
        } else {
            (cp_line, state, stack, true)
        };
        let mut found = Checkpoints::new();
        let mut out = Vec::with_capacity(end.saturating_sub(first));
        let mut broken = false;
        for l in start..end {
            if exact && l % CHECKPOINT_EVERY == 0 && l > cp_line {
                found.insert(l, Snapshot(state.clone(), stack.clone()));
            }
            let len = doc.line_len(l);
            let styles = if broken || len > MAX_HIGHLIGHT_LINE {
                None
            } else {
                let text = doc.line_string(l);
                match state.parse_line(&text, ss) {
                    Ok(ops) => {
                        let mut spans: Vec<(usize, u8)> = Vec::new();
                        let mut last = u8::MAX;
                        let mut push = |at: usize, c: u8, spans: &mut Vec<(usize, u8)>| {
                            if c != last {
                                if spans.last().is_some_and(|s| s.0 == at) {
                                    spans.pop();
                                }
                                spans.push((at, c));
                                last = c;
                            }
                        };
                        push(0, self.class_of(&stack), &mut spans);
                        for (at, op) in ops {
                            if stack.apply(&op).is_err() {
                                broken = true;
                                break;
                            }
                            if !matches!(op, ScopeStackOp::Noop) {
                                let c = self.class_of(&stack);
                                push(at, c, &mut spans);
                            }
                        }
                        Some(spans)
                    }
                    Err(_) => {
                        broken = true;
                        None
                    }
                }
            };
            if l >= first {
                self.cache.insert(l, styles.clone());
                out.push(styles);
            }
        }
        if !found.is_empty() {
            self.lock().checkpoints.append(&mut found);
        }
        if self.cache.len() > 4_000 {
            self.cache.clear();
        }
        self.cache_approx = !exact;
        (out, !exact)
    }
}

/// A line of a rope without its break; `None` for lines too long to highlight.
fn rope_line(rope: &Rope, l: usize) -> Option<String> {
    if l >= rope.len_lines() {
        return None;
    }
    let slice = rope.line(l);
    if slice.len_chars() > MAX_HIGHLIGHT_LINE + 2 {
        return None;
    }
    let mut s = slice.to_string();
    while s.ends_with(['\n', '\r']) {
        s.pop();
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes_of(doc: &Document, line: usize) -> Vec<(String, &'static str)> {
        let lang = detect(Some("x.rs"), "").unwrap();
        let mut h = Highlighter::new(&lang).unwrap();
        let (styles, _) = h.styles(doc, 0, doc.line_count());
        let text = doc.line_string(line);
        let spans = styles[line].clone().unwrap();
        spans
            .iter()
            .enumerate()
            .map(|(i, (at, c))| {
                let to = spans.get(i + 1).map_or(text.len(), |s| s.0);
                (text[*at..to].to_owned(), CLASSES[*c as usize])
            })
            .filter(|(t, _)| !t.trim().is_empty())
            .collect()
    }

    #[test]
    fn detects_languages() {
        assert_eq!(detect(Some("a.rs"), "").as_deref(), Some("Rust"));
        assert_eq!(detect(Some("a.ts"), "").as_deref(), Some("JavaScript"));
        assert_eq!(detect(Some("server.log"), "").as_deref(), Some("Log"));
        assert_eq!(
            detect(Some("Cargo.toml"), "").as_deref(),
            Some("INI / Properties / TOML")
        );
        assert_eq!(
            detect(Some("script"), "#!/bin/bash").as_deref(),
            Some("Bourne Again Shell (bash)")
        );
        assert_eq!(detect(Some("notes.txt"), "hello"), None);
        assert!(languages().len() > 30);
    }

    #[test]
    fn rust_tokens_get_classes() {
        let doc = Document::from_text("// note\nfn main() { let s = \"hi\"; 42 }\n");
        let c = classes_of(&doc, 0);
        assert_eq!(c, vec![("// note".to_owned(), "comment")]);
        let c = classes_of(&doc, 1);
        assert!(
            c.contains(&("fn".to_owned(), "keyword")) || c.contains(&("fn".to_owned(), "type")),
            "{c:?}"
        );
        assert!(
            c.iter().any(|(t, k)| t.contains("hi") && *k == "string"),
            "{c:?}"
        );
        assert!(c.contains(&("42".to_owned(), "number")), "{c:?}");
    }

    #[test]
    fn block_comments_carry_over_lines_and_edits_invalidate() {
        let mut text = String::from("/* start\n");
        for _ in 0..500 {
            text.push_str("still comment\n");
        }
        text.push_str("end */ let x = 1;\n");
        let doc = Document::from_text(&text);
        let mut h = Highlighter::new("Rust").unwrap();
        let (s, approx) = h.styles(&doc, 300, 302);
        assert!(!approx);
        assert_eq!(
            s[0].as_ref().unwrap()[0].1,
            1,
            "line 300 is inside the comment"
        );
        h.invalidate_from(0);
        let (s, _) = h.styles(&doc, 300, 301);
        assert_eq!(s[0].as_ref().unwrap()[0].1, 1);
    }

    #[test]
    fn far_jumps_are_approximate() {
        let text = "let x = 1;\n".repeat(MAX_CATCHUP + 2_000);
        let doc = Document::from_text(&text);
        let mut h = Highlighter::new("Rust").unwrap();
        let (_, approx) = h.styles(&doc, MAX_CATCHUP + 1_500, MAX_CATCHUP + 1_510);
        assert!(approx);
        // the catch-up thread fills the gap; then the same view is exact
        let t = std::time::Instant::now();
        while h.lock().working && t.elapsed().as_secs() < 20 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        h.cache.clear();
        let (_, approx) = h.styles(&doc, MAX_CATCHUP + 1_500, MAX_CATCHUP + 1_510);
        assert!(!approx);
        assert!(h.checkpoint_count() > 10);
    }

    #[test]
    fn log_levels() {
        let doc = Document::from_text("2026-10-26 08:00:00 ERROR boom\n  at x.y(Z.java:1)\n");
        let mut h = Highlighter::new("Log").unwrap();
        let (s, _) = h.styles(&doc, 0, 2);
        let classes: Vec<u8> = s[0].as_ref().unwrap().iter().map(|x| x.1).collect();
        assert!(classes.contains(&22), "{classes:?}");
        assert!(classes.contains(&4));
        assert_eq!(s[1].as_ref().unwrap()[0].1, 1, "stack trace line");
    }
}
