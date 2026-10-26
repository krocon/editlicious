//! Tests of the document model (design 002). The shared cases in `testdata/document.cases.json`
//! also run against the TypeScript `MemoryDocumentApi`.

use super::load::{LineEndingCounter, load_file};
use super::model::*;
use super::*;
use serde_json::Value;
use std::fs;
use std::sync::atomic::AtomicBool;

fn lines_of(d: &Document) -> Vec<String> {
    let v = d.view(&ViewRequest {
        first_line: 0,
        line_count: MAX_VIEW_LINES,
        window_from: 0,
        window_count: MAX_WINDOW,
    });
    v.lines.into_iter().map(|l| l.text).collect()
}

fn sel_tuple(s: &Sel) -> Vec<usize> {
    vec![s.anchor.line, s.anchor.ch, s.head.line, s.head.ch]
}

fn sel_from(v: &Value) -> Sel {
    let n: Vec<usize> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as usize)
        .collect();
    Sel {
        anchor: Pos::new(n[0], n[1]),
        head: Pos::new(n[2], n[3]),
    }
}

#[test]
fn shared_cases() {
    let cases: Value =
        serde_json::from_str(include_str!("../../testdata/document.cases.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut d = Document::from_text(case["text"].as_str().unwrap());
        let sel_before: Vec<Sel> = case
            .get("selBefore")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(sel_from).collect())
            .unwrap_or_default();
        for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let ctx = format!("{name} / step {i}");
            let mut sel: Option<Vec<Sel>> = None;
            let mut error: Option<String> = None;
            match step["op"].as_str().unwrap() {
                "edit" => {
                    let changes: Vec<Change> =
                        serde_json::from_value(step["changes"].clone()).unwrap();
                    let kind: EditKind =
                        serde_json::from_value(step["kind"].clone()).unwrap_or_default();
                    match d.apply(EditRequest {
                        changes,
                        kind,
                        sel_before: sel_before.clone(),
                        sel_after: None,
                    }) {
                        Ok(s) => sel = Some(s),
                        Err(e) => {
                            error = Some(
                                serde_json::to_value(e.code)
                                    .unwrap()
                                    .as_str()
                                    .unwrap()
                                    .to_owned(),
                            )
                        }
                    }
                }
                "undo" => sel = d.undo(),
                "redo" => sel = d.redo(),
                "check" => {}
                op => panic!("{ctx}: unknown op {op}"),
            }
            let Some(expect) = step.get("expect") else {
                assert!(error.is_none(), "{ctx}: unexpected error {error:?}");
                continue;
            };
            match expect.get("error") {
                Some(code) => assert_eq!(error.as_deref(), code.as_str(), "{ctx}: error"),
                None => assert!(error.is_none(), "{ctx}: unexpected error {error:?}"),
            }
            if let Some(lines) = expect.get("lines") {
                let want: Vec<String> = serde_json::from_value(lines.clone()).unwrap();
                assert_eq!(lines_of(&d), want, "{ctx}: lines");
            }
            if let Some(raw) = expect.get("raw") {
                assert_eq!(d.rope().to_string(), raw.as_str().unwrap(), "{ctx}: raw");
            }
            if let Some(n) = expect.get("lineCount") {
                assert_eq!(
                    d.line_count() as u64,
                    n.as_u64().unwrap(),
                    "{ctx}: lineCount"
                );
            }
            if let Some(le) = expect.get("lineEnding") {
                assert_eq!(
                    serde_json::to_value(d.line_ending).unwrap(),
                    *le,
                    "{ctx}: lineEnding"
                );
            }
            let info = d.info(1);
            for (key, got) in [
                ("dirty", info.dirty),
                ("canUndo", info.can_undo),
                ("canRedo", info.can_redo),
            ] {
                if let Some(want) = expect.get(key) {
                    assert_eq!(got, want.as_bool().unwrap(), "{ctx}: {key}");
                }
            }
            if let Some(want) = expect.get("sel") {
                let want: Vec<Vec<usize>> = serde_json::from_value(want.clone()).unwrap();
                let got: Vec<Vec<usize>> = sel.unwrap_or_default().iter().map(sel_tuple).collect();
                assert_eq!(got, want, "{ctx}: sel");
            }
        }
    }
}

