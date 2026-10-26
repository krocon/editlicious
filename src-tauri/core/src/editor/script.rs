//! Typing scripts `*.edscript` (design 015 E5): a small line language to choreograph what the
//! editor types – for videos in several languages with the same timing.
//!
//! ```text
//! # comment
//! tempo human cpm=320 seed=42 typos=0.02
//! type "fn main() {\n    println!(\"hi\");\n}"
//! type <<END
//! more text
//! END
//! type @snippets/example.rs
//! pause 800
//! goto 12:5
//! select 12:5 12:9
//! move wordRight select
//! backspace 3
//! find "needle" case word
//! do {"type":"duplicateLines"}
//! ```

use super::model::{Command, Motion};
use super::playback::{Item, Tempo};
use super::recording::{Recording, invalid};
use crate::document::model::{Pos, Sel};
use crate::error::FsError;
use crate::search::Query;

/// Longest script text (all `type` texts and files together).
pub const MAX_SCRIPT_CHARS: usize = 4 << 20;

/// Commands without arguments that may carry a repeat count (`backspace 3`).
const SIMPLE: &[&str] = &[
    "newline",
    "backspace",
    "delete",
    "deleteWordLeft",
    "deleteWordRight",
    "undo",
    "redo",
    "indent",
    "outdent",
    "selectAll",
    "copy",
    "cut",
    "paste",
    "duplicateLines",
    "deleteLines",
    "joinLines",
    "toggleMarker",
    "clearMarkers",
    "singleCursor",
    "addNextOccurrence",
];

/// Parses a script. `read` loads `type @file` texts (relative to the script).
pub fn parse(
    text: &str,
    read: &mut dyn FnMut(&str) -> Result<String, FsError>,
) -> Result<Vec<Item>, FsError> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut total = 0usize;
    let mut i = 0;
    while i < lines.len() {
        let no = i + 1;
        let line = lines[i].trim();
        i += 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (word, rest) = split_word(line);
        let item = match word {
            "tempo" => Item::Tempo(parse_tempo(rest).map_err(|e| invalid(no, &e))?),
            "type" => {
                let rest = rest.trim();
                let text = if let Some(tag) = rest.strip_prefix("<<") {
                    let tag = tag.trim();
                    if tag.is_empty() {
                        return Err(invalid(no, "missing end tag after <<"));
                    }
                    let mut body = Vec::new();
                    loop {
                        let Some(l) = lines.get(i) else {
                            return Err(invalid(no, &format!("no line {tag} closes the text")));
                        };
                        i += 1;
                        if l.trim() == tag {
                            break;
                        }
                        body.push(*l);
                    }
                    body.join("\n")
                } else if let Some(file) = rest.strip_prefix('@') {
                    read(file.trim()).map_err(|e| invalid(no, &e.message))?
                } else {
                    let (s, tail) = parse_string(rest).map_err(|e| invalid(no, &e))?;
                    if !tail.trim().is_empty() {
                        return Err(invalid(no, "text after the closing quote"));
                    }
                    s
                };
                total += text.chars().count();
                if total > MAX_SCRIPT_CHARS {
                    return Err(invalid(no, "script too long"));
                }
                Item::Type(text.replace("\r\n", "\n").replace('\r', "\n"))
            }
            "pause" => Item::Pause(parse_num(rest.trim()).map_err(|e| invalid(no, &e))?),
            "goto" => {
                let p = parse_pos(rest.trim()).map_err(|e| invalid(no, &e))?;
                cmd(Command::GoTo {
                    line: p.line,
                    ch: p.ch,
                    select: false,
                })
            }
            "select" => {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() != 2 {
                    return Err(invalid(no, "select needs two positions: select 3:1 3:9"));
                }
                let a = parse_pos(parts[0]).map_err(|e| invalid(no, &e))?;
                let b = parse_pos(parts[1]).map_err(|e| invalid(no, &e))?;
                cmd(Command::Select {
                    sels: vec![Sel { anchor: a, head: b }],
                })
            }
            "move" => {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                let Some(name) = parts.first() else {
                    return Err(invalid(no, "move needs a direction"));
                };
                let to: Motion = serde_json::from_value(serde_json::Value::String((*name).into()))
                    .map_err(|_| invalid(no, &format!("unknown direction {name}")))?;
                let select = parts.get(1) == Some(&"select");
                let n = parts
                    .iter()
                    .skip(1)
                    .find(|p| p.chars().all(|c| c.is_ascii_digit()))
                    .map_or(Ok(1), |p| parse_num(p))
                    .map_err(|e| invalid(no, &e))?;
                for _ in 1..n {
                    out.push(cmd(Command::Move { to, select }));
                }
                cmd(Command::Move { to, select })
            }
            "find" => {
                let (pattern, tail) = parse_string(rest.trim()).map_err(|e| invalid(no, &e))?;
                let flags: Vec<&str> = tail.split_whitespace().collect();
                for f in &flags {
                    if !["regex", "case", "word", "back"].contains(f) {
                        return Err(invalid(no, &format!("unknown option {f}")));
                    }
                }
                cmd(Command::Find {
                    query: Query {
                        pattern,
                        regex: flags.contains(&"regex"),
                        case_sensitive: flags.contains(&"case"),
                        whole_word: flags.contains(&"word"),
                    },
                    backwards: flags.contains(&"back"),
                    incremental: false,
                })
            }
            "do" => {
                let c: Command = serde_json::from_str(rest.trim())
                    .map_err(|e| invalid(no, &format!("not an editor command: {e}")))?;
                cmd(c)
            }
            w if SIMPLE.contains(&w) => {
                let n = match rest.trim() {
                    "" => 1,
                    r => parse_num(r).map_err(|e| invalid(no, &e))?,
                };
                if n > 10_000 {
                    return Err(invalid(no, "count too large"));
                }
                let c: Command = serde_json::from_value(serde_json::json!({ "type": w }))
                    .map_err(|e| invalid(no, &e.to_string()))?;
                for _ in 1..n {
                    out.push(cmd(c.clone()));
                }
                cmd(c)
            }
            w => return Err(invalid(no, &format!("unknown command {w}"))),
        };
        out.push(item);
    }
    Ok(out)
}

