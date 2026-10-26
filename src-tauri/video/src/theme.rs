//! Colour schemes for the video (design 017 E3): read from the app's own `src/styles.css` at
//! build time – one source of truth, a new scheme there is a new scheme here.

use ed_core::highlight::CLASSES;
use std::collections::HashMap;

/// The app's stylesheet, compiled in.
pub const STYLES_CSS: &str = include_str!("../../../src/styles.css");

/// A colour with straight alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b, 255)
    }
}

/// Everything the renderer paints with.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub dark: bool,
    /// Around the window (the app's background).
    pub page: Rgba,
    pub border: Rgba,
    pub muted: Rgba,
    pub bg: Rgba,
    pub text: Rgba,
    pub gutter_bg: Rgba,
    pub gutter_text: Rgba,
    pub gutter_active: Rgba,
    pub current_line: Rgba,
    pub selection: Rgba,
    pub caret: Rgba,
    pub indent_guide: Rgba,
    pub search_match: Rgba,
    pub search_current: Rgba,
    pub markers: [Rgba; 10],
    /// By style class (`highlight::CLASSES`); 0 = the text colour.
    pub syntax: Vec<Rgba>,
}

/// Parses `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(r g b / a%)`, `rgb(r, g, b)`, `rgba(…)`.
pub fn parse_color(v: &str) -> Option<Rgba> {
    let v = v.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let n = |s: &str| u8::from_str_radix(s, 16).ok();
        return match hex.len() {
            3 => {
                let d: Vec<u8> = hex
                    .chars()
                    .map(|c| u8::from_str_radix(&c.to_string(), 16).ok().map(|x| x * 17))
                    .collect::<Option<_>>()?;
                Some(Rgba(d[0], d[1], d[2], 255))
            }
            6 => Some(Rgba(n(&hex[0..2])?, n(&hex[2..4])?, n(&hex[4..6])?, 255)),
            8 => Some(Rgba(
                n(&hex[0..2])?,
                n(&hex[2..4])?,
                n(&hex[4..6])?,
                n(&hex[6..8])?,
            )),
            _ => None,
        };
    }
    let inner = v
        .strip_prefix("rgba(")
        .or_else(|| v.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let (rgb, alpha) = match inner.split_once('/') {
        Some((a, b)) => (a, Some(b.trim())),
        None => (inner, None),
    };
    let mut parts: Vec<&str> = rgb.split([' ', ',']).filter(|p| !p.is_empty()).collect();
    let mut alpha = alpha.map(str::to_owned);
    if parts.len() == 4 && alpha.is_none() {
        alpha = parts.pop().map(str::to_owned);
    }
    if parts.len() != 3 {
        return None;
    }
    let c = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .map(|x| x.clamp(0.0, 255.0).round() as u8)
    };
    let a = match alpha {
        None => 255,
        Some(a) if a.ends_with('%') => {
            (a.trim_end_matches('%').trim().parse::<f64>().ok()? * 2.55).round() as u8
        }
        Some(a) => (a.parse::<f64>().ok()?.clamp(0.0, 1.0) * 255.0).round() as u8,
    };
    Some(Rgba(c(parts[0])?, c(parts[1])?, c(parts[2])?, a))
}

/// Custom properties by selector (top-level and inside `@media` blocks alike).
fn blocks(css: &str) -> Vec<(String, HashMap<String, String>)> {
    // drop comments
    let mut text = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = rest.find("/*") {
        text.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map_or("", |j| &rest[i + j + 2..]);
    }
    text.push_str(rest);
    let mut out = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        match c {
            '{' => {
                stack.push(buf.trim().to_owned());
                buf.clear();
            }
            '}' => {
                let sel = stack.pop().unwrap_or_default();
                let decls: HashMap<String, String> = buf
                    .split(';')
                    .filter_map(|d| {
                        let (k, v) = d.split_once(':')?;
                        let k = k.trim();
                        k.starts_with("--")
                            .then(|| (k.to_owned(), v.trim().to_owned()))
                    })
                    .collect();
                if !decls.is_empty() {
                    let inside_media = stack.iter().any(|s| s.starts_with("@media"));
                    // `@media (prefers-color-scheme: dark)` repeats the dark block: skip it
                    if !inside_media {
                        out.push((sel, decls));
                    }
                }
                buf.clear();
            }
            c => buf.push(c),
        }
    }
    out
}

/// The scheme names the stylesheet knows: `paper`, `ink` and every `data-scheme`.
pub fn names() -> Vec<String> {
    let mut out = vec!["paper".to_owned(), "ink".to_owned()];
    for (sel, _) in blocks(STYLES_CSS) {
        if let Some(rest) = sel.strip_prefix(":root[data-scheme='")
            && let Some(name) = rest.strip_suffix("']")
            && !out.iter().any(|n| n == name)
        {
            out.push(name.to_owned());
        }
    }
    out
}

/// Known bases of the schemes defined before `--scheme-base` existed.
fn default_base(name: &str) -> bool {
    matches!(name, "ink" | "night" | "dark")
}

