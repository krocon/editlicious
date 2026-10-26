//! editlicious – Tauri backend.
//!
//! Module overview:
//! - [`error`]         structured, serialisable errors (`FsError { code, message, path }`)
//! - [`model`]         DTOs shared with the frontend
//! - [`config_file`], [`fs_atomic`] loader of small config files / atomic writes (from files-and-folders)
//! - [`about`]         data for the About dialog (version, platform)
//! - [`commands`]      thin Tauri command layer (validate, `spawn_blocking`, never synchronous)

pub mod about;
pub mod commands;
pub mod config_file;
pub mod error;
pub mod fs_atomic;
pub mod model;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // only the Rust API is used (open/save dialogs) – the WebView gets no `dialog:` permission
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![commands::get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
