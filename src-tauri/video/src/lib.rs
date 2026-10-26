//! ed-video – video export of editlicious (design 017): a recording or typing script plays on
//! a virtual clock through the real engine (`ed-core`), every frame is drawn here – sharp at any
//! size, the same every time – and goes to PNG files or an external `ffmpeg`.
//!
//! - [`theme`]   colour schemes, read from the app's `styles.css`
//! - [`raster`]  RGBA canvas, glyphs (bundled JetBrains Mono, optional fallbacks)
//! - [`render`]  one editor frame as an image (window, gutter, text, selection, carets, captions)
//! - [`video`]   the virtual clock, smooth scrolling, sinks (PNG, ffmpeg, hash)
//! - [`cli`]     `editlicious render …`

pub mod cli;
pub mod raster;
pub mod render;
pub mod theme;
pub mod video;
