//! Arithmetic at the caret (prompt 020): which expression the caret is at – the selection,
//! the expression ending at the caret (with or without `=`), or the whole line – and what to
//! suggest for it. Careful about text that only looks like arithmetic (dates, ranges, versions,
//! numbers inside names).

use crate::math;
use std::collections::HashMap;

/// Characters an expression can consist of.
fn expr_char(c: char) -> bool {
    c.is_alphanumeric()
        || matches!(
            c,
            ' ' | '\t'
                | '.'
                | ','
                | '_'
                | '+'
                | '-'
                | '*'
                | '/'
                | '^'
                | '%'
                | '('
                | ')'
                | '!'
                | '×'
                | '÷'
                | '−'
                | '·'
        )
}

/// Characters an expression can start with.
fn start_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '(' | '-' | '+' | '.' | '−')
}

/// Longest stretch looked at to the left of the caret.
const WINDOW: usize = 200;

/// Settings of the arithmetic (prompt 020 E6).
#[derive(Debug, Clone, PartialEq)]
pub struct MathOptions {
    pub hints: bool,
    pub suggest: bool,
    /// Fixed decimal places of results, `None` = as needed.
    pub decimals: Option<usize>,
    pub comma: bool,
}

impl Default for MathOptions {
    fn default() -> Self {
        Self {
            hints: true,
            suggest: true,
            decimals: None,
            comma: false,
        }
    }
}

/// An expression found in a line (characters `from..to`).
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub from: usize,
    pub to: usize,
    pub value: f64,
    /// The expression written tidily.
    pub shown: String,
    /// It ends with `=` at `to` (then the caret is behind it).
    pub eq: bool,
}

/// Looks like a date, range or number list rather than arithmetic: only digits and one kind of
/// `-` or `/` without spaces (`2026-10-26`, `10-12`, `10/26/2026`).
fn looks_like_data(text: &str) -> bool {
    let t = text.trim();
    let only = |sep: char| t.chars().all(|c| c.is_ascii_digit() || c == sep) && t.contains(sep);
    only('-') || (only('/') && t.matches('/').count() >= 2)
}

fn try_expr(text: &str, vars: &HashMap<String, f64>) -> Option<(f64, String)> {
    if looks_like_data(text) {
        return None;
    }
    let p = math::parse(text, vars)?;
    if !p.arithmetic {
        return None;
    }
    Some((math::eval(&p.expr, vars), math::show(&p.expr)))
}

/// The expression at the caret: the selection `sel` (characters), else the one ending at the
/// caret (a trailing `=` allowed), else the whole line.
pub fn find(
    line: &str,
    caret: usize,
    sel: Option<(usize, usize)>,
    vars: &HashMap<String, f64>,
) -> Option<Found> {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() > math::MAX_LEN * 4 {
        return None;
    }
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    if let Some((a, b)) = sel.filter(|(a, b)| a < b && *b <= chars.len()) {
        let t = text(a, b);
        let t = t.trim_end().trim_end_matches('=');
        let (value, shown) = try_expr(t, vars)?;
        return Some(Found {
            from: a,
            to: b,
            value,
            shown,
            eq: false,
        });
    }
    let caret = caret.min(chars.len());
    // ending at the caret, maybe with "=" (and spaces) right before it
    let mut end = caret;
    while end > 0 && chars[end - 1] == ' ' {
        end -= 1;
    }
    let eq = end > 0
        && chars[end - 1] == '='
        && !(end > 1 && matches!(chars[end - 2], '=' | '!' | '<' | '>'));
    let expr_end = if eq { end - 1 } else { caret };
    let mut left = expr_end;
    while left > 0 && expr_end - left < WINDOW && expr_char(chars[left - 1]) {
        left -= 1;
    }
    for start in left..expr_end {
        // never in the middle of a name, number or expression (`26/2026` of a date)
        if !start_char(chars[start])
            || (start > 0
                && (chars[start - 1].is_alphanumeric()
                    || matches!(
                        chars[start - 1],
                        '_' | '.' | '/' | '-' | '+' | '*' | '^' | '%' | '×' | '÷' | '−' | '·' | ','
                    )))
        {
            continue;
        }
        let t = text(start, expr_end);
        if t.trim().is_empty() {
            continue;
        }
        if let Some((value, shown)) = try_expr(&t, vars) {
            let lead = t.chars().take_while(|c| c.is_whitespace()).count();
            let trail = t.chars().rev().take_while(|c| c.is_whitespace()).count();
            return Some(Found {
                from: start + lead,
                to: if eq { end } else { expr_end - trail },
                value,
                shown,
                eq,
            });
        }
    }
    // the whole line (the caret somewhere inside)
    let t = line.trim();
    if !t.is_empty() && !eq {
        let lead = chars.iter().take_while(|c| c.is_whitespace()).count();
        let body = t.trim_end_matches('=').trim_end();
        if let Some((value, shown)) = try_expr(body, vars) {
            return Some(Found {
                from: lead,
                to: lead + t.chars().count(),
                value,
                shown,
                eq: t.ends_with('='),
            });
        }
    }
    None
}

