//! One editor frame as an image (design 017 E3): the same layout as the app's editor view –
//! gutter with line numbers, one padding cell, text in cells, selections, search and word marks,
//! indentation guides, carets – inside an optional window frame, plus a caption.

use crate::raster::{Canvas, Fonts};
use crate::theme::{Rgba, Theme};
use ed_core::editor::model::{Frame, FrameLine, RunKind};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// Look of the window around the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Chrome {
    /// The editor fills the picture.
    None,
    /// A rounded window with a title on the page colour.
    #[default]
    Plain,
    /// Like a macOS window (three lights).
    Mac,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Look {
    pub width: u32,
    pub height: u32,
    /// Font size in px.
    pub font_size: f32,
    pub chrome: Chrome,
    /// Space around the window (px).
    pub padding: u32,
    pub line_numbers: bool,
    pub indent_guides: bool,
    pub current_line: bool,
    /// Window title (`None` = document name).
    pub title: Option<String>,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            font_size: 28.0,
            chrome: Chrome::Plain,
            padding: 48,
            line_numbers: true,
            indent_guides: true,
            current_line: true,
            title: None,
        }
    }
}

/// The editor's place in the picture and its size in rows and cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Title bar above the editor (0 without).
    pub title_h: f64,
    pub gutter: f64,
    pub text_left: f64,
    pub rows: f64,
    pub cols: f64,
}

/// A font size that fits about 60 columns and 34 lines (1920×1080 → 28 px, 1080×1920 → 27 px).
pub fn default_font_size(width: u32, height: u32) -> f32 {
    (height as f32 / 38.0)
        .min(width as f32 / 40.0)
        .round()
        .max(8.0)
}

/// Cells of padding between gutter and text (app: `PAD_CELLS`).
const PAD_CELLS: f64 = 1.0;

pub fn layout(look: &Look, fonts: &Fonts, line_count: usize) -> Layout {
    let (w, h) = (look.width as f64, look.height as f64);
    let (pad, title_h) = match look.chrome {
        Chrome::None => (0.0, 0.0),
        _ => (look.padding as f64, (fonts.lh * 1.4).round()),
    };
    let digits = line_count.max(1).to_string().len().max(3) as f64;
    let gutter = if look.line_numbers {
        (digits + 2.0) * fonts.cw
    } else {
        fonts.cw
    };
    let text_left = gutter + PAD_CELLS * fonts.cw;
    let ew = w - 2.0 * pad;
    let eh = h - 2.0 * pad - title_h;
    Layout {
        x: pad,
        y: pad + title_h,
        w: ew,
        h: eh,
        title_h,
        gutter,
        text_left,
        rows: (eh / fonts.lh).max(1.0),
        cols: ((ew - text_left) / fonts.cw).max(1.0),
    }
}

/// Horizontal scroll of a line: wrapped lines never scroll sideways.
fn offset(f: &Frame, line: &FrameLine) -> f64 {
    if f.wrap && !line.long { 0.0 } else { f.left }
}

fn row_of(line: &FrameLine, cell: usize) -> usize {
    line.wraps.iter().take_while(|&&w| w <= cell).count()
}

/// Position of a cell within the text area: x in px, row of its line.
fn px(f: &Frame, line: &FrameLine, cell: usize, cw: f64) -> (f64, usize) {
    let r = row_of(line, cell);
    let start = if r == 0 { 0 } else { line.wraps[r - 1] };
    ((cell as f64 - start as f64 - offset(f, line)) * cw, r)
}

/// A cell range `[a, b)` as boxes (x, row, width), one per row.
fn segs(f: &Frame, line: &FrameLine, a: usize, b: usize, cw: f64) -> Vec<(f64, usize, f64)> {
    let mut out = Vec::new();
    let mut from = a;
    while from < b {
        let r = row_of(line, from);
        let end = b.min(line.wraps.get(r).copied().unwrap_or(b));
        let (x, row) = px(f, line, from, cw);
        out.push((x, row, (end - from) as f64 * cw));
        if end <= from {
            break;
        }
        from = end;
    }
    out
}