fn cmd(c: Command) -> Item {
    Item::Cmd {
        cmd: c,
        clip: None,
        gap: None,
    }
}

fn split_word(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], &line[i..]),
        None => (line, ""),
    }
}

fn parse_num(s: &str) -> Result<u64, String> {
    s.parse::<u64>().map_err(|_| format!("not a number: {s}"))
}

/// `line:column`, both 1-based → 0-based position.
fn parse_pos(s: &str) -> Result<Pos, String> {
    let (l, c) = s.split_once(':').unwrap_or((s, "1"));
    let l = parse_num(l)?;
    let c = parse_num(c)?;
    if l == 0 || c == 0 {
        return Err("lines and columns start at 1".into());
    }
    Ok(Pos::new(l as usize - 1, c as usize - 1))
}

/// A quoted string with `\n \t \r \" \\ \u{…}` escapes; returns the rest after it.
fn parse_string(s: &str) -> Result<(String, &str), String> {
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, '"')) => {}
        _ => return Err("expected a text in quotes".into()),
    }
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((out, &s[i + 1..])),
            '\\' => match chars.next().map(|(_, c)| c) {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('u') => {
                    let rest: String = chars
                        .by_ref()
                        .map(|(_, c)| c)
                        .take_while(|&c| c != '}')
                        .collect();
                    let hex = rest.strip_prefix('{').ok_or("\\u needs {hex}")?;
                    let ch = u32::from_str_radix(hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or("bad \\u{…} escape")?;
                    out.push(ch);
                }
                other => return Err(format!("unknown escape \\{}", other.unwrap_or(' '))),
            },
            c => out.push(c),
        }
    }
    Err("the closing quote is missing".into())
}

fn parse_tempo(rest: &str) -> Result<Tempo, String> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let num = |s: &str| s.parse::<f64>().map_err(|_| format!("not a number: {s}"));
    match parts.first().copied() {
        Some("original") => Ok(Tempo::Original),
        Some("instant") => Ok(Tempo::Instant),
        Some("factor") => {
            let f = num(parts.get(1).ok_or("factor needs a value")?)?;
            Ok(Tempo::Factor {
                factor: f.clamp(0.25, 8.0),
            })
        }
        Some("even") => {
            let ms = parse_num(
                parts
                    .get(1)
                    .ok_or("even needs milliseconds")?
                    .trim_end_matches("ms"),
            )?;
            Ok(Tempo::Even { ms: ms.min(60_000) })
        }
        Some("human") => {
            let mut t = Tempo::human();
            if let Tempo::Human { cpm, seed, typos } = &mut t {
                for p in &parts[1..] {
                    let (k, v) = p
                        .split_once('=')
                        .ok_or(format!("expected key=value: {p}"))?;
                    match k {
                        "cpm" => *cpm = num(v)?.clamp(30.0, 3000.0),
                        "seed" => *seed = parse_num(v)?,
                        "typos" => *typos = num(v)?.clamp(0.0, 0.3),
                        _ => return Err(format!("unknown option {k}")),
                    }
                }
            }
            Ok(t)
        }
        Some(o) => Err(format!("unknown tempo {o}")),
        None => Err("tempo needs a mode".into()),
    }
}