// ---------------------------------------------------------------- line endings

fn count(pieces: &[&str]) -> load::LineEndingStats {
    let mut c = LineEndingCounter::default();
    for p in pieces {
        c.feed(p);
    }
    c.finish()
}

#[test]
fn counts_line_endings_across_pieces() {
    let s = count(&["a\r", "\nb\n", "c\r", "d\r"]);
    assert_eq!((s.lf, s.crlf, s.cr), (1, 1, 2));
    assert!(s.mixed());
    let s = count(&["x\r\n\r\n", "\r", "\n"]);
    assert_eq!((s.lf, s.crlf, s.cr), (0, 3, 0));
    assert_eq!(s.dominant(), Some(LineEnding::Crlf));
    assert_eq!(count(&["no breaks"]).dominant(), None);
    let s = count(&["\r", "", "\r"]);
    assert_eq!(s.cr, 2);
}

// ---------------------------------------------------------------- loading

fn write(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = dir.path().join(name);
    fs::write(&p, bytes).unwrap();
    p
}

fn load(p: &std::path::Path) -> Result<load::Loaded, FsError> {
    load_file(p, &AtomicBool::new(false), &mut |_, _| {})
}

#[test]
fn loads_utf8_with_and_without_bom() {
    let dir = tempfile::tempdir().unwrap();
    let l = load(&write(&dir, "a.txt", "grüß 😀\n".as_bytes())).unwrap();
    assert_eq!(l.encoding, UTF_8);
    assert!(!l.bom);
    assert_eq!(l.rope.to_string(), "grüß 😀\n");
    let l = load(&write(&dir, "b.txt", b"\xEF\xBB\xBFhi")).unwrap();
    assert!(l.bom);
    assert_eq!(l.rope.to_string(), "hi");
}

#[test]
fn loads_utf16_by_bom() {
    let dir = tempfile::tempdir().unwrap();
    let mut le = vec![0xFF, 0xFE];
    le.extend("Zeile\r\nä".encode_utf16().flat_map(|u| u.to_le_bytes()));
    let l = load(&write(&dir, "le.txt", &le)).unwrap();
    assert_eq!(l.encoding, UTF_16LE);
    assert_eq!(l.rope.to_string(), "Zeile\r\nä");
    assert_eq!(l.stats.dominant(), Some(LineEnding::Crlf));
    let mut be = vec![0xFE, 0xFF];
    be.extend("ok".encode_utf16().flat_map(|u| u.to_be_bytes()));
    let l = load(&write(&dir, "be.txt", &be)).unwrap();
    assert_eq!(l.encoding, UTF_16BE);
    assert_eq!(l.rope.to_string(), "ok");
}

#[test]
fn guesses_legacy_encodings() {
    let dir = tempfile::tempdir().unwrap();
    // windows-1252: "Grüße aus Königstein – schön"
    let (bytes, _, _) =
        encoding_rs::WINDOWS_1252.encode("Grüße aus Königstein – schön, Äpfel für Öl.\n");
    let l = load(&write(&dir, "w.txt", &bytes)).unwrap();
    assert_eq!(l.encoding.name(), "windows-1252");
    assert!(l.rope.to_string().starts_with("Grüße aus Königstein"));
    assert!(!l.decode_errors);
    // Shift_JIS
    let (bytes, _, _) = encoding_rs::SHIFT_JIS
        .encode("日本語のテキストファイルです。これはテストです。漢字とひらがなとカタカナ。\n");
    let l = load(&write(&dir, "sjis.txt", &bytes)).unwrap();
    assert_eq!(l.encoding.name(), "Shift_JIS");
    assert!(l.rope.to_string().starts_with("日本語"));
}

#[test]
fn refuses_binary_files_and_folders() {
    let dir = tempfile::tempdir().unwrap();
    let e = load(&write(&dir, "bin", b"PK\x03\x04\x00\x00abc")).unwrap_err();
    assert_eq!(e.code, FsErrorCode::BinaryFile);
    let e = load(dir.path()).unwrap_err();
    assert_eq!(e.code, FsErrorCode::IsADirectory);
    let e = load(&dir.path().join("missing")).unwrap_err();
    assert_eq!(e.code, FsErrorCode::NotFound);
}

