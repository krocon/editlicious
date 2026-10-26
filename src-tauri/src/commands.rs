//! Thin Tauri command layer: validate the input, run blocking work on `spawn_blocking`, map
//! errors to `FsError`. Logic lives in pure modules. Every command is `async` – a synchronous
//! command would run on the main thread and freeze the window on a slow disk. The test
//! [`command_rules`] checks this.

use crate::about;
use crate::document::DiskState;
use crate::document::model::{DocInfo, LineEnding, LoadEvent, SaveRequest};
use crate::document::registry::{Registry, lock_doc};
use crate::document::{Document, load};
use crate::editor::model::{Command, Frame};
use crate::editor::playback::Tempo;
use crate::editor::{Clipboard, Session};
use crate::error::{FsError, FsErrorCode};
use crate::model::AppInfo;
use crate::playback::{self, PlaybackCheck, PlaybackEvent, RecordingInfo, Source};
use crate::session_file::{self, SessionFile};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// App-wide state: the editor sessions (one per open document, design 003) and the clipboard.
#[derive(Default)]
pub struct AppState {
    sessions: Arc<Registry<Session>>,
    clipboard: Arc<Mutex<SystemClipboard>>,
    /// The frontend asks about unsaved files before the window closes (design 004 E4).
    pub close_handled: AtomicBool,
    /// The running match count – a new one cancels it (design 006 E4).
    counting: Mutex<Option<Arc<AtomicBool>>>,
    /// The running overview (design 010 E3) – a new one cancels it.
    overviewing: Mutex<Option<Arc<AtomicBool>>>,
    /// Files handed to the app ("open with", a second start): `(frontend ready, waiting files)`
    /// – before the frontend asked for its start files they wait here (design 012 E3).
    opened: Mutex<(bool, Vec<String>)>,
    /// Config files that could not be read (rights, locked): never written over with defaults
    /// in this run (design 013 review #10).
    keep: Mutex<std::collections::HashSet<&'static str>>,
    /// Running playbacks by document (design 015 E9/E10).
    playing: crate::mcp::Playing,
    /// AI control (design 016): what the UI told the tools, and the bridge while it is on.
    mcp_ui: Arc<Mutex<crate::mcp::UiState>>,
    mcp_server: Mutex<Option<crate::mcp::Server>>,
    /// The running video export – a new one or `cancel_export` stops it (design 017).
    exporting: Mutex<Option<Arc<AtomicBool>>>,
}

impl AppState {
    fn playback_of(&self, id: u64) -> Option<Arc<playback::Control>> {
        self.playing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
    }

    fn set_keep(&self, file: &'static str, keep: bool) {
        let mut k = self.keep.lock().unwrap_or_else(|e| e.into_inner());
        if keep {
            k.insert(file);
        } else {
            k.remove(file);
        }
    }

    fn keeps(&self, file: &'static str) -> bool {
        self.keep
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(file)
    }
}

/// Event with paths to open while the app runs (design 012 E3).
pub const OPEN_FILES_EVENT: &str = "open-files";

/// Files the system hands over (macOS "open with", a second start on Windows/Linux): queued
/// until the frontend has started, then sent as [`OPEN_FILES_EVENT`].
pub fn files_opened(app: &AppHandle, paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let state = app.state::<AppState>();
    let mut q = state.opened.lock().unwrap_or_else(|e| e.into_inner());
    if q.0 {
        drop(q);
        use tauri::Emitter;
        let _ = app.emit(OPEN_FILES_EVENT, paths);
    } else {
        q.1.extend(paths);
    }
}

/// Existing files among command-line arguments (`-` options skipped, relative to `cwd`).
pub fn paths_from_args(args: impl IntoIterator<Item = String>, cwd: Option<&Path>) -> Vec<String> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .filter_map(|a| {
            let p = PathBuf::from(&a);
            let p = if p.is_absolute() { p } else { cwd?.join(p) };
            p.is_file().then(|| p.to_string_lossy().into_owned())
        })
        .collect()
}

/// Answer of `count_matches`: all matches and the number of the selected one (0 = none).
#[derive(serde::Serialize)]
pub struct MatchCount {
    pub total: usize,
    pub current: usize,
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

