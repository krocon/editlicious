//! Measurements of prompt 013 (ignored tests, release build):
//!
//! ```text
//! cd src-tauri && cargo test -p ed-core --release -- --ignored --nocapture --test-threads 1 measure_013
//! ED_MEASURE_MB=1000 …   (size of the big file, default 1000 MB)
//! ```
//!
//! Every step goes through the same session commands as the app; memory is the resident set of
//! the test process (Linux).

use crate::document::Document;
use crate::document::load::load_file;
use crate::document::model::{LineEnding, SaveRequest};
use crate::editor::model::{Command, Motion};
use crate::editor::{MemoryClipboard, Session};
use crate::search::{self, Query};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

fn rss() -> String {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let get = |k: &str| {
        s.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|n| n.parse::<u64>().ok())
            .map(|kb| kb >> 10)
            .unwrap_or(0)
    };
    format!("RSS {} MB (peak {} MB)", get("VmRSS:"), get("VmHWM:"))
}

fn step<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    println!(
        "{what:<44} {:>10.1} ms   {}",
        t.elapsed().as_secs_f64() * 1e3,
        rss()
    );
    r
}

fn run(s: &mut Session, cmd: Command) {
    s.run(cmd, &mut MemoryClipboard::default()).unwrap();
}

fn open(p: &Path) -> Session {
    let l = load_file(p, &AtomicBool::new(false), &mut |_, _| {}).unwrap();
    let mut s = Session::new(Document::from_loaded(l, p), 1);
    run(
        &mut s,
        Command::Resize {
            lines: 45.0,
            cols: 140.0,
        },
    );
    s
}

fn q(p: &str) -> Query {
    Query {
        pattern: p.into(),
        ..Default::default()
    }
}

/// `lines` lines of ~100 bytes; every 1000th has "needle".
fn write_big(p: &Path, lines: usize) {
    let mut f = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(p).unwrap());
    for i in 0..lines {
        let lvl = ["INFO ", "DEBUG", "WARN ", "ERROR"][i % 4];
        let tag = if i % 1000 == 0 { "needle" } else { "hay" };
        writeln!(
            f,
            "2026-10-26 08:{:02}:{:02}.{:03} {lvl} [worker-{}] request {i:>9} served {tag:>6} in {:>4} ms",
            (i / 60_000) % 60,
            (i / 1000) % 60,
            i % 1000,
            i % 8,
            (i * 7919) % 1000
        )
        .unwrap();
    }
}

#[test]
#[ignore]
fn measure_013() {
    let mb: usize = std::env::var("ED_MEASURE_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let dir = tempfile::tempdir().unwrap();
    println!(
        "start                                                         {}",
        rss()
    );

    // ---------------------------------------------------------------- 1 GB, 10 M lines
    let lines = mb * 10_000;
    let big = dir.path().join("big.log");
    step(
        &format!("write {mb} MB / {lines} lines (generator)"),
        || write_big(&big, lines),
    );
    let mut s = step("open", || open(&big));
    step("first frame", || run(&mut s, Command::Frame));
    step("jump to the end", || {
        run(
            &mut s,
            Command::Move {
                to: Motion::DocEnd,
                select: false,
            },
        )
    });
    step("jump to the middle (go to line)", || {
        run(
            &mut s,
            Command::GoTo {
                line: lines / 2,
                ch: 0,
                select: false,
            },
        )
    });
    step("100 keystrokes in the middle", || {
        for _ in 0..100 {
            run(&mut s, Command::Type { text: "x".into() });
        }
    });
    step("  … same without highlighting", || {
        run(&mut s, Command::SetLanguage { name: None });
        for _ in 0..100 {
            run(&mut s, Command::Type { text: "x".into() });
        }
    });
    step("find next from the middle", || {
        run(
            &mut s,
            Command::Find {
                query: q("needle"),
                backwards: false,
                incremental: false,
            },
        )
    });
    let (rope, _, _) = s.search_snapshot();
    let re = search::compile(&q("needle")).unwrap();
    let n = step("count all matches (1 per 1000 lines)", || {
        search::count(&rope, &re, 0, &AtomicBool::new(false))
            .unwrap()
            .0
    });
    assert_eq!(n, lines / 1000);
    drop(rope);
    step("replace all (10 000 per GB)", || {
        run(
            &mut s,
            Command::ReplaceAll {
                query: q("needle"),
                replacement: "NEEDLE".into(),
            },
        )
    });
    step("undo replace all", || run(&mut s, Command::Undo));
    let saved = dir.path().join("saved.log");
    step("save as (atomic)", || {
        s.doc
            .save(&SaveRequest {
                path: Some(saved.to_string_lossy().into_owned()),
                ..Default::default()
            })
            .unwrap()
    });
    let _ = std::fs::remove_file(&saved);
    step("convert line endings to CRLF", || {
        run(
            &mut s,
            Command::SetLineEnding {
                line_ending: LineEnding::Crlf,
            },
        )
    });
    step("undo it", || run(&mut s, Command::Undo));
    step("sort all lines: refused above 512 MB", || {
        run(&mut s, Command::SelectAll);
        let f = s
            .run(
                Command::SortLines {
                    options: Default::default(),
                },
                &mut MemoryClipboard::default(),
            )
            .unwrap();
        assert_eq!(f.notice.as_deref(), Some("sort_too_large"));
    });
    step("sort 2 M lines (~200 MB)", || {
        run(
            &mut s,
            Command::GoTo {
                line: 0,
                ch: 0,
                select: false,
            },
        );
        run(
            &mut s,
            Command::GoTo {
                line: 2_000_000,
                ch: 0,
                select: true,
            },
        );
        run(
            &mut s,
            Command::SortLines {
                options: Default::default(),
            },
        )
    });
    step("undo the sort", || run(&mut s, Command::Undo));
    drop(s);
    let _ = std::fs::remove_file(&big);
    println!(
        "after closing                                                 {}",
        rss()
    );

    // ---------------------------------------------------------------- one 100 MB line
    let long = dir.path().join("long.txt");
    {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&long).unwrap());
        for i in 0..10_000_000u32 {
            write!(f, "{:09},", i).unwrap();
        }
        writeln!(f).unwrap();
    }
    let mut s = step("open a 100 MB line", || open(&long));
    step("end of the line (window)", || {
        run(
            &mut s,
            Command::Move {
                to: Motion::LineEnd,
                select: false,
            },
        )
    });
    step("100 keystrokes at its end", || {
        for _ in 0..100 {
            run(&mut s, Command::Type { text: "y".into() });
        }
    });
    step("find backwards from the end", || {
        run(
            &mut s,
            Command::Find {
                query: q("000000001,"),
                backwards: true,
                incremental: false,
            },
        )
    });
    step("save in place", || {
        s.doc.save(&SaveRequest::default()).unwrap()
    });
    drop(s);

    // ---------------------------------------------------------------- mixed line endings
    let mixed = dir.path().join("mixed.txt");
    {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&mixed).unwrap());
        for i in 0..1_000_000 {
            let le = ["\n", "\r\n", "\r"][i % 3];
            write!(f, "line {i} with mixed endings{le}").unwrap();
        }
    }
    let mut s = step("open 1 M lines with LF/CRLF/CR", || open(&mixed));
    assert!(s.doc.info(1).mixed_line_endings);
    step("convert all to LF", || {
        run(
            &mut s,
            Command::SetLineEnding {
                line_ending: LineEnding::Lf,
            },
        )
    });
    step("save", || s.doc.save(&SaveRequest::default()).unwrap());
}
