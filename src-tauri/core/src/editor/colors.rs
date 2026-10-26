//! Colour values in the text (design 018): found in the visible lines only, shown as swatches
//! in the gutter, converted between notations.
//!
//! Recognised: CSS `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb()/rgba()`, `hsl()/hsla()`
//! (commas or spaces, percentages, `/ alpha`); `0xRRGGBB` and `0xAARRGGBB`; bare hex in quotes
//! `"ffffff00"` (8 digits = AARRGGBB like Android/Java, 6 = RRGGBB).

use regex::Regex;
use std::sync::OnceLock;

/// Characters scanned per line – a 100 MB line is never scanned as a whole.
pub const SCAN_CHARS: usize = 4096;
/// Colours per line at most.
pub const MAX_PER_LINE: usize = 3;

/// A colour in a line: characters `from..to`, `rgba` = 0xRRGGBBAA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorHit {
    pub from: usize,
    pub to: usize,
    pub rgba: u32,
    pub kind: Kind,
}

/// How the colour was written (what "convert" turns it into next).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Hash,
    Rgb,
    Hsl,
    Hex0x,
    Quoted,
}

fn re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)",
            // #hex – not after a word char, & (entities) or another #
            r"(?P<hash>(?:^|[^\w&#])#(?P<h>[0-9a-f]{8}|[0-9a-f]{6}|[0-9a-f]{3,4})\b)",
            r"|(?P<rgb>\brgba?\(\s*(?P<rargs>[0-9.%\s,/]+)\))",
            r"|(?P<hsl>\bhsla?\(\s*(?P<hargs>[0-9.%\s,/deg]+)\))",
            r"|(?P<x>\b0x(?P<xh>[0-9a-f]{8}|[0-9a-f]{6})\b)",
            r#"|(?P<q>(?P<qq>["'])(?P<qh>[0-9a-f]{8}|[0-9a-f]{6})["'])"#,
        ))
        .expect("colour regex")
    })
}

fn hex_byte(s: &str) -> Option<u32> {
    u32::from_str_radix(s, 16).ok()
}

fn pack(r: u32, g: u32, b: u32, a: u32) -> u32 {
    (r.min(255) << 24) | (g.min(255) << 16) | (b.min(255) << 8) | a.min(255)
}

/// `#…` digits → rgba.
fn from_hash(h: &str) -> Option<u32> {
    let d = |i: usize| hex_byte(&h[i..i + 1]).map(|v| v * 17);
    match h.len() {
        3 => Some(pack(d(0)?, d(1)?, d(2)?, 255)),
        4 => Some(pack(d(0)?, d(1)?, d(2)?, d(3)?)),
        6 => Some(pack(
            hex_byte(&h[0..2])?,
            hex_byte(&h[2..4])?,
            hex_byte(&h[4..6])?,
            255,
        )),
        8 => Some(pack(
            hex_byte(&h[0..2])?,
            hex_byte(&h[2..4])?,
            hex_byte(&h[4..6])?,
            hex_byte(&h[6..8])?,
        )),
        _ => None,
    }
}

/// 6 digits RRGGBB or 8 digits AARRGGBB (0x…, quoted).
fn from_argb(h: &str) -> Option<u32> {
    match h.len() {
        6 => from_hash(h),
        8 => Some(pack(
            hex_byte(&h[2..4])?,
            hex_byte(&h[4..6])?,
            hex_byte(&h[6..8])?,
            hex_byte(&h[0..2])?,
        )),
        _ => None,
    }
}

/// The numbers of `rgb(…)`/`hsl(…)`: three or four, alpha after `/` or as the fourth.
fn args(s: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = s
        .replace('/', " / ")
        .split([' ', ','])
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    let mut out: Vec<String> = parts.into_iter().filter(|p| p != "/").collect();
    if out.len() == 3 {
        out.push("1".into());
    }
    (out.len() == 4).then_some(out)
}

fn alpha(s: &str) -> Option<u32> {
    let v = if let Some(p) = s.strip_suffix('%') {
        p.parse::<f64>().ok()? / 100.0
    } else {
        s.parse::<f64>().ok()?
    };
    (0.0..=1.0).contains(&v).then(|| (v * 255.0).round() as u32)
}

fn channel(s: &str) -> Option<u32> {
    let v = if let Some(p) = s.strip_suffix('%') {
        p.parse::<f64>().ok()? * 2.55
    } else {
        s.parse::<f64>().ok()?
    };
    (0.0..=255.0).contains(&v).then(|| v.round() as u32)
}

