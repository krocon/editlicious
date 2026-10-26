//! The video (design 017 E1/E4/E5): a timeline from 015 runs on a **virtual clock** through a
//! real editor session; every video frame shows the state at its time – with a blinking caret,
//! smooth scrolling and captions – and goes to a sink (PNG files, `ffmpeg`, a hash in tests).

use crate::raster::{Canvas, Fonts};
use crate::render::{self, Look};
use crate::theme::Theme;
use ed_core::document::Document;
use ed_core::editor::model::{Command, Frame};
use ed_core::editor::playback::{Playback, Step};
use ed_core::editor::{MemoryClipboard, Session};
use ed_core::error::{FsError, FsErrorCode};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as Process, Stdio};

/// How long a caption stays (ms).
pub const CAPTION_MS: u64 = 1600;
/// Caret blink: on, off (ms); after a change it is on.
pub const BLINK_MS: u64 = 530;

/// What to render.
pub struct Job {
    pub steps: Vec<Step>,
    /// Text the document starts with.
    pub start_text: String,
    /// File name for the language (`example.rs`) and the title.
    pub name: Option<String>,
    /// Language of the highlighting by name (`Rust`); `None` = from `name`.
    pub language: Option<String>,
    pub theme: Theme,
    pub look: Look,
    pub fps: u32,
    /// Still picture before and after (ms).
    pub hold_start: u64,
    pub hold_end: u64,
    pub blink: bool,
    pub tab_width: usize,
}

/// Where frames go.
pub trait Sink {
    fn frame(&mut self, rgba: &[u8]) -> Result<(), FsError>;
    fn finish(&mut self) -> Result<(), FsError>;
}

/// What a render did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub frames: u64,
    /// Frames that had to be drawn (the others repeated the one before).
    pub drawn: u64,
    pub duration_ms: u64,
}

/// Number of video frames for a duration.
pub fn frame_count(duration_ms: u64, fps: u32) -> u64 {
    duration_ms * fps as u64 / 1000 + 1
}

