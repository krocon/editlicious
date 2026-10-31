//! Tests of the editor session (design 003).

use super::layout::{LineLayout, next_grapheme, prev_grapheme};
use super::model::{CaseChange, Command, Motion, RunKind, SortMode, SortOptions};
use super::*;

fn session(text: &str) -> Session {
    let mut s = Session::new(Document::from_text(text), 1);
    let mut clip = MemoryClipboard::default();
    s.run(
        Command::Resize {
            lines: 10.0,
            cols: 40.0,
        },
        &mut clip,
    )
    .unwrap();
    s
}

fn run(s: &mut Session, cmd: Command) -> Frame {
    s.run(cmd, &mut MemoryClipboard::default()).unwrap()
}

fn text(s: &Session) -> String {
    s.doc.rope().to_string()
}

fn caret(s: &Session) -> (usize, usize) {
    let h = s.cursors()[0].head;
    (h.line, h.ch)
}

fn put(s: &mut Session, line: usize, ch: usize) {
    s.set_cursors(vec![Cursor::caret(Pos::new(line, ch))]);
}

fn mv(s: &mut Session, to: Motion) {
    run(s, Command::Move { to, select: false });
}

fn typ(s: &mut Session, t: &str) {
    run(s, Command::Type { text: t.into() });
}

// ---------------------------------------------------------------- layout

#[test]
fn layout_tabs_wide_and_combining() {
    let l = LineLayout::new("a\tb日e\u{301}x", 4);
    // a=0, tab 1..4, b=4, 日=5..7, e=7, combining=8 (0 wide), x=8
    assert_eq!(l.cells, vec![0, 1, 4, 5, 7, 8, 8, 9]);
    assert_eq!(l.width(), 9);
    assert_eq!(l.ch_at(0.4), 0);
    assert_eq!(l.ch_at(0.6), 1);
    assert_eq!(l.ch_at(2.0), 1, "first half of the tab");
    assert_eq!(l.ch_at(3.0), 2, "second half of the tab");
    assert_eq!(l.ch_at(5.9), 3, "first half of the wide char");
    assert_eq!(l.ch_at(6.1), 4);
    assert_eq!(l.ch_at(7.6), 6, "never between e and its accent");
    assert_eq!(l.ch_at_or_before(6), 3);
    assert_eq!(l.ch_at_or_before(100), 7);
}

#[test]
fn graphemes() {
    let s = "a👍🏽b"; // thumbs up + skin tone = 2 chars, 1 grapheme
    assert_eq!(next_grapheme(s, 1), Some(3));
    assert_eq!(prev_grapheme(s, 3), Some(1));
    assert_eq!(next_grapheme(s, 4), None);
    assert_eq!(prev_grapheme(s, 0), None);
}

// ---------------------------------------------------------------- motion

#[test]
fn left_right_cross_lines_and_graphemes() {
    let mut s = session("ab\r\n👍🏽c");
    put(&mut s, 0, 2);
    mv(&mut s, Motion::Right);
    assert_eq!(caret(&s), (1, 0));
    mv(&mut s, Motion::Right);
    assert_eq!(
        caret(&s),
        (1, 2),
        "skin tone modifier is part of the grapheme"
    );
    mv(&mut s, Motion::Left);
    assert_eq!(caret(&s), (1, 0));
    mv(&mut s, Motion::Left);
    assert_eq!(caret(&s), (0, 2), "CRLF is one step");
    mv(&mut s, Motion::DocStart);
    mv(&mut s, Motion::Left);
    assert_eq!(caret(&s), (0, 0));
}

#[test]
fn left_right_collapse_selection() {
    let mut s = session("hello");
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 1),
        head: Pos::new(0, 4),
        goal: None,
    }]);
    mv(&mut s, Motion::Left);
    assert_eq!(caret(&s), (0, 1));
    assert!(s.cursors()[0].is_empty());
}

#[test]
fn vertical_moves_keep_the_goal_cell() {
    let mut s = session("abcdef\nab\n\tx\nabcdefgh");
    put(&mut s, 0, 5);
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s), (1, 2));
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s), (2, 2), "goal cell 5: tab 0..4, x 4..5");
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s), (3, 5));
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s), (3, 8), "down on the last line goes to its end");
    mv(&mut s, Motion::DocStart);
    mv(&mut s, Motion::Right);
    mv(&mut s, Motion::Up);
    assert_eq!(caret(&s), (0, 0), "up on the first line goes to its start");
}

#[test]
fn word_moves() {
    let mut s = session("foo.bar  baz_1(x)\nnext");
    put(&mut s, 0, 0);
    let mut stops = vec![];
    for _ in 0..8 {
        mv(&mut s, Motion::WordRight);
        stops.push(caret(&s));
    }
    assert_eq!(
        stops,
        vec![
            (0, 3),
            (0, 4),
            (0, 7),
            (0, 14),
            (0, 15),
            (0, 16),
            (0, 17),
            (1, 0)
        ]
    );
    let mut back = vec![];
    for _ in 0..4 {
        mv(&mut s, Motion::WordLeft);
        back.push(caret(&s));
    }
    assert_eq!(back, vec![(0, 17), (0, 16), (0, 15), (0, 14)]);
    mv(&mut s, Motion::WordLeft);
    assert_eq!(caret(&s), (0, 9), "skips the spaces, then the word");
}

#[test]
fn smart_line_start_and_end() {
    let mut s = session("    indented");
    put(&mut s, 0, 8);
    mv(&mut s, Motion::LineStart);
    assert_eq!(caret(&s), (0, 4));
    mv(&mut s, Motion::LineStart);
    assert_eq!(caret(&s), (0, 0));
    mv(&mut s, Motion::LineEnd);
    assert_eq!(caret(&s), (0, 12));
}

#[test]
fn page_moves_scroll() {
    let text: String = (0..100).map(|i| format!("line {i}\n")).collect();
    let mut s = session(&text);
    mv(&mut s, Motion::PageDown);
    assert_eq!(caret(&s).0, 9);
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.top, 9.0);
    mv(&mut s, Motion::DocEnd);
    let f = run(&mut s, Command::Frame);
    assert_eq!(caret(&s), (100, 0));
    assert_eq!(f.top, 91.0, "last line at the bottom");
}

#[test]
fn select_with_shift_moves() {
    let mut s = session("one two");
    run(
        &mut s,
        Command::Move {
            to: Motion::WordRight,
            select: true,
        },
    );
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.sel_chars, 3);
    assert_eq!(f.lines[0].sel, vec![[0, 3]]);
}

// ---------------------------------------------------------------- typing

#[test]
fn typing_replaces_selection_and_undo_is_one_step() {
    let mut s = session("hello world");
    run(&mut s, Command::SelectAll);
    for c in ["n", "e", "w"] {
        typ(&mut s, c);
    }
    assert_eq!(text(&s), "new");
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "hello world");
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.sel_chars, 11, "selection before the edit comes back");
}

#[test]
fn newline_keeps_indentation_and_line_ending() {
    let mut s = session("\tif x {\r\n}");
    put(&mut s, 0, 7);
    run(&mut s, Command::Newline);
    assert_eq!(text(&s), "\tif x {\r\n\t\r\n}");
    assert_eq!(caret(&s), (1, 1));
}

#[test]
fn backspace_and_delete() {
    let mut s = session("ab\ncd");
    put(&mut s, 1, 0);
    run(&mut s, Command::Backspace);
    assert_eq!(text(&s), "abcd");
    assert_eq!(caret(&s), (0, 2));
    run(&mut s, Command::Delete);
    assert_eq!(text(&s), "abd");
    run(&mut s, Command::DeleteWordLeft);
    assert_eq!(text(&s), "d");
    put(&mut s, 0, 0);
    run(&mut s, Command::Backspace);
    assert_eq!(text(&s), "d", "nothing before the start");
}

#[test]
fn backspace_removes_a_soft_tab() {
    let mut s = session("x");
    run(
        &mut s,
        Command::Configure {
            tab_width: Some(4),
            insert_spaces: Some(true),
            detect: false,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    put(&mut s, 0, 0);
    run(&mut s, Command::Indent);
    run(&mut s, Command::Indent);
    assert_eq!(text(&s), "        x");
    run(&mut s, Command::Backspace);
    assert_eq!(text(&s), "    x");
}

#[test]
fn overwrite_mode() {
    let mut s = session("abcd");
    run(&mut s, Command::ToggleOverwrite);
    put(&mut s, 0, 1);
    typ(&mut s, "X");
    typ(&mut s, "Y");
    assert_eq!(text(&s), "aXYd");
    put(&mut s, 0, 4);
    typ(&mut s, "Z");
    assert_eq!(text(&s), "aXYdZ", "appends at the line end");
}

#[test]
fn indent_and_outdent_lines() {
    let mut s = session("a\n\nb\nc");
    run(
        &mut s,
        Command::Configure {
            tab_width: Some(2),
            insert_spaces: Some(true),
            detect: false,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 1),
        head: Pos::new(3, 0),
        goal: None,
    }]);
    run(&mut s, Command::Indent);
    assert_eq!(
        text(&s),
        "  a\n\n  b\nc",
        "empty lines and the line at column 0 stay"
    );
    assert_eq!(s.cursors()[0].anchor, Pos::new(0, 3));
    run(&mut s, Command::Outdent);
    assert_eq!(text(&s), "a\n\nb\nc");
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "  a\n\n  b\nc");
}

