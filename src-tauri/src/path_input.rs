//! "Open path…" (prompt 024): a path typed or pasted by the user becomes a file to open, and
//! the folder entries matching what was typed so far become completions (like a shell's Tab).
//!
//! Pure functions without Tauri: the home folder and the folder of the active document come in
//! as arguments. The frontend treats paths as opaque strings (CLAUDE.md §4) – everything that
//! splits, expands or joins paths happens here.

use crate::error::{FsError, FsErrorCode};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

/// Most entries sent to the frontend per completion (design 024 E4).
pub const MAX_ENTRIES: usize = 200;

/// One entry of a folder that matches the typed name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathEntry {
    /// The file name as shown in the list (folders end with a separator).
    pub label: String,
    /// The whole input text after choosing this entry (keeps `~` as typed).
    pub value: String,
    pub dir: bool,
}

/// The completions of an input text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PathCompletion {
    pub entries: Vec<PathEntry>,
    /// The input extended by what all matches share (Tab); `None` = nothing to add.
    pub common: Option<String>,
    /// More entries matched than [`MAX_ENTRIES`].
    pub more: bool,
}

fn is_sep(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

fn invalid(input: &str) -> FsError {
    FsError::new(
        FsErrorCode::InvalidPath,
        format!("invalid path: {input:?}"),
        None,
    )
}

/// Pasted paths often come quoted (Finder's "Copy as pathname" in a shell, Windows' "Copy as
/// path") or as a `file://` URL.
fn clean(input: &str) -> String {
    let t = input.trim();
    let t = ['"', '\'']
        .iter()
        .find_map(|q| t.strip_prefix(*q).and_then(|s| s.strip_suffix(*q)))
        .unwrap_or(t);
    match t.strip_prefix("file://") {
        // `file:///Users/x` or `file://localhost/Users/x`
        Some(rest) => {
            let rest = rest.strip_prefix("localhost").unwrap_or(rest);
            let decoded = percent_decode(rest);
            // `file:///C:/x` on Windows
            if cfg!(windows) {
                decoded
                    .strip_prefix('/')
                    .filter(|s| s.as_bytes().get(1) == Some(&b':'))
                    .map(str::to_string)
                    .unwrap_or(decoded)
            } else {
                decoded
            }
        }
        None => t.to_string(),
    }
}

/// `%20` → space; invalid escapes or bytes that are no UTF-8 stay as they are.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |j: usize| b.get(j).and_then(|c| (*c as char).to_digit(16));
        if b[i] == b'%'
            && let (Some(h), Some(l)) = (hex(i + 1), hex(i + 2))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// `a/./b/../c` → `a/c` without touching the disk (symlinks stay as typed, like the native
/// dialog – the same file must get the same path to be found among the open ones).
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(c);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Expands `~` and makes a relative path absolute against `base` (the folder of the active
/// document) or the home folder.
fn expand(text: &str, home: Option<&Path>, base: Option<&Path>) -> Result<PathBuf, FsError> {
    if text.contains('\0') {
        return Err(invalid(text));
    }
    let tilde_rest = if text == "~" {
        Some("")
    } else {
        text.strip_prefix('~')
            .filter(|r| r.starts_with(is_sep))
            .map(|r| r.trim_start_matches(is_sep))
    };
    let p = match tilde_rest {
        Some(rest) => home.ok_or_else(|| invalid(text))?.join(rest),
        None => {
            let p = PathBuf::from(text);
            if p.is_absolute() {
                p
            } else {
                base.or(home).ok_or_else(|| invalid(text))?.join(p)
            }
        }
    };
    Ok(normalize(&p))
}

/// The input text → the path of an existing file to open.
pub fn resolve(input: &str, home: Option<&Path>, base: Option<&Path>) -> Result<String, FsError> {
    let text = clean(input);
    if text.is_empty() {
        return Err(invalid(input));
    }
    let p = expand(&text, home, base)?;
    let meta = std::fs::metadata(&p).map_err(|e| FsError::from_io(&e, &p))?;
    if meta.is_dir() {
        return Err(FsError::new(
            FsErrorCode::IsADirectory,
            "is a directory",
            Some(&p),
        ));
    }
    if !meta.is_file() {
        return Err(FsError::new(
            FsErrorCode::NotAFile,
            "not a regular file",
            Some(&p),
        ));
    }
    Ok(p.to_string_lossy().into_owned())
}

/// The text the dialog starts with: the folder of `near` (with `~` for the home folder) and a
/// separator, else `~/`.
pub fn start_text(near: Option<&Path>, home: Option<&Path>) -> String {
    let sep = std::path::MAIN_SEPARATOR;
    let Some(dir) = near.and_then(Path::parent).filter(|d| d.is_absolute()) else {
        return if home.is_some() {
            format!("~{sep}")
        } else {
            String::new()
        };
    };
    let mut s = match home.and_then(|h| dir.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~{sep}{}", rest.to_string_lossy()),
        None => dir.to_string_lossy().into_owned(),
    };
    if !s.ends_with(is_sep) {
        s.push(sep);
    }
    s
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

/// The longest start shared by all names, ignoring case; taken from the first name.
fn common_start<'a>(names: &[&'a str]) -> &'a str {
    let Some(first) = names.first() else {
        return "";
    };
    let mut end = first.len();
    for n in &names[1..] {
        let mut shared = 0;
        for ((i, a), b) in first.char_indices().zip(n.chars()) {
            if i >= end || lower(&a.to_string()) != lower(&b.to_string()) {
                break;
            }
            shared = i + a.len_utf8();
        }
        end = end.min(shared);
    }
    &first[..end]
}