    fn set_html(&mut self, html: String, alt: String) -> bool {
        self.handle()
            .is_some_and(|c| c.set_html(html, Some(alt)).is_ok())
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

/// A new, empty document in its own session; `line_ending` from the settings (`auto` =
/// the platform's).
#[tauri::command]
pub async fn new_document(
    state: State<'_, AppState>,
    line_ending: Option<LineEnding>,
) -> Result<DocInfo, FsError> {
    let id = state.sessions.reserve_id();
    let doc = Document::new_empty(line_ending.unwrap_or(if cfg!(windows) {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }));
    state.sessions.insert(id, Session::new(doc, id));
    info_of(&state.sessions, id)
}

#[tauri::command]
pub async fn close_document(state: State<'_, AppState>, id: u64) -> Result<(), FsError> {
    if let Some(c) = state.playback_of(id) {
        c.stop();
    }
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
    // a playback runs: only the view may change (design 015 E10)
    if !cmd.is_view_only() && state.playback_of(id).is_some() {
        return Err(FsError::new(
            FsErrorCode::PlaybackRunning,
            "playback running",
            None,
        ));
    }
    let handle = state.sessions.get(id)?;
    let clipboard = state.clipboard.clone();
    blocking(move || {
        let mut session = lock_doc(&handle);
        let mut clip = clipboard.lock().unwrap_or_else(|e| e.into_inner());
        session.run(cmd, &mut *clip)
    })
    .await
}

// ---------------------------------------------------------------- recording (design 015)

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    id: u64,
) -> Result<(), FsError> {
    let handle = state.sessions.get(id)?;
    let version = app.package_info().version.to_string();
    blocking(move || {
        lock_doc(&handle).start_recording(&version);
        Ok(())
    })
    .await
}

/// Stops the recording; `None` when none ran.
#[tauri::command]
pub async fn stop_recording(
    state: State<'_, AppState>,
    id: u64,
) -> Result<Option<RecordingInfo>, FsError> {
    let handle = state.sessions.get(id)?;
    blocking(move || Ok(lock_doc(&handle).stop_recording().map(RecordingInfo::of))).await
}

/// Saves the last recording as `*.edrec.jsonl` or as a script.
#[tauri::command]
pub async fn save_recording(
    state: State<'_, AppState>,
    id: u64,
    path: String,
    script: bool,
) -> Result<(), FsError> {
    let path = file_path(&path)?;
    let handle = state.sessions.get(id)?;
    blocking(move || playback::save(&lock_doc(&handle), &path, script)).await
}

/// What would play: kind, steps, duration, recorded on another text?
#[tauri::command]
pub async fn check_playback(
    state: State<'_, AppState>,
    id: u64,
    source: Source,
) -> Result<PlaybackCheck, FsError> {
    if let Source::Path { path } = &source {
        file_path(path)?;
    }
    let handle = state.sessions.get(id)?;
    blocking(move || {
        let s = lock_doc(&handle);
        let loaded = playback::load(&source, &s)?;
        Ok(playback::check(&loaded, &s))
    })
    .await
}

/// Plays a recording or script into the document; frames and progress go to `on_event`
/// until `done`. `tempo` `None` = as the source says.
#[tauri::command]
pub async fn play(
    state: State<'_, AppState>,
    id: u64,
    source: Source,
    tempo: Option<Tempo>,
    on_event: Channel<PlaybackEvent>,
) -> Result<(), FsError> {
    if let Source::Path { path } = &source {
        file_path(path)?;
    }
    let handle = state.sessions.get(id)?;
    let control = Arc::new(playback::Control::default());
    {
        let mut playing = state.playing.lock().unwrap_or_else(|e| e.into_inner());
        if playing.contains_key(&id) {
            return Err(FsError::new(
                FsErrorCode::PlaybackRunning,
                "playback running",
                None,
            ));
        }
        playing.insert(id, control.clone());
    }
    let playing = state.playing.clone();
    let release = move || {
        playing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
    };
    let h = handle.clone();
    let steps = blocking(move || {
        let s = lock_doc(&h);
        let loaded = playback::load(&source, &s)?;
        Ok(crate::editor::playback::timeline(
            &loaded.items,
            tempo.unwrap_or(loaded.default_tempo),
        ))
    })
    .await;
    let steps = match steps {
        Ok(s) => s,
        Err(e) => {
            release();
            return Err(e);
        }
    };
    std::thread::spawn(move || {
        playback::run(handle, steps, control, |e| {
            let _ = on_event.send(e);
        });
        release();
    });
    Ok(())
}

/// `pause`, `resume`, `stop` or `step` (one step while paused).
#[tauri::command]
pub async fn playback_control(
    state: State<'_, AppState>,
    id: u64,
    action: String,
) -> Result<(), FsError> {
    let Some(c) = state.playback_of(id) else {
        return Ok(());
    };
    match action.as_str() {
        "pause" => c.pause(),
        "resume" => c.resume(),
        "stop" => c.stop(),
        "step" => c.step(),
        _ => {
            return Err(FsError::new(
                FsErrorCode::InvalidEdit,
                "unknown action",
                None,
            ));
        }
    }
    Ok(())
}

/// Counts the matches of a query on a snapshot of the text (O(1) clone) – typing goes on while
/// a big file is counted; a newer count cancels this one (`cancelled`).
#[tauri::command]
pub async fn count_matches(
    state: State<'_, AppState>,
    id: u64,
    query: crate::search::Query,
) -> Result<MatchCount, FsError> {
    let re = crate::search::compile(&query)?;
    let (rope, start, end) = lock_doc(&state.sessions.get(id)?).search_snapshot();
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(old) = state
        .counting
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .replace(cancel.clone())
    {
        old.store(true, Ordering::Relaxed);
    }
    blocking(move || {
        // the selected match counts as "before" so its number is `ahead`
        let before = if end > start { start + 1 } else { start };
        let (total, ahead) = crate::search::count(&rope, &re, before, &cancel)
            .ok_or_else(|| FsError::new(FsErrorCode::Cancelled, "cancelled", None))?;
        let current =
            if end > start && crate::search::find_next(&rope, &re, start) == Some((start, end)) {
                ahead
            } else {
                0
            };
        Ok(MatchCount { total, current })
    })
    .await
}

/// Where search matches and markers are, for the scrollbar (design 010 E3): one byte per
/// slice of the lines (bit 1 = match, bit 2 = marker). A newer call cancels this one.
#[tauri::command]
pub async fn overview(
    state: State<'_, AppState>,
    id: u64,
    buckets: usize,
) -> Result<Vec<u8>, FsError> {
    let (rope, layers) = lock_doc(&state.sessions.get(id)?).overview_snapshot();
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(old) = state
        .overviewing
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .replace(cancel.clone())
    {
        old.store(true, Ordering::Relaxed);
    }
    if layers.is_empty() {
        return Ok(Vec::new());
    }
    blocking(move || {
        crate::search::overview(&rope, &layers, buckets, &cancel)
            .ok_or_else(|| FsError::new(FsErrorCode::Cancelled, "cancelled", None))
    })
    .await
}

/// Encodings the status bar offers (design 010 E2).
#[tauri::command]
pub async fn list_encodings() -> Result<Vec<String>, FsError> {
    Ok(crate::document::ENCODINGS
        .iter()
        .map(|e| (*e).to_owned())
        .collect())
}

/// All languages of the highlighter (design 008).
#[tauri::command]
pub async fn list_languages() -> Result<Vec<String>, FsError> {
    blocking(|| Ok(crate::highlight::languages())).await
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

/// Files given on the command line (open with …, `editlicious file.txt`) and those the system
/// handed over before the frontend was ready; from now on new ones come as events.
#[tauri::command]
pub async fn startup_paths(state: State<'_, AppState>) -> Result<Vec<String>, FsError> {
    let cwd = std::env::current_dir().ok();
    let mut paths = paths_from_args(std::env::args().skip(1), cwd.as_deref());
    let mut q = state.opened.lock().unwrap_or_else(|e| e.into_inner());
    q.0 = true;
    paths.append(&mut q.1);
    Ok(paths)
}

/// Reloads a document from disk (changed outside, design 004 E6); caret and scroll stay.
#[tauri::command]
pub async fn reload_document(
    state: State<'_, AppState>,
    id: u64,
    encoding: Option<String>,
    only_if_clean: Option<bool>,
) -> Result<DocInfo, FsError> {
    let only_if_clean = only_if_clean.unwrap_or(false);
    let handle = state.sessions.get(id)?;
    let encoding = encoding
        .as_deref()
        .map(crate::document::encoding_for_label)
        .transpose()?;
    blocking(move || {
        let path = lock_doc(&handle)
            .doc
            .path
            .clone()
            .ok_or_else(|| FsError::new(FsErrorCode::InvalidPath, "no path", None))?;
        let never = AtomicBool::new(false);
        let loaded = match encoding {
            // "reopen with encoding" (design 010 E2)
            Some(enc) => load::load_file_as(&path, enc, &never, &mut |_, _| {})?,
            None => load::load_file(&path, &never, &mut |_, _| {})?,
        };
        let mut s = lock_doc(&handle);
        // typed into while the file was read: the user decides (design 013 review #3)
        if only_if_clean && s.doc.dirty() {
            return Err(FsError::new(
                FsErrorCode::DocumentEdited,
                "edited meanwhile",
                Some(&path),
            ));
        }
        s.replace_document(Document::from_loaded(loaded, &path));
        Ok(s.doc.info(id))
    })
    .await
}

/// Did the file change on disk? (design 004 E6)
#[tauri::command]
pub async fn disk_state(state: State<'_, AppState>, id: u64) -> Result<DiskState, FsError> {
    let handle = state.sessions.get(id)?;
    blocking(move || Ok(lock_doc(&handle).doc.disk_state())).await
}

/// The folder of a file path from the frontend (paths are opaque there, CLAUDE.md §4).
fn folder_of(file: Option<&str>) -> Option<PathBuf> {
    let p = file_path(file?).ok()?;
    p.parent().filter(|d| d.is_dir()).map(|d| d.to_path_buf())
}

/// Native "open" dialog (several files, starting next to `near`); `None` = cancelled.
#[tauri::command]
pub async fn pick_open_files(
    app: AppHandle,
    near: Option<String>,
) -> Result<Option<Vec<String>>, FsError> {
    blocking(move || {
        let mut d = app.dialog().file();
        if let Some(dir) = folder_of(near.as_deref()) {
            d = d.set_directory(dir);
        }
        Ok(d.blocking_pick_files().map(|files| {
            files
                .into_iter()
                .filter_map(|f| f.into_path().ok())
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        }))
    })
    .await
}

/// Native "save as" dialog; `None` = cancelled.
#[tauri::command]
pub async fn pick_save_path(
    app: AppHandle,
    name: String,
    near: Option<String>,
) -> Result<Option<String>, FsError> {
    blocking(move || {
        let mut d = app.dialog().file().set_file_name(name);
        if let Some(dir) = folder_of(near.as_deref()) {
            d = d.set_directory(dir);
        }
        Ok(d.blocking_save_file()
            .and_then(|f| f.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned()))
    })
    .await
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, FsError> {
    app.path()
        .app_config_dir()
        .map_err(|e| FsError::new(FsErrorCode::Io, e.to_string(), None))
}

/// The settings (design 009); problems with the file come as a warning.
#[tauri::command]
pub async fn load_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::config_file::Loaded<crate::settings::Settings>, FsError> {
    let dir = config_dir(&app)?;
    let loaded = blocking(move || Ok(crate::settings::load(&dir))).await?;
    state.set_keep("settings", !loaded.persist);
    Ok(loaded)
}

/// Saves and returns the settings as stored (checked, clamped).
#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: crate::settings::Settings,
) -> Result<crate::settings::Settings, FsError> {
    if state.keeps("settings") {
        return Ok(settings);
    }
    let dir = config_dir(&app)?;
    blocking(move || crate::settings::save(&dir, settings).map_err(|e| FsError::from_io(&e, &dir)))
        .await
}

/// The session of the last run (design 004 E5).
#[tauri::command]
pub async fn load_session(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SessionFile, FsError> {
    let dir = config_dir(&app)?;
    let loaded = blocking(move || Ok(session_file::load(&dir))).await?;
    state.set_keep("session", !loaded.persist);
    Ok(loaded.value)
}

#[tauri::command]
pub async fn save_session(
    app: AppHandle,
    state: State<'_, AppState>,
    session: SessionFile,
) -> Result<(), FsError> {
    if state.keeps("session") {
        return Ok(());
    }
    let dir = config_dir(&app)?;
    blocking(move || session_file::save(&dir, session).map_err(|e| FsError::from_io(&e, &dir)))
        .await
}

#[tauri::command]
pub async fn set_title(app: AppHandle, title: String) -> Result<(), FsError> {
    if let Some(w) = app.get_webview_window("main") {
        let title: String = title.chars().take(300).collect();
        w.set_title(&title)
            .map_err(|e| FsError::new(FsErrorCode::Io, e.to_string(), None))?;
    }
    Ok(())
}

/// From now on the window's close button asks the frontend first (event `close-requested`).
#[tauri::command]
pub async fn take_over_close(state: State<'_, AppState>) -> Result<(), FsError> {
    state.close_handled.store(true, Ordering::Relaxed);
    Ok(())
}

/// Quits after the frontend has dealt with unsaved files.
#[tauri::command]
pub async fn quit_app(app: AppHandle) -> Result<(), FsError> {
    app.exit(0);
    Ok(())
}

fn io_error(message: impl Into<String>) -> FsError {
    FsError::new(FsErrorCode::Io, message, None)
}

/// Full screen on/off (design 005); returns the new state.
#[tauri::command]
pub async fn toggle_fullscreen(window: tauri::WebviewWindow) -> Result<bool, FsError> {
    let on = !window
        .is_fullscreen()
        .map_err(|e| io_error(e.to_string()))?;
    window
        .set_fullscreen(on)
        .map_err(|e| io_error(e.to_string()))?;
    Ok(on)
}

/// The native menu bar (macOS) from the frontend's menu model (design 005, from comic-commander).
#[tauri::command]
pub async fn set_app_menu(app: AppHandle, spec: crate::app_menu::MenuSpec) -> Result<(), FsError> {
    crate::app_menu::validate(&spec)?;
    apply_app_menu(app, spec).await
}

/// macOS: menus are built on the main thread; the answer is awaited off the async runtime.
#[cfg(target_os = "macos")]
async fn apply_app_menu(app: AppHandle, spec: crate::app_menu::MenuSpec) -> Result<(), FsError> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(crate::app_menu::apply(&handle, &spec));
    })
    .map_err(|e| io_error(e.to_string()))?;
    blocking(move || {
        rx.recv()
            .map_err(|e| io_error(e.to_string()))?
            .map_err(|e| io_error(e.to_string()))
    })
    .await
}

