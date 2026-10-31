//! Structured, serialisable errors (`FsError { code, message, path }`), as in comic-commander.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Machine-readable error codes. Serialised as `snake_case` strings so the frontend can map
/// them to user-facing texts (`describeFsError()` in `src/app/core/errors/fs-error.ts`).
/// A new code goes into both files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsErrorCode {
    NotFound,
    PermissionDenied,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    /// Any other I/O error (message carries the details).
    Io,
    /// A document id that is not (or no longer) open (design 002).
    UnknownDocument,
    /// The file contains NUL bytes – not a text file (design 002 E4).
    BinaryFile,
    /// The file exceeds `MAX_FILE_BYTES` (design 002 E4).
    FileTooLarge,
    /// An encoding name the Encoding Standard does not know.
    UnsupportedEncoding,
    /// A character cannot be written in the document's encoding (design 002 E8); the message
    /// carries `line:ch` (0-based) of the first one.
    UnmappableCharacter,
    /// The requested text is too large for the IPC bridge (design 002 E11).
    SelectionTooLarge,
    /// Overlapping or otherwise invalid changes sent to `apply_edits`.
    InvalidEdit,
    /// The operation was cancelled by the user.
    Cancelled,
    /// The menu sent to `set_app_menu` is not valid (design 005).
    InvalidMenu,
    /// A search pattern that does not compile (design 006); the message has the details.
    InvalidSearch,
    /// A device, pipe or socket – no regular file (design 013).
    NotAFile,
    /// The file changed on disk since it was loaded or saved; saving would overwrite that
    /// (design 013) – save again with `force` after asking.
    ChangedOnDisk,
    /// The document was edited meanwhile – a reload that may only replace a clean one stops.
    DocumentEdited,
    /// A recording or script that cannot be read; the message has the line (design 015).
    RecordingInvalid,
    /// A recording step that cannot be played back (an embedded clipboard text that was too
    /// large to keep, design 015 E3).
    RecordingUnsupported,
    /// A playback runs in this document: edits wait until it ends (design 015 E10).
    PlaybackRunning,
    /// There is no recording to save or play (design 015).
    NoRecording,
    /// The document changed since the revision an AI tool call expected (design 016 E6).
    RevisionConflict,
    /// A tool call with missing or wrong arguments (design 016).
    InvalidRequest,
    /// The video export found no `ffmpeg` (design 017).
    FfmpegMissing,
    /// The session is read-only (design 025 E1): the command would change the text.
    ReadOnly,
}

/// The one error type every command returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FsError {
    pub code: FsErrorCode,
    pub message: String,
    pub path: Option<String>,
}

impl FsError {
    pub fn new(code: FsErrorCode, message: impl Into<String>, path: Option<&Path>) -> Self {
        Self {
            code,
            message: message.into(),
            path: path.map(|p| p.to_string_lossy().into_owned()),
        }
    }

    /// Maps an `io::Error` to a code; the path is the one the operation was about.
    pub fn from_io(err: &io::Error, path: &Path) -> Self {
        let code = match err.kind() {
            io::ErrorKind::NotFound => FsErrorCode::NotFound,
            io::ErrorKind::PermissionDenied => FsErrorCode::PermissionDenied,
            io::ErrorKind::NotADirectory => FsErrorCode::NotADirectory,
            io::ErrorKind::IsADirectory => FsErrorCode::IsADirectory,
            _ => FsErrorCode::Io,
        };
        Self::new(code, err.to_string(), Some(path))
    }
}

impl std::fmt::Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for FsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_serialise_as_snake_case() {
        let e = FsError::new(FsErrorCode::NotADirectory, "x", Some(Path::new("/a")));
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(
            json,
            r#"{"code":"not_a_directory","message":"x","path":"/a"}"#
        );
    }

    #[test]
    fn io_errors_map_to_codes() {
        let p = Path::new("/x");
        let nf = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(FsError::from_io(&nf, p).code, FsErrorCode::NotFound);
        let pd = io::Error::from(io::ErrorKind::PermissionDenied);
        assert_eq!(FsError::from_io(&pd, p).code, FsErrorCode::PermissionDenied);
        let other = io::Error::other("boom");
        assert_eq!(FsError::from_io(&other, p).code, FsErrorCode::Io);
    }
}
