//! editlicious – Tauri backend.
//!
//! Module overview:
//! - [`error`]         structured, serialisable errors (`FsError { code, message, path }`)
//! - [`model`]         DTOs shared with the frontend
//! - [`config_file`], [`fs_atomic`] loader of small config files / atomic writes (from files-and-folders)
//! - [`document`]      the document model: rope, load (encodings), view, edits, undo, save (design 002)
//! - [`session_file`]  open files, recent files and tab mode between starts (design 004)
//! - [`about`]         data for the About dialog (version, platform)
//! - [`commands`]      thin Tauri command layer (validate, `spawn_blocking`, never synchronous)

pub mod about;
pub mod commands;
pub mod config_file;
pub use ed_core::{document, editor, error};
pub mod fs_atomic;
pub mod model;
pub mod session_file;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // only the Rust API is used (open/save dialogs) – the WebView gets no `dialog:` permission
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        // unsaved files: the frontend asks first (design 004 E4) – once it has said it will
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
            commands::save_document,
            commands::startup_paths,
            commands::reload_document,
            commands::disk_state,
            commands::pick_open_files,
            commands::pick_save_path,
            commands::load_session,
            commands::save_session,
            commands::set_title,
            commands::take_over_close,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