fn from_rgb(a: &str) -> Option<u32> {
    let v = args(a)?;
    Some(pack(
        channel(&v[0])?,
        channel(&v[1])?,
        channel(&v[2])?,
        alpha(&v[3])?,
    ))
}

/// HSL (degrees, %, %) → RGB.
pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u32, u32, u32) {
    let h = h.rem_euclid(360.0) / 360.0;
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    if s == 0.0 {
        let v = (l * 255.0).round() as u32;
        return (v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let f = |t: f64| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v * 255.0).round() as u32
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

/// RGB → HSL (degrees, 0..1, 0..1).
pub fn rgb_to_hsl(r: u32, g: u32, b: u32) -> (f64, f64, f64) {
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

fn from_hsl(a: &str) -> Option<u32> {
    let v = args(a)?;
    let h = v[0].trim_end_matches("deg").parse::<f64>().ok()?;
    let pct = |s: &str| -> Option<f64> {
        let x = s.strip_suffix('%')?.parse::<f64>().ok()?;
        (0.0..=100.0).contains(&x).then_some(x / 100.0)
    };
    let (r, g, b) = hsl_to_rgb(h, pct(&v[1])?, pct(&v[2])?);
    Some(pack(r, g, b, alpha(&v[3])?))
}

/// Colours in a line (at most [`MAX_PER_LINE`]); positions in characters.
pub fn find(line: &str) -> Vec<ColorHit> {
    let mut out = Vec::new();
    let text: &str = match line.char_indices().nth(SCAN_CHARS) {
        Some((b, _)) => &line[..b],
        None => line,
    };
    // quick exit: nothing that could start a colour
    if !text.contains(['#', '(', 'x', 'X', '"', '\'']) {
        return out;
    }
    for c in re().captures_iter(text) {
        let (m, rgba, kind) = if let Some(m) = c.name("hash") {
            let h = c.name("h").map_or("", |x| x.as_str());
            // the match may start with the char before '#'
            let start = m.start() + m.as_str().find('#').unwrap_or(0);
            (
                (start, m.end()),
                from_hash(&h.to_ascii_lowercase()),
                Kind::Hash,
            )
        } else if let Some(m) = c.name("rgb") {
            ((m.start(), m.end()), from_rgb(&c["rargs"]), Kind::Rgb)
        } else if let Some(m) = c.name("hsl") {
            ((m.start(), m.end()), from_hsl(&c["hargs"]), Kind::Hsl)
        } else if let Some(m) = c.name("x") {
            ((m.start(), m.end()), from_argb(&c["xh"]), Kind::Hex0x)
        } else if c.name("q").is_some() {
            // the quoted digits only (the quotes stay)
            let h = c.name("qh").map_or((0, 0), |x| (x.start(), x.end()));
            (h, from_argb(&c["qh"]), Kind::Quoted)
        } else {
            continue;
        };
        let Some(rgba) = rgba else { continue };
        let from = text[..m.0].chars().count();
        let to = from + text[m.0..m.1].chars().count();
        out.push(ColorHit {
            from,
            to,
            rgba,
            kind,
        });
        if out.len() >= MAX_PER_LINE {
            break;
        }
    }
    out
}

/// `#rrggbb` or `#rrggbbaa`.
pub fn to_hash(rgba: u32) -> String {
    if rgba & 0xff == 0xff {
        format!("#{:06x}", rgba >> 8)
    } else {
        format!("#{rgba:08x}")
    }
}

fn alpha_text(a: u32) -> String {
    let v = (a as f64 / 255.0 * 100.0).round() / 100.0;
    format!("{v}")
}

/// The next notation of a colour: `#…` → `rgb()` → `hsl()` → `#…`; `0x…` and quoted → `#…`.
pub fn convert(hit: &ColorHit) -> String {
    let (r, g, b, a) = (
        hit.rgba >> 24,
        (hit.rgba >> 16) & 0xff,
        (hit.rgba >> 8) & 0xff,
        hit.rgba & 0xff,
    );
    match hit.kind {
        Kind::Hash => {
            if a == 255 {
                format!("rgb({r}, {g}, {b})")
            } else {
                format!("rgba({r}, {g}, {b}, {})", alpha_text(a))
            }
        }
        Kind::Rgb => {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (h, s, l) = (h.round(), (s * 100.0).round(), (l * 100.0).round());
            if a == 255 {
                format!("hsl({h}, {s}%, {l}%)")
            } else {
                format!("hsla({h}, {s}%, {l}%, {})", alpha_text(a))
            }
        }
        Kind::Hsl | Kind::Hex0x | Kind::Quoted => to_hash(hit.rgba),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(s: &str) -> Option<(usize, usize, u32)> {
        find(s).first().map(|h| (h.from, h.to, h.rgba))
    }

    #[test]
    fn css_hex() {
        assert_eq!(one("color: #2f98e1;"), Some((7, 14, 0x2f98e1ff)));
        assert_eq!(one("#fff"), Some((0, 4, 0xffffffff)));
        assert_eq!(one("x #f008"), Some((2, 7, 0xff000088)));
        assert_eq!(one("#7dff333a"), Some((0, 9, 0x7dff333a)));
        assert_eq!(one("background:#ABCDEF"), Some((11, 18, 0xabcdefff)));
    }

    #[test]
    fn functions() {
        assert_eq!(one("rgb(255, 0, 0)").map(|h| h.2), Some(0xff0000ff));
        assert_eq!(one("rgb(0 128 255 / 50%)").map(|h| h.2), Some(0x0080ff80));
        assert_eq!(one("rgba(10,20,30,0.5)").map(|h| h.2), Some(0x0a141e80));
        assert_eq!(one("rgb(100%, 0%, 0%)").map(|h| h.2), Some(0xff0000ff));
        assert_eq!(one("hsl(120, 100%, 50%)").map(|h| h.2), Some(0x00ff00ff));
        assert_eq!(
            one("hsl(240deg 100% 50% / 0.25)").map(|h| h.2),
            Some(0x0000ff40)
        );
        assert_eq!(one("hsla(0, 0%, 50%, 1)").map(|h| h.2), Some(0x808080ff));
    }

    #[test]
    fn android_and_java_style() {
        // like the picture of the old version: bg="ffRRGGBB" (AARRGGBB)
        assert_eq!(
            one(r#"bg="ffffff00" fontSize=3"#),
            Some((4, 12, 0xffff00ff))
        );
        assert_eq!(one(r#"bg="ff0098db""#).map(|h| h.2), Some(0x0098dbff));
        assert_eq!(one(r#"bg="80000000""#).map(|h| h.2), Some(0x00000080));
        assert_eq!(one("'0098db'").map(|h| h.2), Some(0x0098dbff));
        assert_eq!(
            one("int c = 0xFF2F98E1;").map(|h| (h.0, h.2)),
            Some((8, 0x2f98e1ff))
        );
        assert_eq!(one("0x2f98e1").map(|h| h.2), Some(0x2f98e1ff));
    }

    #[test]
    fn no_false_hits() {
        for s in [
            "#include <stdio.h>",
            "see #top",
            "git 1a2b3c4d5e6f7a8b9c0d",
            "commit abcdef12",
            "&#123; entity",
            "id 550e8400-e29b-41d4-a716-446655440000",
            "#ffffffx",
            "##fff",
            "rgb(300, 0, 0)",
            "hsl(1, 2, 3)",
            "\"abcdefg1\"",
            "0x12345",
            "x = 1 + (2)",
        ] {
            assert_eq!(find(s), vec![], "{s}");
        }
    }

    #[test]
    fn positions_are_characters_and_at_most_three() {
        // umlauts and emoji before the colour
        assert_eq!(one("äö🙂 #fff").map(|h| (h.0, h.1)), Some((4, 8)));
        let many = "#111 #222 #333 #444 #555";
        assert_eq!(find(many).len(), MAX_PER_LINE);
        // a very long line: only the start is scanned
        let long = format!("{}#fff", "x".repeat(SCAN_CHARS + 10));
        assert_eq!(find(&long), vec![]);
    }

    #[test]
    fn converting_cycles_the_notations() {
        let hit = find("#2f98e1")[0];
        assert_eq!(convert(&hit), "rgb(47, 152, 225)");
        let hit = find("rgb(47, 152, 225)")[0];
        let hsl = convert(&hit);
        assert_eq!(hsl, "hsl(205, 75%, 53%)");
        // back to hex: a rounding step away at most
        let back = find(&hsl)[0];
        let d = |a: u32, b: u32, s: u32| ((a >> s) & 0xff).abs_diff((b >> s) & 0xff);
        assert!(
            [24, 16, 8]
                .iter()
                .all(|&s| d(back.rgba, 0x2f98e1ff, s) <= 2)
        );
        assert_eq!(convert(&back).len(), 7);
        let t = find("#ff000080")[0];
        assert_eq!(convert(&t), "rgba(255, 0, 0, 0.5)");
        assert_eq!(
            convert(&find("rgba(255, 0, 0, 0.5)")[0]),
            "hsla(0, 100%, 50%, 0.5)"
        );
        assert_eq!(convert(&find("0x80ff0000")[0]), "#ff000080");
        assert_eq!(convert(&find("\"ffffff00\"")[0]), "#ffff00");
    }
}
