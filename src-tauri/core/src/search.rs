//! Search and replace on the rope (design 006): the text is searched in blocks of whole lines
//! (≤ 1 MiB, a longer line is its own block) so huge files never become one string. Matches are
//! character ranges of the rope. Pure – the session and the commands use it.

use crate::error::{FsError, FsErrorCode};
use regex::{Regex, RegexBuilder};
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};

/// Target size of a search block in bytes.
pub const BLOCK: usize = 1 << 20;
/// Largest compiled pattern (protects against pathological input).
const REGEX_SIZE_LIMIT: usize = 8 << 20;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Query {
    pub pattern: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The regex for a query; `invalid_search` with the parser's message for a bad pattern.
pub fn compile(q: &Query) -> Result<Regex, FsError> {
    if q.pattern.is_empty() {
        return Err(FsError::new(FsErrorCode::InvalidSearch, "empty", None));
    }
    let mut pat = if q.regex {
        q.pattern.clone()
    } else {
        regex::escape(&q.pattern)
    };
    if q.whole_word {
        if q.regex {
            pat = format!(r"\b(?:{pat})\b");
        } else {
            let first = q.pattern.chars().next().is_some_and(is_word);
            let last = q.pattern.chars().last().is_some_and(is_word);
            pat = format!(
                "{}{pat}{}",
                if first { r"\b" } else { "" },
                if last { r"\b" } else { "" }
            );
        }
    }
    RegexBuilder::new(&pat)
        .case_insensitive(!q.case_sensitive)
        .multi_line(true)
        .size_limit(REGEX_SIZE_LIMIT)
        .build()
        .map_err(|e| FsError::new(FsErrorCode::InvalidSearch, e.to_string(), None))
}

/// The text of a byte range (borrowed when it lies in one chunk).
fn text_of(rope: &Rope, a: usize, b: usize) -> Cow<'_, str> {
    let s = rope.byte_slice(a..b);
    match s.as_str() {
        Some(t) => Cow::Borrowed(t),
        None => Cow::Owned(s.to_string()),
    }
}

/// End (byte) of the block of whole lines that starts at the line containing `byte`.
fn block_end(rope: &Rope, start: usize) -> usize {
    let len = rope.len_bytes();
    let target = (start + BLOCK).min(len);
    if target >= len {
        return len;
    }
    let line = rope.byte_to_line(target);
    let end = if line + 1 < rope.len_lines() {
        rope.line_to_byte(line + 1)
    } else {
        len
    };
    end.max(start + 1).min(len)
}

/// Start (byte) of the block of whole lines that ends at `end`.
fn block_start(rope: &Rope, end: usize) -> usize {
    if end <= BLOCK {
        return 0;
    }
    let line = rope.byte_to_line(end - BLOCK);
    rope.line_to_byte(line).min(end.saturating_sub(1))
}

/// Like [`each_match_bytes`] with character positions.
fn each_match(
    rope: &Rope,
    re: &Regex,
    from: usize,
    to: usize,
    cancel: Option<&AtomicBool>,
    f: &mut dyn FnMut(usize, usize) -> bool,
) -> bool {
    each_match_bytes(rope, re, from, to, cancel, &mut |s, e| {
        f(rope.byte_to_char(s), rope.byte_to_char(e))
    })
}

/// Calls `f(start_byte, end_byte)` for every non-empty match in bytes `[from, to)` (whole lines
/// around them give `^`, `$` and `\b` their context); `false` from `f` stops.
fn each_match_bytes(
    rope: &Rope,
    re: &Regex,
    from: usize,
    to: usize,
    cancel: Option<&AtomicBool>,
    f: &mut dyn FnMut(usize, usize) -> bool,
) -> bool {
    let line_start = rope.line_to_byte(rope.byte_to_line(from));
    let mut a = line_start;
    while a < to {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return false;
        }
        let b = block_end(rope, a);
        let text = text_of(rope, a, b);
        let at = from.saturating_sub(a).min(text.len());
        // find_iter steps over empty matches itself – calling find_at once per character made
        // patterns like `a*` hundreds of times slower (design 013 review #11)
        for m in re.find_iter(&text) {
            if m.start() < at || m.end() == m.start() {
                continue;
            }
            if a + m.start() >= to {
                break;
            }
            if !f(a + m.start(), a + m.end()) {
                return false;
            }
        }
        a = b;
    }
    true
}