/// Draws one frame. `caret_on` = carets visible (blinking), `caption` = keys or a note.
pub fn draw(
    canvas: &mut Canvas,
    f: &Frame,
    theme: &Theme,
    fonts: &mut Fonts,
    look: &Look,
    caret_on: bool,
    caption: Option<&str>,
) {
    draw_window(canvas, &f.info.name, theme, fonts, look);
    draw_editor(canvas, f, theme, fonts, look, caret_on, caption);
}

/// Page, shadow, window and title – the part that does not change while a video runs.
pub fn draw_window(canvas: &mut Canvas, name: &str, theme: &Theme, fonts: &mut Fonts, look: &Look) {
    let l = layout(look, fonts, 1);
    let lh = fonts.lh;
    canvas.set_clip(None);
    canvas.fill(if look.chrome == Chrome::None {
        theme.bg
    } else {
        theme.page
    });
    // ---- window
    if look.chrome != Chrome::None {
        let radius = (lh * 0.45).round();
        let (wx, wy, ww, wh) = (l.x, l.y - l.title_h, l.w, l.h + l.title_h);
        let shadow = if theme.dark {
            Rgba(0, 0, 0, 90)
        } else {
            Rgba(0, 0, 0, 28)
        };
        for i in 1..=6 {
            let s = i as f64 * 3.0;
            canvas.round_rect(
                wx - s / 2.0,
                wy - s / 4.0,
                ww + s,
                wh + s,
                radius + s,
                Rgba(shadow.0, shadow.1, shadow.2, shadow.3 / 6),
            );
        }
        canvas.round_rect(
            wx - 1.0,
            wy - 1.0,
            ww + 2.0,
            wh + 2.0,
            radius + 1.0,
            theme.border,
        );
        canvas.round_rect(wx, wy, ww, wh, radius, theme.gutter_bg);
        let title = look.title.clone().unwrap_or_else(|| name.to_owned());
        let ty = wy + l.title_h / 2.0 + fonts.size as f64 * 0.32;
        let small = fonts.size * 0.8;
        if look.chrome == Chrome::Mac {
            let r = (l.title_h * 0.16).round().max(4.0);
            for (i, c) in [
                Rgba::opaque(0xff, 0x5f, 0x57),
                Rgba::opaque(0xfe, 0xbc, 0x2e),
                Rgba::opaque(0x28, 0xc8, 0x40),
            ]
            .iter()
            .enumerate()
            {
                canvas.circle(
                    wx + r * 2.5 + i as f64 * r * 3.2,
                    wy + l.title_h / 2.0,
                    r,
                    *c,
                );
            }
        }
        let tw = fonts.text_width(&title, small);
        fonts.draw_sized(canvas, &title, wx + (ww - tw) / 2.0, ty, theme.muted, small);
        canvas.rect(
            wx,
            l.y - 1.0,
            ww,
            1.0,
            Rgba(
                theme.border.0,
                theme.border.1,
                theme.border.2,
                theme.border.3 / 2,
            ),
        );
    }
}