/// Renders `job` into `sink`; `progress(done, total)` returns `false` to cancel.
pub fn render(
    job: &Job,
    sink: &mut dyn Sink,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<Stats, FsError> {
    let look = &job.look;
    if look.width < 64 || look.height < 64 || look.width > 8192 || look.height > 8192 {
        return Err(FsError::new(
            FsErrorCode::InvalidRequest,
            "the size must be between 64 and 8192 px",
            None,
        ));
    }
    let fps = job.fps.clamp(1, 120);
    let mut fonts = Fonts::new(None, &[], look.font_size)?;
    render_with(job, fps, &mut fonts, sink, progress)
}

/// Like [`render`] with fonts chosen by the caller (`--font`, fallbacks).
pub fn render_with(
    job: &Job,
    fps: u32,
    fonts: &mut Fonts,
    sink: &mut dyn Sink,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<Stats, FsError> {
    let look = &job.look;
    let mut doc = Document::from_text(&job.start_text);
    if let Some(n) = &job.name {
        doc.path = Some(PathBuf::from(n));
    }
    let mut session = Session::new(doc, 1);
    let mut clip = MemoryClipboard::default();
    if let Some(lang) = &job.language {
        session.run(
            Command::SetLanguage {
                name: Some(lang.clone()),
            },
            &mut clip,
        )?;
    }
    session.run(
        Command::Configure {
            tab_width: Some(job.tab_width),
            insert_spaces: Some(true),
            detect: false,
            wrap: Some(false),
            colors: None,
            expand_weights: None,
        },
        &mut clip,
    )?;
    let size_for = |lines: usize, fonts: &Fonts| {
        let l = render::layout(look, fonts, lines);
        Command::Resize {
            lines: l.rows,
            cols: l.cols,
        }
    };
    session.run(size_for(session.doc.line_count(), fonts), &mut clip)?;
    let mut playback = Playback::new(job.steps.clone());
    let duration = job.hold_start + playback.duration_ms() + job.hold_end;
    let total = frame_count(duration, fps);
    playback.begin(&mut session);
    let mut canvas = Canvas::new(look.width, look.height);
    // the window does not change: drawn once, copied for every picture
    render::draw_window(
        &mut canvas,
        &job.look.title.clone().unwrap_or_else(|| session.doc.name()),
        &job.theme,
        fonts,
        look,
    );
    let background = canvas.pixels.clone();
    let mut last: Option<(Frame, bool, Option<String>)> = None;
    let mut last_change = 0u64;
    let mut caption: Option<(String, u64)> = None;
    let (mut shown_top, mut shown_left) = session.scroll();
    let mut drawn = 0;
    let mut gutter_digits = session.doc.line_count().to_string().len();
    for i in 0..total {
        if !progress(i, total) {
            return Err(FsError::new(FsErrorCode::Cancelled, "cancelled", None));
        }
        let t = i * 1000 / fps as u64;
        if t >= job.hold_start {
            let pt = t - job.hold_start;
            let before = playback.next;
            if playback.run_due(&mut session, pt)?.is_some() {
                last_change = t;
                for s in &playback.steps[before..playback.next] {
                    if let Some(o) = &s.overlay {
                        caption = Some((o.clone(), t));
                    }
                }
            }
        }
        // the gutter grows with the lines: fewer columns
        let digits = session.doc.line_count().to_string().len();
        if digits != gutter_digits {
            gutter_digits = digits;
            session.run(size_for(session.doc.line_count(), fonts), &mut clip)?;
        }
        // smooth scrolling towards where the engine scrolled
        let (top, left) = session.scroll();
        let rows = render::layout(look, fonts, session.doc.line_count()).rows;
        let ease = 1.0 - 0.6f64.powf(30.0 / fps as f64);
        let step = |shown: f64, target: f64, far: f64| {
            if (target - shown).abs() > far || (target - shown).abs() < 0.02 {
                target
            } else {
                shown + (target - shown) * ease
            }
        };
        shown_top = step(shown_top, top, rows);
        shown_left = step(shown_left, left, 80.0);
        let frame = session.run(
            Command::ScrollTo {
                top: Some(shown_top),
                left: Some(shown_left),
            },
            &mut clip,
        )?;
        let caret_on = !job.blink || ((t - last_change.min(t)) / BLINK_MS).is_multiple_of(2);
        if let Some((_, since)) = &caption
            && t >= since + CAPTION_MS
        {
            caption = None;
        }
        let cap = caption.as_ref().map(|(c, _)| c.clone());
        // nothing changed: the same picture again
        let same = last
            .as_ref()
            .is_some_and(|(f, c, o)| *c == caret_on && *o == cap && same_picture(f, &frame));
        if !same {
            canvas.pixels.copy_from_slice(&background);
            render::draw_editor(
                &mut canvas,
                &frame,
                &job.theme,
                fonts,
                look,
                caret_on,
                cap.as_deref(),
            );
            drawn += 1;
        }
        sink.frame(canvas.rgba())?;
        last = Some((frame, caret_on, cap));
    }
    playback.end(&mut session);
    sink.finish()?;
    progress(total, total);
    Ok(Stats {
        frames: total,
        drawn,
        duration_ms: duration,
    })
}

/// Two frames that look the same (the sequence number does not show).
fn same_picture(a: &Frame, b: &Frame) -> bool {
    let mut a = a.clone();
    a.seq = b.seq;
    a.notice = b.notice.clone();
    a == *b
}

// ---------------------------------------------------------------- sinks

/// Numbered PNG files `frame-000001.png` in a folder.
pub struct PngDir {
    dir: PathBuf,
    width: u32,
    height: u32,
    n: u64,
}

impl PngDir {
    pub fn new(dir: &Path, width: u32, height: u32) -> Result<Self, FsError> {
        std::fs::create_dir_all(dir).map_err(|e| FsError::from_io(&e, dir))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            width,
            height,
            n: 0,
        })
    }
}

