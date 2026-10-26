//! Progressive selection by repeated double clicks (prompt 018a): the word under the pointer,
//! then step by step larger units – across `_`, `-`, `.`, path separators, brackets and quotes,
//! words, the line and the paragraph – and then from the start again.
//!
//! One rule for every case: separators have **weights** (`_` 1 < `-` 2 < `.` 3 < `/` 3.5 <
//! other punctuation 3.8 < space 4 < tab 5). A step crosses the **lightest** boundary next to the
//! selection and takes the piece behind it – everything up to the next boundary at least as
//! heavy. With equal weights left and right the smaller piece wins, then the left one. A bracket
//! or quote pair around the selection is a candidate too (first its content, then with the
//! delimiters); of all candidates the smallest that is larger than the selection is next.

use crate::document::Document;
use crate::document::model::Pos;
use std::collections::HashMap;

/// Weights ×10 (integers): `_` 10, `-` 20, `.` 30, path/URL/key separators 35, other
/// punctuation 38, space 40, tab 50.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weights {
    chars: HashMap<char, u8>,
    punct: u8,
    space: u8,
    tab: u8,
}

impl Default for Weights {
    fn default() -> Self {
        let mut chars = HashMap::new();
        chars.insert('_', 10);
        chars.insert('-', 20);
        chars.insert('.', 30);
        for c in ['/', '\\', ':', '@', '?', '#', '&', '='] {
            chars.insert(c, 35);
        }
        Self {
            chars,
            punct: 38,
            space: 40,
            tab: 50,
        }
    }
}

/// Above every separator inside a line.
const LINE: u8 = 255;
/// Lines of a paragraph looked at, at most (huge files).
const MAX_PARAGRAPH_LINES: usize = 10_000;

impl Weights {
    /// Overrides from the settings: a character (`"_": 1`) or `space`, `tab`, `punctuation`;
    /// the values like the table (1–9, decimals allowed as tenths: `3.5`).
    pub fn with(overrides: &HashMap<String, f64>) -> Self {
        let mut w = Self::default();
        for (k, v) in overrides {
            let v = (v * 10.0).round().clamp(1.0, 90.0) as u8;
            match k.as_str() {
                "space" => w.space = v,
                "tab" => w.tab = v,
                "punctuation" => w.punct = v,
                k => {
                    let mut it = k.chars();
                    if let (Some(c), None) = (it.next(), it.next())
                        && !c.is_alphanumeric()
                    {
                        w.chars.insert(c, v);
                    }
                }
            }
        }
        w
    }

    /// 0 = part of a word (letters, digits, combining marks, emoji …).
    pub fn of(&self, c: char) -> u8 {
        if c == '\t' {
            return self.tab;
        }
        if c.is_whitespace() {
            return self.space;
        }
        if let Some(&w) = self.chars.get(&c) {
            return w;
        }
        if c.is_ascii_punctuation() || is_unicode_punct(c) {
            return self.punct;
        }
        0
    }
}

/// Punctuation outside ASCII that separates words (quotes, dashes, brackets).
fn is_unicode_punct(c: char) -> bool {
    matches!(
        c,
        '«' | '»'
            | '„'
            | '“'
            | '”'
            | '‚'
            | '‘'
            | '’'
            | '‹'
            | '›'
            | '–'
            | '—'
            | '…'
            | '·'
            | '§'
            | '¿'
            | '¡'
            | '「'
            | '」'
            | '『'
            | '』'
            | '（'
            | '）'
            | '【'
            | '】'
            | '、'
            | '。'
            | '，'
    )
}

type Span = (usize, usize);

fn contains(outer: Span, inner: Span) -> bool {
    outer.0 <= inner.0 && inner.1 <= outer.1 && outer != inner
}

/// The word at `ch` (a run of weight-0 characters); on a separator: the word before it, else
/// the run of that separator.
fn word(line: &[char], ch: usize, w: &Weights) -> Span {
    let n = line.len();
    if n == 0 {
        return (0, 0);
    }
    let mut at = ch.min(n - 1);
    if w.of(line[at]) != 0 && at > 0 && w.of(line[at - 1]) == 0 {
        at -= 1;
    }
    let k = w.of(line[at]);
    let same = |c: char| if k == 0 { w.of(c) == 0 } else { c == line[at] };
    let mut a = at;
    while a > 0 && same(line[a - 1]) {
        a -= 1;
    }
    let mut b = at + 1;
    while b < n && same(line[b]) {
        b += 1;
    }
    (a, b)
}