/// Variables of `name = expression` lines (top to bottom).
pub fn vars_of<'a>(lines: impl Iterator<Item = &'a str>) -> HashMap<String, f64> {
    let mut vars = HashMap::new();
    for l in lines {
        let l = l.trim().trim_end_matches(';');
        let Some((name, rhs)) = l.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let ok = name
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            && name.len() <= 32
            && !rhs.starts_with('=');
        if !ok {
            continue;
        }
        if let Some(p) = math::parse(rhs.trim(), &vars) {
            let v = math::eval(&p.expr, &vars);
            if v.is_finite() {
                vars.insert(name.to_owned(), v);
            }
        }
    }
    vars
}

/// The result as text (settings: decimals, comma).
pub fn result_text(v: f64, o: &MathOptions) -> String {
    math::format_number(v, o.decimals, o.comma)
}

/// `0x1f` for whole numbers from 16 on.
pub fn hex(v: f64) -> Option<String> {
    (v.fract() == 0.0 && v.abs() >= 16.0 && v.abs() < 9.007_199_254_740_992e15).then(|| {
        if v < 0.0 {
            format!("-0x{:x}", (-v) as u64)
        } else {
            format!("0x{:x}", v as u64)
        }
    })
}

/// One suggestion: what to show and the change it makes in the line (characters).
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    /// `result`, `replace`, `rounded`, `hex` – the UI names them.
    pub kind: &'static str,
    pub label: String,
    pub from: usize,
    pub to: usize,
    pub text: String,
}