/// Entries of the folder in `input` whose names start with the text after the last separator
/// (ignoring case). Hidden entries only when that text starts with a dot. Errors (no such
/// folder, no rights) give no entries – completing is a convenience.
pub fn complete(
    input: &str,
    home: Option<&Path>,
    base: Option<&Path>,
    limit: usize,
) -> PathCompletion {
    let input = input.trim_start();
    if input == "~" && home.is_some() {
        let value = format!("~{}", std::path::MAIN_SEPARATOR);
        return PathCompletion {
            entries: vec![PathEntry {
                label: value.clone(),
                value: value.clone(),
                dir: true,
            }],
            common: Some(value),
            more: false,
        };
    }
    let (dir_part, prefix) = match input.rfind(is_sep) {
        Some(i) => input.split_at(i + 1),
        None => ("", input),
    };
    let sep = dir_part
        .chars()
        .rev()
        .find(|c| is_sep(*c))
        .unwrap_or(std::path::MAIN_SEPARATOR);
    let dir = if dir_part.is_empty() {
        match base.or(home) {
            Some(d) => d.to_path_buf(),
            None => return PathCompletion::default(),
        }
    } else {
        match expand(dir_part, home, base) {
            Ok(d) => d,
            Err(_) => return PathCompletion::default(),
        }
    };
    let Ok(read) = std::fs::read_dir(&dir) else {
        return PathCompletion::default();
    };
    let want = lower(prefix);
    let hidden = prefix.starts_with('.');
    let mut names: Vec<String> = read
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| (hidden || !n.starts_with('.')) && lower(n).starts_with(&want))
        .collect();
    names.sort_by_cached_key(|n| (lower(n), n.clone()));
    let is_dir = |n: &str| dir.join(n).is_dir();
    let common = {
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let shared = common_start(&refs);
        let mut c = format!("{dir_part}{shared}");
        if names.len() == 1 && is_dir(&names[0]) {
            c.push(sep);
        }
        // Tab adds something or fixes the case of what was typed
        (c.len() > input.len() || (names.len() == 1 && c != input)).then_some(c)
    };
    let more = names.len() > limit;
    let entries = names
        .iter()
        .take(limit)
        .map(|n| {
            let dir = is_dir(n);
            let suffix = if dir { sep.to_string() } else { String::new() };
            PathEntry {
                label: format!("{n}{suffix}"),
                value: format!("{dir_part}{n}{suffix}"),
                dir,
            }
        })
        .collect();
    PathCompletion {
        entries,
        common,
        more,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tree() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        fs::create_dir_all(r.join("Library/Application Support/Claude")).unwrap();
        fs::write(
            r.join("Library/Application Support/Claude/claude_desktop_config.json"),
            "{}",
        )
        .unwrap();
        fs::create_dir(r.join("Lib2")).unwrap();
        fs::write(r.join("libnotes.txt"), "x").unwrap();
        fs::write(r.join(".hidden"), "x").unwrap();
        fs::write(r.join("Grüße ü.txt"), "x").unwrap();
        t
    }

    #[test]
    fn resolves_tilde_paths() {
        let t = tree();
        let home = t.path();
        let p = resolve(
            "~/Library/Application Support/Claude/claude_desktop_config.json",
            Some(home),
            None,
        )
        .unwrap();
        assert_eq!(
            PathBuf::from(p),
            home.join("Library/Application Support/Claude/claude_desktop_config.json")
        );
    }

    #[test]
    fn resolves_quoted_urls_relative_and_dot_dot() {
        let t = tree();
        let home = t.path();
        let want = home.join("libnotes.txt");
        let abs = want.to_string_lossy().into_owned();
        for input in [
            format!("  \"{abs}\"  "),
            format!("'{abs}'"),
            format!("file://{}", abs.replace(' ', "%20")),
            "libnotes.txt".to_string(),
            "./Lib2/../libnotes.txt".to_string(),
        ] {
            let got = resolve(&input, Some(home), Some(home)).unwrap();
            assert_eq!(PathBuf::from(got), want, "{input}");
        }
        let url = format!(
            "file://{}",
            home.join("Library/Application Support/Claude/claude_desktop_config.json")
                .to_string_lossy()
                .replace(' ', "%20")
        );
        assert!(resolve(&url, Some(home), None).is_ok());
        let umlaut = format!(
            "file://{}/Gr%C3%BC%C3%9Fe%20%C3%BC.txt",
            home.to_string_lossy()
        );
        assert!(resolve(&umlaut, Some(home), None).is_ok());
    }

    #[test]
    fn resolve_errors_have_codes() {
        let t = tree();
        let home = Some(t.path());
        let code = |s: &str| resolve(s, home, None).unwrap_err().code;
        assert_eq!(code("~/nope.txt"), FsErrorCode::NotFound);
        assert_eq!(code("~/Library"), FsErrorCode::IsADirectory);
        assert_eq!(code("   "), FsErrorCode::InvalidPath);
        assert_eq!(code("a\0b"), FsErrorCode::InvalidPath);
        // no home, no base: a relative path has nothing to stand on
        assert_eq!(
            resolve("x.txt", None, None).unwrap_err().code,
            FsErrorCode::InvalidPath
        );
    }

    #[test]
    fn tilde_of_other_users_is_not_expanded() {
        let t = tree();
        // `~bob/x` is a relative name here, not bob's home
        let e = resolve("~bob/x", Some(t.path()), Some(t.path())).unwrap_err();
        assert_eq!(e.code, FsErrorCode::NotFound);
    }

    #[test]
    fn percent_decoding_keeps_bad_escapes() {
        assert_eq!(percent_decode("/a%20b"), "/a b");
        assert_eq!(percent_decode("/100%"), "/100%");
        assert_eq!(percent_decode("/%zz"), "/%zz");
        assert_eq!(percent_decode("/%ff"), "/%ff");
    }

    #[test]
    fn completes_names_ignoring_case() {
        let t = tree();
        let home = Some(t.path());
        let c = complete("~/lib", home, None, MAX_ENTRIES);
        let labels: Vec<_> = c.entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["Lib2/", "libnotes.txt", "Library/"]);
        assert_eq!(c.entries[2].value, "~/Library/");
        assert!(c.entries[2].dir && !c.entries[1].dir);
        // nothing to add to "lib" for all three; "li" grows to the shared "Lib"
        assert_eq!(c.common, None);
        assert!(!c.more);
        let c = complete("~/li", home, None, MAX_ENTRIES);
        assert_eq!(c.common.as_deref(), Some("~/Lib"));
    }

    #[test]
    fn single_match_completes_with_separator() {
        let t = tree();
        let home = Some(t.path());
        let c = complete("~/Library/app", home, None, MAX_ENTRIES);
        assert_eq!(c.common.as_deref(), Some("~/Library/Application Support/"));
        let c = complete(
            "~/Library/Application Support/Claude/c",
            home,
            None,
            MAX_ENTRIES,
        );
        assert_eq!(
            c.common.as_deref(),
            Some("~/Library/Application Support/Claude/claude_desktop_config.json")
        );
        // already complete: nothing to add
        let full = "~/Library/Application Support/Claude/claude_desktop_config.json";
        assert_eq!(complete(full, home, None, MAX_ENTRIES).common, None);
    }

    #[test]
    fn hidden_entries_only_after_a_dot() {
        let t = tree();
        let home = Some(t.path());
        let all = complete("~/", home, None, MAX_ENTRIES);
        assert!(all.entries.iter().all(|e| !e.label.starts_with('.')));
        let dot = complete("~/.", home, None, MAX_ENTRIES);
        assert_eq!(dot.entries.len(), 1);
        assert_eq!(dot.entries[0].label, ".hidden");
    }

    #[test]
    fn relative_input_completes_in_the_base_folder() {
        let t = tree();
        let c = complete("libn", None, Some(t.path()), MAX_ENTRIES);
        assert_eq!(c.entries[0].value, "libnotes.txt");
    }

    #[test]
    fn limit_sets_more_and_missing_folders_give_nothing() {
        let t = tree();
        let home = Some(t.path());
        let c = complete("~/", home, None, 2);
        assert_eq!(c.entries.len(), 2);
        assert!(c.more);
        assert_eq!(
            complete("~/nope/x", home, None, 10),
            PathCompletion::default()
        );
        let tilde = complete("~", home, None, 10);
        assert_eq!(
            tilde.common.as_deref(),
            Some(&*format!("~{}", std::path::MAIN_SEPARATOR))
        );
    }

    #[test]
    fn common_start_respects_char_boundaries() {
        assert_eq!(common_start(&["Grüße", "grün"]), "Grü");
        assert_eq!(common_start(&["abc"]), "abc");
        assert_eq!(common_start(&[]), "");
    }

    #[test]
    fn start_text_uses_the_folder_of_the_document() {
        let sep = std::path::MAIN_SEPARATOR;
        let home = PathBuf::from(if cfg!(windows) {
            r"C:\Users\m"
        } else {
            "/home/m"
        });
        let doc = home.join("src").join("a.txt");
        assert_eq!(
            start_text(Some(&doc), Some(&home)),
            format!("~{sep}src{sep}")
        );
        assert_eq!(
            start_text(Some(&home.join("a.txt")), Some(&home)),
            format!("~{sep}")
        );
        assert_eq!(start_text(None, Some(&home)), format!("~{sep}"));
        let other = if cfg!(windows) {
            PathBuf::from(r"D:\x\y.txt")
        } else {
            PathBuf::from("/x/y.txt")
        };
        assert_eq!(
            start_text(Some(&other), Some(&home)),
            format!("{}{sep}", other.parent().unwrap().to_string_lossy())
        );
    }
}
