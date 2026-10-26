//! editlicious – Tauri backend.
//!
//! Module overview:
//! - [`error`]         structured, serialisable errors (`FsError { code, message, path }`)
//! - [`model`]         DTOs shared with the frontend
//! - [`config_file`], [`fs_atomic`] loader of small config files / atomic writes (from files-and-folders)
//! - [`document`]      the document model: rope, load (encodings), view, edits, undo, save (design 002)
//! - [`about`]         data for the About dialog (version, platform)
//! - [`commands`]      thin Tauri command layer (validate, `spawn_blocking`, never synchronous)

pub mod about;
pub mod commands;
pub mod config_file;
pub use ed_core::{document, error};
pub mod fs_atomic;
pub mod model;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // only the Rust API is used (open/save dialogs) – the WebView gets no `dialog:` permission
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::open_document,
            commands::cancel_load,
            commands::new_document,
            commands::close_document,
            commands::document_info,
            commands::get_view,
            commands::apply_edits,
            commands::undo,
            commands::redo,
            commands::get_text,
            commands::save_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