/// Matched bracket and quote pairs of a line: positions of the two delimiters.
fn pairs(line: &[char], w: &Weights) -> Vec<Span> {
    let mut out = Vec::new();
    let mut stack: Vec<(char, usize)> = Vec::new();
    for (i, &c) in line.iter().enumerate() {
        match c {
            '(' | '[' | '{' | '<' => stack.push((c, i)),
            ')' | ']' | '}' | '>' => {
                let open = match c {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    _ => '<',
                };
                if let Some(pos) = stack.iter().rposition(|&(o, _)| o == open) {
                    let (_, o) = stack[pos];
                    stack.truncate(pos);
                    out.push((o, i));
                }
            }
            _ => {}
        }
    }
    for q in ['"', '\'', '`'] {
        // quotes pair up in order; an apostrophe inside a word is no quote
        let idx: Vec<usize> = line
            .iter()
            .enumerate()
            .filter(|&(i, &c)| {
                c == q
                    && !(q == '\''
                        && i > 0
                        && i + 1 < line.len()
                        && w.of(line[i - 1]) == 0
                        && w.of(line[i + 1]) == 0)
            })
            .map(|(i, _)| i)
            .collect();
        for p in idx.chunks_exact(2) {
            out.push((p[0], p[1]));
        }
    }
    out
}

/// One step of growth across the lightest boundary next to `s`, inside `r`; `eff` = the
/// weight of each character (bracket groups beside the selection count as word: 0).
fn grow(s: Span, r: Span, eff: &[u8]) -> Option<Span> {
    let left = (s.0 > r.0).then(|| {
        let wl = eff[s.0 - 1];
        let mut i = s.0;
        if wl == 0 {
            // a word (or bracket group) right next to the selection: take it
            while i > r.0 && eff[i - 1] == 0 {
                i -= 1;
            }
            return (0u8, (i, s.1));
        }
        // the separator run, then everything lighter than it
        while i > r.0 && eff[i - 1] == wl {
            i -= 1;
        }
        while i > r.0 && eff[i - 1] < wl {
            i -= 1;
        }
        (wl, (i, s.1))
    });
    let right = (s.1 < r.1).then(|| {
        let wr = eff[s.1];
        let mut j = s.1;
        if wr == 0 {
            while j < r.1 && eff[j] == 0 {
                j += 1;
            }
            return (0u8, (s.0, j));
        }
        while j < r.1 && eff[j] == wr {
            j += 1;
        }
        while j < r.1 && eff[j] < wr {
            j += 1;
        }
        (wr, (s.0, j))
    });
    match (left, right) {
        (None, None) => None,
        (Some((_, l)), None) => Some(l),
        (None, Some((_, r))) => Some(r),
        (Some((wl, l)), Some((wr, r))) => {
            let (gl, gr) = (s.0 - l.0, r.1 - s.1);
            Some(if wl < wr || (wl == wr && gl <= gr) {
                l
            } else {
                r
            })
        }
    }
}

