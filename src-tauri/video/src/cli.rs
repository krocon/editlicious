//! `editlicious render <script|recording> -o <out.mp4|folder/> [options]` (design 017 E6) –
//! for video pipelines: the same script gives the same video, in any size and scheme.

use crate::raster::{Fonts, read_font};
use crate::render::{Chrome, Look, default_font_size};
use crate::theme;
use crate::video::{self, Ffmpeg, HashSink, Job, PngDir, Sink};
use ed_core::editor::playback::{self, Tempo};
use ed_core::editor::recording::Recording;
use ed_core::editor::script;
use ed_core::error::{FsError, FsErrorCode};
use std::path::{Path, PathBuf};

pub const USAGE: &str = "\
usage: editlicious render <script.edscript | recording.edrec.jsonl> -o <out.mp4|out.mov|out.webm|folder/> [options]

  --size WxH          picture size (default 1920x1080; 1080x1920 for shorts)
  --fps N             frames per second (default 60)
  --theme NAME        colour scheme of the app (paper, ink, sand, night …; default paper)
  --font-size PX      font size in px (default: about 60 columns, 34 lines)
  --font FILE         main font (default: JetBrains Mono, built in)
  --fallback-font F   font for characters the main font lacks (CJK, emoji); repeatable
  --chrome KIND       none | plain | mac (default plain)
  --padding PX        space around the window (default 48)
  --title TEXT        window title (default: the file name)
  --no-line-numbers   --no-blink   --no-current-line   --no-guides
  --hold S,E          still picture at start and end in seconds (default 1,2)
  --start FILE        text the document starts with (default: empty)
  --name NAME         file name for the language and title (default: from --start)
  --language NAME     language of the colours (e.g. Rust)
  --tempo T           original | instant | human | even:MS | factor:X (default: as recorded / scripted)
  --seed N            seed for --tempo human
  --tab-width N       (default 4)
  --best              slower, better H.264
  --ffmpeg PATH       ffmpeg to use (default: ffmpeg on the PATH)
  --hash              print a hash of all frames instead of writing (tests)
";

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub input: PathBuf,
    pub out: Option<PathBuf>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub theme: String,
    pub font_size: Option<f32>,
    pub font: Option<PathBuf>,
    pub fallback_fonts: Vec<PathBuf>,
    pub chrome: Chrome,
    pub padding: u32,
    pub title: Option<String>,
    pub line_numbers: bool,
    pub blink: bool,
    pub current_line: bool,
    pub guides: bool,
    pub hold: (f64, f64),
    pub start: Option<PathBuf>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub tempo: Option<Tempo>,
    pub seed: Option<u64>,
    pub tab_width: usize,
    pub best: bool,
    pub ffmpeg: String,
    pub hash: bool,
}

fn bad(msg: impl Into<String>) -> FsError {
    FsError::new(FsErrorCode::InvalidRequest, msg, None)
}

pub fn parse_tempo(s: &str) -> Result<Tempo, FsError> {
    let (k, v) = s.split_once(':').unwrap_or((s, ""));
    let num = |v: &str| v.parse::<f64>().map_err(|_| bad(format!("bad tempo {s}")));
    Ok(match k {
        "original" => Tempo::Original,
        "instant" => Tempo::Instant,
        "human" => Tempo::human(),
        "even" => Tempo::Even {
            ms: num(v)?.clamp(0.0, 10_000.0) as u64,
        },
        "factor" => Tempo::Factor {
            factor: num(v)?.clamp(0.25, 8.0),
        },
        _ => return Err(bad(format!("unknown tempo {s}"))),
    })
}

