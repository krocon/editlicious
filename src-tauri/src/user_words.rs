//! The user's own word lists for the completion (design 023 E13): `<app config>/words/<key>.txt`,
//! one word per line, `#`-lines as in the built-in lists (`case-insensitive`, `word-chars: -`).
//! `<key>` is what `complete::key_for` names (`java`, `kotlin`, `typescript`, `sql`, `markdown`,
//! `text` …); `all.txt` counts for every file. Read again when settings load and a file opens –
//! small files, at most [`MAX_FILES`] × [`MAX_FILE`].

use ed_core::editor::complete;
use std::collections::HashMap;
use std::path::Path;

pub const DIR_NAME: &str = "words";
const MAX_FILES: usize = 64;
const MAX_FILE: u64 = 256 << 10;

/// The lists in `dir` (key → text); missing folder = none, unreadable files are skipped.
pub fn read(dir: &Path) -> HashMap<String, String> {
    let mut lists = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return lists;
    };
    for e in entries.flatten().take(MAX_FILES * 4) {
        if lists.len() >= MAX_FILES {
            break;
        }
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("txt") {
            continue;
        }
        let Some(key) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let small = e
            .metadata()
            .is_ok_and(|m| m.is_file() && m.len() <= MAX_FILE);
        if !small {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            lists.insert(key.to_lowercase(), text);
        }
    }
    lists
}

/// Reads `<config>/words` and hands the lists to the engine.
pub fn load(config_dir: &Path) {
    complete::set_user_words(read(&config_dir.join(DIR_NAME)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_small_text_files_by_key() {
        let dir = tempfile::tempdir().unwrap();
        let w = dir.path().join(DIR_NAME);
        std::fs::create_dir(&w).unwrap();
        std::fs::write(w.join("Kotlin.txt"), "myKotlinWord\n").unwrap();
        std::fs::write(w.join("all.txt"), "everywhere\n").unwrap();
        std::fs::write(w.join("notes.md"), "ignored\n").unwrap();
        std::fs::write(w.join("big.txt"), "x".repeat(MAX_FILE as usize + 1)).unwrap();
        std::fs::create_dir(w.join("sub.txt")).unwrap();
        let lists = read(&w);
        let mut keys: Vec<_> = lists.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["all", "kotlin"]);
        assert_eq!(lists["kotlin"], "myKotlinWord\n");
        // no folder: no lists
        assert!(read(&dir.path().join("missing")).is_empty());
    }
}
