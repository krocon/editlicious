//! Snippets with placeholders (design 030): the usual syntax – `$1`, `${1:default}`,
//! `${1|one,two|}`, `$0` (where the caret ends), `$NAME` / `${NAME:default}` (variables) and
//! `\$`, `\}`, `\\` for the characters themselves. The same number in several places is one
//! placeholder typed together.

use super::track::{Bias, EditMap, remap_lines};
use super::{Cursor, Session};
use crate::document::model::{EditKind, Pos};
use crate::error::FsError;
use ropey::Rope;
use std::collections::{BTreeMap, HashMap};

/// One placeholder: every place it appears in the rendered text (character offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub number: u32,
    pub ranges: Vec<(usize, usize)>,
}

/// A template turned into text: the placeholders in visiting order (1, 2, … and `$0` last).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub text: String,
    pub stops: Vec<Stop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Text(String),
    Stop { number: u32, children: Vec<Node> },
    Var { name: String, children: Vec<Node> },
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl Parser<'_> {
    /// Nodes up to the end, or – inside `${…}` – up to the closing brace.
    fn nodes(&mut self, nested: bool) -> Vec<Node> {
        let mut out = Vec::new();
        let mut text = String::new();
        let flush = |text: &mut String, out: &mut Vec<Node>| {
            if !text.is_empty() {
                out.push(Node::Text(std::mem::take(text)));
            }
        };
        while let Some(&c) = self.chars.peek() {
            match c {
                '}' if nested => break,
                '\\' => {
                    self.chars.next();
                    match self.chars.peek() {
                        Some(&e @ ('$' | '}' | '\\')) => {
                            text.push(e);
                            self.chars.next();
                        }
                        // any other backslash is just a backslash
                        _ => text.push('\\'),
                    }
                }
                '$' => {
                    self.chars.next();
                    match self.dollar() {
                        Some(n) => {
                            flush(&mut text, &mut out);
                            out.push(n);
                        }
                        None => text.push('$'),
                    }
                }
                _ => {
                    text.push(c);
                    self.chars.next();
                }
            }
        }
        flush(&mut text, &mut out);
        out
    }

    fn word(&mut self, ok: impl Fn(char) -> bool) -> String {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if !ok(c) {
                break;
            }
            s.push(c);
            self.chars.next();
        }
        s
    }

    fn name_or_number(&mut self) -> Option<Result<u32, String>> {
        match self.chars.peek() {
            Some(c) if c.is_ascii_digit() => {
                let digits = self.word(|c| c.is_ascii_digit());
                Some(Ok(digits.parse().unwrap_or(u32::MAX)))
            }
            Some(&c) if c == '_' || c.is_ascii_alphabetic() => {
                Some(Err(self.word(|c| c == '_' || c.is_ascii_alphanumeric())))
            }
            _ => None,
        }
    }

    /// What follows a `$`; `None` = nothing of a snippet (the `$` stays text).
    fn dollar(&mut self) -> Option<Node> {
        if self.chars.peek() != Some(&'{') {
            return Some(match self.name_or_number()? {
                Ok(number) => Node::Stop {
                    number,
                    children: Vec::new(),
                },
                Err(name) => Node::Var {
                    name,
                    children: Vec::new(),
                },
            });
        }
        // `${` – only a snippet when a name or number and a proper end follow
        let saved = self.chars.clone();
        self.chars.next();
        let Some(id) = self.name_or_number() else {
            self.chars = saved;
            return None;
        };
        let children = match self.chars.peek() {
            Some('}') => Vec::new(),
            Some(':') => {
                self.chars.next();
                self.nodes(true)
            }
            Some('|') => {
                // a choice: the first option is the text
                self.chars.next();
                let mut first = String::new();
                let mut done = false;
                let mut closed = false;
                while let Some(c) = self.chars.next() {
                    match c {
                        '\\' => {
                            if let Some(e) = self.chars.next()
                                && !done
                            {
                                first.push(e);
                            }
                        }
                        ',' => done = true,
                        '|' => {
                            closed = true;
                            break;
                        }
                        c if !done => first.push(c),
                        _ => {}
                    }
                }
                if !closed {
                    self.chars = saved;
                    return None;
                }
                vec![Node::Text(first)]
            }
            _ => {
                self.chars = saved;
                return None;
            }
        };
        if self.chars.next() != Some('}') {
            self.chars = saved;
            return None;
        }
        Some(match id {
            Ok(number) => Node::Stop { number, children },
            Err(name) => Node::Var { name, children },
        })
    }
}