// ---------------------------------------------------------------- clipboard

#[test]
fn copy_cut_paste() {
    let mut s = session("one\ntwo");
    let mut clip = MemoryClipboard::default();
    put(&mut s, 0, 1);
    s.run(Command::Cut, &mut clip).unwrap();
    assert_eq!(text(&s), "two", "cut without selection takes the line");
    assert_eq!(clip.0.as_deref(), Some("one\n"));
    put(&mut s, 0, 3);
    s.run(Command::Paste, &mut clip).unwrap();
    assert_eq!(text(&s), "twoone\n");
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 0),
        head: Pos::new(0, 3),
        goal: None,
    }]);
    s.run(Command::Copy, &mut clip).unwrap();
    assert_eq!(clip.0.as_deref(), Some("two"));
    let f = s
        .run(Command::Paste, &mut MemoryClipboard::default())
        .unwrap();
    assert_eq!(f.notice.as_deref(), Some("clipboard_empty"));
}

// ---------------------------------------------------------------- mouse

#[test]
fn clicks_words_lines_and_drags() {
    let mut s = session("alpha beta\ngamma");
    run(
        &mut s,
        Command::Click {
            line: 0.5,
            cell: 7.2,
            count: 2,
            extend: false,
            add: false,
        },
    );
    let c = s.cursors()[0];
    assert_eq!((c.anchor, c.head), (Pos::new(0, 6), Pos::new(0, 10)));
    run(
        &mut s,
        Command::Drag {
            line: 0.2,
            cell: 1.0,
            column: false,
        },
    );
    let c = s.cursors()[0];
    assert_eq!((c.start(), c.end()), (Pos::new(0, 0), Pos::new(0, 10)));
    run(
        &mut s,
        Command::Click {
            line: 1.0,
            cell: 2.0,
            count: 3,
            extend: false,
            add: false,
        },
    );
    let c = s.cursors()[0];
    assert_eq!((c.anchor, c.head), (Pos::new(1, 0), Pos::new(1, 5)));
    run(
        &mut s,
        Command::Click {
            line: 0.0,
            cell: 2.4,
            count: 1,
            extend: false,
            add: false,
        },
    );
    run(
        &mut s,
        Command::Click {
            line: 1.0,
            cell: 99.0,
            count: 1,
            extend: true,
            add: false,
        },
    );
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.lines[0].sel, vec![[2, 10]]);
    assert!(f.lines[0].sel_eol);
    assert_eq!(f.lines[1].sel, vec![[0, 5]]);
    // below the last line: end of the document
    run(
        &mut s,
        Command::Click {
            line: 50.0,
            cell: 0.0,
            count: 1,
            extend: false,
            add: false,
        },
    );
    assert_eq!(caret(&s), (1, 5));
}

// ---------------------------------------------------------------- frames

#[test]
fn frame_runs_by_kind() {
    let mut s = session("a\tb日本\u{1}é");
    let f = run(&mut s, Command::Frame);
    let kinds: Vec<(usize, RunKind, &str, usize)> = f.lines[0]
        .runs
        .iter()
        .map(|r| (r.cell, r.kind, r.text.as_str(), r.width))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (0, RunKind::Text, "a", 1),
            (1, RunKind::Tab, "", 3),
            (4, RunKind::Text, "b", 1),
            (5, RunKind::Wide, "日本", 4),
            (9, RunKind::Ctrl, "\u{2401}", 1),
            (10, RunKind::Text, "é", 1),
        ]
    );
    assert_eq!(f.lines[0].width, 11);
}

#[test]
fn frame_only_holds_the_visible_window() {
    let line = "x".repeat(5_000);
    let mut s = session(&format!("{line}\n"));
    run(
        &mut s,
        Command::Move {
            to: Motion::LineEnd,
            select: false,
        },
    );
    let f = run(&mut s, Command::Frame);
    let run0 = &f.lines[0].runs[0];
    assert!(run0.text.len() < 100, "only around the visible cells");
    assert!(f.left > 4_900.0);
    assert!(run0.cell <= f.left as usize && run0.cell + run0.width >= 5_000);
    assert_eq!(f.lines[0].width, 5_000);
}

#[test]
fn long_lines_are_windows_and_typing_at_the_end_is_fast() {
    let line = "y".repeat(3_000_000);
    let mut s = session(&line);
    mv(&mut s, Motion::LineEnd);
    assert_eq!(caret(&s), (0, 3_000_000));
    let t = std::time::Instant::now();
    for _ in 0..50 {
        typ(&mut s, "z");
    }
    let f = run(&mut s, Command::Frame);
    assert!(f.lines[0].long);
    assert_eq!(f.lines[0].width, 3_000_050);
    assert_eq!(f.carets[0].cell, 3_000_050);
    assert!(f.lines[0].runs[0].text.ends_with("zzzz"));
    // debug build: generous bound, the release numbers are in docs/architecture.md
    assert!(t.elapsed().as_millis() < 2_000, "{:?}", t.elapsed());
    mv(&mut s, Motion::WordLeft);
    assert_eq!(caret(&s), (0, 0), "one word of 3 M characters");
}

#[test]
fn scrolling_is_clamped() {
    let mut s = session("a\nb\nc");
    let f = run(
        &mut s,
        Command::Scroll {
            lines: 50.0,
            cells: -3.0,
        },
    );
    assert_eq!(f.top, 2.0);
    assert_eq!(f.left, 0.0);
    let f = run(
        &mut s,
        Command::ScrollTo {
            top: Some(-1.0),
            left: None,
        },
    );
    assert_eq!(f.top, 0.0);
}

#[test]
fn several_cursors_type_together_and_merge() {
    let mut s = session("ab\ncd");
    s.set_cursors(vec![
        Cursor::caret(Pos::new(0, 2)),
        Cursor::caret(Pos::new(1, 2)),
        Cursor::caret(Pos::new(1, 2)),
    ]);
    assert_eq!(s.cursors().len(), 2, "duplicates merge");
    typ(&mut s, ";");
    assert_eq!(text(&s), "ab;\ncd;");
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.carets.len(), 2);
}

#[test]
fn detects_indentation() {
    let s = session("fn x() {\n\tlet a;\n\tlet b;\n}");
    let f = s.doc.info(1);
    let _ = f;
    let mut s = s;
    let f = run(&mut s, Command::Frame);
    assert!(!f.insert_spaces);
    let mut s = session("a:\n  b:\n    c\n  d");
    let f = run(&mut s, Command::Frame);
    assert!(f.insert_spaces);
    assert_eq!(f.tab_width, 2);
}

// ---------------------------------------------------------------- search (design 006)

fn query(p: &str) -> crate::search::Query {
    crate::search::Query {
        pattern: p.into(),
        ..Default::default()
    }
}

#[test]
fn find_selects_and_highlights() {
    let mut s = session(
        "one two one
three one",
    );
    let f = run(
        &mut s,
        Command::Find {
            query: query("one"),
            backwards: false,
            incremental: true,
        },
    );
    assert_eq!(f.sel_chars, 3);
    assert_eq!(f.lines[0].marks, vec![[0, 3, 1], [8, 11, 0]]);
    assert_eq!(f.lines[1].marks, vec![[6, 9, 0]]);
    assert!(f.search);
    assert_eq!(f.selection_text.as_deref(), Some("one"));
    let next = Command::Find {
        query: query("one"),
        backwards: false,
        incremental: false,
    };
    run(&mut s, next.clone());
    run(&mut s, next.clone());
    assert_eq!(caret(&s), (1, 9));
    run(&mut s, next);
    assert_eq!(caret(&s), (0, 3), "wraps");
    run(
        &mut s,
        Command::Find {
            query: query("one"),
            backwards: true,
            incremental: false,
        },
    );
    assert_eq!(caret(&s), (1, 9));
    let f = run(&mut s, Command::ClearSearch);
    assert!(f.lines[0].marks.is_empty());
}