/// Encodes RGBA as PNG bytes.
pub fn png_bytes(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, FsError> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc
            .write_header()
            .map_err(|e| FsError::new(FsErrorCode::Io, e.to_string(), None))?;
        w.write_image_data(rgba)
            .map_err(|e| FsError::new(FsErrorCode::Io, e.to_string(), None))?;
    }
    Ok(out)
}

impl Sink for PngDir {
    fn frame(&mut self, rgba: &[u8]) -> Result<(), FsError> {
        self.n += 1;
        let path = self.dir.join(format!("frame-{:06}.png", self.n));
        let bytes = png_bytes(rgba, self.width, self.height)?;
        std::fs::write(&path, bytes).map_err(|e| FsError::from_io(&e, &path))
    }
    fn finish(&mut self) -> Result<(), FsError> {
        Ok(())
    }
}

/// Raw frames into an external `ffmpeg` (design 017 E4) – no encoder inside the app.
pub struct Ffmpeg {
    child: Child,
    out: PathBuf,
}

/// Arguments for an output file by its extension: `.mp4` H.264, `.mov` ProRes, `.webm` VP9.
pub fn ffmpeg_args(out: &Path, width: u32, height: u32, fps: u32, best: bool) -> Vec<String> {
    let mut a: Vec<String> = [
        "-y",
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-s",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    a.push(format!("{width}x{height}"));
    a.push("-r".into());
    a.push(fps.to_string());
    a.extend(["-i", "-"].map(String::from));
    let ext = out
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let codec: &[&str] = match ext.as_str() {
        "mov" => &[
            "-c:v",
            "prores_ks",
            "-profile:v",
            "3",
            "-pix_fmt",
            "yuv422p10le",
        ],
        "webm" => &[
            "-c:v",
            "libvpx-vp9",
            "-b:v",
            "0",
            "-crf",
            "30",
            "-pix_fmt",
            "yuv420p",
        ],
        _ => {
            if best {
                &[
                    "-c:v",
                    "libx264",
                    "-preset",
                    "slow",
                    "-crf",
                    "14",
                    "-pix_fmt",
                    "yuv420p",
                    "-movflags",
                    "+faststart",
                ]
            } else {
                &[
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-crf",
                    "18",
                    "-pix_fmt",
                    "yuv420p",
                    "-movflags",
                    "+faststart",
                ]
            }
        }
    };
    a.extend(codec.iter().map(|s| s.to_string()));
    a.push(out.to_string_lossy().into_owned());
    a
}

impl Ffmpeg {
    pub fn start(
        ffmpeg: &str,
        out: &Path,
        width: u32,
        height: u32,
        fps: u32,
        best: bool,
    ) -> Result<Self, FsError> {
        if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return Err(FsError::new(
                FsErrorCode::InvalidRequest,
                "width and height must be even for video",
                None,
            ));
        }
        let child = Process::new(ffmpeg)
            .args(ffmpeg_args(out, width, height, fps, best))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                FsError::new(
                    FsErrorCode::FfmpegMissing,
                    format!("{ffmpeg}: {e} – install ffmpeg or give its path"),
                    None,
                )
            })?;
        Ok(Self {
            child,
            out: out.to_path_buf(),
        })
    }
}

impl Sink for Ffmpeg {
    fn frame(&mut self, rgba: &[u8]) -> Result<(), FsError> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| FsError::new(FsErrorCode::Io, "ffmpeg closed its input", None))?;
        stdin.write_all(rgba).map_err(|e| {
            FsError::new(
                FsErrorCode::Io,
                format!("ffmpeg stopped: {e}"),
                Some(&self.out),
            )
        })
    }

    fn finish(&mut self) -> Result<(), FsError> {
        drop(self.child.stdin.take());
        let out = self
            .child
            .wait_with_output_ref()
            .map_err(|e| FsError::from_io(&e, &self.out))?;
        if out.0.success() {
            Ok(())
        } else {
            Err(FsError::new(
                FsErrorCode::Io,
                format!("ffmpeg failed: {}", out.1.trim()),
                Some(&self.out),
            ))
        }
    }
}