struct Renderer<'a> {
    vars: &'a dyn Fn(&str) -> Option<String>,
    text_of: &'a dyn Fn(&str) -> String,
    /// The text of each placeholder: its first appearance with a default.
    defaults: BTreeMap<u32, String>,
    text: String,
    len: usize,
    stops: BTreeMap<u32, Vec<(usize, usize)>>,
}

impl Renderer<'_> {
    fn push(&mut self, s: &str) {
        self.len += s.chars().count();
        self.text.push_str(s);
    }

    /// The plain text of nodes (for defaults), placeholders inside included.
    fn plain(&self, nodes: &[Node], out: &mut String) {
        for n in nodes {
            match n {
                Node::Text(t) => out.push_str(&(self.text_of)(t)),
                Node::Stop { number, children } => {
                    if children.is_empty() {
                        if let Some(d) = self.defaults.get(number) {
                            out.push_str(d);
                        }
                    } else {
                        self.plain(children, out);
                    }
                }
                Node::Var { name, children } => match (self.vars)(name) {
                    Some(v) => out.push_str(&v),
                    None => self.plain(children, out),
                },
            }
        }
    }

    fn collect_defaults(&mut self, nodes: &[Node]) {
        for n in nodes {
            match n {
                Node::Stop { number, children } if !children.is_empty() => {
                    self.collect_defaults(children);
                    if !self.defaults.contains_key(number) {
                        let mut d = String::new();
                        self.plain(children, &mut d);
                        self.defaults.insert(*number, d);
                    }
                }
                Node::Var { children, .. } => self.collect_defaults(children),
                _ => {}
            }
        }
    }

    fn render(&mut self, nodes: &[Node]) {
        for n in nodes {
            match n {
                Node::Text(t) => {
                    let t = (self.text_of)(t);
                    self.push(&t);
                }
                Node::Stop { number, children } => {
                    let from = self.len;
                    if children.is_empty() {
                        if let Some(d) = self.defaults.get(number).cloned() {
                            self.push(&d);
                        }
                    } else {
                        self.render(children);
                    }
                    self.stops
                        .entry(*number)
                        .or_default()
                        .push((from, self.len));
                }
                Node::Var { name, children } => match (self.vars)(name) {
                    Some(v) => self.push(&v),
                    None => self.render(children),
                },
            }
        }
    }
}

/// Renders a template. `vars(name)` gives the value of a variable (`None`: its default, else
/// nothing); `text_of` turns literal template text into document text (indentation, line breaks).
pub fn render(
    template: &str,
    vars: &dyn Fn(&str) -> Option<String>,
    text_of: &dyn Fn(&str) -> String,
) -> Rendered {
    let nodes = Parser {
        chars: template.chars().peekable(),
    }
    .nodes(false);
    let mut r = Renderer {
        vars,
        text_of,
        defaults: BTreeMap::new(),
        text: String::new(),
        len: 0,
        stops: BTreeMap::new(),
    };
    r.collect_defaults(&nodes);
    r.render(&nodes);
    let end = r.len;
    let mut stops: Vec<Stop> = r
        .stops
        .into_iter()
        .map(|(number, ranges)| Stop { number, ranges })
        .collect();
    // `$0` is visited last; without one the snippet ends behind its text
    match stops.first().map(|s| s.number) {
        Some(0) => stops.rotate_left(1),
        _ => stops.push(Stop {
            number: 0,
            ranges: vec![(end, end)],
        }),
    }
    Rendered {
        text: r.text,
        stops,
    }
}

/// The placeholders of the snippet being filled in (design 030 E3): absolute character offsets
/// that follow the text, and which one is selected.
#[derive(Debug, Clone)]
pub(super) struct SnippetState {
    stops: Vec<Stop>,
    at: usize,
    /// Text state before the snippet went in – undoing back to it ends the mode.
    state_before: u64,
}

impl SnippetState {
    /// Follows a batch of changes: the selected placeholder (and what contains it) grows with
    /// text typed at its edges, the others are pushed along.
    pub(super) fn follow(&mut self, map: &EditMap) {
        let current = self.stops[self.at].ranges.clone();
        for (i, stop) in self.stops.iter_mut().enumerate() {
            for r in &mut stop.ranges {
                let grows = i == self.at || current.iter().any(|c| r.0 <= c.0 && c.1 <= r.1);
                *r = if grows {
                    (map.map(r.0, Bias::Left), map.map(r.1, Bias::Right))
                } else {
                    let start = map.map(r.0, Bias::Right);
                    (start, map.map(r.1, Bias::Left).max(start))
                };
            }
        }
    }

