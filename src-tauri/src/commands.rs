//! Thin Tauri command layer: validate the input, run blocking work on `spawn_blocking`, map
//! errors to `FsError`. Logic lives in pure modules. Every command is `async` – a synchronous
//! command would run on the main thread and freeze the window on a slow disk. The test
//! [`command_rules`] checks this.

use crate::about;
use crate::document::model::{DocInfo, LineEnding, LoadEvent, SaveRequest};
use crate::document::registry::{Registry, lock_doc};
use crate::document::{Document, load};
use crate::editor::model::{Command, Frame};
use crate::editor::{Clipboard, Session};
use crate::error::{FsError, FsErrorCode};
use crate::model::AppInfo;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

/// App-wide state: the editor sessions (one per open document, design 003) and the clipboard.
#[derive(Default)]
pub struct AppState {
    sessions: Arc<Registry<Session>>,
    clipboard: Arc<Mutex<SystemClipboard>>,
}

/// The system clipboard (design 003 E5): one long-lived `arboard` handle – on Linux the text
/// lives only as long as the handle.
#[derive(Default)]
pub struct SystemClipboard(Option<arboard::Clipboard>);

impl SystemClipboard {
    fn handle(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.0.is_none() {
            self.0 = arboard::Clipboard::new().ok();
        }
        self.0.as_mut()
    }
}

impl Clipboard for SystemClipboard {
    fn get(&mut self) -> Option<String> {
        self.handle()?.get_text().ok()
    }

    fn set(&mut self, text: String) -> bool {
        self.handle().is_some_and(|c| c.set_text(text).is_ok())
    }
}

/// Runs blocking work off the async runtime.
async fn blocking<T, F>(f: F) -> Result<T, FsError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, FsError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| FsError::new(FsErrorCode::Io, e.to_string(), None))?
}

/// Name, version (from `package.json` via `tauri.conf.json`) and platform for the About dialog.
#[tauri::command]
pub async fn get_app_info(app: AppHandle) -> Result<AppInfo, FsError> {
    let pkg = app.package_info();
    Ok(about::app_info("editlicious", &pkg.version.to_string()))
}

/// A path from the frontend: absolute, not empty, no NUL.
fn file_path(path: &str) -> Result<PathBuf, FsError> {
    let p = PathBuf::from(path);
    if path.is_empty() || path.contains('\0') || !p.is_absolute() {
        return Err(FsError::new(FsErrorCode::InvalidPath, "invalid path", None));
    }
    Ok(p)
}

fn info_of(sessions: &Registry<Session>, id: u64) -> Result<DocInfo, FsError> {
    let handle = sessions.get(id)?;
    let s = lock_doc(&handle);
    Ok(s.doc.info(id))
}

/// Opens a text file (design 002 E4) in a new editor session; progress goes to `on_event`,
/// `cancel_load(load_id)` stops it.
#[tauri::command]
pub async fn open_document(
    state: State<'_, AppState>,
    path: String,
    load_id: u64,
    on_event: Channel<LoadEvent>,
) -> Result<DocInfo, FsError> {
    let path = file_path(&path)?;
    let sessions = state.sessions.clone();
    blocking(move || {
        let cancel = sessions.start_load(load_id);
        let result = load::load_file(&path, &cancel, &mut |loaded, total| {
            let _ = on_event.send(LoadEvent::Progress { loaded, total });
        });
        sessions.finish_load(load_id);
        let doc = Document::from_loaded(result?, &path);
        let id = sessions.reserve_id();
        sessions.insert(id, Session::new(doc, id));
        info_of(&sessions, id)
    })
    .await
}

#[tauri::command]
pub async fn cancel_load(state: State<'_, AppState>, load_id: u64) -> Result<(), FsError> {
    state.sessions.cancel_load(load_id);
    Ok(())
}

/// A new, empty document in its own session.
#[tauri::command]
pub async fn new_document(state: State<'_, AppState>) -> Result<DocInfo, FsError> {
    let id = state.sessions.reserve_id();
    let doc = Document::new_empty(if cfg!(windows) {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    });
    state.sessions.insert(id, Session::new(doc, id));
    info_of(&state.sessions, id)
}

#[tauri::command]
pub async fn close_document(state: State<'_, AppState>, id: u64) -> Result<(), FsError> {
    state.sessions.remove(id);
    Ok(())
}

#[tauri::command]
pub async fn document_info(state: State<'_, AppState>, id: u64) -> Result<DocInfo, FsError> {
    let sessions = state.sessions.clone();
    blocking(move || info_of(&sessions, id)).await
}

/// Runs one editor command (design 003) and returns the frame to draw.
#[tauri::command]
pub async fn editor_command(
    state: State<'_, AppState>,
    id: u64,
    cmd: Command,
) -> Result<Frame, FsError> {
    let handle = state.sessions.get(id)?;
    let clipboard = state.clipboard.clone();
    blocking(move || {
        let mut session = lock_doc(&handle);
        let mut clip = clipboard.lock().unwrap_or_else(|e| e.into_inner());
        session.run(cmd, &mut *clip)
    })
    .await
}

/// Saves (design 002 E8); "Save as" passes a path, "Save with encoding" an encoding.
#[tauri::command]
pub async fn save_document(
    state: State<'_, AppState>,
    id: u64,
    req: SaveRequest,
) -> Result<DocInfo, FsError> {
    if let Some(p) = &req.path {
        file_path(p)?;
    }
    let handle = state.sessions.get(id)?;
    blocking(move || {
        let mut s = lock_doc(&handle);
        s.doc.save(&req)?;
        Ok(s.doc.info(id))
    })
    .await
}

/// Files given on the command line (open with …, `editlicious file.txt`).
#[tauri::command]
pub async fn startup_paths() -> Result<Vec<String>, FsError> {
    Ok(std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .filter_map(|a| {
            let p = PathBuf::from(&a);
            let p = if p.is_absolute() {
                p
            } else {
                std::env::current_dir().ok()?.join(p)
            };
            p.is_file().then(|| p.to_string_lossy().into_owned())
        })
        .collect())
}

#[cfg(test)]
mod command_rules {
    /// Every `#[tauri::command]` in this file is an `async fn`.
    #[test]
    fn commands_are_async() {
        let src = include_str!("commands.rs");
        let mut lines = src.lines();
        let mut count = 0;
        while let Some(line) = lines.next() {
            if line.trim() == "#[tauri::command]" {
                let next = lines.next().unwrap_or_default().trim();
                assert!(
                    next.starts_with("pub async fn"),
                    "synchronous command: {next}"
                );
                count += 1;
            }
        }
        assert!(count >= 9);
    }
}
