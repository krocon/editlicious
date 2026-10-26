//! Word classes for word moves and double clicks, and the indentation a file uses.

use crate::document::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    Space,
    Word,
    Punct,
    Break,
}

impl CharClass {
    pub fn of(c: char) -> Self {
        match c {
            '\n' | '\r' => CharClass::Break,
            c if c.is_whitespace() => CharClass::Space,
            c if c.is_alphanumeric() || c == '_' => CharClass::Word,
            // combining marks belong to the word they sit on
            c if unicode_width::UnicodeWidthChar::width(c) == Some(0) => CharClass::Word,
            _ => CharClass::Punct,
        }
    }
}

/// Lines looked at to guess the indentation.
const INDENT_SAMPLE: usize = 2_000;

/// `(insert_spaces, tab_width)` the file seems to use; new files: 4 spaces.
pub fn detect_indent(doc: &Document) -> (bool, usize) {
    let (mut tabs, mut spaces, mut two) = (0usize, 0usize, 0usize);
    let n = doc.line_count().min(INDENT_SAMPLE);
    for line in 0..n {
        let head = doc.line_slice(line, 0, 16);
        if head.starts_with('\t') {
            tabs += 1;
        } else if head.starts_with(' ') {
            let k = head.chars().take_while(|&c| c == ' ').count();
            if head.chars().nth(k).is_some_and(|c| !c.is_whitespace()) {
                spaces += 1;
                if k % 4 == 2 {
                    two += 1;
                }
            }
        }
    }
    if tabs > spaces {
        (false, 4)
    } else if spaces > 0 && two * 5 >= spaces {
        (true, 2)
    } else {
        (true, 4)
    }
}
