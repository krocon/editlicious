//! ed-core – the editor engine of editlicious, free of Tauri (design 002 E12): the app crate
//! only wraps it in commands; it can also be built for WebAssembly (site demo, prompt 011).
//!
//! - [`error`]    structured, serialisable errors (`FsError { code, message, path }`)
//! - [`document`] the document model: rope, load (encodings), view, edits, undo, save (design 002)

pub mod document;
pub mod error;