/// The first match starting at or after char `from`, wrapping around to the start.
pub fn find_next(rope: &Rope, re: &Regex, from: usize) -> Option<(usize, usize)> {
    let from_b = rope.char_to_byte(from.min(rope.len_chars()));
    let len = rope.len_bytes();
    let mut found = None;
    each_match(rope, re, from_b, len, None, &mut |s, e| {
        found = Some((s, e));
        false
    });
    if found.is_none() && from_b > 0 {
        each_match(rope, re, 0, from_b, None, &mut |s, e| {
            found = Some((s, e));
            false
        });
    }
    found
}

/// The last match starting before char `before`, wrapping around to the end.
pub fn find_prev(rope: &Rope, re: &Regex, before: usize) -> Option<(usize, usize)> {
    let limit = rope.char_to_byte(before.min(rope.len_chars()));
    let last_in = |from: usize, to: usize| {
        let mut last = None;
        each_match(rope, re, from, to, None, &mut |s, e| {
            last = Some((s, e));
            true
        });
        last
    };
    // walk back block by block
    let mut end = limit;
    while end > 0 {
        let start = block_start(rope, end);
        if let Some(m) = last_in(start, end) {
            return Some(m);
        }
        end = start;
    }
    let len = rope.len_bytes();
    let mut end = len;
    while end > limit {
        let start = block_start(rope, end).max(limit);
        if let Some(m) = last_in(start, end) {
            return Some(m);
        }
        if start == limit {
            break;
        }
        end = start;
    }
    None
}

/// How many matches there are, and how many of them start before char `before`.
/// `None` when cancelled.
pub fn count(
    rope: &Rope,
    re: &Regex,
    before: usize,
    cancel: &AtomicBool,
) -> Option<(usize, usize)> {
    let (mut total, mut ahead) = (0usize, 0usize);
    let ok = each_match(rope, re, 0, rope.len_bytes(), Some(cancel), &mut |s, _| {
        total += 1;
        if s < before {
            ahead += 1;
        }
        true
    });
    ok.then_some((total, ahead))
}

/// The first `max` matches from the start as char ranges of the rope (AI `find`, design 016).
pub fn first_matches(rope: &Rope, re: &Regex, max: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if max == 0 {
        return out;
    }
    each_match(rope, re, 0, rope.len_bytes(), None, &mut |s, e| {
        out.push((s, e));
        out.len() < max
    });
    out
}

/// Matches in a piece of text (a visible line): char ranges relative to the text.
pub fn matches_in(text: &str, re: &Regex) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut chars = 0usize;
    let mut last = 0usize;
    for m in re.find_iter(text) {
        if m.end() == m.start() {
            continue;
        }
        chars += text[last..m.start()].chars().count();
        let len = text[m.start()..m.end()].chars().count();
        out.push((chars, chars + len));
        chars += len;
        last = m.end();
        if out.len() >= 10_000 {
            break;
        }
    }
    out
}

/// Replacement text in regex mode: `\n`, `\t`, `\\` escapes, then `$1`/`${name}` groups –
/// the groups of the match at byte `at` of `haystack` (in its context, design 013 review #6).
pub fn expand_at(
    re: &Regex,
    haystack: &str,
    at: usize,
    replacement: &str,
    regex_mode: bool,
) -> String {
    if !regex_mode {
        return replacement.to_owned();
    }
    let unescaped = unescape(replacement);
    match re.captures_at(haystack, at) {
        Some(caps) => {
            let mut out = String::new();
            caps.expand(&unescaped, &mut out);
            out
        }
        None => unescaped,
    }
}

/// [`expand_at`] for a match on its own (tests).
pub fn expand(re: &Regex, matched: &str, replacement: &str, regex_mode: bool) -> String {
    expand_at(re, matched, 0, replacement, regex_mode)
}

fn expand_caps(caps: &regex::Captures, unescaped: &str) -> String {
    let mut out = String::new();
    caps.expand(unescaped, &mut out);
    out
}

fn unescape(replacement: &str) -> String {
    let mut unescaped = String::with_capacity(replacement.len());
    let mut it = replacement.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.peek() {
                Some('n') => {
                    it.next();
                    unescaped.push('\n');
                }
                Some('t') => {
                    it.next();
                    unescaped.push('\t');
                }
                Some('\\') => {
                    it.next();
                    unescaped.push('\\');
                }
                _ => unescaped.push(c),
            }
        } else {
            unescaped.push(c);
        }
    }
    unescaped
}

