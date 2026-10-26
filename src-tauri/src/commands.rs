//! Thin Tauri command layer: validate the input, run blocking work on `spawn_blocking`, map
//! errors to `FsError`. Logic lives in pure modules. Every command is `async` – a synchronous
//! command would run on the main thread and freeze the window on a slow disk. The test
//! [`command_rules`] checks this.

use crate::about;
use crate::document::model::{
    DocInfo, EditRequest, EditResult, LineEnding, LoadEvent, Pos, SaveRequest, View, ViewRequest,
};
use crate::document::registry::{Registry, lock_doc};
use crate::document::{Document, load};
use crate::error::{FsError, FsErrorCode};
use crate::model::AppInfo;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

/// App-wide state: the open documents (design 002 E9).
#[derive(Default)]
pub struct AppState {
    docs: Arc<Registry>,
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

/// Opens a text file (design 002 E4); progress goes to `on_event`, `cancel_load(load_id)` stops it.
#[tauri::command]
pub async fn open_document(
    state: State<'_, AppState>,
    path: String,
    load_id: u64,
    on_event: Channel<LoadEvent>,
) -> Result<DocInfo, FsError> {
    let path = file_path(&path)?;
    let docs = state.docs.clone();
    blocking(move || {
        let cancel = docs.start_load(load_id);
        let result = load::load_file(&path, &cancel, &mut |loaded, total| {
            let _ = on_event.send(LoadEvent::Progress { loaded, total });
        });
        docs.finish_load(load_id);
        let doc = Document::from_loaded(result?, &path);
        let id = docs.insert(doc);
        let handle = docs.get(id)?;
        let info = lock_doc(&handle).info(id);
        Ok(info)
    })
    .await
}

#[tauri::command]
pub async fn cancel_load(state: State<'_, AppState>, load_id: u64) -> Result<(), FsError> {
    state.docs.cancel_load(load_id);
    Ok(())
}

/// A new, empty document.
#[tauri::command]
pub async fn new_document(state: State<'_, AppState>) -> Result<DocInfo, FsError> {
    let doc = Document::new_empty(LineEnding::Lf);
    let id = state.docs.insert(doc);
    let handle = state.docs.get(id)?;
    let info = lock_doc(&handle).info(id);
    Ok(info)
}

#[tauri::command]
pub async fn close_document(state: State<'_, AppState>, id: u64) -> Result<(), FsError> {
    state.docs.remove(id);
    Ok(())
}

#[tauri::command]
pub async fn document_info(state: State<'_, AppState>, id: u64) -> Result<DocInfo, FsError> {
    let handle = state.docs.get(id)?;
    blocking(move || Ok(lock_doc(&handle).info(id))).await
}

/// The visible part of a document (design 002 E3).
#[tauri::command]
pub async fn get_view(
    state: State<'_, AppState>,
    id: u64,
    req: ViewRequest,
) -> Result<View, FsError> {
    let handle = state.docs.get(id)?;
    blocking(move || Ok(lock_doc(&handle).view(&req))).await
}

/// Applies changes (design 002 E6) and returns the new info and selections.
#[tauri::command]
pub async fn apply_edits(
    state: State<'_, AppState>,
    id: u64,
    req: EditRequest,
) -> Result<EditResult, FsError> {
    let handle = state.docs.get(id)?;
    blocking(move || {
        let mut doc = lock_doc(&handle);
        let selections = doc.apply(req)?;
        Ok(EditResult {
            info: doc.info(id),
            selections,
        })
    })
    .await
}

#[tauri::command]
pub async fn undo(state: State<'_, AppState>, id: u64) -> Result<Option<EditResult>, FsError> {
    let handle = state.docs.get(id)?;
    blocking(move || {
        let mut doc = lock_doc(&handle);
        Ok(doc.undo().map(|selections| EditResult {
            info: doc.info(id),
            selections,
        }))
    })
    .await
}

#[tauri::command]
pub async fn redo(state: State<'_, AppState>, id: u64) -> Result<Option<EditResult>, FsError> {
    let handle = state.docs.get(id)?;
    blocking(move || {
        let mut doc = lock_doc(&handle);
        Ok(doc.redo().map(|selections| EditResult {
            info: doc.info(id),
            selections,
        }))
    })
    .await
}

/// Text between two positions (design 002 E11, at most 64 MiB).
#[tauri::command]
pub async fn get_text(
    state: State<'_, AppState>,
    id: u64,
    from: Pos,
    to: Pos,
) -> Result<String, FsError> {
    let handle = state.docs.get(id)?;
    blocking(move || lock_doc(&handle).text(from, to)).await
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
    let handle = state.docs.get(id)?;
    blocking(move || {
        let mut doc = lock_doc(&handle);
        doc.save(&req)?;
        Ok(doc.info(id))
    })
    .await
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
        assert!(count >= 12);
    }
}