#[test]
fn find_reports_no_match_and_bad_patterns() {
    let mut s = session("abc");
    let f = run(
        &mut s,
        Command::Find {
            query: query("zzz"),
            backwards: false,
            incremental: true,
        },
    );
    assert_eq!(f.notice.as_deref(), Some("no_match"));
    let f = run(
        &mut s,
        Command::Find {
            query: crate::search::Query {
                pattern: "[".into(),
                regex: true,
                ..Default::default()
            },
            backwards: false,
            incremental: true,
        },
    );
    assert_eq!(f.notice.as_deref(), Some("invalid_search"));
    assert!(!f.search);
}

#[test]
fn replace_one_and_all() {
    let mut s = session(
        "cat cat
cat",
    );
    let r = |s: &mut Session| {
        run(
            s,
            Command::Replace {
                query: query("cat"),
                replacement: "dog".into(),
            },
        )
    };
    r(&mut s); // selects the first match
    assert_eq!(
        text(&s),
        "cat cat
cat"
    );
    r(&mut s);
    assert_eq!(
        text(&s),
        "dog cat
cat"
    );
    assert_eq!(caret(&s), (0, 7), "next match selected");
    let f = run(
        &mut s,
        Command::ReplaceAll {
            query: crate::search::Query {
                pattern: "(c)at".into(),
                regex: true,
                ..Default::default()
            },
            replacement: "${1}ow".into(),
        },
    );
    assert_eq!(
        text(&s),
        "dog cow
cow"
    );
    assert_eq!(f.notice.as_deref(), Some("replaced:2"));
    run(&mut s, Command::Undo);
    assert_eq!(
        text(&s),
        "dog cat
cat",
        "one undo step"
    );
}

#[test]
fn find_far_away_centres_the_match() {
    let mut text: String = (0..200).map(|i| format!("line {i}\n")).collect();
    text.push_str("target\n");
    let mut s = session(&text);
    let f = run(
        &mut s,
        Command::Find {
            query: query("target"),
            backwards: false,
            incremental: true,
        },
    );
    assert_eq!(caret(&s), (200, 6));
    assert_eq!(f.top, 195.0, "middle of 10 rows");
}

// ---------------------------------------------------------------- design 007

#[test]
fn sort_selection_or_document() {
    let mut s = session("b10\nb9\na\n");
    let opts = SortOptions::default();
    run(&mut s, Command::SortLines { options: opts });
    assert_eq!(text(&s), "a\nb9\nb10\n", "final empty line stays last");
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "b10\nb9\na\n");
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 1),
        head: Pos::new(1, 1),
        goal: None,
    }]);
    run(
        &mut s,
        Command::SortLines {
            options: SortOptions {
                descending: true,
                mode: SortMode::Text,
                ..Default::default()
            },
        },
    );
    assert_eq!(text(&s), "b9\nb10\na\n");
}

#[test]
fn case_of_selection_and_word() {
    let mut s = session("hello world");
    put(&mut s, 0, 2);
    run(
        &mut s,
        Command::ChangeCase {
            to: CaseChange::Toggle,
        },
    );
    assert_eq!(text(&s), "HELLO world");
    run(&mut s, Command::SelectAll);
    run(
        &mut s,
        Command::ChangeCase {
            to: CaseChange::Title,
        },
    );
    assert_eq!(text(&s), "Hello World");
}

#[test]
fn duplicate_move_delete_join_lines() {
    let mut s = session("one\ntwo\nthree");
    put(&mut s, 1, 1);
    run(&mut s, Command::DuplicateLines);
    assert_eq!(text(&s), "one\ntwo\ntwo\nthree");
    assert_eq!(caret(&s), (2, 1));
    run(&mut s, Command::MoveLines { up: true });
    run(&mut s, Command::MoveLines { up: true });
    assert_eq!(text(&s), "two\none\ntwo\nthree");
    assert_eq!(caret(&s), (0, 1));
    run(&mut s, Command::MoveLines { up: true });
    assert_eq!(text(&s), "two\none\ntwo\nthree", "nothing above");
    put(&mut s, 3, 0);
    run(&mut s, Command::MoveLines { up: false });
    assert_eq!(text(&s), "two\none\ntwo\nthree", "nothing below");
    run(&mut s, Command::DeleteLines);
    assert_eq!(text(&s), "two\none\ntwo");
    put(&mut s, 0, 0);
    run(&mut s, Command::DeleteLines);
    assert_eq!(text(&s), "one\ntwo");
    run(&mut s, Command::JoinLines);
    assert_eq!(text(&s), "one two");
    assert_eq!(caret(&s), (0, 3));
}

