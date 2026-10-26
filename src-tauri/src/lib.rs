//! editlicious – Tauri backend.
//!
//! Module overview:
//! - [`error`]         structured, serialisable errors (`FsError { code, message, path }`)
//! - [`model`]         DTOs shared with the frontend
//! - [`config_file`], [`fs_atomic`] loader of small config files / atomic writes (from files-and-folders)
//! - [`document`]      the document model: rope, load (encodings), view, edits, undo, save (design 002)
//! - [`settings`]      settings.json (design 009)
//! - [`session_file`]  open files, recent files and tab mode between starts (design 004)
//! - [`app_menu`]      native menu bar of macOS from the frontend's menu model (design 005)
//! - [`keymap_file`]   the user's shortcut overrides `keymap.json` (design 005)
//! - [`about`]         data for the About dialog (version, platform)
//! - [`mcp`]           AI control: bridge server for the MCP sidecar and its tools (design 016)
//! - [`playback`]      recordings and scripts: read, save, play in real time (design 015)
//! - [`commands`]      thin Tauri command layer (validate, `spawn_blocking`, never synchronous)

pub mod about;
pub mod app_menu;
pub mod commands;
pub mod config_file;
pub use ed_core::{document, editor, error, highlight, search};
pub mod fs_atomic;
pub mod keymap_file;
pub mod mcp;
pub mod model;
pub mod playback;
pub mod session_file;
pub mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // a second start (Windows/Linux "open with") hands its files to the running app
        // (design 012 E3) – must be the first plugin
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let paths = commands::paths_from_args(
                argv.into_iter().skip(1),
                Some(std::path::Path::new(&cwd)),
            );
            commands::files_opened(app, paths);
            if let Some(w) = tauri::Manager::get_webview_window(app, "main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        // only the Rust API is used (open/save dialogs) – the WebView gets no `dialog:` permission
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        // unsaved files: the frontend asks first (design 004 E4) – once it has said it will
        // native menu (macOS): a click is an action of the frontend; Quit asks there too
        // (unsaved files) – unless the frontend never took over closing
        .on_menu_event(|app, event| {
            use tauri::Emitter;
            let id = event.id().0.clone();
            if id == "app.quit" && !commands::close_handled(app) {
                app.exit(0);
            } else if app_menu::valid_id(&id) {
                let _ = app.emit(app_menu::EVENT, app_menu::MenuAction { id });
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && commands::close_handled(tauri::Manager::app_handle(window))
            {
                use tauri::Emitter;
                api.prevent_close();
                let _ = window.emit("close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::open_document,
            commands::cancel_load,
            commands::new_document,
            commands::close_document,
            commands::document_info,
            commands::editor_command,
            commands::count_matches,
            commands::overview,
            commands::list_encodings,
            commands::list_languages,
            commands::save_document,
            commands::startup_paths,
            commands::reload_document,
            commands::disk_state,
            commands::pick_open_files,
            commands::pick_save_path,
            commands::load_session,
            commands::load_settings,
            commands::save_settings,
            commands::save_session,
            commands::set_title,
            commands::take_over_close,
            commands::quit_app,
            commands::toggle_fullscreen,
            commands::set_app_menu,
            commands::load_keymap_overrides,
            commands::start_recording,
            commands::stop_recording,
            commands::save_recording,
            commands::check_playback,
            commands::play,
            commands::playback_control,
            commands::mcp_enable,
            commands::mcp_info,
            commands::mcp_set_actions,
            commands::mcp_set_active,
            commands::mcp_disconnect,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // macOS hands files from Finder ("open with", double click) as an event
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = _event {
                let paths = urls
                    .into_iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                commands::files_opened(_app, paths);
            }
        });
}