#[test]
fn loads_empty_file_and_reports_progress() {
    let dir = tempfile::tempdir().unwrap();
    let l = load(&write(&dir, "empty", b"")).unwrap();
    assert_eq!(l.rope.len_chars(), 0);
    let big = "0123456789abcdef\n".repeat(600_000); // ~10 MB, three blocks
    let p = write(&dir, "big.txt", big.as_bytes());
    let mut last = (0, 0);
    let l = load_file(&p, &AtomicBool::new(false), &mut |a, b| last = (a, b)).unwrap();
    assert_eq!(last, (big.len() as u64, big.len() as u64));
    assert_eq!(l.rope.len_lines(), 600_001);
    assert_eq!(l.stats.lf, 600_000);
}

#[test]
fn crlf_split_between_blocks_stays_one_break() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = "x".repeat(load::CHUNK - 1);
    s.push_str("\r\nend");
    let l = load(&write(&dir, "split.txt", s.as_bytes())).unwrap();
    assert_eq!(l.rope.len_lines(), 2);
    assert_eq!((l.stats.crlf, l.stats.cr, l.stats.lf), (1, 0, 0));
}

#[test]
fn utf8_char_split_between_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = "x".repeat(load::CHUNK - 1);
    s.push('€'); // 3 bytes across the block border
    let l = load(&write(&dir, "eur.txt", s.as_bytes())).unwrap();
    assert_eq!(l.encoding, UTF_8);
    assert!(l.rope.to_string().ends_with('€'));
}

#[test]
fn cancelling_a_load() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "c.txt", b"abc");
    let e = load_file(&p, &AtomicBool::new(true), &mut |_, _| {}).unwrap_err();
    assert_eq!(e.code, FsErrorCode::Cancelled);
}

// ---------------------------------------------------------------- view

#[test]
fn long_lines_come_as_windows() {
    let long = "ab".repeat(FULL_LINE_LIMIT); // 20 000 chars
    let d = Document::from_text(&format!("short\n{long}\n"));
    let v = d.view(&ViewRequest {
        first_line: 0,
        line_count: 10,
        window_from: 15_000,
        window_count: 100,
    });
    assert_eq!(v.lines.len(), 3);
    assert_eq!(v.lines[0].text, "short");
    assert_eq!(v.lines[0].from, 0);
    assert_eq!(v.lines[1].from, 15_000);
    assert_eq!(v.lines[1].len, 20_000);
    assert_eq!(v.lines[1].text.len(), 100);
    assert_eq!(v.lines[2].text, "");
    // a window behind the end is empty, not an error
    let v = d.view(&ViewRequest {
        first_line: 1,
        line_count: 1,
        window_from: 99_999,
        window_count: 10,
    });
    assert_eq!(v.lines[0].text, "");
    assert_eq!(v.lines[0].from, 20_000);
}

#[test]
fn view_clamps_lines() {
    let d = Document::from_text("a\nb");
    let v = d.view(&ViewRequest {
        first_line: 7,
        line_count: 5,
        window_from: 0,
        window_count: 0,
    });
    assert_eq!(v.first_line, 1);
    assert_eq!(v.lines.len(), 1);
    assert_eq!(v.line_count, 2);
}

#[test]
fn text_between_positions() {
    let d = Document::from_text("one\r\ntwo\r\nthree");
    assert_eq!(d.text(Pos::new(0, 1), Pos::new(1, 2)).unwrap(), "ne\r\ntw");
    // reversed positions work too
    assert_eq!(d.text(Pos::new(2, 5), Pos::new(2, 0)).unwrap(), "three");
}

// ---------------------------------------------------------------- saving

fn saved_bytes(d: &mut Document, dir: &tempfile::TempDir, name: &str) -> Vec<u8> {
    let p = dir.path().join(name);
    d.save(&SaveRequest {
        path: Some(p.to_string_lossy().into_owned()),
        ..Default::default()
    })
    .unwrap();
    fs::read(p).unwrap()
}