#[test]
fn whitespace_commands() {
    let mut s = session("a  \n\tb\t\n    c");
    let f = run(&mut s, Command::TrimTrailingWhitespace);
    assert_eq!(text(&s), "a\n\tb\n    c");
    assert_eq!(f.notice.as_deref(), Some("trimmed:2"));
    run(
        &mut s,
        Command::Configure {
            tab_width: Some(4),
            insert_spaces: None,
            detect: false,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    run(&mut s, Command::ConvertIndentation { spaces: true });
    assert_eq!(text(&s), "a\n    b\n    c");
    run(&mut s, Command::ConvertIndentation { spaces: false });
    assert_eq!(text(&s), "a\n\tb\n\tc");
}

#[test]
fn column_selection_types_on_every_line() {
    let mut s = session("abcd\nab\nabcd");
    put(&mut s, 0, 1);
    run(&mut s, Command::BlockMove { dx: 0, dy: 2 });
    let f = run(&mut s, Command::BlockMove { dx: 2, dy: 0 });
    assert_eq!(f.carets.len(), 3);
    assert_eq!(f.lines[1].sel, vec![[1, 2]], "short line: up to its end");
    typ(&mut s, "X");
    assert_eq!(text(&s), "aXd\naX\naXd");
    // the cursors stay after typing; Escape goes back to one
    let f = run(&mut s, Command::Frame);
    assert!(f.multi);
    run(&mut s, Command::SingleCursor);
    assert_eq!(s.cursors().len(), 1);
}

#[test]
fn alt_drag_and_alt_click() {
    let mut s = session("aaaa\nbbbb\ncccc");
    run(
        &mut s,
        Command::Click {
            line: 0.2,
            cell: 1.0,
            count: 1,
            extend: false,
            add: false,
        },
    );
    run(
        &mut s,
        Command::Drag {
            line: 2.3,
            cell: 3.0,
            column: true,
        },
    );
    assert_eq!(s.cursors().len(), 3);
    assert_eq!(s.cursors()[2].head, Pos::new(2, 3));
    run(
        &mut s,
        Command::Click {
            line: 0.0,
            cell: 0.0,
            count: 1,
            extend: false,
            add: false,
        },
    );
    run(
        &mut s,
        Command::Click {
            line: 2.0,
            cell: 4.0,
            count: 1,
            extend: false,
            add: true,
        },
    );
    typ(&mut s, "!");
    assert_eq!(text(&s), "!aaaa\nbbbb\ncccc!");
}

#[test]
fn next_occurrence_adds_cursors() {
    let mut s = session("foo bar foo foo");
    put(&mut s, 0, 1);
    run(&mut s, Command::AddNextOccurrence);
    assert_eq!(s.cursors()[0].end(), Pos::new(0, 3), "word selected first");
    run(&mut s, Command::AddNextOccurrence);
    run(&mut s, Command::AddNextOccurrence);
    assert_eq!(s.cursors().len(), 3);
    let f = run(&mut s, Command::AddNextOccurrence);
    assert_eq!(f.notice.as_deref(), Some("no_more_matches"));
    typ(&mut s, "baz");
    assert_eq!(text(&s), "baz bar baz baz");
}

#[test]
fn paste_one_line_per_cursor() {
    let mut s = session("a\nb");
    s.set_cursors(vec![
        Cursor::caret(Pos::new(0, 1)),
        Cursor::caret(Pos::new(1, 1)),
    ]);
    let mut clip = MemoryClipboard(Some("1\n2\n".into()));
    s.run(Command::Paste, &mut clip).unwrap();
    assert_eq!(text(&s), "a1\nb2");
}

#[test]
fn markers_colour_words() {
    let mut s = session("let x = x + xx;");
    put(&mut s, 0, 4);
    let f = run(&mut s, Command::ToggleMarker);
    assert_eq!(
        f.lines[0].markers,
        vec![[4, 5, 0], [8, 9, 0]],
        "whole words only"
    );
    put(&mut s, 0, 0);
    let f = run(&mut s, Command::ToggleMarker);
    assert_eq!(f.lines[0].markers.len(), 3);
    assert_eq!(f.lines[0].markers[2], [0, 3, 1]);
    put(&mut s, 0, 4);
    let f = run(&mut s, Command::ToggleMarker);
    assert_eq!(f.lines[0].markers, vec![[0, 3, 1]], "again = unmark");
    let f = run(&mut s, Command::ClearMarkers);
    assert!(f.lines[0].markers.is_empty());
}

// ---------------------------------------------------------------- highlighting (design 008)

#[test]
fn frames_carry_styles_by_language() {
    let mut s = session("// hi\nlet x = 1;");
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.language, None, "no path, plain first line");
    let f = run(
        &mut s,
        Command::SetLanguage {
            name: Some("Rust".into()),
        },
    );
    assert_eq!(f.language.as_deref(), Some("Rust"));
    assert_eq!(f.lines[0].runs[0].style, 1, "comment");
    assert!(
        f.lines[1]
            .runs
            .iter()
            .any(|r| r.style == 4 && r.text == "1")
    );
    // typing invalidates from the changed line on
    put(&mut s, 0, 0);
    typ(&mut s, "x");
    let f = run(&mut s, Command::Frame);
    assert_eq!(
        f.lines[0]
            .runs
            .iter()
            .map(|r| r.text.as_str())
            .collect::<String>(),
        "x// hi"
    );
    let f = run(&mut s, Command::SetLanguage { name: None });
    assert!(f.lines[1].runs.iter().all(|r| r.style == 0));
}

/// `cargo test -p ed-core --release -- --ignored --nocapture measure_` (design 008).
#[test]
#[ignore]
fn measure_highlighting() {
    use std::time::Instant;
    let code = "fn main() {\n    let s = \"text\"; // comment\n    println!(\"{}\", s + 42);\n}\n";
    let mut s = session(&code.repeat(25_000)); // 100 000 lines
    run(
        &mut s,
        Command::SetLanguage {
            name: Some("Rust".into()),
        },
    );
    run(
        &mut s,
        Command::Resize {
            lines: 50.0,
            cols: 120.0,
        },
    );
    let t = Instant::now();
    run(&mut s, Command::Frame);
    println!("first frame: {:?}", t.elapsed());
    put(&mut s, 2_000, 4);
    let t = Instant::now();
    typ(&mut s, "x");
    println!("first keystroke at line 2000 (catch up): {:?}", t.elapsed());
    let t = Instant::now();
    for _ in 0..20 {
        typ(&mut s, "x");
    }
    println!("20 keystrokes with highlighting: {:?}", t.elapsed());
    mv(&mut s, Motion::DocEnd);
    let t = Instant::now();
    let f = run(&mut s, Command::Frame);
    println!(
        "frame at the end (approx {}): {:?}",
        f.approx_highlight,
        t.elapsed()
    );
}

#[test]
fn configure_detect_follows_the_file() {
    // indented with two spaces: the file wins over the setting
    let mut s = session("a\n  b\n    c\n");
    let f = run(
        &mut s,
        Command::Configure {
            tab_width: Some(8),
            insert_spaces: Some(false),
            detect: true,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    assert!(f.insert_spaces);
    assert_eq!(f.tab_width, 2);
    // not indented: the settings stay
    let mut s = session("a\nb\n");
    let f = run(
        &mut s,
        Command::Configure {
            tab_width: Some(8),
            insert_spaces: Some(false),
            detect: true,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    assert!(!f.insert_spaces);
    assert_eq!(f.tab_width, 8);
    // tabs: tabs, width from the setting
    let mut s = session("a\n\tb\n");
    let f = run(
        &mut s,
        Command::Configure {
            tab_width: Some(3),
            insert_spaces: Some(true),
            detect: true,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    assert!(!f.insert_spaces);
    assert_eq!(f.tab_width, 3);
}

#[test]
fn frame_lines_carry_their_indentation() {
    let mut s = session("a\n    b\n\n\tc\n  \n");
    run(
        &mut s,
        Command::Configure {
            tab_width: Some(4),
            insert_spaces: None,
            detect: false,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    let f = run(&mut s, Command::Frame);
    let ind: Vec<usize> = f.lines.iter().map(|l| l.indent).collect();
    // blank lines take the indentation of the line above
    assert_eq!(ind, vec![0, 4, 4, 4, 4, 4]);
}

// ---------------------------------------------------------------- soft wrap (design 010 E4)

fn wrapped(text: &str, lines: f64, cols: f64) -> Session {
    let mut s = session(text);
    run(&mut s, Command::Resize { lines, cols });
    run(
        &mut s,
        Command::Configure {
            tab_width: Some(4),
            insert_spaces: None,
            detect: false,
            wrap: Some(true),
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    s
}

#[test]
fn wrap_splits_lines_into_rows() {
    // view 11 cells → rows of 10
    let mut s = wrapped("aaaa bbbb cccc dddd\nshort\n", 10.0, 11.0);
    let f = run(&mut s, Command::Frame);
    assert!(f.wrap);
    assert_eq!(f.lines[0].wraps, vec![10]);
    assert_eq!(f.lines[0].row, 0.0);
    assert_eq!(f.lines[1].row, 2.0);
    // runs never cross a row start
    for r in &f.lines[0].runs {
        assert!(!(r.cell < 10 && r.cell + r.width > 10), "{r:?}");
    }
    // the caret at the end of the first line is on its second row
    put(&mut s, 0, 19);
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.caret_xy[0].row, 1.0);
    assert_eq!(f.caret_xy[0].x, 9);
    assert_eq!(f.left, 0.0);
}

#[test]
fn wrap_moves_by_rows() {
    let mut s = wrapped("aaaa bbbb cccc dddd\nxy\n", 10.0, 11.0);
    put(&mut s, 0, 2);
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s), (0, 12)); // second row, same x
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s), (1, 2));
    mv(&mut s, Motion::Up);
    assert_eq!(caret(&s), (0, 12));
    mv(&mut s, Motion::Up);
    assert_eq!(caret(&s), (0, 2));
    // the goal stays before the row break
    put(&mut s, 1, 2);
    run(
        &mut s,
        Command::Move {
            to: Motion::LineEnd,
            select: false,
        },
    );
    put(&mut s, 0, 9); // "aaaa bbbb |" end of first row (break after the space at 10)
    mv(&mut s, Motion::Down);
    assert_eq!(caret(&s).0, 0);
}

#[test]
fn wrap_scrolls_and_clicks_by_rows() {
    // 30 lines of 3 rows each, view of 5 rows
    let line = "aaaaaaaa bbbbbbbb cccccccc";
    let text = vec![line; 30].join("\n");
    let mut s = wrapped(&text, 5.0, 11.0);
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.lines[0].wraps.len(), 2);
    run(
        &mut s,
        Command::Scroll {
            lines: 4.0,
            cells: 0.0,
        },
    );
    let f = run(&mut s, Command::Frame);
    // 4 rows down: line 1, second row
    assert_eq!(f.first_line, 1);
    assert_eq!(f.lines[0].row, -1.0);
    // a click on view row 0 is line 1, row 1
    let top = f.top;
    run(
        &mut s,
        Command::Click {
            line: top,
            cell: 2.0,
            count: 1,
            extend: false,
            add: false,
        },
    );
    assert_eq!(caret(&s), (1, 11));
    // the caret far below: reveal scrolls by rows, the caret ends on the last view row
    put(&mut s, 20, 0);
    run(
        &mut s,
        Command::Move {
            to: Motion::Right,
            select: false,
        },
    );
    let f = run(&mut s, Command::Frame);
    let xy = f.caret_xy[f.primary];
    assert!(xy.row >= 0.0 && xy.row < 5.0, "{xy:?}");
    // page down moves the caret by rows too
    put(&mut s, 0, 0);
    run(
        &mut s,
        Command::ScrollTo {
            top: Some(0.0),
            left: None,
        },
    );
    mv(&mut s, Motion::PageDown);
    assert_eq!(caret(&s), (1, 9)); // 4 rows down
    // wrap off again: one row per line
    run(
        &mut s,
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: Some(false),
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    let f = run(&mut s, Command::Frame);
    assert!(f.lines.iter().all(|l| l.wraps.is_empty()));
}

#[test]
fn wrap_leaves_long_lines_alone() {
    let long = "x".repeat(crate::document::FULL_LINE_LIMIT + 5);
    let mut s = wrapped(&format!("{long}\nab\n"), 10.0, 11.0);
    let f = run(&mut s, Command::Frame);
    assert!(f.lines[0].long);
    assert!(f.lines[0].wraps.is_empty());
    assert_eq!(f.lines[1].row, 1.0);
}

// ---------------------------------------------------------------- line endings, HTML (design 010)

#[test]
fn line_endings_convert_in_one_undo_step() {
    let mut s = session("a\r\nb\nc\rd");
    put(&mut s, 2, 1);
    let f = run(
        &mut s,
        Command::SetLineEnding {
            line_ending: crate::document::model::LineEnding::Crlf,
        },
    );
    assert_eq!(text(&s), "a\r\nb\r\nc\r\nd");
    assert_eq!(caret(&s), (2, 1));
    assert!(f.info.dirty);
    assert!(!f.info.mixed_line_endings);
    // new lines follow
    typ(&mut s, "x");
    run(&mut s, Command::Newline);
    assert!(text(&s).contains("cx\r\n"));
    run(&mut s, Command::Undo);
    run(&mut s, Command::Undo);
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "a\r\nb\nc\rd");
    // lone CRs anywhere, also at the very end and twice in a row
    let mut s = session("a\r\rb\r");
    run(
        &mut s,
        Command::SetLineEnding {
            line_ending: crate::document::model::LineEnding::Lf,
        },
    );
    assert_eq!(text(&s), "a\n\nb\n");
    // nothing to do: still fine
    let mut s = session("a\nb");
    run(
        &mut s,
        Command::SetLineEnding {
            line_ending: crate::document::model::LineEnding::Lf,
        },
    );
    assert!(!s.doc.dirty());
}

#[derive(Default)]
struct HtmlClip {
    html: Option<String>,
    text: Option<String>,
}

impl Clipboard for HtmlClip {
    fn get(&mut self) -> Option<String> {
        self.text.clone()
    }
    fn set(&mut self, text: String) -> bool {
        self.text = Some(text);
        true
    }
    fn set_html(&mut self, html: String, alt: String) -> bool {
        self.html = Some(html);
        self.text = Some(alt);
        true
    }
}

#[test]
fn copy_as_html_with_styles_and_line_numbers() {
    let mut s = Session::new(
        Document::from_text("fn main() {\n    let a = \"<x>\";\n}\n"),
        1,
    );
    let mut clip = HtmlClip::default();
    s.run(
        Command::SetLanguage {
            name: Some("Rust".into()),
        },
        &mut clip,
    )
    .unwrap();
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 0),
        head: Pos::new(2, 0),
        goal: None,
    }]);
    let styles: Vec<String> = (0..30).map(|i| format!("color:#{i:06}")).collect();
    s.run(
        Command::CopyHtml {
            line_numbers: true,
            styles,
            base: "color:black;background:white".into(),
            gutter: "color:gray".into(),
            markers: vec![],
            background: String::new(),
        },
        &mut clip,
    )
    .unwrap();
    let html = clip.html.unwrap();
    assert!(html.starts_with("<pre style=\"color:black;background:white;tab-size:4"));
    assert!(html.contains("&quot;&lt;x&gt;&quot;"), "{html}");
    assert!(html.contains("<span style=\"color:gray;user-select:none\">1 </span>"));
    assert!(html.contains("<span style=\"color:#"), "styled: {html}");
    // two lines (the end at the start of line 3 does not count)
    assert_eq!(html.matches("user-select:none").count(), 2);
    assert_eq!(clip.text.unwrap(), "fn main() {\n    let a = \"<x>\";");
}

#[test]
#[ignore]
fn measure_wrap_and_line_endings() {
    use std::time::Instant;
    let line = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor ";
    let text = format!("{}\n", line.repeat(3)).repeat(1_000_000); // 1M lines, ~240 MB? no: 237 chars × 1M
    let mut s = session(&text);
    run(
        &mut s,
        Command::Resize {
            lines: 50.0,
            cols: 100.0,
        },
    );
    run(
        &mut s,
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: Some(true),
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    let t = Instant::now();
    run(&mut s, Command::Frame);
    println!("wrapped frame: {:?}", t.elapsed());
    let t = Instant::now();
    for _ in 0..50 {
        run(
            &mut s,
            Command::Scroll {
                lines: 3.0,
                cells: 0.0,
            },
        );
    }
    println!("50 wrapped scroll steps: {:?}", t.elapsed());
    let t = Instant::now();
    run(
        &mut s,
        Command::Move {
            to: Motion::DocEnd,
            select: false,
        },
    );
    println!("jump to the end (wrapped): {:?}", t.elapsed());
    let t = Instant::now();
    run(
        &mut s,
        Command::SetLineEnding {
            line_ending: crate::document::model::LineEnding::Crlf,
        },
    );
    println!("1M line breaks to CRLF: {:?}", t.elapsed());
    let t = Instant::now();
    run(&mut s, Command::Undo);
    println!("undo of it: {:?}", t.elapsed());
}

#[test]
fn wrap_width_follows_the_view() {
    // configured before the first resize (default view): later frames use the real width
    let mut s = Session::new(Document::from_text(&"word ".repeat(100)), 1);
    let mut clip = MemoryClipboard::default();
    s.run(
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: Some(true),
            colors: None,
            expand_weights: None,
            math: None,
            words: None,
        },
        &mut clip,
    )
    .unwrap();
    let f = s
        .run(
            Command::Resize {
                lines: 10.0,
                cols: 41.0,
            },
            &mut clip,
        )
        .unwrap();
    assert!(f.max_width <= 40, "{}", f.max_width);
    assert_eq!(f.lines[0].wraps[0], 40);
}

#[test]
fn line_ending_conversion_undoes_and_redoes_compactly() {
    let orig = "a\r\nb\nc\rd\r\re\n";
    let mut s = session(orig);
    assert!(s.doc.mixed_line_endings);
    run(
        &mut s,
        Command::SetLineEnding {
            line_ending: crate::document::model::LineEnding::Lf,
        },
    );
    assert_eq!(text(&s), "a\nb\nc\nd\n\ne\n");
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), orig);
    assert!(s.doc.mixed_line_endings);
    assert!(!s.doc.dirty());
    run(&mut s, Command::Redo);
    assert_eq!(text(&s), "a\nb\nc\nd\n\ne\n");
    assert_eq!(s.doc.line_ending, crate::document::model::LineEnding::Lf);
    assert!(s.doc.dirty());
}