/// Windows/Linux: the HTML menu bar needs nothing from the backend.
#[cfg(not(target_os = "macos"))]
async fn apply_app_menu(_app: AppHandle, _spec: crate::app_menu::MenuSpec) -> Result<(), FsError> {
    Ok(())
}

/// The user's shortcut overrides (`keymap.json`, design 005 E5).
#[tauri::command]
pub async fn load_keymap_overrides(
    app: AppHandle,
) -> Result<crate::keymap_file::KeymapOverrides, FsError> {
    blocking(move || Ok(crate::keymap_file::load(&config_dir(&app)?))).await
}

/// Should a close request wait for the frontend?
pub fn close_handled(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .close_handled
        .load(Ordering::Relaxed)
}

#[cfg(test)]
mod args_tests {
    use super::paths_from_args;

    #[test]
    fn paths_from_args_keeps_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        let abs = dir.path().join("a.txt").to_string_lossy().into_owned();
        let got = paths_from_args(
            [
                "-psn_0_1".into(),
                "a.txt".into(),
                "missing.txt".into(),
                abs.clone(),
            ],
            Some(dir.path()),
        );
        assert_eq!(got, vec![abs.clone(), abs]);
        assert!(paths_from_args(["a.txt".to_string()], None).is_empty());
    }
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
        assert!(count >= 27);
    }
}