    pub(super) fn rewrite(&mut self, old: &Rope, new: &Rope) {
        for r in self.stops.iter_mut().flat_map(|s| s.ranges.iter_mut()) {
            *r = (remap_lines(old, new, r.0), remap_lines(old, new, r.1));
        }
    }
}

impl Session {
    /// Inserts a snippet (design 030) in place of `range` (default: the primary selection) as
    /// one undo step and selects its first placeholder. Line breaks of the template take the
    /// indentation of the line, tabs the indentation unit. `vars` are the page's variables;
    /// `SELECTION` is the text the primary cursor had selected.
    pub(super) fn insert_snippet(
        &mut self,
        template: &str,
        range: Option<(Pos, Pos)>,
        vars: &HashMap<String, String>,
    ) -> Result<(), FsError> {
        let c = self.primary();
        let (from, to) = match range {
            Some((a, b)) => {
                let (a, b) = (self.doc.clamp(a), self.doc.clamp(b));
                (a.min(b), a.max(b))
            }
            None => (c.start(), c.end()),
        };
        let selected = if c.is_empty() {
            String::new()
        } else {
            self.doc.text(c.start(), c.end())?
        };
        let le = self.doc.line_ending.as_str().to_owned();
        let indent =
            self.doc
                .line_slice(from.line, 0, self.first_non_blank(from.line).min(from.ch));
        let unit = self.indent_unit();
        // literal template text: the document's line break plus the line's indentation, tabs
        // as the indentation unit; values of variables only get the document's line breaks
        let breaks = |t: &str, after: &str, tab: &str| {
            let mut out = String::with_capacity(t.len());
            let mut chars = t.chars().peekable();
            while let Some(ch) = chars.next() {
                match ch {
                    '\r' | '\n' => {
                        if ch == '\r' && chars.peek() == Some(&'\n') {
                            chars.next();
                        }
                        out.push_str(&le);
                        out.push_str(after);
                    }
                    '\t' => out.push_str(tab),
                    c => out.push(c),
                }
            }
            out
        };
        let text_of = |t: &str| breaks(t, &indent, &unit);
        let var = |name: &str| {
            vars.get(name)
                .cloned()
                .or_else(|| {
                    matches!(name, "SELECTION" | "TM_SELECTED_TEXT").then(|| selected.clone())
                })
                .map(|v| breaks(&v, "", "\t"))
        };
        let r = render(template, &var, &text_of);
        let a = self.doc.pos_to_char(from);
        let b = self.doc.pos_to_char(to);
        let state_before = self.doc.state();
        self.snippet = None;
        self.doc
            .apply_chars(vec![(a, b, r.text)], self.sels(), None, EditKind::Other)?;
        // what else follows the text (markers) sees the insertion before the placeholders exist
        self.sync_tracked();
        let mut stops = r.stops;
        for range in stops.iter_mut().flat_map(|s| s.ranges.iter_mut()) {
            *range = (range.0 + a, range.1 + a);
        }
        self.snippet = Some(SnippetState {
            stops,
            at: 0,
            state_before,
        });
        self.select_stop();
        Ok(())
    }

    /// Selects every place of the current placeholder; the last one (`$0`) ends the mode.
    fn select_stop(&mut self) {
        let Some(sn) = &self.snippet else { return };
        let last = sn.at + 1 >= sn.stops.len();
        self.cursors = sn.stops[sn.at]
            .ranges
            .iter()
            .map(|&(s, e)| Cursor {
                anchor: self.doc.char_to_pos(s),
                head: self.doc.char_to_pos(e),
                goal: None,
            })
            .collect();
        self.primary = 0;
        self.normalize();
        if last {
            self.snippet = None;
        }
        self.track();
    }

    /// Tab / Shift+Tab while a snippet is filled in: the next or previous placeholder.
    /// `false` = no snippet (the key does what it always does).
    pub(super) fn snippet_step(&mut self, back: bool) -> bool {
        let Some(sn) = self.snippet.as_mut() else {
            return false;
        };
        if back {
            sn.at = sn.at.saturating_sub(1);
        } else {
            sn.at += 1;
        }
        self.select_stop();
        true
    }