pub fn parse(args: &[String]) -> Result<Args, FsError> {
    let mut a = Args {
        input: PathBuf::new(),
        out: None,
        width: 1920,
        height: 1080,
        fps: 60,
        theme: "paper".into(),
        font_size: None,
        font: None,
        fallback_fonts: Vec::new(),
        chrome: Chrome::Plain,
        padding: 48,
        title: None,
        line_numbers: true,
        blink: true,
        current_line: true,
        guides: true,
        hold: (1.0, 2.0),
        start: None,
        name: None,
        language: None,
        tempo: None,
        seed: None,
        tab_width: 4,
        best: false,
        ffmpeg: "ffmpeg".into(),
        hash: false,
    };
    let mut input = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut val = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| bad(format!("{name} needs a value")))
        };
        let num = |v: String, name: &str| {
            v.parse::<f64>()
                .map_err(|_| bad(format!("{name}: not a number: {v}")))
        };
        match arg.as_str() {
            "-o" | "--out" => a.out = Some(PathBuf::from(val("-o")?)),
            "--size" => {
                let v = val("--size")?;
                let (w, h) = v.split_once(['x', 'X']).ok_or_else(|| bad("--size WxH"))?;
                a.width = w.parse().map_err(|_| bad("--size WxH"))?;
                a.height = h.parse().map_err(|_| bad("--size WxH"))?;
            }
            "--fps" => a.fps = num(val("--fps")?, "--fps")? as u32,
            "--theme" => a.theme = val("--theme")?,
            "--font-size" => a.font_size = Some(num(val("--font-size")?, "--font-size")? as f32),
            "--font" => a.font = Some(PathBuf::from(val("--font")?)),
            "--fallback-font" => a
                .fallback_fonts
                .push(PathBuf::from(val("--fallback-font")?)),
            "--chrome" => {
                a.chrome = match val("--chrome")?.as_str() {
                    "none" => Chrome::None,
                    "plain" => Chrome::Plain,
                    "mac" => Chrome::Mac,
                    c => return Err(bad(format!("unknown chrome {c}"))),
                }
            }
            "--padding" => a.padding = num(val("--padding")?, "--padding")? as u32,
            "--title" => a.title = Some(val("--title")?),
            "--no-line-numbers" => a.line_numbers = false,
            "--no-blink" => a.blink = false,
            "--no-current-line" => a.current_line = false,
            "--no-guides" => a.guides = false,
            "--hold" => {
                let v = val("--hold")?;
                let (s, e) = v.split_once(',').unwrap_or((&v, &v));
                a.hold = (num(s.into(), "--hold")?, num(e.into(), "--hold")?);
            }
            "--start" => a.start = Some(PathBuf::from(val("--start")?)),
            "--name" => a.name = Some(val("--name")?),
            "--language" => a.language = Some(val("--language")?),
            "--tempo" => a.tempo = Some(parse_tempo(&val("--tempo")?)?),
            "--seed" => a.seed = Some(num(val("--seed")?, "--seed")? as u64),
            "--tab-width" => a.tab_width = num(val("--tab-width")?, "--tab-width")? as usize,
            "--best" => a.best = true,
            "--ffmpeg" => a.ffmpeg = val("--ffmpeg")?,
            "--hash" => a.hash = true,
            s if s.starts_with('-') => return Err(bad(format!("unknown option {s}"))),
            s => {
                if input.is_some() {
                    return Err(bad("only one script or recording"));
                }
                input = Some(PathBuf::from(s));
            }
        }
    }
    a.input = input.ok_or_else(|| bad("a script or recording is missing"))?;
    if a.out.is_none() && !a.hash {
        return Err(bad("-o is missing"));
    }
    Ok(a)
}

fn read_text(p: &Path) -> Result<String, FsError> {
    std::fs::read_to_string(p).map_err(|e| FsError::from_io(&e, p))
}

/// Builds the job of the arguments: script or recording, start text, theme, look.
pub fn job(a: &Args) -> Result<Job, FsError> {
    let text = read_text(&a.input)?;
    let is_script =
        a.input.extension().is_some_and(|e| e == "edscript") || !text.trim_start().starts_with('{');
    let (items, mut name) = if is_script {
        let dir = a.input.parent().map(Path::to_path_buf).unwrap_or_default();
        let items = script::parse(&text, &mut |f| read_text(&dir.join(f)))?;
        (items, None)
    } else {
        let rec = Recording::from_jsonl(&text)?;
        let name = Some(rec.header.doc.name.clone());
        (playback::items_of(&rec, true)?, name)
    };
    let start_text = match &a.start {
        Some(p) => {
            name = p.file_name().map(|n| n.to_string_lossy().into_owned());
            read_text(p)?
        }
        None => String::new(),
    };
    if a.name.is_some() {
        name = a.name.clone();
    }
    let mut tempo = a.tempo.unwrap_or(Tempo::Original);
    if let (Tempo::Human { seed, .. }, Some(s)) = (&mut tempo, a.seed) {
        *seed = s;
    }
    let theme = theme::theme(&a.theme).ok_or_else(|| {
        bad(format!(
            "unknown theme {} – known: {}",
            a.theme,
            theme::names().join(", ")
        ))
    })?;
    Ok(Job {
        steps: playback::timeline(&items, tempo),
        start_text,
        name,
        language: a.language.clone(),
        theme,
        look: Look {
            width: a.width,
            height: a.height,
            font_size: a.font_size.unwrap_or(default_font_size(a.width, a.height)),
            chrome: a.chrome,
            padding: a.padding,
            line_numbers: a.line_numbers,
            indent_guides: a.guides,
            current_line: a.current_line,
            title: a.title.clone(),
        },
        fps: a.fps,
        hold_start: (a.hold.0.max(0.0) * 1000.0) as u64,
        hold_end: (a.hold.1.max(0.0) * 1000.0) as u64,
        blink: a.blink,
        tab_width: a.tab_width.clamp(1, 16),
    })
}