#[test]
fn save_round_trips_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let samples: Vec<Vec<u8>> = vec![
        "plain\nutf8 😀\n".as_bytes().to_vec(),
        b"\xEF\xBB\xBFbom\r\nkept".to_vec(),
        encoding_rs::WINDOWS_1252
            .encode("Grüße aus Königstein, schöne Äpfel und Öl für Straßen.\r\n")
            .0
            .to_vec(),
        {
            let mut v = vec![0xFF, 0xFE];
            v.extend("utf16 ä\n".encode_utf16().flat_map(|u| u.to_le_bytes()));
            v
        },
        b"mixed\rline\nendings\r\n".to_vec(),
    ];
    for (i, bytes) in samples.iter().enumerate() {
        let p = write(&dir, &format!("in{i}"), bytes);
        let mut d = Document::from_loaded(load(&p).unwrap(), &p);
        assert_eq!(
            &saved_bytes(&mut d, &dir, &format!("out{i}")),
            bytes,
            "sample {i}"
        );
    }
}

#[test]
fn save_marks_clean_and_undo_back_is_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "f.txt", b"abc");
    let mut d = Document::from_loaded(load(&p).unwrap(), &p);
    d.apply(EditRequest {
        changes: vec![Change {
            from: Pos::new(0, 3),
            to: Pos::new(0, 3),
            text: "d".into(),
        }],
        kind: EditKind::Type,
        sel_before: vec![],
        sel_after: None,
    })
    .unwrap();
    assert!(d.dirty());
    d.save(&SaveRequest::default()).unwrap();
    assert!(!d.dirty());
    assert_eq!(fs::read(&p).unwrap(), b"abcd");
    // typing after the save is a new step (no merge across the save)
    d.apply(EditRequest {
        changes: vec![Change {
            from: Pos::new(0, 4),
            to: Pos::new(0, 4),
            text: "e".into(),
        }],
        kind: EditKind::Type,
        sel_before: vec![],
        sel_after: None,
    })
    .unwrap();
    d.undo();
    assert!(!d.dirty(), "back at the saved text");
    d.undo();
    assert!(d.dirty());
    assert_eq!(d.rope().to_string(), "abc");
}

#[test]
fn unmappable_characters_are_refused_and_file_stays() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode("Grüße und schöne Äpfel für Öl.\n");
    let p = write(&dir, "w.txt", &bytes);
    let mut d = Document::from_loaded(load(&p).unwrap(), &p);
    d.apply(EditRequest {
        changes: vec![Change {
            from: Pos::new(1, 0),
            to: Pos::new(1, 0),
            text: "ok 日本".into(),
        }],
        kind: EditKind::Other,
        sel_before: vec![],
        sel_after: None,
    })
    .unwrap();
    let e = d.save(&SaveRequest::default()).unwrap_err();
    assert_eq!(e.code, FsErrorCode::UnmappableCharacter);
    assert_eq!(e.message, "1:3");
    assert_eq!(fs::read(&p).unwrap(), bytes.to_vec(), "old file untouched");
    // saving as UTF-8 works
    d.save(&SaveRequest {
        encoding: Some("utf-8".into()),
        ..Default::default()
    })
    .unwrap();
    assert!(
        String::from_utf8(fs::read(&p).unwrap())
            .unwrap()
            .ends_with("ok 日本")
    );
    assert_eq!(d.info(1).encoding, "UTF-8");
}

#[test]
fn save_rejects_bad_targets() {
    let mut d = Document::from_text("x");
    assert_eq!(
        d.save(&SaveRequest::default()).unwrap_err().code,
        FsErrorCode::InvalidPath
    );
    assert_eq!(
        d.save(&SaveRequest {
            path: Some("relative.txt".into()),
            ..Default::default()
        })
        .unwrap_err()
        .code,
        FsErrorCode::InvalidPath
    );
    assert_eq!(
        d.save(&SaveRequest {
            path: Some("/tmp/x.txt".into()),
            encoding: Some("no-such-encoding".into()),
            ..Default::default()
        })
        .unwrap_err()
        .code,
        FsErrorCode::UnsupportedEncoding
    );
    assert_eq!(
        encoding_for_label("replacement").unwrap_err().code,
        FsErrorCode::UnsupportedEncoding
    );
}