    /// Ends the mode when the caret left the placeholder or the snippet was undone.
    pub(super) fn snippet_check(&mut self) {
        let Some(sn) = &self.snippet else { return };
        let c = self.primary();
        let (s, e) = (
            self.doc.pos_to_char(c.start()),
            self.doc.pos_to_char(c.end()),
        );
        let inside = sn.stops[sn.at].ranges.iter().any(|r| r.0 <= s && e <= r.1);
        if !inside || self.doc.state() == sn.state_before {
            self.snippet = None;
            self.track();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(template: &str) -> Rendered {
        render(template, &|_| None, &|t| t.to_owned())
    }

    fn stop(number: u32, ranges: &[(usize, usize)]) -> Stop {
        Stop {
            number,
            ranges: ranges.to_vec(),
        }
    }

    #[test]
    fn placeholders_defaults_and_the_final_position() {
        let r = plain("SELECT ${1:*} FROM ${2:table} WHERE ${3:1=1};$0");
        assert_eq!(r.text, "SELECT * FROM table WHERE 1=1;");
        assert_eq!(
            r.stops,
            vec![
                stop(1, &[(7, 8)]),
                stop(2, &[(14, 19)]),
                stop(3, &[(26, 29)]),
                stop(0, &[(30, 30)]),
            ]
        );
        // no `$0`: the end of the text
        let r = plain("a $1 b");
        assert_eq!(r.text, "a  b");
        assert_eq!(r.stops, vec![stop(1, &[(2, 2)]), stop(0, &[(4, 4)])]);
        // `$0` anywhere is still visited last
        let r = plain("$0x$2y$1");
        assert_eq!(
            r.stops.iter().map(|s| s.number).collect::<Vec<_>>(),
            [1, 2, 0]
        );
    }

    #[test]
    fn the_same_number_is_one_placeholder_in_several_places() {
        let r = plain("for (${1:i} = 0; $1 < n; ${1}++)");
        assert_eq!(r.text, "for (i = 0; i < n; i++)");
        assert_eq!(r.stops[0], stop(1, &[(5, 6), (12, 13), (19, 20)]));
        // the default may come later than the first appearance
        let r = plain("$1-${1:x}");
        assert_eq!(r.text, "x-x");
    }

    #[test]
    fn literal_dollars_braces_and_backslashes() {
        let r = plain(r"cost: \$5 \} \\ $ ${ ${x $1}");
        assert_eq!(r.text, r"cost: $5 } \ $ ${ ${x }");
        assert_eq!(r.stops[0].number, 1);
        // inside a default the closing brace is escaped, a lone backslash stays
        let r = plain(r"${1:a\}b} c:\d \$1");
        assert_eq!(r.text, r"a}b c:\d $1");
        assert_eq!(r.stops[0], stop(1, &[(0, 3)]));
        // an unclosed placeholder is text
        assert_eq!(plain("${1:abc").text, "${1:abc");
        assert_eq!(plain("100$").text, "100$");
        assert_eq!(plain("a$%b").text, "a$%b");
    }

    #[test]
    fn nested_placeholders_and_choices() {
        let r = plain("${1:foo ${2:bar}} ${3|one,two|}");
        assert_eq!(r.text, "foo bar one");
        assert_eq!(
            r.stops,
            vec![
                stop(1, &[(0, 7)]),
                stop(2, &[(4, 7)]),
                stop(3, &[(8, 11)]),
                stop(0, &[(11, 11)]),
            ]
        );
    }

    #[test]
    fn variables_take_values_defaults_or_nothing() {
        let vars = |n: &str| match n {
            "TABLE" => Some("orders".to_owned()),
            "SELECTION" => Some("a\nb".to_owned()),
            _ => None,
        };
        let r = render(
            "FROM $TABLE ${WHO:nobody} ${NOPE} [$SELECTION] ${1:$TABLE}",
            &vars,
            &|t| t.to_owned(),
        );
        assert_eq!(r.text, "FROM orders nobody  [a\nb] orders");
        assert_eq!(r.stops[0], stop(1, &[(26, 32)]));
    }

    #[test]
    fn template_text_goes_through_the_document_filter() {
        // offsets count the characters after the filter (indentation, CRLF), emoji as one
        let r = render("a\n\t${1:👍}\n$0", &|_| None, &|t| {
            t.replace('\n', "\r\n  ").replace('\t', "    ")
        });
        assert_eq!(r.text, "a\r\n      👍\r\n  ");
        assert_eq!(r.stops, vec![stop(1, &[(9, 10)]), stop(0, &[(14, 14)])]);
    }
}
