//! Word completion (prompt 023): the word prefix at the caret, candidates from the lines around
//! it (nearest first, bounded – never proportional to the file) and from a small lexicon of
//! typical words of the language (`core/lexicon/*.txt`).

use super::words::CharClass;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Most suggestions in the list.
pub const MAX_ITEMS: usize = 12;
/// Longest prefix and longest suggested word (characters).
pub const MAX_WORD: usize = 64;
/// Characters read per line (from its start), lines and characters in all (design 023 E4).
pub const LINE_CHARS: usize = 1_000;
pub const MAX_LINES: usize = 20_000;
pub const BUDGET_CHARS: usize = 400_000;
/// Prefix length from which typing opens the list by itself (legacy todo „erst ab 2 Zeichen“).
pub const AUTO_MIN: usize = 2;

/// Settings of the word completion (design 023 E10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WordOptions {
    /// Open the list while typing (from [`AUTO_MIN`] characters).
    pub auto: bool,
    /// Offer the words of the language's lexicon.
    pub keywords: bool,
}

impl Default for WordOptions {
    fn default() -> Self {
        Self {
            auto: false,
            keywords: true,
        }
    }
}

/// The typical words of one language.
#[derive(Debug)]
pub struct Lexicon {
    pub words: Vec<&'static str>,
    /// SQL & co.: the spelling follows the prefix.
    pub case_insensitive: bool,
    /// Characters that belong to a word besides letters, digits and `_` (CSS: `-`).
    pub extra: &'static str,
}

const FILES: &[(&str, &str)] = &[
    ("c", include_str!("../../lexicon/c.txt")),
    ("cpp", include_str!("../../lexicon/cpp.txt")),
    ("csharp", include_str!("../../lexicon/csharp.txt")),
    ("css", include_str!("../../lexicon/css.txt")),
    ("go", include_str!("../../lexicon/go.txt")),
    ("html", include_str!("../../lexicon/html.txt")),
    ("java", include_str!("../../lexicon/java.txt")),
    ("javascript", include_str!("../../lexicon/javascript.txt")),
    ("php", include_str!("../../lexicon/php.txt")),
    ("python", include_str!("../../lexicon/python.txt")),
    ("ruby", include_str!("../../lexicon/ruby.txt")),
    ("rust", include_str!("../../lexicon/rust.txt")),
    ("shell", include_str!("../../lexicon/shell.txt")),
    ("sql", include_str!("../../lexicon/sql.txt")),
];

fn parse(src: &'static str) -> Lexicon {
    let mut lex = Lexicon {
        words: Vec::new(),
        case_insensitive: false,
        extra: "",
    };
    for l in src.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(c) = l.strip_prefix('#') {
            let c = c.trim();
            if c == "case-insensitive" {
                lex.case_insensitive = true;
            } else if let Some(x) = c.strip_prefix("word-chars:") {
                lex.extra = x.trim();
            }
        } else {
            lex.words.push(l);
        }
    }
    lex
}

/// Which lexicon a language of the highlighting (design 008) uses.
fn file_of(language: &str) -> Option<&'static str> {
    Some(match language {
        "Java" => "java",
        "JavaScript" | "TypeScript" | "TypeScriptReact" | "JavaScript (Rails)" => "javascript",
        "Python" => "python",
        "SQL" | "SQL (Rails)" => "sql",
        "Rust" => "rust",
        "C" => "c",
        "C++" => "cpp",
        "C#" => "csharp",
        "Go" => "go",
        "PHP" | "PHP Source" => "php",
        "Ruby" | "Ruby on Rails" => "ruby",
        "Bourne Again Shell (bash)" | "Shell-Unix-Generic" => "shell",
        "CSS" => "css",
        l if l == "HTML" || l.starts_with("HTML (") => "html",
        _ => return None,
    })
}

/// The lexicon of a language, `None` = none for it.
pub fn lexicon(language: &str) -> Option<&'static Lexicon> {
    static ALL: OnceLock<HashMap<&'static str, Lexicon>> = OnceLock::new();
    let all = ALL.get_or_init(|| FILES.iter().map(|(k, src)| (*k, parse(src))).collect());
    all.get(file_of(language)?)
}

/// Belongs to a word (letters, digits, `_`, combining marks and the language's `extra`).
pub fn is_word(c: char, extra: &str) -> bool {
    CharClass::of(c) == CharClass::Word || extra.contains(c)
}

/// Where the word prefix left of `caret` starts; `None` = no prefix (nothing, a number, too long).
pub fn prefix_start(line: &[char], caret: usize, extra: &str) -> Option<usize> {
    let caret = caret.min(line.len());
    let mut start = caret;
    while start > 0 && is_word(line[start - 1], extra) {
        start -= 1;
        if caret - start > MAX_WORD {
            return None;
        }
    }
    // a word never starts with `-` (CSS), nor with a digit
    while start < caret && extra.contains(line[start]) {
        start += 1;
    }
    (start < caret && !line[start].is_ascii_digit()).then_some(start)
}