#[cfg(unix)]
#[test]
fn save_keeps_permissions_and_symlinks() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let real = write(&dir, "real.sh", b"echo hi\n");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o750)).unwrap();
    let link = dir.path().join("link.sh");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let mut d = Document::from_loaded(load(&link).unwrap(), &link);
    d.apply(EditRequest {
        changes: vec![Change {
            from: Pos::new(1, 0),
            to: Pos::new(1, 0),
            text: "echo 2\n".into(),
        }],
        kind: EditKind::Other,
        sel_before: vec![],
        sel_after: None,
    })
    .unwrap();
    d.save(&SaveRequest::default()).unwrap();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&real).unwrap(), b"echo hi\necho 2\n");
    assert_eq!(
        fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o750
    );
    // no temp files left behind
    let names: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(names.len(), 2);
}

#[test]
fn new_document_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let mut d = Document::new_empty(LineEnding::Lf);
    assert_eq!(d.name(), "Untitled");
    assert!(!d.dirty());
    d.apply(EditRequest {
        changes: vec![Change {
            from: Pos::new(0, 0),
            to: Pos::new(0, 0),
            text: "a\r\nb".into(),
        }],
        kind: EditKind::Other,
        sel_before: vec![],
        sel_after: None,
    })
    .unwrap();
    assert_eq!(saved_bytes(&mut d, &dir, "neu ä 😀.txt"), b"a\nb");
    assert_eq!(d.name(), "neu ä 😀.txt");
}

// ---------------------------------------------------------------- measurements (ignored)

/// `cargo test --release -- --ignored --nocapture measure_` – numbers go to docs/architecture.md.
#[test]
#[ignore]
fn measure_big_files() {
    use std::time::Instant;
    let dir = tempfile::tempdir().unwrap();
    let line = "2026-10-26 08:00:00 INFO  [main] editlicious handles big files with ease – line\n";
    let text = line.repeat(1_300_000); // ~100 MB, 1.3 M lines
    let p = write(&dir, "big.log", text.as_bytes());
    drop(text);
    let t = Instant::now();
    let d = Document::from_loaded(load(&p).unwrap(), &p);
    println!(
        "open {} MB / {} lines: {:?}",
        d.info(1).file_size >> 20,
        d.line_count(),
        t.elapsed()
    );
    let t = Instant::now();
    let v = d.view(&ViewRequest {
        first_line: d.line_count() - 60,
        line_count: 60,
        window_from: 0,
        window_count: 400,
    });
    println!(
        "view at the end: {:?} ({} lines)",
        t.elapsed(),
        v.lines.len()
    );

    let long = "x".repeat(10_000_000);
    let mut d = Document::from_text(&long);
    let t = Instant::now();
    let v = d.view(&ViewRequest {
        first_line: 0,
        line_count: 1,
        window_from: 5_000_000,
        window_count: 400,
    });
    println!("window in a 10 M char line: {:?}", t.elapsed());
    assert_eq!(v.lines[0].text.len(), 400);
    let t = Instant::now();
    for i in 0..100 {
        d.apply(EditRequest {
            changes: vec![Change {
                from: Pos::new(0, 10_000_000 + i),
                to: Pos::new(0, 10_000_000 + i),
                text: "y".into(),
            }],
            kind: EditKind::Type,
            sel_before: vec![],
            sel_after: None,
        })
        .unwrap();
    }
    println!(
        "100 keystrokes at the end of a 10 M char line: {:?}",
        t.elapsed()
    );
}

#[test]
fn disk_state_sees_changes_from_outside() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(&dir, "d.txt", b"one");
    let d = Document::from_loaded(load(&p).unwrap(), &p);
    assert_eq!(d.disk_state(), DiskState::default());
    fs::write(&p, b"other text").unwrap();
    assert!(d.disk_state().changed);
    fs::remove_file(&p).unwrap();
    assert!(d.disk_state().missing);
    assert_eq!(Document::from_text("x").disk_state(), DiskState::default());
}
