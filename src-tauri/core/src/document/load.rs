//! Loading a text file into a rope (design 002 E4/E5): streamed in blocks, encoding from the BOM,
//! strict UTF-8 or a guess (`chardetng`), line endings counted on the way, cancellable.

use super::model::LineEnding;
use crate::error::{FsError, FsErrorCode};
use encoding_rs::{DecoderResult, Encoding, UTF_8, WINDOWS_1252};
use ropey::{Rope, RopeBuilder};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Size of a read block.
pub const CHUNK: usize = 4 << 20;
/// Larger files are refused (design 002 E4).
pub const MAX_FILE_BYTES: u64 = 4 << 30;
/// How much of the file `chardetng` looks at.
const SNIFF_BYTES: usize = 1 << 20;
/// Progress events at most this often.
const PROGRESS_EVERY: Duration = Duration::from_millis(50);

/// How often each line ending occurs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LineEndingStats {
    pub lf: u64,
    pub crlf: u64,
    pub cr: u64,
}

impl LineEndingStats {
    /// The most frequent line ending; ties go to LF, then CRLF. `None` for a single-line text.
    pub fn dominant(&self) -> Option<LineEnding> {
        if self.lf == 0 && self.crlf == 0 && self.cr == 0 {
            return None;
        }
        Some(if self.lf >= self.crlf && self.lf >= self.cr {
            LineEnding::Lf
        } else if self.crlf >= self.cr {
            LineEnding::Crlf
        } else {
            LineEnding::Cr
        })
    }

    pub fn mixed(&self) -> bool {
        [self.lf, self.crlf, self.cr]
            .iter()
            .filter(|&&n| n > 0)
            .count()
            > 1
    }
}

/// Counts line endings in a text that arrives in pieces (a CRLF may be split between two).
#[derive(Debug, Default)]
pub struct LineEndingCounter {
    stats: LineEndingStats,
    pending_cr: bool,
}

impl LineEndingCounter {
    pub fn feed(&mut self, text: &str) {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return;
        }
        let mut start = 0;
        if self.pending_cr {
            self.pending_cr = false;
            if bytes[0] == b'\n' {
                self.stats.crlf += 1;
                start = 1;
            } else {
                self.stats.cr += 1;
            }
        }
        let rest = &bytes[start..];
        let mut hits = memchr::memchr2_iter(b'\n', b'\r', rest);
        while let Some(i) = hits.next() {
            if rest[i] == b'\n' {
                self.stats.lf += 1;
            } else if i + 1 == rest.len() {
                // a CR at the end of a piece waits for the next piece
                self.pending_cr = true;
            } else if rest[i + 1] == b'\n' {
                self.stats.crlf += 1;
                hits.next(); // the LF of this CRLF
            } else {
                self.stats.cr += 1;
            }
        }
    }

    pub fn finish(mut self) -> LineEndingStats {
        if self.pending_cr {
            self.stats.cr += 1;
        }
        self.stats
    }
}

/// A file decoded into a rope.
#[derive(Debug)]
pub struct Loaded {
    pub rope: Rope,
    pub encoding: &'static Encoding,
    pub bom: bool,
    pub stats: LineEndingStats,
    pub decode_errors: bool,
    pub file_size: u64,
    pub mtime: Option<SystemTime>,
}

enum Decoded {
    Done(Rope, LineEndingStats, bool),
    /// Strict UTF-8 failed – try again with a guessed encoding.
    NotUtf8,
}