trait WaitRef {
    fn wait_with_output_ref(&mut self) -> std::io::Result<(std::process::ExitStatus, String)>;
}

impl WaitRef for Child {
    fn wait_with_output_ref(&mut self) -> std::io::Result<(std::process::ExitStatus, String)> {
        let mut err = String::new();
        if let Some(mut e) = self.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut err);
        }
        Ok((self.wait()?, err))
    }
}

impl Drop for Ffmpeg {
    fn drop(&mut self) {
        // cancelled: do not leave ffmpeg running
        if self.child.stdin.is_some() {
            drop(self.child.stdin.take());
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// FNV-1a over every frame – the determinism test and `--hash`.
#[derive(Debug, Default)]
pub struct HashSink {
    pub hash: u64,
    pub frames: u64,
    /// Keeps the frames with these numbers (golden images).
    pub keep: Vec<u64>,
    pub kept: Vec<Vec<u8>>,
}

impl Sink for HashSink {
    fn frame(&mut self, rgba: &[u8]) -> Result<(), FsError> {
        if self.frames == 0 {
            self.hash = 0xcbf2_9ce4_8422_2325;
        }
        for b in rgba {
            self.hash ^= *b as u64;
            self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        if self.keep.contains(&self.frames) {
            self.kept.push(rgba.to_vec());
        }
        self.frames += 1;
        Ok(())
    }
    fn finish(&mut self) -> Result<(), FsError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;
    use ed_core::editor::playback::{Item, Tempo, timeline};
    use ed_core::editor::script;

    fn job(script_text: &str, theme_name: &str, w: u32, h: u32) -> Job {
        let items = script::parse(script_text, &mut |_| unreachable!()).unwrap();
        Job {
            steps: timeline(&items, Tempo::Original),
            start_text: String::new(),
            name: Some("demo.rs".into()),
            language: None,
            theme: theme::theme(theme_name).unwrap(),
            look: Look {
                width: w,
                height: h,
                font_size: 14.0,
                padding: 12,
                ..Look::default()
            },
            fps: 10,
            hold_start: 200,
            hold_end: 300,
            blink: true,
            tab_width: 4,
        }
    }

    const SCRIPT: &str = "tempo even 50\ntype \"fn main() {\\n    let x = 1; // 中文 🙂\\n}\"\nkeys \"Ctrl+D\"\ndo {\"type\":\"duplicateLines\"}\npause 200";

    #[test]
    fn the_same_job_gives_the_same_pictures() {
        let run = |theme: &str| {
            let mut sink = HashSink::default();
            let stats = render(&job(SCRIPT, theme, 320, 180), &mut sink, &mut |_, _| true).unwrap();
            (sink.hash, stats)
        };
        let (a, stats) = run("paper");
        assert_eq!(run("paper").0, a);
        assert_ne!(run("ink").0, a);
        assert_eq!(stats.frames, frame_count(stats.duration_ms, 10));
        assert!(stats.drawn < stats.frames, "still frames are reused");
    }

    #[test]
    fn the_virtual_clock_shows_the_state_at_its_time() {
        // a script of 5 characters at 100 ms each: at 10 fps frame k shows k-2 characters
        let items = vec![Item::Type("abcde".into())];
        let mut j = job("", "paper", 160, 120);
        j.steps = timeline(&items, Tempo::Even { ms: 100 });
        j.blink = false;
        let mut sink = HashSink {
            keep: vec![0, 2, 3, 7],
            ..Default::default()
        };
        render(&j, &mut sink, &mut |_, _| true).unwrap();
        assert_eq!(sink.kept.len(), 4);
        assert_eq!(
            sink.kept[0], sink.kept[1],
            "hold: nothing typed yet at 200 ms"
        );
        assert_ne!(sink.kept[1], sink.kept[2], "300 ms: 'a' typed");
        assert_eq!(sink.frames, frame_count(200 + 500 + 300, 10));
    }

    #[test]
    fn cancel_and_bad_sizes() {
        let e = render(
            &job(SCRIPT, "paper", 320, 180),
            &mut HashSink::default(),
            &mut |i, _| i < 3,
        )
        .unwrap_err();
        assert_eq!(e.code, FsErrorCode::Cancelled);
        let e = render(
            &job(SCRIPT, "paper", 20, 180),
            &mut HashSink::default(),
            &mut |_, _| true,
        )
        .unwrap_err();
        assert_eq!(e.code, FsErrorCode::InvalidRequest);
    }

    #[test]
    fn png_frames_and_missing_ffmpeg() {
        let dir = tempfile::tempdir().unwrap();
        let mut sink = PngDir::new(&dir.path().join("f"), 160, 120).unwrap();
        let j = job("type \"x\"", "night", 160, 120);
        let stats = render(&j, &mut sink, &mut |_, _| true).unwrap();
        let n = std::fs::read_dir(dir.path().join("f")).unwrap().count() as u64;
        assert_eq!(n, stats.frames);
        let e = Ffmpeg::start(
            "/no/such/ffmpeg",
            &dir.path().join("o.mp4"),
            160,
            120,
            10,
            false,
        )
        .err()
        .unwrap();
        assert_eq!(e.code, FsErrorCode::FfmpegMissing);
        assert!(Ffmpeg::start("ffmpeg", &dir.path().join("o.mp4"), 161, 120, 10, false).is_err());
    }

    #[test]
    fn ffmpeg_arguments_follow_the_extension() {
        let a = ffmpeg_args(Path::new("/o/v.mp4"), 1080, 1920, 60, false).join(" ");
        assert!(
            a.contains("-s 1080x1920 -r 60 -i - -c:v libx264 -preset veryfast"),
            "{a}"
        );
        assert!(
            ffmpeg_args(Path::new("v.mp4"), 2, 2, 30, true)
                .join(" ")
                .contains("-preset slow")
        );
        assert!(
            ffmpeg_args(Path::new("v.mov"), 2, 2, 30, false)
                .join(" ")
                .contains("prores_ks")
        );
        assert!(
            ffmpeg_args(Path::new("v.webm"), 2, 2, 30, false)
                .join(" ")
                .contains("libvpx-vp9")
        );
    }

    /// Golden images (design 017 tests): key frames light/dark with CJK/emoji and a selection,
    /// compared with a tolerance. `UPDATE_GOLDEN=1` writes them anew.
    #[test]
    fn golden_images() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
        for theme_name in ["paper", "night"] {
            let script = "tempo instant\ntype \"fn main() {\\n    let s = \\\"中文 🙂 é\\\";\\n}\"\nselect 2:9 2:22\nkeys \"Ctrl+D\"";
            let mut j = job(script, theme_name, 400, 200);
            j.hold_start = 0;
            j.hold_end = 0;
            j.blink = false;
            let mut sink = HashSink {
                keep: vec![0],
                ..Default::default()
            };
            render(&j, &mut sink, &mut |_, _| true).unwrap();
            let got = &sink.kept[0];
            let file = dir.join(format!("golden-{theme_name}.png"));
            if std::env::var_os("UPDATE_GOLDEN").is_some() || !file.exists() {
                std::fs::write(&file, png_bytes(got, 400, 200).unwrap()).unwrap();
            }
            let decoder =
                png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&file).unwrap()));
            let mut reader = decoder.read_info().unwrap();
            let mut want = vec![0; reader.output_buffer_size().unwrap()];
            reader.next_frame(&mut want).unwrap();
            let diff: u64 = got
                .iter()
                .zip(&want)
                .map(|(a, b)| a.abs_diff(*b) as u64)
                .sum();
            let mean = diff as f64 / got.len() as f64;
            assert!(mean < 1.5, "{theme_name}: mean difference {mean}");
        }
    }
}