/// The spelling of a case-insensitive word as the prefix writes it.
fn adapt(word: &str, prefix: &str) -> String {
    let letters = || prefix.chars().filter(|c| c.is_alphabetic());
    if letters().next().is_none() {
        word.to_owned()
    } else if letters().all(char::is_lowercase) {
        word.to_lowercase()
    } else if letters().all(char::is_uppercase) {
        word.to_uppercase()
    } else {
        word.to_owned()
    }
}

/// One suggested word.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub word: String,
    /// From the lexicon, not from the document.
    pub keyword: bool,
}

/// The candidates for `prefix`: words of `lines` (nearest first; each with the character range
/// to skip – the word the caret is in), then the lexicon (design 023 E5).
pub fn candidates<'a>(
    prefix: &str,
    lines: impl Iterator<Item = (&'a str, Option<(usize, usize)>)>,
    lex: Option<&Lexicon>,
    extra: &str,
) -> Vec<Candidate> {
    let low = prefix.to_lowercase();
    let mut seen: HashSet<String> = HashSet::new();
    let (mut exact, mut loose) = (Vec::new(), Vec::new());
    'lines: for (text, skip) in lines {
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if !is_word(chars[i], extra) {
                i += 1;
                continue;
            }
            let from = i;
            while i < chars.len() && is_word(chars[i], extra) {
                i += 1;
            }
            if skip.is_some_and(|(a, b)| from < b && i > a) {
                continue;
            }
            // `-` (CSS) neither starts nor ends a word
            let (mut a, mut b) = (from, i);
            while a < b && extra.contains(chars[a]) {
                a += 1;
            }
            while b > a && extra.contains(chars[b - 1]) {
                b -= 1;
            }
            let n = b - a;
            if !(2..=MAX_WORD).contains(&n) || chars[a].is_ascii_digit() {
                continue;
            }
            let w: String = chars[a..b].iter().collect();
            if w == prefix || seen.contains(&w) {
                continue;
            }
            if w.starts_with(prefix) {
                seen.insert(w.clone());
                exact.push(w);
                if exact.len() >= MAX_ITEMS {
                    break 'lines;
                }
            } else if w.to_lowercase().starts_with(&low) {
                seen.insert(w.clone());
                loose.push(w);
            }
        }
    }
    let (mut lex_exact, mut lex_loose) = (Vec::new(), Vec::new());
    if let Some(lex) = lex {
        let mut words: Vec<&str> = lex
            .words
            .iter()
            .copied()
            .filter(|w| w.to_lowercase().starts_with(&low))
            .collect();
        words.sort_by_key(|w| (w.chars().count(), *w));
        for w in words {
            if lex.case_insensitive {
                let a = adapt(w, prefix);
                let dup = seen.iter().any(|s| s.eq_ignore_ascii_case(&a));
                if a != prefix && !dup {
                    lex_exact.push(a);
                }
            } else if w != prefix && !seen.contains(w) {
                if w.starts_with(prefix) {
                    lex_exact.push(w.to_owned());
                } else {
                    lex_loose.push(w.to_owned());
                }
            }
        }
    }
    let doc = |w: String| Candidate {
        word: w,
        keyword: false,
    };
    let key = |w: String| Candidate {
        word: w,
        keyword: true,
    };
    exact
        .into_iter()
        .map(doc)
        .chain(lex_exact.into_iter().map(key))
        .chain(loose.into_iter().map(doc))
        .chain(lex_loose.into_iter().map(key))
        .take(MAX_ITEMS)
        .collect()
}