// ---------------------------------------------------------------- review fixes (design 013)

fn rq(p: &str) -> crate::search::Query {
    crate::search::Query {
        pattern: p.into(),
        regex: true,
        ..Default::default()
    }
}

#[test]
fn regex_groups_are_taken_in_context() {
    let mut s = session("ab cd");
    run(
        &mut s,
        Command::ReplaceAll {
            query: rq(r"\B(\w)"),
            replacement: "<$1>".into(),
        },
    );
    assert_eq!(text(&s), "a<b> c<d>");
    let mut s = session("ab cd");
    put(&mut s, 0, 0);
    run(
        &mut s,
        Command::Find {
            query: rq(r"\B(\w)"),
            backwards: false,
            incremental: false,
        },
    );
    run(
        &mut s,
        Command::Replace {
            query: rq(r"\B(\w)"),
            replacement: "[$1]".into(),
        },
    );
    assert_eq!(text(&s), "a[b] cd");
}

#[test]
fn replace_all_can_cut_into_crlf() {
    let mut s = session("a\r\nb\r\nc");
    run(
        &mut s,
        Command::ReplaceAll {
            query: rq(r"\r"),
            replacement: String::new(),
        },
    );
    assert_eq!(text(&s), "a\nb\nc");
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "a\r\nb\r\nc");
}

/// Design 018: colour values in the frame, the column that stays, converting under the caret.
#[test]
fn colour_swatches_and_converting() {
    let mut s = session("plain\nbg=\"ffffff00\" x\na { color: #2f98e1; }");
    let f = run(&mut s, Command::Frame);
    assert!(f.lines[0].colors.is_empty());
    assert_eq!(f.lines[1].colors, vec![[4, 12, 0xffff00ff]]);
    assert_eq!(f.lines[2].colors, vec![[11, 18, 0x2f98e1ff]]);
    assert!(f.color_column);
    assert_eq!(f.caret_color, None);
    put(&mut s, 2, 14);
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.caret_color.as_deref(), Some("#2f98e1"));
    run(&mut s, Command::ConvertColor);
    assert_eq!(
        text(&s).lines().nth(2),
        Some("a { color: rgb(47, 152, 225); }")
    );
    run(&mut s, Command::ConvertColor);
    assert!(text(&s).contains("hsl(205, 75%, 53%)"));
    run(&mut s, Command::Undo);
    assert!(text(&s).contains("rgb(47, 152, 225)"));
    // nothing under the caret
    put(&mut s, 0, 1);
    assert_eq!(
        run(&mut s, Command::ConvertColor).notice.as_deref(),
        Some("no_color")
    );
    // switched off: no colours, no column
    let f = run(
        &mut s,
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: None,
            colors: Some(false),
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    assert!(f.lines.iter().all(|l| l.colors.is_empty()));
    assert!(!f.color_column);
}