/// Every match with its replacement: `(start_char, end_char, text)` – for "replace all".
/// Empty matches count too here (`^` → prefix every line).
pub fn replacements(
    rope: &Rope,
    re: &Regex,
    replacement: &str,
    regex_mode: bool,
    cancel: &AtomicBool,
) -> Option<Vec<(usize, usize, String)>> {
    let mut out = Vec::new();
    let len = rope.len_bytes();
    let mut a = 0usize;
    while a < len || (len == 0 && a == 0) {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let b = if len == 0 { 0 } else { block_end(rope, a) };
        let text = text_of(rope, a, b);
        let unescaped = unescape(replacement);
        for caps in re.captures_iter(&text) {
            let Some(m) = caps.get(0) else { continue };
            let s = rope.byte_to_char(a + m.start());
            let e = rope.byte_to_char(a + m.end());
            let text = if regex_mode {
                expand_caps(&caps, &unescaped)
            } else {
                replacement.to_owned()
            };
            out.push((s, e, text));
        }
        if b == a {
            break;
        }
        a = b;
    }
    // an empty match at the very end of one block and the start of the next is the same place
    out.dedup_by(|x, y| x.0 == y.0 && x.1 == y.1);
    Some(out)
}

/// Overview of the document for the scrollbar (design 010 E3): the lines are cut into
/// `buckets` equal slices; a slice gets the bits of every layer with a match in it.
/// `None` = cancelled.
pub fn overview(
    rope: &Rope,
    layers: &[(Regex, u8)],
    buckets: usize,
    cancel: &AtomicBool,
) -> Option<Vec<u8>> {
    let buckets = buckets.clamp(1, 4096);
    let mut out = vec![0u8; buckets];
    let lines = rope.len_lines().max(1);
    let bucket_of = |line: usize| (line * buckets / lines).min(buckets - 1);
    let len = rope.len_bytes();
    let mut a = 0usize;
    while a < len {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let b = block_end(rope, a);
        let text = text_of(rope, a, b);
        for (re, bit) in layers {
            // one hit per bucket is enough: jump to the next bucket's first byte
            let mut from = 0;
            while from <= text.len() {
                let Some(m) = re.find_at(&text, from) else {
                    break;
                };
                let line = rope.byte_to_line(a + m.start());
                let k = bucket_of(line);
                out[k] |= bit;
                // first line of the next bucket
                let next_line = ((k + 1) * lines).div_ceil(buckets);
                if next_line >= lines {
                    break;
                }
                let next_byte = rope.line_to_byte(next_line);
                if next_byte >= b {
                    break;
                }
                from = (next_byte - a).max(m.end().max(from + 1));
                while from < text.len() && !text.is_char_boundary(from) {
                    from += 1;
                }
            }
        }
        if b == a {
            break;
        }
        a = b;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_marks_buckets_per_layer() {
        let text: String = (0..100)
            .map(|i| {
                if i == 10 || i == 95 {
                    "foo\n"
                } else if i == 50 {
                    "bar\n"
                } else {
                    "x\n"
                }
            })
            .collect();
        let rope = Rope::from_str(&text);
        let layers = [
            (Regex::new("foo").unwrap(), 1u8),
            (Regex::new("bar").unwrap(), 2u8),
        ];
        let o = overview(&rope, &layers, 10, &AtomicBool::new(false)).unwrap();
        // 101 lines (the last one empty) in 10 buckets
        assert_eq!(o[0] | o[1], 1);
        assert_eq!(o[4] | o[5], 2);
        assert_eq!(o[9], 1);
        assert_eq!(o.iter().filter(|&&b| b != 0).count(), 3);
        assert!(overview(&rope, &layers, 10, &AtomicBool::new(true)).is_none());
    }

    fn q(p: &str) -> Query {
        Query {
            pattern: p.into(),
            ..Default::default()
        }
    }

    #[test]
    fn literal_case_and_words() {
        let rope = Rope::from_str("Foo foo food FOO.bar");
        let re = compile(&q("foo")).unwrap();
        assert_eq!(find_next(&rope, &re, 0), Some((0, 3)));
        assert_eq!(find_next(&rope, &re, 1), Some((4, 7)));
        let re = compile(&Query {
            pattern: "foo".into(),
            case_sensitive: true,
            whole_word: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(find_next(&rope, &re, 0), Some((4, 7)));
        assert_eq!(find_next(&rope, &re, 5), Some((4, 7)), "wraps around");
        let re = compile(&Query {
            pattern: ".bar".into(),
            whole_word: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(find_next(&rope, &re, 0), Some((16, 20)), "dot is literal");
    }

    #[test]
    fn regex_errors_are_reported() {
        assert_eq!(
            compile(&Query {
                pattern: "(".into(),
                regex: true,
                ..Default::default()
            })
            .unwrap_err()
            .code,
            FsErrorCode::InvalidSearch
        );
        assert!(compile(&q("")).is_err());
    }

    #[test]
    fn previous_match_wraps() {
        let rope = Rope::from_str("ab\nab\nab");
        let re = compile(&q("ab")).unwrap();
        assert_eq!(find_prev(&rope, &re, 6), Some((3, 5)));
        assert_eq!(find_prev(&rope, &re, 0), Some((6, 8)));
    }

    #[test]
    fn anchors_see_whole_lines_even_mid_line() {
        let rope = Rope::from_str("xab\nab");
        let re = compile(&Query {
            pattern: "^ab".into(),
            regex: true,
            ..Default::default()
        })
        .unwrap();
        // searching from inside the first line must not treat "ab" as a line start
        assert_eq!(find_next(&rope, &re, 1), Some((4, 6)));
    }

    #[test]
    fn unicode_positions_are_chars() {
        let rope = Rope::from_str("äöü 😀 x😀");
        let re = compile(&q("😀")).unwrap();
        assert_eq!(find_next(&rope, &re, 0), Some((4, 5)));
        assert_eq!(find_next(&rope, &re, 5), Some((7, 8)));
        assert_eq!(matches_in("äö😀x😀", &re), vec![(2, 3), (4, 5)]);
    }

    #[test]
    fn blocks_cover_big_texts() {
        let line = "abcdefghij".repeat(10) + "\n";
        let mut text = line.repeat(30_000); // ~3 MB, several blocks
        text.push_str("needle\n");
        text.push_str(&line.repeat(30_000));
        let rope = Rope::from_str(&text);
        let re = compile(&q("needle")).unwrap();
        let at = 30_000 * 101;
        assert_eq!(find_next(&rope, &re, 0), Some((at, at + 6)));
        assert_eq!(find_prev(&rope, &re, rope.len_chars()), Some((at, at + 6)));
        let (total, before) = count(&rope, &re, at + 1, &AtomicBool::new(false)).unwrap();
        assert_eq!((total, before), (1, 1));
        let re = compile(&q("j\n")).unwrap();
        let (total, _) = count(&rope, &re, 0, &AtomicBool::new(false)).unwrap();
        assert_eq!(total, 60_000, "every line, across block borders");
    }

    #[test]
    fn replacement_groups_and_escapes() {
        let re = compile(&Query {
            pattern: r"(\w+)@(\w+)".into(),
            regex: true,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(expand(&re, "me@host", r"$2\t$1\n", true), "host\tme\n");
        assert_eq!(expand(&re, "me@host", "$2", false), "$2");
    }

    #[test]
    fn replace_all_including_empty_matches() {
        let rope = Rope::from_str("a\nb\n");
        let re = compile(&Query {
            pattern: "^".into(),
            regex: true,
            ..Default::default()
        })
        .unwrap();
        let r = replacements(&rope, &re, "> ", true, &AtomicBool::new(false)).unwrap();
        assert_eq!(
            r.iter().map(|x| x.0).collect::<Vec<_>>(),
            vec![0, 2, 4],
            "line starts"
        );
        let cancelled = replacements(&rope, &re, "", true, &AtomicBool::new(true));
        assert!(cancelled.is_none());
    }

    /// `cargo test -p ed-core --release -- --ignored --nocapture measure_` (design 006).
    #[test]
    #[ignore]
    fn measure_search() {
        use std::time::Instant;
        let line = "2026-10-26 08:00:00 INFO  [main] editlicious handles big files with ease\n";
        let mut text = line.repeat(1_400_000); // ~100 MB
        text.push_str("needle at the end\n");
        let rope = Rope::from_str(&text);
        drop(text);
        let re = compile(&q("needle")).unwrap();
        let t = Instant::now();
        assert!(find_next(&rope, &re, 0).is_some());
        println!("find at the end of 100 MB: {:?}", t.elapsed());
        let re = compile(&q("ease")).unwrap();
        let t = Instant::now();
        let (n, _) = count(&rope, &re, 0, &AtomicBool::new(false)).unwrap();
        println!("count {n} matches in 100 MB: {:?}", t.elapsed());
        let t = Instant::now();
        let r = replacements(&rope, &re, "joy", false, &AtomicBool::new(false)).unwrap();
        println!("collect {} replacements: {:?}", r.len(), t.elapsed());
    }
}
