//! Toggle line comment (prompt 031): the marks of a language and the edit on the lines of the
//! cursors. The behaviour follows `toggleComment` of schema-commander
//! (`src/app/features/format/format-logic.ts`, same author, MIT), with its tests as the model.

use super::Session;
use crate::document::model::{Change, Pos, Sel};
use crate::error::FsError;

/// At most this many marks from outside, each at most this long.
const MAX_MARKS: usize = 8;
const MAX_MARK_CHARS: usize = 16;

/// What starts a line comment in a language (grammar name as in [`crate::highlight`]): the
/// first mark is the one set, all are recognised. Empty = the language has none (CSS, HTML,
/// JSON …). A table of our own: the grammars' `Comments.tmPreferences` are not part of the
/// packed set the engine loads (design 031 E5).
pub fn line_comments(language: &str) -> &'static [&'static str] {
    match language {
        "ActionScript" | "C" | "C#" | "C++" | "D" | "Go" | "Graphviz (DOT)" | "Groovy" | "Java"
        | "JavaScript" | "JavaScript (Rails)" | "Objective-C" | "Objective-C++" | "Pascal"
        | "Rust" | "Scala" => &["//"],
        "PHP" | "PHP Source" => &["//", "#"],
        "Bourne Again Shell (bash)"
        | "Shell-Unix-Generic"
        | "commands-builtin-shell-bash"
        | "Makefile"
        | "Perl"
        | "Python"
        | "R"
        | "Ruby"
        | "Ruby on Rails"
        | "Tcl"
        | "YAML" => &["#"],
        "INI / Properties / TOML" => &["#", ";"],
        "Java Properties" => &["#", "!"],
        "SQL" | "SQL (Rails)" | "Haskell" | "Lua" | "AppleScript" => &["--"],
        "LaTeX" | "TeX" | "BibTeX" | "Erlang" | "MATLAB" | "Rd (R Documentation)" => &["%"],
        "Clojure" | "Lisp" => &[";"],
        "ASP" => &["'"],
        "Batch File" => &["::", "REM"],
        _ => &[],
    }
}

/// A line that takes part: where its text starts and, when it is a comment, how many
/// characters the mark takes (with the one blank behind it).
struct Line {
    line: usize,
    indent: usize,
    cut: Option<usize>,
}

impl Session {
    /// The marks from outside (`None` or none that are usable: those of the language again).
    pub(super) fn set_line_comment(&mut self, marks: Option<Vec<String>>) {
        self.comment_marks = marks
            .unwrap_or_default()
            .into_iter()
            .filter(|m| {
                !m.is_empty()
                    && m.chars().count() <= MAX_MARK_CHARS
                    && !m.chars().any(|c| c.is_whitespace() || c.is_control())
            })
            .take(MAX_MARKS)
            .collect();
    }

    /// The marks in use: from outside, else of the language.
    pub fn line_comment(&self) -> Vec<String> {
        if !self.comment_marks.is_empty() {
            return self.comment_marks.clone();
        }
        self.language()
            .map(|l| line_comments(l).iter().map(|&m| m.to_owned()).collect())
            .unwrap_or_default()
    }

    /// Characters of blanks and tabs a line starts with, and whether anything follows them.
    /// Reads in pieces: a line of millions of blanks never becomes one string.
    fn lead(&self, line: usize) -> (usize, bool) {
        const PIECE: usize = 1024;
        let len = self.doc.line_len(line);
        let mut at = 0;
        while at < len {
            for c in self.doc.line_slice(line, at, at + PIECE).chars() {
                if c != ' ' && c != '\t' {
                    return (at, true);
                }
                at += 1;
            }
        }
        (len, false)
    }