// ---------------------------------------------------------------- AI control (design 016)

/// Tauri events for the UI (design 016 E8).
struct TauriUi(AppHandle);

impl crate::mcp::Ui for TauriUi {
    fn opened(&self, info: &DocInfo, activate: bool) {
        use tauri::Emitter;
        let _ = self.0.emit(
            "mcp-opened",
            serde_json::json!({ "info": info, "activate": activate }),
        );
    }
    fn activate(&self, id: u64) {
        use tauri::Emitter;
        let _ = self.0.emit("mcp-activate", serde_json::json!({ "id": id }));
    }
    fn changed(&self, id: u64) {
        use tauri::Emitter;
        let _ = self.0.emit("mcp-changed", serde_json::json!({ "id": id }));
    }
    fn action(&self, id: u64, action: &str) {
        use tauri::Emitter;
        let _ = self.0.emit(
            "mcp-action",
            serde_json::json!({ "id": id, "action": action }),
        );
    }
    fn recording(&self, id: u64, on: bool) {
        use tauri::Emitter;
        let _ = self
            .0
            .emit("mcp-recording", serde_json::json!({ "id": id, "on": on }));
    }
    fn playback(&self, id: u64, event: PlaybackEvent) {
        use tauri::Emitter;
        let _ = self.0.emit(
            "mcp-playback",
            serde_json::json!({ "id": id, "event": event }),
        );
    }
    fn status(&self, connections: usize) {
        use tauri::Emitter;
        let _ = self.0.emit(
            "mcp-status",
            serde_json::json!({ "connections": connections }),
        );
    }
}