/// Measures the colour scan of a full screen (ignored; `measure_` in the release run).
#[test]
#[ignore]
fn measure_colours_per_frame() {
    let line = r#"  .a { color: #2f98e1; background: rgba(0, 0, 0, 0.5); border: 1px solid hsl(10, 50%, 50%) }"#;
    let text = vec![line; 10_000].join("\n");
    let mut s = session(&text);
    run(
        &mut s,
        Command::Resize {
            lines: 100.0,
            cols: 200.0,
        },
    );
    let t = web_time::Instant::now();
    for i in 0..100 {
        s.run(
            Command::ScrollTo {
                top: Some(i as f64 * 50.0),
                left: None,
            },
            &mut MemoryClipboard::default(),
        )
        .unwrap();
    }
    let per = t.elapsed() / 100;
    let mut off = session(&text);
    run(
        &mut off,
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: None,
            colors: Some(false),
            expand_weights: None,
            math: None,
            words: None,
        },
    );
    run(
        &mut off,
        Command::Resize {
            lines: 100.0,
            cols: 200.0,
        },
    );
    let t = web_time::Instant::now();
    for i in 0..100 {
        off.run(
            Command::ScrollTo {
                top: Some(i as f64 * 50.0),
                left: None,
            },
            &mut MemoryClipboard::default(),
        )
        .unwrap();
    }
    println!(
        "frame of 100 lines with 300 colours: {per:?} (without colours {:?})",
        t.elapsed() / 100
    );
}

// ---------------------------------------------------------------- progressive double click (018a)

/// A double click like the browser sends it: a click (detail 1), then detail 2.
fn dbl(s: &mut Session, line: usize, cell: usize) -> String {
    for count in [1, 2] {
        run(
            s,
            Command::Click {
                line: line as f64,
                cell: cell as f64 + 0.2,
                count,
                extend: false,
                add: false,
            },
        );
    }
    sel_text(s)
}

fn sel_text(s: &Session) -> String {
    let c = s.primary();
    s.doc.text(c.start(), c.end()).unwrap()
}

#[test]
fn repeated_double_clicks_grow_step_by_step_and_start_over() {
    let mut s = session("let x = user_profile_name;\nnext");
    // every step, then the line, then the paragraph, then the word again
    let got: Vec<String> = (0..7).map(|_| dbl(&mut s, 0, 15)).collect();
    assert_eq!(
        got,
        [
            "profile",
            "user_profile",
            "user_profile_name",
            "user_profile_name;",
            "= user_profile_name;",
            "x = user_profile_name;",
            "let x = user_profile_name;",
        ]
    );
    assert_eq!(dbl(&mut s, 0, 15), "let x = user_profile_name;\nnext");
    assert_eq!(
        dbl(&mut s, 0, 15),
        "profile",
        "after the largest: the word again"
    );
}

#[test]
fn start_middle_and_end_of_a_token_and_slow_repeats() {
    for cell in [4, 7, 10] {
        let mut s = session("com.example.service");
        assert_eq!(dbl(&mut s, 0, cell), "example");
        // slow: a click between the double clicks keeps the sequence as long as it is inside
        run(
            &mut s,
            Command::Click {
                line: 0.0,
                cell: 9.0,
                count: 1,
                extend: false,
                add: false,
            },
        );
        assert_eq!(dbl(&mut s, 0, cell), "com.example", "cell {cell}");
        assert_eq!(dbl(&mut s, 0, cell), "com.example.service");
    }
}

#[test]
fn another_place_or_any_other_action_starts_with_the_word() {
    let mut s = session("my-component-name other_thing");
    assert_eq!(dbl(&mut s, 0, 4), "component");
    assert_eq!(dbl(&mut s, 0, 4), "my-component");
    // another place
    assert_eq!(dbl(&mut s, 0, 26), "thing");
    // a cursor move in between
    assert_eq!(dbl(&mut s, 0, 26), "other_thing");
    mv(&mut s, Motion::Left);
    assert_eq!(dbl(&mut s, 0, 26), "thing");
    // typing (a text change) in between
    assert_eq!(dbl(&mut s, 0, 4), "component");
    run(
        &mut s,
        Command::Move {
            to: Motion::DocEnd,
            select: false,
        },
    );
    typ(&mut s, "!");
    assert_eq!(dbl(&mut s, 0, 4), "component");
    // a change of the text by undo also starts over
    assert_eq!(dbl(&mut s, 0, 4), "my-component");
    run(&mut s, Command::Undo);
    assert_eq!(dbl(&mut s, 0, 4), "component");
    // a triple click still takes the line
    run(
        &mut s,
        Command::Click {
            line: 0.0,
            cell: 4.0,
            count: 3,
            extend: false,
            add: false,
        },
    );
    assert_eq!(sel_text(&s), "my-component-name other_thing");
}

#[test]
fn double_clicks_with_umlauts_emoji_and_wide_characters() {
    let mut s = session("Straße: Königs_straße 🙂中文_x");
    // cells: wide characters take two
    assert_eq!(dbl(&mut s, 0, 10), "Königs");
    assert_eq!(dbl(&mut s, 0, 10), "Königs_straße");
    let mut s = session("Straße: Königs_straße 🙂中文_x");
    assert_eq!(dbl(&mut s, 0, 24), "🙂中文");
    assert_eq!(dbl(&mut s, 0, 24), "🙂中文_x");
}

#[test]
fn custom_weights_from_the_settings() {
    let mut s = session("a.b_c");
    let mut w = std::collections::HashMap::new();
    w.insert(".".to_string(), 1.0);
    w.insert("_".to_string(), 3.5);
    run(
        &mut s,
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: None,
            colors: None,
            expand_weights: Some(w),
            math: None,
            words: None,
        },
    );
    assert_eq!(dbl(&mut s, 0, 0), "a");
    assert_eq!(dbl(&mut s, 0, 0), "a.b");
    assert_eq!(dbl(&mut s, 0, 0), "a.b_c");
}

// ---------------------------------------------------------------- markers (design 019)

fn html_of(s: &mut Session, markers: Vec<String>, bg: &str) -> String {
    #[derive(Default)]
    struct Clip(Option<String>);
    impl Clipboard for Clip {
        fn get(&mut self) -> Option<String> {
            None
        }
        fn set(&mut self, _: String) -> bool {
            true
        }
        fn set_html(&mut self, html: String, _: String) -> bool {
            self.0 = Some(html);
            true
        }
    }
    let mut clip = Clip::default();
    s.run(
        Command::CopyHtml {
            line_numbers: false,
            styles: vec![],
            base: "color:#222".into(),
            gutter: String::new(),
            markers,
            background: bg.into(),
        },
        &mut clip,
    )
    .unwrap();
    clip.0.unwrap()
}

#[test]
fn copy_as_html_keeps_markers_in_solid_colours() {
    let mut s = session("let row = col + row;\nrow");
    put(&mut s, 0, 5);
    run(&mut s, Command::SetMarker { colour: 6 }); // row: yellow
    put(&mut s, 0, 11);
    run(&mut s, Command::SetMarker { colour: 1 }); // col: blue
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 0),
        head: Pos::new(1, 3),
        goal: None,
    }]);
    let mut markers = vec!["rgb(0 0 0 / 0%)".to_string(); 10];
    markers[6] = "rgba(230, 200, 0, 0.34)".into();
    markers[1] = "rgb(54 162 235 / 30%)".into();
    let html = html_of(&mut s, markers, "rgb(251, 250, 247)");
    // 230*0.34 + 251*0.66 = 243.9 → f4; 200*.34+250*.66 = 233 → e9; 0*.34+247*.66=163 → a3
    assert!(html.contains("background-color:#f4e9a3"), "{html}");
    assert_eq!(
        html.matches("background-color:#f4e9a3").count(),
        3,
        "row ×3: {html}"
    );
    assert!(html.contains("background-color:#c0dff3"), "col: {html}");
    assert!(
        html.starts_with("<pre style=\"color:#222;background-color:#fbfaf7"),
        "{html}"
    );
    // without marker colours: as before
    let plain = html_of(&mut s, vec![], "");
    assert!(!plain.contains("background-color"), "{plain}");
}

