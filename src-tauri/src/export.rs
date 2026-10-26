//! "Export as video…" in the app (design 017 E7): the same renderer as `editlicious render`,
//! on a thread, progress over a channel, cancellable.

use crate::document::registry::lock_doc;
use crate::editor::Session;
use crate::editor::playback::{self, Tempo};
use crate::error::{FsError, FsErrorCode};
use crate::playback::{Source, load_path};
use ed_video::render::{Chrome, Look};
use ed_video::video::{self, Ffmpeg, Job, PngDir, Sink};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    /// `.mp4`/`.mov`/`.webm` (ffmpeg) or a folder for PNG frames.
    pub path: String,
    pub width: u32,
    pub height: u32,
    #[serde(default = "sixty")]
    pub fps: u32,
    pub theme: String,
    #[serde(default)]
    pub font_size: Option<f32>,
    #[serde(default)]
    pub chrome: Chrome,
    #[serde(default)]
    pub best: bool,
    #[serde(default)]
    pub tempo: Option<Tempo>,
    /// Empty = `ffmpeg` on the PATH.
    #[serde(default)]
    pub ffmpeg: String,
}

fn sixty() -> u32 {
    60
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ExportEvent {
    Progress {
        done: u64,
        total: u64,
    },
    /// `reason`: `finished`, `cancelled` or an error code.
    Done {
        reason: String,
    },
}

/// The job: steps of the source, the text it starts on, the document's name and the look.
pub fn job(session: &Session, source: &Source, o: &ExportOptions) -> Result<Job, FsError> {
    let (items, start_text, default_tempo) = match source {
        Source::Last => {
            let rec = session
                .last_recording()
                .ok_or_else(|| FsError::new(FsErrorCode::NoRecording, "no recording", None))?;
            let start = rec
                .start_text
                .as_ref()
                .map(|r| r.to_string())
                .unwrap_or_default();
            (playback::items_of(rec, true)?, start, Tempo::Original)
        }
        Source::Path { path } => {
            let loaded = load_path(&PathBuf::from(path))?;
            // a recording made on this very text starts from it; a script on an empty one
            let start = match &loaded.recording {
                Some(r) => {
                    let now = crate::editor::recording::stamp(&session.doc);
                    if now.hash == r.header.doc.hash {
                        session.doc.rope().to_string()
                    } else {
                        String::new()
                    }
                }
                None => String::new(),
            };
            (loaded.items, start, loaded.default_tempo)
        }
    };
    let theme = ed_video::theme::theme(&o.theme).ok_or_else(|| {
        FsError::new(
            FsErrorCode::InvalidRequest,
            format!("unknown theme {}", o.theme),
            None,
        )
    })?;
    let font_size = o
        .font_size
        .unwrap_or(ed_video::render::default_font_size(o.width, o.height));
    Ok(Job {
        steps: playback::timeline(&items, o.tempo.unwrap_or(default_tempo)),
        start_text,
        name: Some(session.doc.name()),
        language: None,
        theme,
        look: Look {
            width: o.width,
            height: o.height,
            font_size,
            chrome: o.chrome,
            ..Look::default()
        },
        fps: o.fps,
        hold_start: 1000,
        hold_end: 2000,
        blink: true,
        tab_width: session.indentation().0,
    })
}

/// Renders on the calling thread; `cancel` stops it.
pub fn run(
    job: Job,
    o: &ExportOptions,
    cancel: Arc<AtomicBool>,
    send: impl Fn(ExportEvent),
) -> Result<(), FsError> {
    let path = PathBuf::from(&o.path);
    let mut sink: Box<dyn Sink> = if path.extension().is_none() {
        Box::new(PngDir::new(&path, o.width, o.height)?)
    } else {
        let ff = if o.ffmpeg.trim().is_empty() {
            "ffmpeg"
        } else {
            o.ffmpeg.trim()
        };
        Box::new(Ffmpeg::start(
            ff,
            &path,
            o.width,
            o.height,
            job.fps.clamp(1, 120),
            o.best,
        )?)
    };
    let mut last = 0u64;
    let r = video::render(&job, sink.as_mut(), &mut |done, total| {
        if done == total || done > last + total / 200 {
            last = done;
            send(ExportEvent::Progress { done, total });
        }
        !cancel.load(Ordering::SeqCst)
    });
    if r.is_err() && path.extension().is_some() {
        drop(sink);
        let _ = std::fs::remove_file(&path);
    }
    r.map(|_| ())
}

/// Builds the job from a document under its lock (cheap: copies of the rope).
pub fn job_of(
    handle: &crate::document::registry::Handle<Session>,
    source: &Source,
    o: &ExportOptions,
) -> Result<Job, FsError> {
    job(&lock_doc(handle), source, o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::editor::MemoryClipboard;
    use crate::editor::model::Command;
    use std::sync::Mutex;

    fn opts(path: &std::path::Path) -> ExportOptions {
        ExportOptions {
            path: path.to_string_lossy().into_owned(),
            width: 160,
            height: 120,
            fps: 5,
            theme: "ink".into(),
            font_size: Some(10.0),
            chrome: Chrome::None,
            best: false,
            tempo: Some(Tempo::Even { ms: 50 }),
            ffmpeg: String::new(),
        }
    }

    #[test]
    fn exports_the_last_recording_from_its_start_text_as_png_frames() {
        let mut s = Session::new(Document::from_text("start"), 1);
        let mut clip = MemoryClipboard::default();
        s.start_recording("t");
        s.run(Command::Type { text: "!".into() }, &mut clip)
            .unwrap();
        s.stop_recording();
        s.run(
            Command::Type {
                text: "later".into(),
            },
            &mut clip,
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("frames");
        let o = opts(&out);
        let j = job(&s, &Source::Last, &o).unwrap();
        assert_eq!(j.start_text, "start");
        let events = Mutex::new(Vec::new());
        run(j, &o, Arc::default(), |e| events.lock().unwrap().push(e)).unwrap();
        let n = std::fs::read_dir(&out).unwrap().count();
        assert!(n > 10, "{n} frames");
        assert!(matches!(
            events.lock().unwrap().last(),
            Some(ExportEvent::Progress { done, total }) if done == total
        ));
        // cancelled
        let cancel = Arc::new(AtomicBool::new(true));
        let e = run(job(&s, &Source::Last, &o).unwrap(), &o, cancel, |_| {}).unwrap_err();
        assert_eq!(e.code, FsErrorCode::Cancelled);
        let e = job(
            &s,
            &Source::Last,
            &ExportOptions {
                theme: "x".into(),
                ..o
            },
        )
        .err()
        .unwrap();
        assert_eq!(e.code, FsErrorCode::InvalidRequest);
    }
}
