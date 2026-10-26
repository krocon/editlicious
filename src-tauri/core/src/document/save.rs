//! Saving a rope (design 002 E8): encode block by block in the document's encoding (UTF-16 by
//! hand), refuse characters the encoding cannot hold, write atomically next to the target while
//! keeping its permissions and symlinks.

use encoding_rs::{EncoderResult, Encoding, UTF_8, UTF_16BE, UTF_16LE};
use ropey::Rope;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    /// Character index (in the rope) of the first character the encoding cannot hold.
    Unmappable(usize),
}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        SaveError::Io(e)
    }
}

/// Writes `rope` in `encoding` (with a BOM if wanted) to `out`.
pub fn encode(
    rope: &Rope,
    encoding: &'static Encoding,
    bom: bool,
    out: &mut dyn Write,
) -> Result<(), SaveError> {
    if encoding == UTF_8 {
        if bom {
            out.write_all(b"\xEF\xBB\xBF")?;
        }
        for chunk in rope.chunks() {
            out.write_all(chunk.as_bytes())?;
        }
        return Ok(());
    }
    if encoding == UTF_16LE || encoding == UTF_16BE {
        let le = encoding == UTF_16LE;
        if bom {
            out.write_all(if le { b"\xFF\xFE" } else { b"\xFE\xFF" })?;
        }
        let mut buf = Vec::with_capacity(64 * 1024);
        for chunk in rope.chunks() {
            buf.clear();
            for unit in chunk.encode_utf16() {
                buf.extend_from_slice(&if le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            out.write_all(&buf)?;
        }
        return Ok(());
    }
    let mut encoder = encoding.new_encoder();
    let mut buf = vec![0u8; 64 * 1024];
    let mut chars_before = 0usize;
    let mut chunks = rope.chunks().peekable();
    loop {
        let (chunk, last) = match chunks.next() {
            Some(c) => (c, chunks.peek().is_none()),
            None => ("", true),
        };
        let mut src = chunk;
        let mut done_chars = 0usize;
        loop {
            let (result, read, written) =
                encoder.encode_from_utf8_without_replacement(src, &mut buf, last);
            out.write_all(&buf[..written])?;
            let consumed = &src[..read];
            match result {
                EncoderResult::InputEmpty => break,
                EncoderResult::OutputFull => {
                    done_chars += consumed.chars().count();
                    src = &src[read..];
                }
                EncoderResult::Unmappable(_) => {
                    // `read` includes the unmappable character
                    let index = chars_before + done_chars + consumed.chars().count() - 1;
                    return Err(SaveError::Unmappable(index));
                }
            }
        }
        chars_before += chunk.chars().count();
        if last {
            break;
        }
    }
    Ok(())
}

/// The file that is really written: a symlink is followed so it stays a symlink.
pub fn resolve_target(path: &Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => fs::canonicalize(path).unwrap_or(path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// Writes atomically: temp file in the target's folder, permissions of the old file, `fsync`,
/// `rename`. If the folder is not writable but the file is, the file is overwritten in place
/// (no atomicity possible there). On error the old file stays as it was (atomic case).
pub fn write_file(
    path: &Path,
    write: &dyn Fn(&mut dyn Write) -> Result<(), SaveError>,
) -> Result<(), SaveError> {
    let target = resolve_target(path);
    let dir = target
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "file has no folder"))?;
    let old_meta = fs::metadata(&target).ok();
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let tmp = dir.join(format!(
        ".{name}.editlicious-{}-{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let file = match OpenOptions::new().write(true).create_new(true).open(&tmp) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && old_meta.is_some() => {
            return write_in_place(&target, write);
        }
        Err(e) => return Err(e.into()),
    };
    let result = (|| {
        let mut w = BufWriter::with_capacity(1 << 20, file);
        write(&mut w)?;
        let file = w.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
        drop(file);
        if let Some(meta) = &old_meta {
            fs::set_permissions(&tmp, meta.permissions())?;
        }
        fs::rename(&tmp, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_in_place(
    target: &Path,
    write: &dyn Fn(&mut dyn Write) -> Result<(), SaveError>,
) -> Result<(), SaveError> {
    // no second chance once the file is truncated: find unmappable characters first
    write(&mut io::sink())?;
    let file = File::options().write(true).truncate(true).open(target)?;
    let mut w = BufWriter::with_capacity(1 << 20, file);
    write(&mut w)?;
    let file = w.into_inner().map_err(|e| e.into_error())?;
    file.sync_all()?;
    Ok(())
}