/// Runs a render from the arguments; prints progress to stderr.
pub fn run(args: &[String]) -> Result<String, FsError> {
    let a = parse(args)?;
    let job = job(&a)?;
    let main_font = a.font.as_deref().map(read_font).transpose()?;
    let fallbacks = a
        .fallback_fonts
        .iter()
        .map(|p| read_font(p))
        .collect::<Result<Vec<_>, _>>()?;
    let mut fonts = Fonts::new(main_font.as_deref(), &fallbacks, job.look.font_size)?;
    let fps = a.fps.clamp(1, 120);
    let mut last_pct = u64::MAX;
    let mut progress = |done: u64, total: u64| {
        let pct = done * 100 / total.max(1);
        if pct != last_pct {
            last_pct = pct;
            eprint!("\rrendering {pct:3} %");
        }
        true
    };
    if a.hash {
        let mut sink = HashSink::default();
        let s = video::render_with(&job, fps, &mut fonts, &mut sink, &mut progress)?;
        eprintln!();
        return Ok(format!("{:016x} {} frames", sink.hash, s.frames));
    }
    let out = a.out.clone().unwrap_or_default();
    let to_folder = out.extension().is_none() || out.to_string_lossy().ends_with('/');
    let mut sink: Box<dyn Sink> = if to_folder {
        Box::new(PngDir::new(&out, job.look.width, job.look.height)?)
    } else {
        Box::new(Ffmpeg::start(
            &a.ffmpeg,
            &out,
            job.look.width,
            job.look.height,
            fps,
            a.best,
        )?)
    };
    let s = video::render_with(&job, fps, &mut fonts, sink.as_mut(), &mut progress)?;
    eprintln!();
    Ok(format!(
        "{} – {} frames, {:.1} s",
        out.display(),
        s.frames,
        s.duration_ms as f64 / 1000.0
    ))
}

/// Entry of `editlicious render …` and `editlicious-render …`: exit code.
pub fn main(args: &[String]) -> i32 {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return if args.is_empty() { 2 } else { 0 };
    }
    match run(args) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("editlicious render: {}", e.message);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_the_options() {
        let a = parse(&s(&[
            "x.edscript",
            "-o",
            "out.mp4",
            "--size",
            "1080x1920",
            "--theme",
            "night",
            "--chrome",
            "mac",
            "--tempo",
            "even:40",
            "--hold",
            "0.5,1",
            "--no-blink",
            "--fps",
            "30",
        ]))
        .unwrap();
        assert_eq!((a.width, a.height, a.fps), (1080, 1920, 30));
        assert_eq!(a.chrome, Chrome::Mac);
        assert_eq!(a.tempo, Some(Tempo::Even { ms: 40 }));
        assert_eq!(a.hold, (0.5, 1.0));
        assert!(!a.blink);
        assert!(parse(&s(&["x.edscript"])).is_err(), "-o missing");
        assert!(parse(&s(&["-o", "a.mp4"])).is_err(), "input missing");
        assert!(parse(&s(&["x", "-o", "a.mp4", "--size", "big"])).is_err());
        assert!(parse(&s(&["x", "-o", "a.mp4", "--wat"])).is_err());
    }

    #[test]
    fn renders_a_script_by_hash_and_names_unknown_themes() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("a.edscript");
        std::fs::write(dir.path().join("snip.rs"), "fn x() {}").unwrap();
        std::fs::write(&script, "tempo even 30\ntype @snip.rs\n").unwrap();
        let args = |theme: &str| {
            s(&[
                script.to_str().unwrap(),
                "--hash",
                "--size",
                "200x120",
                "--fps",
                "10",
                "--theme",
                theme,
                "--hold",
                "0,0",
            ])
        };
        let a = run(&args("ink")).unwrap();
        assert_eq!(run(&args("ink")).unwrap(), a);
        assert!(a.ends_with("frames"), "{a}");
        let e = run(&args("rainbow")).unwrap_err();
        assert!(e.message.contains("paper"), "{}", e.message);
        let e = run(&s(&[
            script.to_str().unwrap(),
            "--hash",
            "--font",
            "/no/font.ttf",
        ]))
        .unwrap_err();
        assert_eq!(e.code, FsErrorCode::NotFound);
    }
}