/// Every step inside one line, from the word at `ch` to the whole line (characters).
pub fn line_steps(text: &str, ch: usize, w: &Weights) -> Vec<Span> {
    let line: Vec<char> = text.chars().collect();
    let n = line.len();
    let mut s = word(&line, ch, w);
    let mut out = vec![s];
    let all_pairs = pairs(&line, w);
    // whitespace at both ends of the line belongs to the line step, not to a word step
    let lead = line.iter().take_while(|c| c.is_whitespace()).count();
    let trail = n - line.iter().rev().take_while(|c| c.is_whitespace()).count();
    let content = (lead.min(trail), trail.max(lead));
    loop {
        // the innermost pair around the selection limits the growth (its content) …
        let around: Vec<Span> = all_pairs
            .iter()
            .copied()
            .filter(|&(o, c)| o < s.0 && s.1 <= c)
            .collect();
        let r = around
            .iter()
            .max_by_key(|&&(o, _)| o)
            .map_or(content, |&(o, c)| (o + 1, c));
        // … and pairs beside it are whole groups
        let mut eff: Vec<u8> = line.iter().map(|&c| w.of(c)).collect();
        for &(o, c) in &all_pairs {
            if o >= r.0 && c < r.1 && (c < s.0 || o >= s.1) {
                eff[o..=c].iter_mut().for_each(|x| *x = 0);
            }
        }
        let mut cands: Vec<Span> = Vec::new();
        for &(o, c) in &around {
            cands.push((o + 1, c));
            cands.push((o, c + 1));
        }
        if let Some(g) = grow(s, r, &eff) {
            cands.push(g);
        }
        if contains(content, s) {
            cands.push(content);
        }
        let Some(next) = cands
            .into_iter()
            .filter(|&c| contains(c, s))
            .min_by_key(|&(a, b)| (b - a, a))
        else {
            break;
        };
        out.push(next);
        s = next;
    }
    if s != (0, n) && n > 0 {
        out.push((0, n));
    }
    out
}

