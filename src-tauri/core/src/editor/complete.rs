//! Word completion (prompt 023): the word prefix at the caret, candidates from the lines around
//! it (nearest first, bounded – never proportional to the file) and from a small lexicon of
//! typical words of the language (`core/lexicon/*.txt`).

use super::words::CharClass;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

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

/// The typical words of one language: the built-in lists plus the user's own.
#[derive(Debug, Default)]
pub struct Lexicon {
    pub words: Vec<String>,
    /// SQL & co.: the spelling follows the prefix.
    pub case_insensitive: bool,
    /// Characters that belong to a word besides letters, digits and `_` (CSS: `-`).
    pub extra: String,
}

const FILES: &[(&str, &str)] = &[
    ("angular", include_str!("../../lexicon/angular.txt")),
    ("c", include_str!("../../lexicon/c.txt")),
    ("cpp", include_str!("../../lexicon/cpp.txt")),
    ("csharp", include_str!("../../lexicon/csharp.txt")),
    ("css", include_str!("../../lexicon/css.txt")),
    ("go", include_str!("../../lexicon/go.txt")),
    ("html", include_str!("../../lexicon/html.txt")),
    ("java", include_str!("../../lexicon/java.txt")),
    ("javascript", include_str!("../../lexicon/javascript.txt")),
    ("kotlin", include_str!("../../lexicon/kotlin.txt")),
    ("php", include_str!("../../lexicon/php.txt")),
    ("python", include_str!("../../lexicon/python.txt")),
    ("ruby", include_str!("../../lexicon/ruby.txt")),
    ("rust", include_str!("../../lexicon/rust.txt")),
    ("shell", include_str!("../../lexicon/shell.txt")),
    ("spring", include_str!("../../lexicon/spring.txt")),
    ("sql", include_str!("../../lexicon/sql.txt")),
];

/// The built-in lists a lexicon is made of (Spring Boot for Java and Kotlin, Angular for
/// TypeScript).
fn parts(key: &str) -> &'static [&'static str] {
    match key {
        "java" => &["java", "spring"],
        "kotlin" => &["kotlin", "spring"],
        "typescript" => &["javascript", "angular"],
        "javascript" => &["javascript"],
        "python" => &["python"],
        "sql" => &["sql"],
        "rust" => &["rust"],
        "c" => &["c"],
        "cpp" => &["cpp"],
        "csharp" => &["csharp"],
        "go" => &["go"],
        "php" => &["php"],
        "ruby" => &["ruby"],
        "shell" => &["shell"],
        "css" => &["css"],
        "html" => &["html"],
        _ => &[],
    }
}

/// Most words taken from one user list, longest user word.
const USER_WORDS: usize = 5_000;

/// Adds the words and settings of one list (`#`-lines: `case-insensitive`, `word-chars: …`).
fn add(lex: &mut Lexicon, src: &str, seen: &mut HashSet<String>, limit: usize) {
    let mut n = 0;
    for l in src.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(c) = l.strip_prefix('#') {
            let c = c.trim();
            if c == "case-insensitive" {
                lex.case_insensitive = true;
            } else if let Some(x) = c.strip_prefix("word-chars:") {
                for ch in x.trim().chars().filter(|c| !c.is_whitespace()).take(8) {
                    if !lex.extra.contains(ch) {
                        lex.extra.push(ch);
                    }
                }
            }
        } else if n < limit
            && l.chars().count() <= MAX_WORD
            && !l.contains(char::is_whitespace)
            && seen.insert(l.to_owned())
        {
            lex.words.push(l.to_owned());
            n += 1;
        }
    }
}

/// The user's own lists (`<config>/words/<key>.txt`, key → text), `all` for every language.
static USER: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);
static CACHE: RwLock<Option<HashMap<String, Arc<Lexicon>>>> = RwLock::new(None);

/// Puts in the user's own word lists (design 023 E13); `key` as [`key_for`] names it.
pub fn set_user_words(lists: HashMap<String, String>) {
    let mut user = USER.write().unwrap_or_else(|e| e.into_inner());
    if user.as_ref() == Some(&lists) {
        return;
    }
    *user = Some(lists);
    *CACHE.write().unwrap_or_else(|e| e.into_inner()) = None;
}

/// The lexicon of `key` (built-in lists + the user's), `None` = no words at all.
pub fn lexicon(key: &str) -> Option<Arc<Lexicon>> {
    if let Some(l) = CACHE
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|c| c.get(key))
    {
        return Some(l.clone());
    }
    let mut lex = Lexicon::default();
    let mut seen = HashSet::new();
    for part in parts(key) {
        if let Some((_, src)) = FILES.iter().find(|(k, _)| k == part) {
            add(&mut lex, src, &mut seen, usize::MAX);
        }
    }
    if let Some(user) = USER.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        for k in [key, "all"] {
            if let Some(src) = user.get(k) {
                add(&mut lex, src, &mut seen, USER_WORDS);
            }
        }
    }
    if lex.words.is_empty() {
        return None;
    }
    let lex = Arc::new(lex);
    CACHE
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(key.to_owned(), lex.clone());
    Some(lex)
}