/// State of AI control for the settings dialog.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInfo {
    pub enabled: bool,
    pub connections: usize,
    /// The sidecar program next to the app, `None` when it is missing.
    pub sidecar: Option<String>,
}

fn sidecar_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let name = if cfg!(windows) {
        "editlicious-mcp.exe"
    } else {
        "editlicious-mcp"
    };
    let p = exe.parent()?.join(name);
    p.is_file().then(|| p.to_string_lossy().into_owned())
}

fn mcp_info_of(state: &AppState) -> McpInfo {
    let server = state.mcp_server.lock().unwrap_or_else(|e| e.into_inner());
    McpInfo {
        enabled: server.is_some(),
        connections: server.as_ref().map_or(0, |s| s.connections()),
        sidecar: sidecar_path(),
    }
}

/// Turns AI control on (bridge + endpoint file) or off (both gone, connections closed).
#[tauri::command]
pub async fn mcp_enable(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<McpInfo, FsError> {
    let mut server = state.mcp_server.lock().unwrap_or_else(|e| e.into_inner());
    if enabled && server.is_none() {
        let file = config_dir(&app)?.join(ed_mcp::bridge::ENDPOINT_FILE);
        let tools = crate::mcp::Tools {
            sessions: state.sessions.clone(),
            playing: state.playing.clone(),
            ui_state: state.mcp_ui.clone(),
            ui: Arc::new(TauriUi(app.clone())),
            version: app.package_info().version.to_string(),
        };
        *server = Some(crate::mcp::Server::start(Arc::new(tools), &file)?);
    } else if !enabled {
        *server = None;
    }
    drop(server);
    Ok(mcp_info_of(&state))
}

#[tauri::command]
pub async fn mcp_info(state: State<'_, AppState>) -> Result<McpInfo, FsError> {
    Ok(mcp_info_of(&state))
}

/// The UI's actions (id, label) for `list_actions`/`run_action`.
#[tauri::command]
pub async fn mcp_set_actions(
    state: State<'_, AppState>,
    actions: Vec<(String, String)>,
) -> Result<(), FsError> {
    state
        .mcp_ui
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .actions = actions;
    Ok(())
}

/// The active document – tools without `id` work on it.
#[tauri::command]
pub async fn mcp_set_active(state: State<'_, AppState>, id: Option<u64>) -> Result<(), FsError> {
    state
        .mcp_ui
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .active = id;
    Ok(())
}

/// Closes every AI connection and stops the playbacks the AI started.
#[tauri::command]
pub async fn mcp_disconnect(state: State<'_, AppState>) -> Result<(), FsError> {
    if let Some(s) = state
        .mcp_server
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        s.disconnect();
    }
    for c in state
        .playing
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .values()
    {
        c.stop();
    }
    Ok(())
}