    pub(super) fn toggle_comment(&mut self) -> Result<(), FsError> {
        let marks = self.line_comment();
        if marks.is_empty() {
            self.notice = Some("no_line_comment".into());
            return Ok(());
        }
        let longest = marks.iter().map(|m| m.chars().count()).max().unwrap_or(0);
        let mut numbers: Vec<usize> = self
            .cursors
            .iter()
            .flat_map(|c| {
                let (a, b) = Self::lines_of(c);
                a..=b
            })
            .collect();
        numbers.sort_unstable();
        numbers.dedup();
        // lines with text only: empty ones stay empty
        let lines: Vec<Line> = numbers
            .into_iter()
            .filter_map(|line| {
                let (indent, filled) = self.lead(line);
                if !filled {
                    return None;
                }
                let head = self.doc.line_slice(line, indent, indent + longest + 1);
                let cut = marks
                    .iter()
                    .filter(|m| head.starts_with(m.as_str()))
                    .map(|m| m.chars().count())
                    .max()
                    // the one blank that was put behind the mark goes with it
                    .map(|n| n + usize::from(head.chars().nth(n) == Some(' ')));
                Some(Line { line, indent, cut })
            })
            .collect();
        if lines.is_empty() {
            return Ok(());
        }
        let at = |line: usize| {
            lines
                .binary_search_by_key(&line, |l| l.line)
                .ok()
                .map(|i| &lines[i])
        };
        let (changes, after): (Vec<Change>, Vec<Sel>) = if lines.iter().all(|l| l.cut.is_some()) {
            let changes = lines
                .iter()
                .map(|l| Change {
                    from: Pos::new(l.line, l.indent),
                    to: Pos::new(l.line, l.indent + l.cut.unwrap_or(0)),
                    text: String::new(),
                })
                .collect();
            let shift = |p: Pos| match at(p.line) {
                Some(l) => {
                    let cut = l.cut.unwrap_or(0);
                    let ch = if p.ch >= l.indent + cut {
                        p.ch - cut
                    } else {
                        p.ch.min(l.indent)
                    };
                    Pos::new(p.line, ch)
                }
                None => p,
            };
            let after = self
                .cursors
                .iter()
                .map(|c| Sel {
                    anchor: shift(c.anchor),
                    head: shift(c.head),
                })
                .collect();
            (changes, after)
        } else {
            // one column for the mark: where the least indented line starts
            let indent = lines.iter().map(|l| l.indent).min().unwrap_or(0);
            let mark = format!("{} ", marks[0]);
            let n = mark.chars().count();
            let changes = lines
                .iter()
                .map(|l| Change {
                    from: Pos::new(l.line, indent),
                    to: Pos::new(l.line, indent),
                    text: mark.clone(),
                })
                .collect();
            let after = self
                .cursors
                .iter()
                .map(|c| {
                    // the start of a selection right at the mark keeps the mark selected
                    let start = (!c.is_empty()).then(|| c.start());
                    let shift = |p: Pos| {
                        if at(p.line).is_some()
                            && p.ch >= indent
                            && !(p.ch == indent && Some(p) == start)
                        {
                            Pos::new(p.line, p.ch + n)
                        } else {
                            p
                        }
                    };
                    Sel {
                        anchor: shift(c.anchor),
                        head: shift(c.head),
                    }
                })
                .collect();
            (changes, after)
        };
        self.apply_lines(changes, after)
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::Command;
    use super::super::{Cursor, MemoryClipboard, Session};
    use super::*;
    use crate::document::Document;

    fn session(text: &str, language: &str) -> Session {
        let mut s = Session::new(Document::from_text(text), 1);
        run(
            &mut s,
            Command::SetLanguage {
                name: Some(language.into()),
            },
        );
        assert_eq!(s.language(), Some(language));
        s
    }

    fn run(s: &mut Session, cmd: Command) -> super::super::model::Frame {
        s.run(cmd, &mut MemoryClipboard::default()).unwrap()
    }

    fn text(s: &Session) -> String {
        s.doc.rope().to_string()
    }

    fn select(s: &mut Session, from: (usize, usize), to: (usize, usize)) {
        s.set_cursors(vec![Cursor {
            anchor: Pos::new(from.0, from.1),
            head: Pos::new(to.0, to.1),
            goal: None,
        }]);
    }

    fn all(s: &mut Session) {
        run(s, Command::SelectAll);
    }

    fn toggle(s: &mut Session) {
        run(s, Command::ToggleComment);
    }

    /// Before → after for the whole text; a second toggle gives the text back.
    fn case(language: &str, before: &str, after: &str) {
        let mut s = session(before, language);
        all(&mut s);
        toggle(&mut s);
        assert_eq!(text(&s), after, "{language}: {before:?}");
        toggle(&mut s);
        assert_eq!(text(&s), before, "{language}: back from {after:?}");
    }

    #[test]
    fn the_catalog_by_language() {
        case("SQL", "select 1;", "-- select 1;");
        case("SQL", "select 1\nfrom t", "-- select 1\n-- from t");
        case("Rust", "fn main() {}", "// fn main() {}");
        case("JavaScript", "let a = 1;", "// let a = 1;");
        case("Python", "x = 1\ny = 2", "# x = 1\n# y = 2");
        case("Bourne Again Shell (bash)", "echo hi", "# echo hi");
        case("YAML", "a:\n  b: 1", "# a:\n#   b: 1");
        case("Lua", "x = 1", "-- x = 1");
        case("LaTeX", "\\section{a}", "% \\section{a}");
        case("Lisp", "(a)", "; (a)");
    }

    #[test]
    fn typescript_counts_as_javascript() {
        let mut s = Session::new(Document::from_text("const a = 1;"), 1);
        s.set_file_name(Some("app.ts"));
        toggle(&mut s);
        assert_eq!(text(&s), "// const a = 1;");
    }

    #[test]
    fn an_indented_block_keeps_its_shape() {
        case(
            "SQL",
            "  select a,\n      b\n  from t",
            "  -- select a,\n  --     b\n  -- from t",
        );
        case("Python", "\tif a:\n\t\tb()", "\t# if a:\n\t# \tb()");
    }

    #[test]
    fn empty_lines_stay_empty_and_line_breaks_what_they_are() {
        case(
            "SQL",
            "select 1;\r\n\r\n   \r\nselect 2;\r\n",
            "-- select 1;\r\n\r\n   \r\n-- select 2;\r\n",
        );
        case("Rust", "a\n\nb\rc", "// a\n\n// b\r// c");
        case("SQL", "\n  \n", "\n  \n");
    }

    #[test]
    fn emoji_wide_and_combining_characters() {
        case(
            "SQL",
            "  😀 x\n  日本 e\u{301}",
            "  -- 😀 x\n  -- 日本 e\u{301}",
        );
        // text before the column of the mark is part of a less indented line only
        case("Python", "😀\n  y", "# 😀\n#   y");
    }

    #[test]
    fn a_mixed_block_gets_marks_on_every_line() {
        let mut s = session("-- a\nb\n  -- c", "SQL");
        all(&mut s);
        toggle(&mut s);
        assert_eq!(text(&s), "-- -- a\n-- b\n--   -- c");
        toggle(&mut s);
        assert_eq!(text(&s), "-- a\nb\n  -- c");
    }

    #[test]
    fn removing_takes_the_mark_and_one_blank_wherever_the_line_starts() {
        let mut s = session("--a\n  --  b\n\n\t-- c", "SQL");
        all(&mut s);
        toggle(&mut s);
        assert_eq!(text(&s), "a\n   b\n\n\tc");
    }

    #[test]
    fn the_line_of_the_caret_and_a_selection_ending_at_a_line_start() {
        let mut s = session("a\nb\nc", "SQL");
        s.set_cursors(vec![Cursor::caret(Pos::new(1, 1))]);
        toggle(&mut s);
        assert_eq!(text(&s), "a\n-- b\nc");
        assert_eq!(s.cursors()[0].head, Pos::new(1, 4), "behind the same b");
        // lines 0 and 1, not 2
        select(&mut s, (0, 0), (2, 0));
        toggle(&mut s);
        assert_eq!(text(&s), "-- a\n-- -- b\nc");
        let c = s.cursors()[0];
        assert_eq!((c.start(), c.end()), (Pos::new(0, 0), Pos::new(2, 0)));
        toggle(&mut s);
        assert_eq!(text(&s), "a\n-- b\nc");
        let c = s.cursors()[0];
        assert_eq!((c.start(), c.end()), (Pos::new(0, 0), Pos::new(2, 0)));
    }

    #[test]
    fn selection_and_caret_stay_on_the_same_text() {
        let mut s = session("  select a\n  from t", "SQL");
        // "lect a\n  fr"
        select(&mut s, (0, 4), (1, 4));
        toggle(&mut s);
        assert_eq!(text(&s), "  -- select a\n  -- from t");
        let c = s.cursors()[0];
        assert_eq!((c.anchor, c.head), (Pos::new(0, 7), Pos::new(1, 7)));
        toggle(&mut s);
        let c = s.cursors()[0];
        assert_eq!((c.anchor, c.head), (Pos::new(0, 4), Pos::new(1, 4)));
        // a caret inside the mark lands where the text starts
        toggle(&mut s);
        s.set_cursors(vec![Cursor::caret(Pos::new(0, 3))]);
        toggle(&mut s);
        assert_eq!(text(&s), "  select a\n  -- from t");
        assert_eq!(s.cursors()[0].head, Pos::new(0, 2));
    }

    #[test]
    fn several_cursors_touch_every_line_once() {
        let mut s = session("a\nb\nc\nd", "Rust");
        s.set_cursors(vec![
            Cursor::caret(Pos::new(0, 0)),
            Cursor::caret(Pos::new(0, 1)),
            Cursor {
                anchor: Pos::new(2, 0),
                head: Pos::new(3, 1),
                goal: None,
            },
            Cursor::caret(Pos::new(3, 1)),
        ]);
        toggle(&mut s);
        assert_eq!(text(&s), "// a\nb\n// c\n// d");
        run(&mut s, Command::Undo);
        assert_eq!(text(&s), "a\nb\nc\nd", "one undo step");
        run(&mut s, Command::Redo);
        assert_eq!(text(&s), "// a\nb\n// c\n// d");
    }

    #[test]
    fn marks_from_outside_win_and_all_of_them_are_recognised() {
        let mut s = session("# a\n-- b", "SQL");
        all(&mut s);
        toggle(&mut s);
        assert_eq!(text(&s), "-- # a\n-- -- b", "plain SQL: # is no comment");
        run(&mut s, Command::Undo);
        run(
            &mut s,
            Command::SetLineComment {
                marks: Some(vec!["--".into(), "#".into()]),
            },
        );
        all(&mut s);
        toggle(&mut s);
        assert_eq!(text(&s), "a\nb");
        toggle(&mut s);
        assert_eq!(text(&s), "-- a\n-- b", "the first mark is the one set");
        // back to the language's
        run(&mut s, Command::SetLineComment { marks: None });
        assert_eq!(s.line_comment(), vec!["--"]);
        // unusable marks are dropped
        run(
            &mut s,
            Command::SetLineComment {
                marks: Some(vec![String::new(), "a b".into(), "\n".into(), "//".into()]),
            },
        );
        assert_eq!(s.line_comment(), vec!["//"]);
    }

    #[test]
    fn plain_text_takes_marks_from_outside() {
        let mut s = Session::new(Document::from_text("a"), 1);
        run(
            &mut s,
            Command::SetLineComment {
                marks: Some(vec!["//".into()]),
            },
        );
        toggle(&mut s);
        assert_eq!(text(&s), "// a");
    }

    #[test]
    fn languages_without_a_line_comment_say_so_and_change_nothing() {
        for language in ["CSS", "HTML", "JSON"] {
            let mut s = session("a { }", language);
            let f = run(&mut s, Command::ToggleComment);
            assert_eq!(f.notice.as_deref(), Some("no_line_comment"), "{language}");
            assert_eq!(text(&s), "a { }");
            assert!(!s.doc.dirty());
        }
        let mut s = Session::new(Document::from_text("a"), 1);
        let f = run(&mut s, Command::ToggleComment);
        assert_eq!(f.notice.as_deref(), Some("no_line_comment"), "plain text");
    }

    #[test]
    fn read_only_refuses() {
        let mut s = session("a", "SQL");
        run(&mut s, Command::SetReadOnly { read_only: true });
        let e = s
            .run(Command::ToggleComment, &mut MemoryClipboard::default())
            .unwrap_err();
        assert_eq!(e.code, crate::error::FsErrorCode::ReadOnly);
        assert_eq!(text(&s), "a");
    }

    #[test]
    fn every_language_of_the_table_is_a_grammar() {
        let known = crate::highlight::languages();
        let mut with = 0;
        for l in &known {
            let marks = line_comments(l);
            with += usize::from(!marks.is_empty());
            assert!(marks.iter().all(|m| !m.is_empty() && !m.contains(' ')));
        }
        assert!(with >= 45, "{with} of {} grammars", known.len());
        assert!(line_comments("Plain Text").is_empty());
    }

    #[test]
    fn a_hundred_thousand_lines_are_one_quick_step() {
        let before: String = (0..100_000).map(|i| format!("  select {i};\n")).collect();
        let mut s = session(&before, "SQL");
        all(&mut s);
        let t = std::time::Instant::now();
        toggle(&mut s);
        let took = t.elapsed();
        assert!(text(&s).starts_with("  -- select 0;\n  -- select 1;\n"));
        assert!(text(&s).ends_with("  -- select 99999;\n"));
        println!("031 toggle comment on 100 000 lines: {took:?}");
        // measured in release: 170 ms (indenting the same lines: 110 ms); debug builds are slow
        if !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "{took:?}");
        }
        run(&mut s, Command::Undo);
        assert_eq!(text(&s), before);
    }
}