#[test]
fn fixed_colours_steps_and_restore() {
    let mut s = session("a row b\nrow\nx row");
    put(&mut s, 0, 3);
    run(&mut s, Command::SetMarker { colour: 4 });
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.marker_words[4].as_deref(), Some("row"));
    assert_eq!(f.caret_marker, Some(4));
    // the same word in another colour moves
    run(&mut s, Command::SetMarker { colour: 2 });
    let f = run(&mut s, Command::Frame);
    assert_eq!(f.marker_words[2].as_deref(), Some("row"));
    assert_eq!(f.marker_words[4], None);
    // next / previous occurrence, wrapping
    run(&mut s, Command::MarkerStep { backwards: false });
    assert_eq!(sel_text(&s), "row");
    assert_eq!(s.primary().start(), Pos::new(1, 0));
    run(&mut s, Command::MarkerStep { backwards: false });
    assert_eq!(s.primary().start(), Pos::new(2, 2));
    run(&mut s, Command::MarkerStep { backwards: false });
    assert_eq!(s.primary().start(), Pos::new(0, 2));
    run(&mut s, Command::MarkerStep { backwards: true });
    assert_eq!(s.primary().start(), Pos::new(2, 2));
    // not on a marker
    put(&mut s, 0, 0);
    assert_eq!(
        run(&mut s, Command::MarkerStep { backwards: false })
            .notice
            .as_deref(),
        Some("no_marker")
    );
    // a selection (not a whole word) can be marked
    s.set_cursors(vec![Cursor {
        anchor: Pos::new(0, 1),
        head: Pos::new(0, 4),
        goal: None,
    }]);
    run(&mut s, Command::ToggleMarker);
    let f = run(&mut s, Command::Frame);
    assert!(f.marker_words.iter().any(|w| w.as_deref() == Some(" ro")));
    // restore in another session
    let mut t = session("a row b\nrow\nx row");
    run(
        &mut t,
        Command::SetMarkers {
            words: f.marker_words.clone(),
        },
    );
    let g = run(&mut t, Command::Frame);
    let mut want = f.marker_words.clone();
    while want.last() == Some(&None) {
        want.pop();
    }
    assert_eq!(g.marker_words, want);
    assert!(!g.lines[1].markers.is_empty());
}

#[test]
fn whitespace_is_not_marked() {
    let mut s = session("a   b");
    put(&mut s, 0, 2);
    run(&mut s, Command::ToggleMarker);
    run(&mut s, Command::SetMarker { colour: 3 });
    let f = run(&mut s, Command::Frame);
    assert!(f.marker_words.iter().all(Option::is_none));
}

// ---------------------------------------------------------------- arithmetic (prompt 020)

#[test]
fn math_hint_at_the_caret() {
    let mut s = session("x = 3\n");
    put(&mut s, 1, 0);
    typ(&mut s, "255/8");
    let f = run(&mut s, Command::Frame);
    let h = f.hint.expect("hint");
    assert_eq!((h.line, h.cell, h.expr.as_str()), (1, 0, "255 / 8"));
    assert_eq!(h.result.as_deref(), Some("31.875"));
    assert!(f.completion.is_none());
    // variables of the lines above
    typ(&mut s, " + 2x");
    let h = run(&mut s, Command::Frame).hint.unwrap();
    assert_eq!(h.result.as_deref(), Some("37.875"));
    // too large: ∞; division by zero: "not defined"
    let mut z = session("");
    typ(&mut z, "fac(171)+1");
    assert_eq!(
        run(&mut z, Command::Frame).hint.unwrap().result.as_deref(),
        Some("∞")
    );
    let mut z = session("");
    typ(&mut z, "1/0");
    assert_eq!(run(&mut z, Command::Frame).hint.unwrap().result, None);
    // no hint for data
    let mut d = session("");
    typ(&mut d, "2026-10-26");
    assert!(run(&mut d, Command::Frame).hint.is_none());
}

#[test]
fn equals_offers_the_result_and_taking_it_is_one_undo_step() {
    let mut s = session("");
    typ(&mut s, "(1+2)*3=");
    let f = run(&mut s, Command::Frame);
    let c = f.completion.expect("suggestions after =");
    assert_eq!(c.items[0].label, "9");
    assert!(f.hint.is_none(), "the list replaces the hint");
    run(&mut s, Command::ApplyCompletion { index: 0 });
    assert_eq!(text(&s), "(1+2)*3=9");
    assert_eq!(caret(&s), (0, 9));
    assert!(run(&mut s, Command::Frame).completion.is_none());
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "(1+2)*3=");
    // 2^3^2 = 512; typing on closes the list
    let mut s = session("");
    typ(&mut s, "2^3^2=");
    assert_eq!(
        run(&mut s, Command::Frame).completion.unwrap().items[0].label,
        "512"
    );
    typ(&mut s, " ");
    assert!(run(&mut s, Command::Frame).completion.is_none());
    // replace the expression by its result
    let mut s = session("");
    typ(&mut s, "255/8=");
    let items = run(&mut s, Command::Frame).completion.unwrap().items;
    let i = items.iter().position(|i| i.kind == "replace").unwrap();
    run(&mut s, Command::ApplyCompletion { index: i });
    assert_eq!(text(&s), "31.875");
}

#[test]
fn complete_evaluate_and_copy_result() {
    let mut s = session("a 255/8 b");
    put(&mut s, 0, 7);
    let f = run(&mut s, Command::Complete);
    let labels: Vec<_> = f
        .completion
        .unwrap()
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(labels, ["= 31.875", "31.875", "31.88"]);
    run(&mut s, Command::CloseCompletion);
    let mut clip = MemoryClipboard::default();
    let f = s.run(Command::CopyResult, &mut clip).unwrap();
    assert_eq!(f.notice.as_deref(), Some("result_copied"));
    assert_eq!(clip.get().as_deref(), Some("31.875"));
    run(&mut s, Command::Evaluate);
    assert_eq!(text(&s), "a 31.875 b");
    assert_eq!(caret(&s), (0, 8));
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "a 255/8 b");
    // nothing there: neither arithmetic nor words (design 023 E12)
    put(&mut s, 0, 1);
    assert_eq!(
        run(&mut s, Command::Complete).notice.as_deref(),
        Some("no_suggestions")
    );
    assert_eq!(
        run(&mut s, Command::Evaluate).notice.as_deref(),
        Some("no_expression")
    );
    // switched off: no hint, no list after "="
    run(
        &mut s,
        Command::Configure {
            tab_width: None,
            insert_spaces: None,
            detect: false,
            wrap: None,
            colors: None,
            expand_weights: None,
            math: Some(model::MathConfig {
                hints: false,
                suggest: false,
                decimals: Some(1),
                comma: true,
            }),
            words: None,
        },
    );
    put(&mut s, 0, 7);
    assert!(run(&mut s, Command::Frame).hint.is_none());
    typ(&mut s, "=");
    assert!(run(&mut s, Command::Frame).completion.is_none());
    // the settings shape the result
    let f = run(&mut s, Command::Complete);
    assert_eq!(f.completion.unwrap().items[0].label, "31,9");
}

#[test]
fn completion_is_dropped_when_the_text_changed() {
    let mut s = session("");
    typ(&mut s, "1+1=");
    run(&mut s, Command::Frame);
    // an edit that does not close the list on its own way (undo) makes it stale
    run(&mut s, Command::Undo);
    assert!(
        run(&mut s, Command::ApplyCompletion { index: 0 })
            .completion
            .is_none()
    );
    assert!(!text(&s).contains('2'));
}

#[test]
fn arithmetic_is_recorded_as_the_edit_it_makes() {
    let mut s = session("");
    s.start_recording("test");
    typ(&mut s, "(1+2)*3=");
    run(&mut s, Command::ApplyCompletion { index: 0 });
    run(&mut s, Command::Complete);
    run(&mut s, Command::CloseCompletion);
    let rec = s.stop_recording().unwrap();
    let cmds: Vec<_> = rec.steps.iter().map(|s| &s.cmd).collect();
    assert!(matches!(cmds[0], Command::Type { .. }));
    assert!(matches!(cmds[1], Command::Select { .. }));
    assert_eq!(cmds[2], &Command::Insert { text: "9".into() });
    assert_eq!(cmds.len(), 3);
}

// ---------------------------------------------------------------- word completion (prompt 023)