/// A scheme by name (`light`/`dark` = `paper`/`ink`); `None` if the stylesheet has none.
pub fn theme(name: &str) -> Option<Theme> {
    let name = match name {
        "light" => "paper",
        "dark" => "ink",
        n => n,
    };
    let all = blocks(STYLES_CSS);
    let find = |sel: &str| -> HashMap<String, String> {
        all.iter()
            .filter(|(s, _)| s.split(',').any(|p| p.trim() == sel))
            .flat_map(|(_, d)| d.clone())
            .collect()
    };
    let scheme_sel = format!(":root[data-scheme='{name}']");
    let scheme = (!matches!(name, "paper" | "ink")).then(|| find(&scheme_sel));
    if let Some(s) = &scheme
        && s.is_empty()
    {
        return None;
    }
    let dark = scheme
        .as_ref()
        .and_then(|s| s.get("--scheme-base"))
        .map_or(default_base(name), |b| b.trim() == "dark");
    let mut vars = find(":root");
    if dark {
        vars.extend(find(":root[data-theme='dark']"));
    }
    if let Some(s) = scheme {
        vars.extend(s);
    }
    let get = |k: &str, fallback: Rgba| -> Rgba {
        let mut v = vars.get(k).cloned();
        // var(--x) references
        for _ in 0..4 {
            match v.as_deref().and_then(|x| x.strip_prefix("var(")) {
                Some(r) => {
                    let key = r
                        .trim_end_matches(')')
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim();
                    v = vars.get(key).cloned();
                }
                None => break,
            }
        }
        v.as_deref().and_then(parse_color).unwrap_or(fallback)
    };
    let text = get("--ed-text", Rgba::opaque(0, 0, 0));
    let bg = get("--ed-bg", Rgba::opaque(255, 255, 255));
    let markers =
        std::array::from_fn(|i| get(&format!("--ed-marker-{i}"), Rgba(128, 128, 128, 80)));
    let syntax = CLASSES
        .iter()
        .map(|c| {
            if c.is_empty() {
                text
            } else {
                get(&format!("--syn-{c}"), text)
            }
        })
        .collect();
    Some(Theme {
        name: name.to_owned(),
        dark,
        page: get("--color-bg", bg),
        border: get("--color-border", Rgba(128, 128, 128, 60)),
        muted: get("--color-text-muted", text),
        bg,
        text,
        gutter_bg: get("--ed-gutter-bg", bg),
        gutter_text: get("--ed-gutter-text", text),
        gutter_active: get("--ed-gutter-text-active", text),
        current_line: get("--ed-current-line", Rgba(0, 0, 0, 0)),
        selection: get("--ed-selection", Rgba(47, 111, 223, 56)),
        caret: get("--ed-caret", text),
        indent_guide: get("--ed-indent-guide", Rgba(0, 0, 0, 0)),
        search_match: get("--ed-search-match", Rgba(255, 196, 0, 97)),
        search_current: get("--ed-search-current", Rgba(255, 140, 0, 140)),
        markers,
        syntax,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours() {
        assert_eq!(parse_color("#fff"), Some(Rgba(255, 255, 255, 255)));
        assert_eq!(parse_color("#2f6fdf"), Some(Rgba(0x2f, 0x6f, 0xdf, 255)));
        assert_eq!(parse_color("#00000080"), Some(Rgba(0, 0, 0, 128)));
        assert_eq!(parse_color("rgb(0 0 0 / 3.5%)"), Some(Rgba(0, 0, 0, 9)));
        assert_eq!(parse_color("rgb(10, 20, 30)"), Some(Rgba(10, 20, 30, 255)));
        assert_eq!(
            parse_color("rgba(10, 20, 30, 0.5)"),
            Some(Rgba(10, 20, 30, 128))
        );
        assert_eq!(parse_color("inherit"), None);
    }

    #[test]
    fn the_app_schemes_come_from_the_stylesheet() {
        let names = names();
        for n in ["paper", "ink", "sand", "night"] {
            assert!(names.iter().any(|x| x == n), "{n} in {names:?}");
        }
        let paper = theme("paper").unwrap();
        assert!(!paper.dark);
        assert_eq!(paper.bg, Rgba::opaque(0xfb, 0xfa, 0xf7));
        assert_eq!(paper.caret, Rgba::opaque(0x2f, 0x6f, 0xdf));
        let ink = theme("dark").unwrap();
        assert!(ink.dark);
        assert_eq!(ink.bg, Rgba::opaque(0x17, 0x19, 0x1d));
        // `inherit` falls back to the text colour
        let var = CLASSES.iter().position(|c| *c == "variable").unwrap();
        assert_eq!(ink.syntax[var], ink.text);
        let night = theme("night").unwrap();
        assert!(night.dark);
        assert_eq!(night.bg, Rgba::opaque(0x22, 0x23, 0x1f));
        let kw = CLASSES.iter().position(|c| *c == "keyword").unwrap();
        assert_eq!(night.syntax[kw], Rgba::opaque(0xf9, 0x26, 0x72));
        assert!(theme("rainbow").is_none());
    }
}

#[cfg(test)]
mod contrast {
    use super::*;

    fn lum(c: Rgba) -> f64 {
        let f = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
    }

    pub fn over(fg: Rgba, bg: Rgba) -> Rgba {
        let a = fg.3 as f64 / 255.0;
        let m = |f: u8, b: u8| (f as f64 * a + b as f64 * (1.0 - a)).round() as u8;
        Rgba::opaque(m(fg.0, bg.0), m(fg.1, bg.1), m(fg.2, bg.2))
    }

    pub fn ratio(a: Rgba, b: Rgba) -> f64 {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Design 019 E5: text on every marker colour stays readable (≥ 4.5 : 1) in every scheme.
    #[test]
    fn text_on_markers_is_readable_in_every_scheme() {
        let mut low = Vec::new();
        for name in names() {
            let t = theme(&name).unwrap();
            for (i, m) in t.markers.iter().enumerate() {
                let r = ratio(t.text, over(*m, t.bg));
                if r < 4.5 {
                    low.push(format!("{name} marker {i}: {r:.2}"));
                }
            }
        }
        assert!(low.is_empty(), "{low:?}");
    }
}
