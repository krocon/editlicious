//! ed-core – the editor engine of editlicious, free of Tauri (design 002 E12): the app crate
//! only wraps it in commands; it can also be built for WebAssembly (site demo, prompt 011).
//!
//! - [`error`]    structured, serialisable errors (`FsError { code, message, path }`)
//! - [`document`] the document model: rope, load (encodings), view, edits, undo, save (design 002)
//! - [`highlight`] syntax highlighting of the visible lines (design 008)
//! - [`search`]   search and replace in blocks of lines (design 006)
//! - [`math`]     arithmetic in the text: parser, evaluation, formatting (prompt 020)
//! - [`editor`]   the editor session: cursors, commands, layout in cells, frames (design 003)

pub mod document;
pub mod editor;
pub mod error;
pub mod highlight;
pub mod math;
pub mod search;

#[cfg(test)]
mod measure;
