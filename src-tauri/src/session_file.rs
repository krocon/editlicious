//! The editing session between starts (design 004 E5): open files with caret and scroll
//! position, the active one, recently used files and the tab mode – `session.json` in the app
//! config folder. Loaded leniently (bad entries dropped, broken file kept as `.bad`).

use crate::config_file::{self, Loaded};
use crate::fs_atomic::write_atomic;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "session.json";
const MAX_BYTES: u64 = 256 * 1024;
pub const MAX_FILES: usize = 100;
pub const MAX_RECENT: usize = 30;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionEntry {
    pub path: String,
    pub line: usize,
    pub ch: usize,
    pub top: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionFile {
    pub version: u32,
    pub files: Vec<SessionEntry>,
    pub active: usize,
    pub recent: Vec<String>,
    /// `auto` (tabs from the second file on), `show` or `hide` (legacy §5).
    pub tabs: String,
    /// Search and replace history, newest first (design 006 E6).
    pub find_history: Vec<String>,
    pub replace_history: Vec<String>,
}

impl Default for SessionFile {
    fn default() -> Self {
        Self {
            version: 1,
            files: Vec::new(),
            active: 0,
            recent: Vec::new(),
            tabs: "auto".into(),
            find_history: Vec::new(),
            replace_history: Vec::new(),
        }
    }
}

pub const MAX_HISTORY: usize = 20;

fn clean_history(h: &mut Vec<String>) {
    h.retain(|s| !s.is_empty() && s.len() <= 4096);
    h.truncate(MAX_HISTORY);
}

fn valid_path(p: &str) -> bool {
    !p.is_empty() && p.len() <= 4096 && !p.contains('\0') && Path::new(p).is_absolute()
}

/// Drops what cannot be right: relative paths, too many entries, unknown tab modes.
pub fn sanitize(mut s: SessionFile) -> SessionFile {
    s.version = 1;
    s.files.retain(|f| valid_path(&f.path));
    s.files.truncate(MAX_FILES);
    for f in &mut s.files {
        if !f.top.is_finite() || f.top < 0.0 {
            f.top = 0.0;
        }
    }
    s.recent.retain(|p| valid_path(p));
    s.recent.dedup();
    s.recent.truncate(MAX_RECENT);
    clean_history(&mut s.find_history);
    clean_history(&mut s.replace_history);
    if s.active >= s.files.len() {
        s.active = 0;
    }
    if !matches!(s.tabs.as_str(), "auto" | "show" | "hide") {
        s.tabs = "auto".into();
    }
    s
}

pub fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

pub fn load(dir: &Path) -> Loaded<SessionFile> {
    config_file::read_config(&path_in(dir), MAX_BYTES, SessionFile::default, |text| {
        serde_json::from_str::<SessionFile>(text)
            .map(sanitize)
            .map_err(|e| e.to_string())
    })
}

pub fn save(dir: &Path, s: SessionFile) -> io::Result<()> {
    let s = sanitize(s);
    let json = serde_json::to_vec_pretty(&s).map_err(io::Error::other)?;
    write_atomic(&path_in(dir), &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_sanitize() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).value, SessionFile::default());
        // absolute on every OS (`/a` is not on Windows)
        let abs = |p: &str| dir.path().join(p).to_string_lossy().into_owned();
        let s = SessionFile {
            files: vec![
                SessionEntry {
                    path: abs("ä b.txt"),
                    line: 3,
                    ch: 1,
                    top: 2.5,
                },
                SessionEntry {
                    path: "relative.txt".into(),
                    ..Default::default()
                },
            ],
            active: 9,
            recent: vec![abs("x"), abs("x"), "".into()],
            tabs: "weird".into(),
            ..Default::default()
        };
        save(dir.path(), s).unwrap();
        let back = load(dir.path()).value;
        assert_eq!(back.files.len(), 1);
        assert_eq!(back.files[0].path, abs("ä b.txt"));
        assert_eq!(back.active, 0);
        assert_eq!(back.recent, vec![abs("x")]);
        assert_eq!(back.tabs, "auto");
    }

    #[test]
    fn broken_file_is_kept_aside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(path_in(dir.path()), b"{ nope").unwrap();
        let l = load(dir.path());
        assert_eq!(l.value, SessionFile::default());
        assert_eq!(l.warning.unwrap().code, "broken");
    }
}