// ---------------------------------------------------------------- video export (design 017)

/// Renders a recording or script as a video (or PNG frames); progress and the end over
/// `on_event`. Runs on its own thread – the editor stays usable.
#[tauri::command]
pub async fn export_video(
    state: State<'_, AppState>,
    id: u64,
    source: Source,
    options: crate::export::ExportOptions,
    on_event: Channel<crate::export::ExportEvent>,
) -> Result<(), FsError> {
    file_path(&options.path)?;
    if let Source::Path { path } = &source {
        file_path(path)?;
    }
    let handle = state.sessions.get(id)?;
    let cancel = Arc::new(AtomicBool::new(false));
    if let Some(old) = state
        .exporting
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .replace(cancel.clone())
    {
        old.store(true, Ordering::SeqCst);
    }
    let o = options.clone();
    let job = blocking(move || crate::export::job_of(&handle, &source, &o)).await?;
    std::thread::spawn(move || {
        let reason = match crate::export::run(job, &options, cancel, |e| {
            let _ = on_event.send(e);
        }) {
            Ok(()) => "finished".to_string(),
            Err(e) => serde_json::to_value(e.code)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| "io".into()),
        };
        let _ = on_event.send(crate::export::ExportEvent::Done { reason });
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_export(state: State<'_, AppState>) -> Result<(), FsError> {
    if let Some(c) = state
        .exporting
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        c.store(true, Ordering::SeqCst);
    }
    Ok(())
}