fn labels(f: &Frame) -> Vec<(String, String)> {
    f.completion
        .as_ref()
        .map(|c| {
            c.items
                .iter()
                .map(|i| (i.label.clone(), i.kind.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn words_config(auto: bool, keywords: bool) -> Command {
    Command::Configure {
        tab_width: None,
        insert_spaces: None,
        detect: false,
        wrap: None,
        colors: None,
        expand_weights: None,
        math: None,
        words: Some(model::WordConfig { auto, keywords }),
    }
}

#[test]
fn completes_words_of_the_document() {
    let mut s = session("printLine(x);\nprintAll();\nint pri");
    put(&mut s, 2, 7);
    let f = run(&mut s, Command::Complete);
    // nearest line first
    assert_eq!(
        labels(&f),
        [
            ("printAll".to_string(), "word".to_string()),
            ("printLine".to_string(), "word".to_string())
        ]
    );
    run(&mut s, Command::ApplyCompletion { index: 1 });
    assert_eq!(text(&s), "printLine(x);\nprintAll();\nint printLine");
    assert_eq!(caret(&s), (2, 13));
    run(&mut s, Command::Undo);
    assert_eq!(text(&s), "printLine(x);\nprintAll();\nint pri");
}

#[test]
fn a_single_word_goes_in_right_away_and_is_recorded() {
    let mut s = session("configuration\ncon");
    put(&mut s, 1, 3);
    s.start_recording("test");
    let f = run(&mut s, Command::Complete);
    assert!(f.completion.is_none());
    assert_eq!(text(&s), "configuration\nconfiguration");
    assert_eq!(caret(&s), (1, 13));
    let rec = s.stop_recording().unwrap();
    let cmds: Vec<_> = rec.steps.iter().map(|s| &s.cmd).collect();
    assert_eq!(cmds.len(), 2);
    assert!(matches!(cmds[0], Command::Select { .. }));
    assert_eq!(
        cmds[1],
        &Command::Insert {
            text: "configuration".into()
        }
    );
    // nothing to complete: a notice, no change
    let mut s = session("zzz qq");
    put(&mut s, 0, 3);
    let f = run(&mut s, Command::Complete);
    assert_eq!(f.notice.as_deref(), Some("no_suggestions"));
    assert_eq!(text(&s), "zzz qq");
}

#[test]
fn typing_filters_the_open_list() {
    let mut s = session("alpha alpine album\nal");
    put(&mut s, 1, 2);
    let f = run(&mut s, Command::Complete);
    assert_eq!(labels(&f).len(), 3);
    typ(&mut s, "p");
    let f = run(&mut s, Command::Frame);
    let l: Vec<_> = labels(&f).into_iter().map(|x| x.0).collect();
    assert_eq!(l, ["alpha", "alpine"]);
    // the list stays below the start of the word (design 023 E14)
    assert_eq!(f.completion.as_ref().unwrap().cell, 0);
    run(&mut s, Command::Backspace);
    assert_eq!(labels(&run(&mut s, Command::Frame)).len(), 3);
    // a move closes it, as does a character that is no word character
    mv(&mut s, Motion::Left);
    assert!(run(&mut s, Command::Frame).completion.is_none());
    put(&mut s, 1, 2);
    run(&mut s, Command::Complete);
    typ(&mut s, " ");
    assert!(run(&mut s, Command::Frame).completion.is_none());
    // without an open list typing opens nothing (auto is off by default)
    typ(&mut s, "al");
    assert!(run(&mut s, Command::Frame).completion.is_none());
}

#[test]
fn auto_mode_opens_from_two_characters() {
    let mut s = session("visible value\n");
    run(&mut s, words_config(true, true));
    put(&mut s, 1, 0);
    typ(&mut s, "v");
    assert!(run(&mut s, Command::Frame).completion.is_none());
    typ(&mut s, "i");
    let f = run(&mut s, Command::Frame);
    assert_eq!(labels(&f), [("visible".to_string(), "word".to_string())]);
    // a single candidate is never put in by itself while typing
    assert_eq!(text(&s), "visible value\nvi");
    // arithmetic still wins after "="
    typ(&mut s, " 2+3=");
    let f = run(&mut s, Command::Frame);
    assert_eq!(labels(&f)[0].1, "result");
    // arithmetic hangs below the caret
    assert_eq!(f.completion.unwrap().cell, 7);
}

#[test]
fn lexicon_of_the_language() {
    let mut s = session("class A {\n  pr\n}");
    run(
        &mut s,
        Command::SetLanguage {
            name: Some("Java".into()),
        },
    );
    put(&mut s, 1, 4);
    let f = run(&mut s, Command::Complete);
    let l = labels(&f);
    assert!(l.contains(&("private".to_string(), "keyword".to_string())));
    assert!(l.contains(&("println".to_string(), "keyword".to_string())));
    run(&mut s, Command::CloseCompletion);
    // switched off: nothing left
    run(&mut s, words_config(false, false));
    let f = run(&mut s, Command::Complete);
    assert!(f.completion.is_none());
    assert_eq!(f.notice.as_deref(), Some("no_suggestions"));
    // SQL: the spelling follows the prefix, a single keyword goes in right away
    let mut s = session("sel");
    run(
        &mut s,
        Command::SetLanguage {
            name: Some("SQL".into()),
        },
    );
    put(&mut s, 0, 3);
    run(&mut s, Command::Complete);
    assert_eq!(text(&s), "select");
    // CSS: words with "-"
    let mut s = session("a { background-co }");
    run(
        &mut s,
        Command::SetLanguage {
            name: Some("CSS".into()),
        },
    );
    put(&mut s, 0, 17);
    run(&mut s, Command::Complete);
    assert_eq!(text(&s), "a { background-color }");
}

#[test]
fn several_cursors_selections_and_long_lines_get_no_words() {
    let mut s = session("alpha\nal\nal");
    s.set_cursors(vec![
        Cursor::caret(Pos::new(1, 2)),
        Cursor::caret(Pos::new(2, 2)),
    ]);
    assert_eq!(
        run(&mut s, Command::Complete).notice.as_deref(),
        Some("no_suggestions")
    );
    let long = format!("alpha\n{} al", "x".repeat(2_000_000));
    let mut s = session(&long);
    put(&mut s, 1, 2_000_003);
    assert!(run(&mut s, Command::Complete).completion.is_none());
}

#[test]
fn words_in_a_big_file_cost_a_bounded_window() {
    // 200 000 lines; the word far away is not looked at, the near one is
    let mut t = String::from("faraway\n");
    for _ in 0..200_000 {
        t.push_str("lorem ipsum dolor sit amet\n");
    }
    t.push_str("nearby\nn");
    let mut s = session(&t);
    let last = s.doc.line_count() - 1;
    put(&mut s, last, 1);
    let start = std::time::Instant::now();
    run(&mut s, Command::Complete);
    assert!(start.elapsed().as_millis() < 2_000);
    assert!(s.doc.line_string(last).starts_with("nearby"));
    let far = format!("{}\nf", t.trim_end_matches("nearby\nn"));
    let mut s = session(&far);
    put(&mut s, last, 1);
    assert_eq!(
        run(&mut s, Command::Complete).notice.as_deref(),
        Some("no_suggestions")
    );
}

// ---------------------------------------------------------------- read-only (design 025 E1)

#[test]
fn read_only_refuses_edits_and_keeps_viewing() {
    let mut s = session("beta\nalpha\nbeta 255/8\n");
    let f = run(&mut s, Command::SetReadOnly { read_only: true });
    assert!(f.read_only);
    let q = || crate::search::Query {
        pattern: "beta".into(),
        regex: false,
        case_sensitive: false,
        whole_word: false,
    };
    let edits = vec![
        Command::Type { text: "x".into() },
        Command::Insert { text: "x".into() },
        Command::Newline,
        Command::Backspace,
        Command::Delete,
        Command::DeleteWordLeft,
        Command::Indent,
        Command::Undo,
        Command::Cut,
        Command::Paste,
        Command::Replace {
            query: q(),
            replacement: "x".into(),
        },
        Command::ReplaceAll {
            query: q(),
            replacement: "x".into(),
        },
        Command::SortLines {
            options: SortOptions {
                mode: SortMode::Natural,
                descending: false,
                ignore_case: false,
                unique: false,
            },
        },
        Command::ChangeCase {
            to: CaseChange::Upper,
        },
        Command::DuplicateLines,
        Command::DeleteLines,
        Command::JoinLines,
        Command::Complete,
        Command::ApplyCompletion { index: 0 },
        Command::Evaluate,
    ];
    let mut clip = MemoryClipboard(Some("pasted".into()));
    for c in edits {
        assert!(c.is_edit(), "{c:?}");
        let e = s.run(c.clone(), &mut clip).unwrap_err();
        assert_eq!(e.code, crate::error::FsErrorCode::ReadOnly, "{c:?}");
    }
    assert_eq!(text(&s), "beta\nalpha\nbeta 255/8\n");
    assert!(!s.doc.info(1).dirty);
    // viewing, selecting, copying, finding and markers still work
    let views = vec![
        Command::Move {
            to: Motion::Down,
            select: false,
        },
        Command::Click {
            line: 0.0,
            cell: 1.0,
            count: 2,
            extend: false,
            add: false,
        },
        Command::Copy,
        Command::Find {
            query: q(),
            backwards: false,
            incremental: false,
        },
        Command::ToggleMarker,
        Command::GoTo {
            line: 2,
            ch: 10,
            select: false,
        },
        Command::SelectAll,
        Command::ToggleOverwrite,
    ];
    for c in views {
        assert!(!c.is_edit(), "{c:?}");
        s.run(c.clone(), &mut clip)
            .unwrap_or_else(|e| panic!("{c:?}: {e:?}"));
    }
    assert_eq!(clip.0.as_deref(), Some("beta"));
    assert_eq!(text(&s), "beta\nalpha\nbeta 255/8\n");
    // off again: edits work
    let f = run(&mut s, Command::SetReadOnly { read_only: false });
    assert!(!f.read_only);
    put(&mut s, 0, 0);
    typ(&mut s, "x");
    assert!(text(&s).starts_with('x'));
}