/// What can be done with a found expression; `caret` = where the caret is.
pub fn suggestions(f: &Found, caret: usize, o: &MathOptions) -> Vec<Suggestion> {
    if !f.value.is_finite() {
        return Vec::new();
    }
    let r = result_text(f.value, o);
    let mut out = Vec::new();
    if f.eq {
        // "255/8 =" → the result after the "="
        out.push(Suggestion {
            kind: "result",
            label: r.clone(),
            from: caret,
            to: caret,
            text: r.clone(),
        });
    } else {
        out.push(Suggestion {
            kind: "result",
            label: format!("= {r}"),
            from: f.to,
            to: f.to,
            text: format!(" = {r}"),
        });
    }
    out.push(Suggestion {
        kind: "replace",
        label: r.clone(),
        from: f.from,
        to: if f.eq { caret.max(f.to) } else { f.to },
        text: r.clone(),
    });
    if f.value.fract() != 0.0 && o.decimals.is_none() {
        let rounded = math::format_number(f.value, Some(2), o.comma);
        if rounded != r {
            out.push(Suggestion {
                kind: "rounded",
                label: rounded.clone(),
                from: if f.eq { caret } else { f.to },
                to: if f.eq { caret } else { f.to },
                text: if f.eq {
                    rounded
                } else {
                    format!(" = {rounded}")
                },
            });
        }
    }
    if let Some(h) = hex(f.value) {
        out.push(Suggestion {
            kind: "hex",
            label: h.clone(),
            from: if f.eq { caret } else { f.to },
            to: if f.eq { caret } else { f.to },
            text: if f.eq { h } else { format!(" = {h}") },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(marked: &str) -> Option<(String, String, bool)> {
        let caret = marked.chars().position(|c| c == '|').unwrap();
        let line: String = marked.chars().filter(|&c| c != '|').collect();
        let f = find(&line, caret, None, &HashMap::new())?;
        let chars: Vec<char> = line.chars().collect();
        let r = math::format_number(f.value, None, false);
        Some((
            chars[f.from..f.to].iter().collect(),
            format!("{} = {r}", f.shown),
            f.eq,
        ))
    }

    #[test]
    fn like_the_picture_of_the_old_version() {
        assert_eq!(
            at("255/8|"),
            Some(("255/8".into(), "255 / 8 = 31.875".into(), false))
        );
        assert_eq!(
            at("  x: (1+2)*3 =|"),
            Some(("(1+2)*3 =".into(), "(1 + 2) * 3 = 9".into(), true))
        );
        assert_eq!(
            at("total 2^3^2= |").map(|x| x.1),
            Some("2 ^ 3 ^ 2 = 512".into())
        );
        // the caret inside: the whole line
        assert_eq!(at("25|5/8").map(|x| x.0), Some("255/8".into()));
        assert_eq!(at("price: 3 * 4.5|").map(|x| x.0), Some("3 * 4.5".into()));
    }

    #[test]
    fn no_false_hints() {
        for s in [
            "2026-10-26|",
            "pages 10-12|",
            "10/26/2026|",
            "version 1.2.3|",
            "ip 192.168.0.1|",
            "call +49 6174 1234|",
            "i++|",
            "---|",
            "- 1|",
            "abc123+4|",
            "a = b + c|",
            "x == 3|",
            "#2f98e1|",
            "foo(1)|",
            "12|",
            "for (int i = 0; i < 10; i++) {|",
            "see chapter 3|",
            // CSS, Java, Markdown while typing (acceptance of prompt 020)
            "  width: calc(100% - 20px);|",
            "  margin: 0 auto;|",
            "  font-size: 1.5em|",
            "  line-height: 1.5;|",
            "  return a * 2;|",
            "  int n = list.size() - 1;|",
            "  x -= 1;|",
            "* item|",
            "1. first|",
            "**bold**|",
            "| a | b |",
            "## 2.1 Intro|",
        ] {
            assert_eq!(at(s), None, "{s}");
        }
    }

    #[test]
    fn variables_from_lines_above() {
        let vars = vars_of(["width = 1920", "height = width / 16 * 9;", "x == 4"].into_iter());
        assert_eq!(vars.get("height"), Some(&1080.0));
        assert!(!vars.contains_key("x"));
        let f = find("height * 2 =", 12, None, &vars).unwrap();
        assert_eq!(f.value, 2160.0);
    }

    #[test]
    fn selection_and_suggestions() {
        let f = find("a 1+1 b", 0, Some((2, 5)), &HashMap::new()).unwrap();
        assert_eq!((f.from, f.to, f.value), (2, 5, 2.0));
        let o = MathOptions::default();
        let eq = find("255/8 =", 7, None, &HashMap::new()).unwrap();
        let s = suggestions(&eq, 7, &o);
        assert_eq!(
            s.iter().map(|x| x.kind).collect::<Vec<_>>(),
            ["result", "replace", "rounded"]
        );
        assert_eq!((s[0].from, s[0].to, s[0].text.as_str()), (7, 7, "31.875"));
        assert_eq!((s[1].from, s[1].to, s[1].text.as_str()), (0, 7, "31.875"));
        assert_eq!(s[2].text, "31.88");
        let plain = find("200+55", 6, None, &HashMap::new()).unwrap();
        let s = suggestions(&plain, 6, &o);
        assert_eq!(s[0].text, " = 255");
        assert_eq!(s.last().unwrap().text, " = 0xff");
        // division by zero: no suggestion
        let z = find("1/0", 3, None, &HashMap::new()).unwrap();
        assert!(z.value.is_nan());
        assert!(suggestions(&z, 3, &o).is_empty());
        // settings: comma and fixed places
        let c = MathOptions {
            decimals: Some(1),
            comma: true,
            ..Default::default()
        };
        assert_eq!(suggestions(&eq, 7, &c)[0].text, "31,9");
    }
}