/// Which lexicon a file uses: Kotlin and TypeScript by the extension (syntect knows neither),
/// else by the language of the highlighting (design 008); other languages by their name in
/// lower case without spaces (`Markdown` → `markdown`), plain text = `text`.
pub fn key_for(file_name: Option<&str>, language: Option<&str>) -> String {
    let ext = file_name
        .and_then(|n| n.rsplit_once('.'))
        .map(|(_, e)| e.to_lowercase());
    match (ext.as_deref(), language) {
        (Some("kt" | "kts"), None | Some("Groovy")) => return "kotlin".into(),
        (Some("ts" | "tsx" | "mts" | "cts"), None | Some("JavaScript")) => {
            return "typescript".into();
        }
        _ => {}
    }
    let Some(language) = language else {
        return "text".into();
    };
    match language {
        "Java" => "java",
        "JavaScript" | "JavaScript (Rails)" => "javascript",
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
        other => {
            return other
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '+' || *c == '#')
                .flat_map(char::to_lowercase)
                .collect();
        }
    }
    .into()
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
            .map(String::as_str)
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

    fn lex(language: &str) -> Arc<Lexicon> {
        lexicon(&key_for(None, Some(language))).unwrap_or_else(|| panic!("{language}"))
    }

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
        let java = lex("Java");
        let c = candidates("pub", [("publishNow()", None)].into_iter(), Some(&java), "");
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
        let c = candidates("Str", [("String s;", None)].into_iter(), Some(&java), "");
        assert_eq!(c[0].word, "String");
        assert!(!c[0].keyword);
        assert_eq!(c.iter().filter(|c| c.word == "String").count(), 1);
        assert!(c.iter().any(|c| c.word == "StringBuilder" && c.keyword));
        // case-sensitive language: other case comes after
        let c = words("str", &[], Some(&java));
        assert!(c.contains(&"strictfp".to_string()));
        assert!(c.iter().position(|w| w == "strictfp") < c.iter().position(|w| w == "String"));
    }

    #[test]
    fn sql_spelling_follows_the_prefix() {
        let sql = lex("SQL");
        assert!(sql.case_insensitive);
        assert_eq!(words("sel", &[], Some(&sql)), ["select"]);
        assert_eq!(words("SEL", &[], Some(&sql)), ["SELECT"]);
        // the file's own spelling wins over the prefix (it writes keywords in capitals)
        assert_eq!(words("sel", &["SELECT id"], Some(&sql)), ["SELECT"]);
        // exact document words, the lexicon, then other case
        assert_eq!(
            words("gr", &["GRANTED"], Some(&sql)),
            ["grant", "group", "GRANTED"]
        );
        assert_eq!(key_for(None, Some("SQL (Rails)")), "sql");
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
            let lex = lex(lang);
            check_list(lang, &lex);
        }
        for key in ["kotlin", "typescript"] {
            check_list(key, &lexicon(key).unwrap());
        }
        assert!(lexicon(&key_for(None, Some("Markdown"))).is_none());
        assert_eq!(lex("CSS").extra, "-");
        let css = lex("CSS");
        let c = candidates("background-c", std::iter::empty(), Some(&css), "-");
        assert_eq!(c[0].word, "background-color");
    }

    fn check_list(lang: &str, lex: &Lexicon) {
        {
            assert!(lex.words.len() >= 40, "{lang}");
            let mut seen = HashSet::new();
            for w in &lex.words {
                assert!(seen.insert(w.clone()), "{lang}: {w} twice");
                assert!(
                    w.chars().all(|c| is_word(c, &lex.extra)) && !w.starts_with('-'),
                    "{lang}: {w}"
                );
            }
        }
    }

    #[test]
    fn kotlin_spring_and_angular() {
        assert_eq!(key_for(Some("App.kt"), None), "kotlin");
        assert_eq!(key_for(Some("build.gradle.kts"), Some("Groovy")), "kotlin");
        assert_eq!(key_for(Some("app.ts"), Some("JavaScript")), "typescript");
        assert_eq!(key_for(Some("app.js"), Some("JavaScript")), "javascript");
        // a language picked by hand wins over the extension
        assert_eq!(key_for(Some("query.ts"), Some("SQL")), "sql");
        assert_eq!(key_for(Some("notes.txt"), None), "text");
        assert_eq!(
            key_for(None, Some("Objective-C++")),
            "objective-c++".replace('-', "")
        );
        let kotlin = lexicon("kotlin").unwrap();
        assert!(words("fu", &[], Some(&kotlin)).contains(&"fun".to_string()));
        // Spring Boot annotations after "@" in Java and Kotlin
        let java = lex("Java");
        assert_eq!(words("GetM", &[], Some(&java)), ["GetMapping"]);
        assert_eq!(
            words("RestCon", &[], Some(&kotlin)),
            ["RestController", "RestControllerAdvice"]
        );
        // Angular in TypeScript only
        let ts = lexicon("typescript").unwrap();
        assert!(words("ngOn", &[], Some(&ts)).contains(&"ngOnInit".to_string()));
        assert!(words("ngOn", &[], Some(&lex("JavaScript"))).is_empty());
    }

    #[test]
    fn the_users_own_lists() {
        let lists = HashMap::from([
            (
                "zz-test".to_string(),
                "# word-chars: -\nmy-own-word\nwith space\nzzTop\n".to_string(),
            ),
            ("all".to_string(), "everywhereWord\n".to_string()),
        ]);
        set_user_words(lists);
        let l = lexicon("zz-test").unwrap();
        assert_eq!(l.extra, "-");
        assert!(l.words.contains(&"my-own-word".to_string()));
        assert!(!l.words.iter().any(|w| w.contains(' ')));
        assert!(l.words.contains(&"everywhereWord".to_string()));
        // built-in lists keep theirs and get "all"
        let java = lexicon("java").unwrap();
        assert!(java.words.contains(&"public".to_string()));
        assert!(java.words.contains(&"everywhereWord".to_string()));
        set_user_words(HashMap::new());
        assert!(lexicon("zz-test").is_none());
        assert!(
            !lexicon("java")
                .unwrap()
                .words
                .contains(&"everywhereWord".to_string())
        );
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