/// Writes a recording as a script (design 015 E12): typing in a row becomes `type "…"`,
/// longer gaps `pause`, the rest editor commands.
pub fn from_recording(rec: &Recording) -> String {
    let mut out = String::from("# editlicious script – from a recording\ntempo original\n");
    let mut typed = String::new();
    let mut last_t = 0u64;
    let flush = |typed: &mut String, out: &mut String| {
        if !typed.is_empty() {
            out.push_str(&format!("type {}\n", quote(typed)));
            typed.clear();
        }
    };
    if let Some(s) = rec.header.start.last() {
        out.push_str(&format!("select {} {}\n", pos(s.anchor), pos(s.head)));
    }
    for s in &rec.steps {
        let gap = s.t.saturating_sub(last_t);
        last_t = s.t;
        if gap > 400 {
            flush(&mut typed, &mut out);
            out.push_str(&format!("pause {gap}\n"));
        }
        match &s.cmd {
            Command::Type { text } if s.clip.is_none() => {
                typed.push_str(text);
                continue;
            }
            _ => flush(&mut typed, &mut out),
        }
        let line = match &s.cmd {
            Command::Select { sels } if sels.len() == 1 => {
                format!("select {} {}", pos(sels[0].anchor), pos(sels[0].head))
            }
            Command::GoTo {
                line,
                ch,
                select: false,
            } => format!("goto {}:{}", line + 1, ch + 1),
            Command::Move { to, select } => {
                let name = serde_json::to_value(to)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                format!("move {name}{}", if *select { " select" } else { "" })
            }
            Command::Paste => match &s.clip {
                // the text a paste brought: typed in at once
                Some(c) => format!("do {}", serde_json::json!({"type": "insert", "text": c})),
                None => "paste".into(),
            },
            c => {
                let v = serde_json::to_value(c).unwrap_or_default();
                let name = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if SIMPLE.contains(&name) && v.as_object().is_some_and(|o| o.len() == 1) {
                    name.to_owned()
                } else {
                    format!("do {v}")
                }
            }
        };
        out.push_str(&line);
        out.push('\n');
    }
    flush(&mut typed, &mut out);
    out
}

fn pos(p: Pos) -> String {
    format!("{}:{}", p.line + 1, p.ch + 1)
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FsErrorCode;

    fn no_files(_: &str) -> Result<String, FsError> {
        Err(FsError::new(FsErrorCode::NotFound, "no files", None))
    }

    fn p(text: &str) -> Result<Vec<Item>, FsError> {
        parse(text, &mut no_files)
    }

    #[test]
    fn parses_every_command() {
        let items = p(r#"
# a comment
tempo human cpm=400 seed=7 typos=0.05
type "a\"b\n\tc\u{1F642}"
type <<END
x
  y
END
pause 800
goto 3:2
select 1:1 2:5
move wordRight select
backspace 3
find "ne.dle" regex case
do {"type":"duplicateLines"}
"#)
        .unwrap();
        assert_eq!(
            items[0],
            Item::Tempo(Tempo::Human {
                cpm: 400.0,
                seed: 7,
                typos: 0.05
            })
        );
        assert_eq!(items[1], Item::Type("a\"b\n\tc🙂".into()));
        assert_eq!(items[2], Item::Type("x\n  y".into()));
        assert_eq!(items[3], Item::Pause(800));
        assert_eq!(
            items[4],
            cmd(Command::GoTo {
                line: 2,
                ch: 1,
                select: false
            })
        );
        assert!(
            matches!(&items[5], Item::Cmd { cmd: Command::Select { sels }, .. }
            if sels[0].head == Pos::new(1, 4))
        );
        assert_eq!(
            items[6],
            cmd(Command::Move {
                to: Motion::WordRight,
                select: true
            })
        );
        assert_eq!(items[7..10], vec![cmd(Command::Backspace); 3][..]);
        assert!(
            matches!(&items[10], Item::Cmd { cmd: Command::Find { query, .. }, .. }
            if query.regex && query.case_sensitive && !query.whole_word)
        );
        assert_eq!(items[11], cmd(Command::DuplicateLines));
        assert_eq!(items.len(), 12);
    }

    #[test]
    fn errors_name_the_line() {
        for (text, line) in [
            ("type \"open", 1),
            ("\n\nfrobnicate", 3),
            ("goto 0:1", 1),
            ("type <<END\nno end", 1),
            ("tempo human speed=3", 1),
            ("do {\"type\":\"nope\"}", 1),
            ("type @x.txt", 1),
            ("move sideways", 1),
        ] {
            let e = p(text).unwrap_err();
            assert_eq!(e.code, FsErrorCode::RecordingInvalid, "{text}");
            assert!(
                e.message.starts_with(&format!("line {line}:")),
                "{text}: {}",
                e.message
            );
        }
    }

    #[test]
    fn files_are_read_through_the_callback() {
        let mut read = |f: &str| Ok(format!("<{f}>"));
        let items = parse("type @snip/a.ts", &mut read).unwrap();
        assert_eq!(items, vec![Item::Type("<snip/a.ts>".into())]);
    }

    #[test]
    fn quoting_round_trips() {
        for s in ["plain", "a\"b\\c", "tab\tnew\nline", "🙂 ü 中", "\u{7}"] {
            let q = quote(s);
            let (back, rest) = parse_string(&q).unwrap();
            assert_eq!((back.as_str(), rest), (s, ""));
        }
    }
}