/// The editor area: gutter, lines, carets, caption.
pub fn draw_editor(
    canvas: &mut Canvas,
    f: &Frame,
    theme: &Theme,
    fonts: &mut Fonts,
    look: &Look,
    caret_on: bool,
    caption: Option<&str>,
) {
    let l = layout(look, fonts, f.info.line_count);
    let (cw, lh) = (fonts.cw, fonts.lh);
    let clip = |c: &mut Canvas, x: f64, w: f64| {
        c.set_clip(Some((
            x as i32,
            l.y as i32,
            w.ceil() as i32,
            l.h.ceil() as i32,
        )))
    };
    clip(canvas, l.x, l.w);
    canvas.rect(l.x, l.y, l.w, l.h, theme.bg);
    canvas.rect(l.x, l.y, l.gutter, l.h, theme.gutter_bg);
    let tx = l.x + l.text_left;
    let base = fonts.baseline();
    let caret_line = f.caret_line;
    let current =
        |line: &FrameLine| f.sel_chars == 0 && f.carets.iter().any(|c| c.line == line.line);
    for line in &f.lines {
        let y = l.y + line.row * lh;
        if y >= l.y + l.h {
            break;
        }
        let rows = (line.wraps.len() + 1) as f64;
        // gutter number
        clip(canvas, l.x, l.gutter);
        if look.line_numbers {
            let label = (line.line + 1).to_string();
            let w = label.len() as f64 * cw;
            let color = if line.line == caret_line {
                theme.gutter_active
            } else {
                theme.gutter_text
            };
            fonts.draw(canvas, &label, l.x + l.gutter - cw - w, y + base, color);
        }
        // text area
        clip(canvas, tx, l.w - l.text_left);
        if look.current_line && current(line) {
            canvas.rect(tx, y, l.w - l.text_left, rows * lh, theme.current_line);
        }
        if look.indent_guides && f.tab_width > 0 {
            let mut c = f.tab_width;
            while c < line.indent {
                let gx = tx + (c as f64 - offset(f, line)) * cw;
                canvas.rect(gx.round(), y, 1.0, rows * lh, theme.indent_guide);
                c += f.tab_width;
            }
        }
        for m in &line.markers {
            let color = theme.markers[m[2] % 10];
            for (x, r, w) in segs(f, line, m[0], m[1], cw) {
                canvas.round_rect(tx + x, y + r as f64 * lh + 1.0, w, lh - 2.0, 2.0, color);
            }
        }
        for m in &line.marks {
            let color = if m[2] == 1 {
                theme.search_current
            } else {
                theme.search_match
            };
            for (x, r, w) in segs(f, line, m[0], m[1], cw) {
                canvas.round_rect(tx + x, y + r as f64 * lh + 1.0, w, lh - 2.0, 2.0, color);
            }
        }
        for s in &line.sel {
            for (x, r, w) in segs(f, line, s[0], s[1], cw) {
                canvas.rect(tx + x, y + r as f64 * lh, w, lh, theme.selection);
            }
        }
        if line.sel_eol {
            let (x, r) = px(f, line, line.width, cw);
            canvas.rect(tx + x, y + r as f64 * lh, cw * 0.6, lh, theme.selection);
        }
        for run in &line.runs {
            let color = theme
                .syntax
                .get(run.style as usize)
                .copied()
                .unwrap_or(theme.text);
            match run.kind {
                RunKind::Tab => {}
                RunKind::Wide => {
                    for (i, ch) in run.text.chars().enumerate() {
                        let (x, r) = px(f, line, run.cell + 2 * i, cw);
                        let s = ch.to_string();
                        fonts.draw_cell(canvas, &s, tx + x, y + r as f64 * lh + base, 2.0, color);
                    }
                }
                RunKind::Text | RunKind::Ctrl => {
                    for (i, g) in run.text.graphemes(true).enumerate() {
                        let (x, r) = px(f, line, run.cell + i, cw);
                        fonts.draw_cell(canvas, g, tx + x, y + r as f64 * lh + base, 1.0, color);
                    }
                }
            }
        }
    }
    // carets
    if caret_on {
        let w = (fonts.size as f64 / 9.0).max(2.0).round();
        for (i, c) in f.carets.iter().enumerate() {
            let Some(xy) = f.caret_xy.get(i) else {
                continue;
            };
            let line = f.lines.iter().find(|x| x.line == c.line);
            let off = line.map_or(f.left, |ln| offset(f, ln));
            let x = tx + (xy.x as f64 - off) * cw;
            canvas.rect(x - w / 2.0, l.y + xy.row * lh, w, lh, theme.caret);
        }
    }
    canvas.set_clip(None);
    // caption: a pill at the bottom
    if let Some(text) = caption.filter(|t| !t.is_empty()) {
        let size = fonts.size * 1.1;
        let tw = fonts.text_width(text, size);
        let (ph, pw) = (lh * 1.5, tw + lh * 1.6);
        let px_ = (look.width as f64 - pw) / 2.0;
        let py = l.y + l.h - ph - lh * 0.8;
        canvas.round_rect(px_, py, pw, ph, ph / 2.0, Rgba(20, 22, 26, 215));
        fonts.draw_sized(
            canvas,
            text,
            px_ + (pw - tw) / 2.0,
            py + ph / 2.0 + size as f64 * 0.35,
            Rgba::opaque(250, 250, 250),
            size,
        );
    }
}