/// Reads `path` as text. `progress(loaded, total)` is called at most every 50 ms;
/// `cancel` stops the work (error `cancelled`).
pub fn load_file(
    path: &Path,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Loaded, FsError> {
    let io_err = |e: io::Error| FsError::from_io(&e, path);
    let meta = fs::metadata(path).map_err(io_err)?;
    if meta.is_dir() {
        return Err(FsError::new(
            FsErrorCode::IsADirectory,
            "is a directory",
            Some(path),
        ));
    }
    let total = meta.len();
    if total > MAX_FILE_BYTES {
        return Err(FsError::new(
            FsErrorCode::FileTooLarge,
            format!("{total} bytes"),
            Some(path),
        ));
    }
    let mut head = vec![0u8; CHUNK.min(total as usize)];
    let n = read_up_to(&mut File::open(path).map_err(io_err)?, &mut head).map_err(io_err)?;
    head.truncate(n);

    let (encoding, bom_len) = match Encoding::for_bom(&head) {
        Some((enc, len)) => (enc, len),
        None => {
            if memchr::memchr(0, &head).is_some() {
                return Err(FsError::new(
                    FsErrorCode::BinaryFile,
                    "NUL byte found",
                    Some(path),
                ));
            }
            (UTF_8, 0)
        }
    };
    let strict = bom_len == 0 && encoding == UTF_8;
    let (encoding, decoded) =
        match decode(path, encoding, bom_len, strict, total, cancel, progress)? {
            Decoded::Done(rope, stats, errors) => (encoding, (rope, stats, errors)),
            Decoded::NotUtf8 => {
                let guess = guess_encoding(&head, head.len() as u64 == total);
                match decode(path, guess, 0, false, total, cancel, progress)? {
                    Decoded::Done(rope, stats, errors) => (guess, (rope, stats, errors)),
                    Decoded::NotUtf8 => unreachable!("only strict decoding restarts"),
                }
            }
        };
    let (rope, stats, decode_errors) = decoded;
    Ok(Loaded {
        rope,
        encoding,
        bom: bom_len > 0,
        stats,
        decode_errors,
        file_size: total,
        mtime: meta.modified().ok(),
    })
}

/// The legacy encoding `chardetng` finds most likely (never UTF-8: that failed already).
pub fn guess_encoding(head: &[u8], whole_file: bool) -> &'static Encoding {
    let mut det = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
    let sniff = &head[..head.len().min(SNIFF_BYTES)];
    det.feed(sniff, whole_file && sniff.len() == head.len());
    let guess = det.guess(None, chardetng::Utf8Detection::Deny);
    if guess == UTF_8 { WINDOWS_1252 } else { guess }
}

fn decode(
    path: &Path,
    encoding: &'static Encoding,
    skip: usize,
    strict: bool,
    total: u64,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Decoded, FsError> {
    let io_err = |e: io::Error| FsError::from_io(&e, path);
    let mut file = File::open(path).map_err(io_err)?;
    let mut decoder = encoding.new_decoder_without_bom_handling();
    let mut builder = RopeBuilder::new();
    let mut counter = LineEndingCounter::default();
    let mut buf = vec![0u8; CHUNK];
    let mut out = String::new();
    let mut loaded = 0u64;
    let mut errors = false;
    let mut last_progress = Instant::now();
    let mut skip = skip;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(FsError::new(
                FsErrorCode::Cancelled,
                "cancelled",
                Some(path),
            ));
        }
        let n = read_up_to(&mut file, &mut buf).map_err(io_err)?;
        let last = n < buf.len();
        loaded += n as u64;
        let src = &buf[skip.min(n)..n];
        skip = 0;
        out.clear();
        let need = decoder
            .max_utf8_buffer_length(src.len())
            .ok_or_else(|| FsError::new(FsErrorCode::FileTooLarge, "buffer", Some(path)))?;
        out.reserve(need);
        if strict {
            let (result, _read) = decoder.decode_to_string_without_replacement(src, &mut out, last);
            match result {
                DecoderResult::InputEmpty => {}
                DecoderResult::Malformed(_, _) => return Ok(Decoded::NotUtf8),
                DecoderResult::OutputFull => {
                    return Err(FsError::new(
                        FsErrorCode::Io,
                        "decoder output full",
                        Some(path),
                    ));
                }
            }
        } else {
            let (result, _read, had_errors) = decoder.decode_to_string(src, &mut out, last);
            if result != encoding_rs::CoderResult::InputEmpty {
                return Err(FsError::new(
                    FsErrorCode::Io,
                    "decoder output full",
                    Some(path),
                ));
            }
            errors |= had_errors;
        }
        counter.feed(&out);
        builder.append(&out);
        if last_progress.elapsed() >= PROGRESS_EVERY {
            progress(loaded, total);
            last_progress = Instant::now();
        }
        if last {
            break;
        }
    }
    progress(loaded, total);
    Ok(Decoded::Done(builder.finish(), counter.finish(), errors))
}

/// Fills `buf` as far as the file allows; returns the number of bytes read.
fn read_up_to(file: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}
