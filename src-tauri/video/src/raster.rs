//! A small RGBA canvas and glyph cache (design 017 E2): rectangles, rounded rectangles,
//! circles and text in cells – all deterministic, no system library.

use crate::theme::Rgba;
use ed_core::error::{FsError, FsErrorCode};
use fontdue::{Font, FontSettings, Metrics};
use std::collections::HashMap;
use std::path::Path;

/// JetBrains Mono Regular (SIL OFL 1.1), the default font of the video.
pub const DEFAULT_FONT: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    clip: (i32, i32, i32, i32),
}

/// Blends `c` over the pixel at `i` with coverage `cov` (0–255).
#[inline]
fn blend(px: &mut [u8], c: Rgba, cov: u32) {
    let a = c.3 as u32 * cov / 255;
    if a == 0 {
        return;
    }
    let inv = 255 - a;
    px[0] = ((c.0 as u32 * a + px[0] as u32 * inv + 127) / 255) as u8;
    px[1] = ((c.1 as u32 * a + px[1] as u32 * inv + 127) / 255) as u8;
    px[2] = ((c.2 as u32 * a + px[2] as u32 * inv + 127) / 255) as u8;
    px[3] = 255;
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
            clip: (0, 0, width as i32, height as i32),
        }
    }

    pub fn fill(&mut self, c: Rgba) {
        for px in self.pixels.chunks_exact_mut(4) {
            px.copy_from_slice(&[c.0, c.1, c.2, 255]);
        }
    }

    /// Only this rectangle is painted from now on (`None` = everything).
    pub fn set_clip(&mut self, r: Option<(i32, i32, i32, i32)>) {
        self.clip = r.map_or(
            (0, 0, self.width as i32, self.height as i32),
            |(x, y, w, h)| {
                (
                    x.max(0),
                    y.max(0),
                    (x + w).min(self.width as i32),
                    (y + h).min(self.height as i32),
                )
            },
        );
    }

    fn put(&mut self, x: i32, y: i32, c: Rgba, cov: u32) {
        let (cx0, cy0, cx1, cy1) = self.clip;
        if x < cx0 || y < cy0 || x >= cx1 || y >= cy1 {
            return;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        blend(&mut self.pixels[i..i + 4], c, cov);
    }

    /// A rectangle with sub-pixel edges (`f64` coordinates).
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Rgba) {
        if w <= 0.0 || h <= 0.0 || c.3 == 0 {
            return;
        }
        let (x0, y0) = (x.floor() as i32, y.floor() as i32);
        let (x1, y1) = ((x + w).ceil() as i32, (y + h).ceil() as i32);
        let (cx0, cy0, cx1, cy1) = self.clip;
        for py in y0.max(cy0)..y1.min(cy1) {
            let cy = cover(py as f64, y, y + h);
            for px in x0.max(cx0)..x1.min(cx1) {
                let cov = cy * cover(px as f64, x, x + w);
                self.put(px, py, c, (cov * 255.0).round() as u32);
            }
        }
    }

    /// A rectangle with rounded corners (radius `r`).
    pub fn round_rect(&mut self, x: f64, y: f64, w: f64, h: f64, r: f64, c: Rgba) {
        let r = r.min(w / 2.0).min(h / 2.0);
        let (x0, y0) = (x.floor() as i32, y.floor() as i32);
        let (x1, y1) = ((x + w).ceil() as i32, (y + h).ceil() as i32);
        for py in y0..y1 {
            for px in x0..x1 {
                let (fx, fy) = (px as f64 + 0.5, py as f64 + 0.5);
                // distance outside the rounded shape
                let qx = (fx - (x + w / 2.0)).abs() - (w / 2.0 - r);
                let qy = (fy - (y + h / 2.0)).abs() - (h / 2.0 - r);
                let outside = (qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0)) - r;
                let cov = (0.5 - outside).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.put(px, py, c, (cov * 255.0).round() as u32);
                }
            }
        }
    }

    pub fn circle(&mut self, cx: f64, cy: f64, r: f64, c: Rgba) {
        self.round_rect(cx - r, cy - r, 2.0 * r, 2.0 * r, r, c);
    }

    /// A coverage bitmap (glyph) with its top-left at `x`, `y`.
    fn bitmap(&mut self, x: i32, y: i32, m: &Metrics, bits: &[u8], c: Rgba) {
        for gy in 0..m.height {
            for gx in 0..m.width {
                let cov = bits[gy * m.width + gx] as u32;
                if cov > 0 {
                    self.put(x + gx as i32, y + gy as i32, c, cov);
                }
            }
        }
    }

    pub fn rgba(&self) -> &[u8] {
        &self.pixels
    }
}

/// How much of pixel `p` (p..p+1) lies in `a..b`.
fn cover(p: f64, a: f64, b: f64) -> f64 {
    (b.min(p + 1.0) - a.max(p)).clamp(0.0, 1.0)
}

/// The fonts of a render: the main one (cell size) and fallbacks for missing glyphs.
pub struct Fonts {
    fonts: Vec<Font>,
    pub size: f32,
    /// Cell width and line height in px.
    pub cw: f64,
    pub lh: f64,
    ascent: f64,
    descent: f64,
    cache: HashMap<(char, u32), (usize, Metrics, Vec<u8>)>,
}

fn load_font(bytes: &[u8], what: &str) -> Result<Font, FsError> {
    Font::from_bytes(
        bytes,
        FontSettings {
            scale: 40.0,
            ..FontSettings::default()
        },
    )
    .map_err(|e| FsError::new(FsErrorCode::InvalidRequest, format!("{what}: {e}"), None))
}

