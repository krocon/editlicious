//! The user's settings (design 009): `settings.json` in the app config folder, loaded leniently
//! (unknown or bad values fall back to the default one by one, a broken file is kept as `.bad`).
//! Options of the old editor: legacy §5.

use crate::config_file::{self, Loaded};
use crate::fs_atomic::write_atomic;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

pub const FILE_NAME: &str = "settings.json";
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    /// `auto` (system), `en`, `de`.
    pub language: String,
    /// `auto`, `paper`, `ink`, `sand`, `night` (design 008 E6).
    pub scheme: String,
    /// Editor font family; empty = the platform's monospace font.
    pub font_family: String,
    /// Editor font size in px (zoom changes it).
    pub font_size: u32,
    pub tab_width: u32,
    /// `auto` (as the file does), `spaces`, `tabs`.
    pub indent_with: String,
    /// `absolute`, `relative`, `off` (legacy `LinenumberStyle`).
    pub line_numbers: String,
    pub indent_guides: bool,
    pub highlight_current_line: bool,
    /// Show tabs as faint arrows.
    pub show_tabs: bool,
    /// Line ending of new files: `auto` (platform), `lf`, `crlf`.
    pub new_line_ending: String,
    /// Reopen the files of the last session (legacy `loadLastFiles`).
    pub restore_session: bool,
    /// Windows/Linux: the menu bar stays visible.
    pub menu_bar: bool,
    pub status_bar: bool,
    /// Soft wrap of lines up to the long-line limit (design 010).
    pub word_wrap: bool,
    /// Search matches and markers in the scrollbar (design 010 E3).
    pub overview: bool,
    /// Colour values as swatches in the gutter (design 018).
    pub color_swatches: bool,
    /// Word boundaries of repeated double clicks (prompt 018a): `"_": 1`, `"space": 4` …;
    /// empty = the built-in table. Only in settings.json.
    pub expand_weights: std::collections::BTreeMap<String, f64>,
    /// An AI may drive the editor over MCP (design 016 E9) – off unless the user turns it on.
    #[serde(default)]
    pub mcp: bool,
    /// ffmpeg for the video export (design 017); empty = the one on the PATH.
    #[serde(default)]
    pub ffmpeg_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            language: "auto".into(),
            scheme: "auto".into(),
            font_family: String::new(),
            font_size: 14,
            tab_width: 4,
            indent_with: "auto".into(),
            line_numbers: "absolute".into(),
            indent_guides: true,
            highlight_current_line: true,
            show_tabs: true,
            new_line_ending: "auto".into(),
            restore_session: true,
            menu_bar: false,
            status_bar: true,
            word_wrap: false,
            overview: true,
            color_swatches: true,
            expand_weights: Default::default(),
            mcp: false,
            ffmpeg_path: String::new(),
        }
    }
}

fn one_of(value: &mut String, allowed: &[&str], default: &str) {
    if !allowed.contains(&value.as_str()) {
        *value = default.to_owned();
    }
}

/// Clamps and checks every field; the result is always usable.
pub fn sanitize(mut s: Settings) -> Settings {
    let d = Settings::default();
    s.version = 1;
    one_of(&mut s.language, &["auto", "en", "de"], &d.language);
    one_of(
        &mut s.scheme,
        &["auto", "paper", "ink", "sand", "night"],
        &d.scheme,
    );
    one_of(
        &mut s.indent_with,
        &["auto", "spaces", "tabs"],
        &d.indent_with,
    );
    one_of(
        &mut s.line_numbers,
        &["absolute", "relative", "off"],
        &d.line_numbers,
    );
    one_of(
        &mut s.new_line_ending,
        &["auto", "lf", "crlf"],
        &d.new_line_ending,
    );
    s.font_size = s.font_size.clamp(6, 72);
    s.tab_width = s.tab_width.clamp(1, 16);
    // a font family is a CSS value: no quotes/semicolons/braces, reasonable length
    if s.font_family.len() > 200 || s.font_family.contains(['"', ';', '{', '}', '\\', '<', '>']) {
        s.font_family = String::new();
    }
    s.expand_weights.retain(|k, v| {
        let ok_key =
            matches!(k.as_str(), "space" | "tab" | "punctuation") || k.chars().count() == 1;
        ok_key && v.is_finite() && (0.1..=9.0).contains(v)
    });
    while s.expand_weights.len() > 64 {
        s.expand_weights.pop_last();
    }
    if s.ffmpeg_path.len() > 1000 || s.ffmpeg_path.contains('\0') {
        s.ffmpeg_path = String::new();
    }
    s
}

pub fn load(dir: &Path) -> Loaded<Settings> {
    config_file::read_config(&dir.join(FILE_NAME), MAX_BYTES, Settings::default, |text| {
        serde_json::from_str::<Settings>(text)
            .map(sanitize)
            .map_err(|e| e.to_string())
    })
}

pub fn save(dir: &Path, s: Settings) -> io::Result<Settings> {
    let s = sanitize(s);
    let json = serde_json::to_vec_pretty(&s).map_err(io::Error::other)?;
    write_atomic(&dir.join(FILE_NAME), &json)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_and_bad_values() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).value, Settings::default());
        let mut s = Settings {
            font_size: 300,
            scheme: "rainbow".into(),
            font_family: "x; } body { color: red".into(),
            ..Default::default()
        };
        s.tab_width = 0;
        let saved = save(dir.path(), s).unwrap();
        assert_eq!(saved.font_size, 72);
        assert_eq!(saved.tab_width, 1);
        assert_eq!(saved.scheme, "auto");
        assert_eq!(saved.font_family, "");
        let mut w = Settings::default();
        w.expand_weights.insert("_".into(), 1.0);
        w.expand_weights.insert("word".into(), 2.0);
        w.expand_weights.insert(".".into(), 99.0);
        let w = sanitize(w);
        assert_eq!(w.expand_weights.keys().collect::<Vec<_>>(), ["_"]);
        assert_eq!(load(dir.path()).value, saved);
    }

    #[test]
    fn unknown_fields_and_missing_ones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(FILE_NAME),
            br#"{"fontSize": 18, "future": true}"#,
        )
        .unwrap();
        let s = load(dir.path()).value;
        assert_eq!(s.font_size, 18);
        assert_eq!(s.tab_width, 4);
    }
}