/// The whole sequence at `p`: steps in the line, then the paragraph (lines between blank
/// lines), as positions. `LINE` keeps the last step inside one line.
pub fn steps(doc: &Document, p: Pos, w: &Weights) -> Vec<(Pos, Pos)> {
    let line = p.line;
    let text = doc.line_string(line);
    let mut out: Vec<(Pos, Pos)> = line_steps(&text, p.ch, w)
        .into_iter()
        .map(|(a, b)| (Pos::new(line, a), Pos::new(line, b)))
        .collect();
    let _ = LINE;
    if text.trim().is_empty() {
        return out;
    }
    let blank = |l: usize| doc.line_slice(l, 0, 200).trim().is_empty() && doc.line_len(l) < 200;
    let mut first = line;
    while first > 0 && line - first < MAX_PARAGRAPH_LINES && !blank(first - 1) {
        first -= 1;
    }
    let count = doc.line_count();
    let mut last = line;
    while last + 1 < count && last - line < MAX_PARAGRAPH_LINES && !blank(last + 1) {
        last += 1;
    }
    if first != line || last != line {
        out.push((Pos::new(first, 0), Pos::new(last, doc.line_len(last))));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The texts of every step when double-clicking at the `|`.
    fn seq(marked: &str) -> Vec<String> {
        let ch = marked.chars().position(|c| c == '|').unwrap();
        let text: String = marked.chars().filter(|&c| c != '|').collect();
        let chars: Vec<char> = text.chars().collect();
        line_steps(&text, ch, &Weights::default())
            .into_iter()
            .map(|(a, b)| chars[a..b].iter().collect())
            .collect()
    }

    #[test]
    fn simple_word() {
        assert_eq!(seq("hel|lo"), ["hello"]);
        assert_eq!(seq("|hello"), ["hello"]);
        assert_eq!(seq("hello|"), ["hello"]);
    }

    #[test]
    fn underscore_one_piece_at_a_time() {
        assert_eq!(
            seq("user_pro|file_name"),
            ["profile", "user_profile", "user_profile_name"]
        );
        assert_eq!(
            seq("us|er_profile_name"),
            ["user", "user_profile", "user_profile_name"]
        );
        assert_eq!(
            seq("user_profile_na|me"),
            ["name", "profile_name", "user_profile_name"]
        );
    }

    #[test]
    fn hyphen_dot_and_mixed() {
        assert_eq!(
            seq("my-comp|onent-name"),
            ["component", "my-component", "my-component-name"]
        );
        assert_eq!(
            seq("com.example.serv|ice"),
            ["service", "example.service", "com.example.service"]
        );
        assert_eq!(
            seq("user-pro|file.name_test"),
            ["profile", "user-profile", "user-profile.name_test"]
        );
    }

    #[test]
    fn spaces_grow_word_by_word() {
        assert_eq!(seq("hel|lo world"), ["hello", "hello world"]);
        assert_eq!(
            seq("  say hel|lo world  "),
            [
                "hello",
                "say hello",
                "say hello world",
                "  say hello world  "
            ]
        );
    }

    #[test]
    fn paths_and_urls() {
        assert_eq!(
            seq("src/components/us|er-profile.ts"),
            [
                "user",
                "user-profile",
                "user-profile.ts",
                "components/user-profile.ts",
                "src/components/user-profile.ts"
            ]
        );
        let url = seq("see https://exa|mple.com/a/b now");
        assert_eq!(
            &url[..5],
            [
                "example",
                "example.com",
                "example.com/a",
                "example.com/a/b",
                "https://example.com/a/b"
            ]
        );
        assert!(
            url.contains(&"https://example.com/a/b".to_string()),
            "{url:?}"
        );
    }

    #[test]
    fn brackets_and_quotes() {
        assert_eq!(
            seq("call(us|er.name)"),
            ["user", "user.name", "(user.name)", "call(user.name)"]
        );
        assert_eq!(
            seq(r#"x = "he|llo world";"#),
            [
                "hello",
                "hello world",
                "\"hello world\"",
                "\"hello world\";",
                "= \"hello world\";",
                "x = \"hello world\";"
            ]
        );
        // a quoted group next to a word is taken as a whole
        assert_eq!(
            seq(r#"sa|y "hi" now"#),
            ["say", "say \"hi\"", "say \"hi\" now"]
        );
        // an apostrophe is no quote
        assert_eq!(seq("do|n't stop"), ["don", "don't", "don't stop"]);
        // markdown link: the text, the brackets, the whole link
        let md = seq("a [li|nk text](http://x.y) b");
        assert_eq!(
            &md[..4],
            [
                "link",
                "link text",
                "[link text]",
                "[link text](http://x.y)"
            ]
        );
    }

    #[test]
    fn umlauts_emoji_and_combining_marks() {
        assert_eq!(seq("Königs|traße"), ["Königstraße"]);
        assert_eq!(
            seq("die Königs|traße hier"),
            ["Königstraße", "die Königstraße", "die Königstraße hier"]
        );
        assert_eq!(seq("go 🙂ok|ay_x"), ["🙂okay", "🙂okay_x", "go 🙂okay_x"]);
        assert_eq!(seq("cafe\u{301}|_bar"), ["cafe\u{301}", "cafe\u{301}_bar"]);
    }

    #[test]
    fn configurable_weights() {
        let mut o = HashMap::new();
        o.insert("_".to_string(), 3.5);
        o.insert(".".to_string(), 1.0);
        let w = Weights::with(&o);
        let steps: Vec<String> = line_steps("a.b_c", 0, &w)
            .into_iter()
            .map(|(x, y)| "a.b_c".chars().collect::<Vec<_>>()[x..y].iter().collect())
            .collect();
        assert_eq!(steps, ["a", "a.b", "a.b_c"]);
    }

    #[test]
    fn every_step_grows_and_ends_at_the_line() {
        for (t, ch) in [
            ("a(b[c]d)e", 4),
            ("x  -- y", 3),
            ("", 0),
            ("((()))", 2),
            ("a.b-c_d/e f\tg", 6),
        ] {
            let s = line_steps(t, ch, &Weights::default());
            for w in s.windows(2) {
                assert!(contains(w[1], w[0]), "{t}: {s:?}");
            }
            assert_eq!(*s.last().unwrap(), (0, t.chars().count()), "{t}");
        }
    }

    #[test]
    fn paragraph_after_the_line() {
        let d = Document::from_text("one\ntwo three\nfour\n\nother");
        let s = steps(&d, Pos::new(1, 5), &Weights::default());
        let last = *s.last().unwrap();
        assert_eq!(last, (Pos::new(0, 0), Pos::new(2, 4)));
        assert_eq!(s[s.len() - 2], (Pos::new(1, 0), Pos::new(1, 9)));
        // a single line paragraph has no extra step
        let s = steps(&d, Pos::new(4, 1), &Weights::default());
        assert_eq!(*s.last().unwrap(), (Pos::new(4, 0), Pos::new(4, 5)));
    }
}