/// Reads a font file – a missing one is an error, never a silent fallback.
pub fn read_font(path: &Path) -> Result<Vec<u8>, FsError> {
    std::fs::read(path).map_err(|e| FsError::from_io(&e, path))
}

impl Fonts {
    /// `main` = the bundled font when `None`; line height = 1.55 × size like the app.
    pub fn new(main: Option<&[u8]>, fallbacks: &[Vec<u8>], size: f32) -> Result<Self, FsError> {
        let mut fonts = vec![load_font(main.unwrap_or(DEFAULT_FONT), "font")?];
        for (i, f) in fallbacks.iter().enumerate() {
            fonts.push(load_font(f, &format!("fallback font {}", i + 1))?);
        }
        let cw = fonts[0].metrics('0', size).advance_width as f64;
        let lm = fonts[0].horizontal_line_metrics(size);
        let (ascent, descent) = lm.map_or((size as f64 * 0.8, -(size as f64) * 0.2), |l| {
            (l.ascent as f64, l.descent as f64)
        });
        Ok(Self {
            fonts,
            size,
            cw,
            lh: (size as f64 * 1.55).round(),
            ascent,
            descent,
            cache: HashMap::new(),
        })
    }

    /// Baseline from the top of a line box.
    pub fn baseline(&self) -> f64 {
        ((self.lh - (self.ascent - self.descent)) / 2.0 + self.ascent).round()
    }

    fn glyph(&mut self, c: char, size: f32) -> &(usize, Metrics, Vec<u8>) {
        let key = (c, size.to_bits());
        if !self.cache.contains_key(&key) {
            let i = self
                .fonts
                .iter()
                .position(|f| f.lookup_glyph_index(c) != 0)
                .unwrap_or(0);
            let (m, bits) = self.fonts[i].rasterize(c, size);
            self.cache.insert(key, (i, m, bits));
        }
        &self.cache[&key]
    }

    /// Draws `text` from `x` with its baseline at `y`; each char advances by its own width
    /// (combining marks: zero). Returns the advance.
    pub fn draw(&mut self, canvas: &mut Canvas, text: &str, x: f64, y: f64, c: Rgba) -> f64 {
        self.draw_sized(canvas, text, x, y, c, self.size)
    }

    pub fn draw_sized(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        x: f64,
        y: f64,
        c: Rgba,
        size: f32,
    ) -> f64 {
        let mut pen = x;
        for ch in text.chars() {
            let (_, m, bits) = self.glyph(ch, size).clone();
            let gx = (pen + m.xmin as f64).round() as i32;
            let gy = (y - m.height as f64 - m.ymin as f64).round() as i32;
            canvas.bitmap(gx, gy, &m, &bits, c);
            pen += m.advance_width as f64;
        }
        pen - x
    }

    /// Draws one grapheme into a box of `cells` cells starting at `x` (centred).
    pub fn draw_cell(&mut self, canvas: &mut Canvas, g: &str, x: f64, y: f64, cells: f64, c: Rgba) {
        let width: f64 = g
            .chars()
            .map(|ch| self.glyph(ch, self.size).1.advance_width as f64)
            .sum();
        let off = ((cells * self.cw - width) / 2.0).max(0.0);
        // a single cell: start at the cell like the browser does
        let off = if cells <= 1.0 { 0.0 } else { off };
        self.draw(canvas, g, x + off, y, c);
    }

    pub fn text_width(&mut self, text: &str, size: f32) -> f64 {
        text.chars()
            .map(|ch| self.glyph(ch, size).1.advance_width as f64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_blend_and_clip() {
        let mut c = Canvas::new(4, 4);
        c.fill(Rgba::opaque(255, 255, 255));
        c.rect(1.0, 1.0, 2.0, 2.0, Rgba(0, 0, 0, 255));
        assert_eq!(&c.pixels[(4 + 1) * 4..(4 + 1) * 4 + 4], &[0, 0, 0, 255]);
        assert_eq!(&c.pixels[0..4], &[255, 255, 255, 255]);
        // half a pixel: half the colour
        c.rect(0.0, 3.5, 1.0, 0.5, Rgba(0, 0, 0, 255));
        assert!((126..=129).contains(&c.pixels[12 * 4]));
        c.set_clip(Some((0, 0, 1, 1)));
        c.rect(0.0, 0.0, 4.0, 4.0, Rgba(255, 0, 0, 255));
        assert_eq!(&c.pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(c.pixels[4], 255);
        assert_eq!(c.pixels[5], 255);
    }

    #[test]
    fn the_bundled_font_is_monospaced_and_draws() {
        let mut f = Fonts::new(None, &[], 20.0).unwrap();
        assert!(f.cw > 10.0 && f.cw < 14.0, "{}", f.cw);
        assert_eq!(f.lh, 31.0);
        assert_eq!(f.text_width("iiii", 20.0), f.text_width("MMMM", 20.0));
        let mut c = Canvas::new(60, 40);
        c.fill(Rgba::opaque(255, 255, 255));
        f.draw(&mut c, "Hi", 2.0, f.baseline(), Rgba::opaque(0, 0, 0));
        assert!(
            c.pixels.chunks(4).any(|p| p[0] < 50),
            "something dark was drawn"
        );
        assert!(read_font(Path::new("/no/such/font.ttf")).is_err());
        assert!(Fonts::new(Some(b"not a font"), &[], 20.0).is_err());
    }
}