/// Line numbers by distance to `line` (the line, −1, +1, −2 …), at most [`MAX_LINES`].
pub fn nearest_lines(line: usize, count: usize) -> impl Iterator<Item = usize> {
    (0..MAX_LINES)
        .flat_map(move |d| {
            let up = (d > 0).then(|| line.checked_sub(d)).flatten();
            let down = (line + d < count).then_some(line + d);
            [up, down]
        })
        .flatten()
        .take(MAX_LINES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(prefix: &str, lines: &[&str], lex: Option<&Lexicon>) -> Vec<String> {
        candidates(prefix, lines.iter().map(|l| (*l, None)), lex, "")
            .into_iter()
            .map(|c| c.word)
            .collect()
    }

    fn pre(marked: &str, extra: &str) -> Option<String> {
        let caret = marked.chars().position(|c| c == '|').unwrap();
        let line: Vec<char> = marked.chars().filter(|&c| c != '|').collect();
        prefix_start(&line, caret, extra).map(|s| line[s..caret].iter().collect())
    }

    #[test]
    fn prefix_at_the_caret() {
        assert_eq!(pre("let foo|", ""), Some("foo".into()));
        assert_eq!(pre("  x.ba|r", ""), Some("ba".into()));
        assert_eq!(pre("größ|", ""), Some("größ".into()));
        assert_eq!(pre("e\u{301}t|", ""), Some("e\u{301}t".into()));
        assert_eq!(pre("a |", ""), None);
        assert_eq!(pre("12|", ""), None);
        assert_eq!(pre("x = 3ab|", ""), None);
        assert_eq!(pre("background-co|", "-"), Some("background-co".into()));
        assert_eq!(pre("--main|", "-"), Some("main".into()));
        assert_eq!(pre("background-co|", ""), Some("co".into()));
        let long = format!("{}|", "a".repeat(MAX_WORD + 1));
        assert_eq!(pre(&long, ""), None);
    }

    #[test]
    fn nearest_first_exact_case_first() {
        let lines = ["printable", "Printer x", "print_all pr"];
        assert_eq!(
            words("pri", &lines, None),
            ["printable", "print_all", "Printer"]
        );
        // the prefix itself, short words and numbers are no suggestions
        assert!(words("pr", &["pr p 12pr"], None).is_empty());
        // unicode words
        assert_eq!(words("Grö", &["Größe größer"], None), ["Größe", "größer"]);
    }

    #[test]
    fn the_word_at_the_caret_is_skipped() {
        let c = candidates(
            "pub",
            [("pub publish", Some((0, 3))), ("pubs", None)].into_iter(),
            None,
            "",
        );
        let w: Vec<_> = c.into_iter().map(|c| c.word).collect();
        assert_eq!(w, ["publish", "pubs"]);
        // the half typed word "publ|" in its line does not suggest itself
        let c = candidates("publ", [("x publ", Some((2, 6)))].into_iter(), None, "");
        assert!(c.is_empty());
    }

    #[test]
    fn lexicon_after_the_document() {
        let java = lexicon("Java").unwrap();
        let c = candidates("pub", [("publishNow()", None)].into_iter(), Some(java), "");
        assert_eq!(
            c,
            [
                Candidate {
                    word: "publishNow".into(),
                    keyword: false
                },
                Candidate {
                    word: "public".into(),
                    keyword: true
                }
            ]
        );
        // a word in both counts once, as a word of the document
        let c = candidates("Str", [("String s;", None)].into_iter(), Some(java), "");
        assert_eq!(c[0].word, "String");
        assert!(!c[0].keyword);
        assert_eq!(c.iter().filter(|c| c.word == "String").count(), 1);
        assert!(c.iter().any(|c| c.word == "StringBuilder" && c.keyword));
        // case-sensitive language: other case comes after
        let c = words("str", &[], Some(java));
        assert!(c.contains(&"strictfp".to_string()));
        assert!(c.iter().position(|w| w == "strictfp") < c.iter().position(|w| w == "String"));
    }

    #[test]
    fn sql_spelling_follows_the_prefix() {
        let sql = lexicon("SQL").unwrap();
        assert!(sql.case_insensitive);
        assert_eq!(words("sel", &[], Some(sql)), ["select"]);
        assert_eq!(words("SEL", &[], Some(sql)), ["SELECT"]);
        // the file's own spelling wins over the prefix (it writes keywords in capitals)
        assert_eq!(words("sel", &["SELECT id"], Some(sql)), ["SELECT"]);
        // exact document words, the lexicon, then other case
        assert_eq!(
            words("gr", &["GRANTED"], Some(sql)),
            ["grant", "group", "GRANTED"]
        );
        assert!(lexicon("SQL (Rails)").is_some());
    }

    #[test]
    fn languages_have_clean_lists() {
        for lang in [
            "Java",
            "JavaScript",
            "Python",
            "SQL",
            "Rust",
            "C",
            "C++",
            "C#",
            "Go",
            "PHP",
            "Ruby",
            "Bourne Again Shell (bash)",
            "CSS",
            "HTML",
            "HTML (Rails)",
        ] {
            let lex = lexicon(lang).unwrap_or_else(|| panic!("{lang}"));
            assert!(lex.words.len() >= 40, "{lang}");
            let mut seen = HashSet::new();
            for w in &lex.words {
                assert!(seen.insert(*w), "{lang}: {w} twice");
                assert!(
                    w.chars().all(|c| is_word(c, lex.extra)) && !w.starts_with('-'),
                    "{lang}: {w}"
                );
            }
        }
        assert!(lexicon("Markdown").is_none());
        assert_eq!(lexicon("CSS").unwrap().extra, "-");
        let css = lexicon("CSS").unwrap();
        let c = candidates("background-c", std::iter::empty(), Some(css), "-");
        assert_eq!(c[0].word, "background-color");
    }

    #[test]
    fn bounded_lines_and_order() {
        let order: Vec<usize> = nearest_lines(2, 5).collect();
        assert_eq!(order, [2, 1, 3, 0, 4]);
        assert_eq!(nearest_lines(0, 1).collect::<Vec<_>>(), [0]);
        assert_eq!(nearest_lines(10_000_000, 20_000_000).count(), MAX_LINES);
    }

    #[test]
    fn at_most_twelve() {
        let line: Vec<String> = (0..40).map(|i| format!("item{i}")).collect();
        let line = line.join(" ");
        assert_eq!(words("it", &[&line], None).len(), MAX_ITEMS);
    }
}
