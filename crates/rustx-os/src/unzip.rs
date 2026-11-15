//! Reading the archive `rustx package` wrote.
//!
//! The other half of the packaging story: the command line writes a
//! `.zip`, and the updater inside a running application reads one. The
//! format's reader is here rather than taken from a crate for the same
//! reason the writer is hand-written over in `rustx-cli` -- what is
//! needed is the central directory, stored and deflated entries, and
//! nothing else.
//!
//! What it refuses rather than does: an entry whose name would land
//! outside the directory it is unpacking into. A `..` in a zip entry is
//! the oldest archive attack there is, and this unpacks something
//! downloaded from the internet.

use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::error::OsError;

/// Unpack `archive` into `into`, and say how many files were written.
pub fn into(archive: &Path, into: &Path) -> Result<usize, OsError> {
    let bytes = std::fs::read(archive).map_err(|error| {
        OsError::Platform(format!("{} could not be read ({error})", archive.display()))
    })?;
    let entries = entries_of(&bytes)?;
    let mut written = 0;
    for entry in entries {
        let Some(relative) = safe(&entry.name) else {
            return Err(OsError::Platform(format!(
                "`{}` in that archive would be written outside {}, so nothing was \
                 unpacked",
                entry.name,
                into.display()
            )));
        };
        if entry.name.ends_with('/') {
            continue;
        }
        let path = into.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                OsError::Platform(format!("{} could not be made ({error})", parent.display()))
            })?;
        }
        let body = body_of(&bytes, &entry)?;
        std::fs::write(&path, &body).map_err(|error| {
            OsError::Platform(format!("{} could not be written ({error})", path.display()))
        })?;
        written += 1;
    }
    Ok(written)
}

/// The names inside an archive, without unpacking it.
pub fn names(archive: &Path) -> Result<Vec<String>, OsError> {
    let bytes = std::fs::read(archive).map_err(|error| {
        OsError::Platform(format!("{} could not be read ({error})", archive.display()))
    })?;
    Ok(entries_of(&bytes)?
        .into_iter()
        .map(|entry| entry.name)
        .collect())
}

/// One file, as the central directory describes it.
#[derive(Debug, Clone)]
struct Entry {
    name: String,
    method: u16,
    stored: u64,
    plain: u64,
    at: u64,
}

/// Read the central directory, which is the only honest way to read a
/// zip: the local headers can lie about sizes, and the directory at the
/// end is what every writer keeps right.
fn entries_of(bytes: &[u8]) -> Result<Vec<Entry>, OsError> {
    let end = bytes
        .len()
        .checked_sub(22)
        .and_then(|from| {
            (0..=from.min(65_557))
                .find(|back| {
                    let at = from - back;
                    bytes[at..at + 4] == [0x50, 0x4b, 0x05, 0x06]
                })
                .map(|back| from - back)
        })
        .ok_or_else(|| {
            OsError::Platform(
                "that file is not a zip archive: it has no end-of-directory record".to_string(),
            )
        })?;

    let mut count = u16::from_le_bytes([bytes[end + 10], bytes[end + 11]]) as u64;
    let mut start = u32::from_le_bytes([
        bytes[end + 16],
        bytes[end + 17],
        bytes[end + 18],
        bytes[end + 19],
    ]) as u64;

    // Zip64, which a package of a browser engine needs: the plain
    // record keeps 0xffff / 0xffffffff and the real numbers are in the
    // record before it.
    if count == u16::MAX as u64 || start == u32::MAX as u64 {
        let locator = end.checked_sub(20).ok_or_else(|| {
            OsError::Platform("that archive says it is zip64 and has no locator".to_string())
        })?;
        if bytes[locator..locator + 4] != [0x50, 0x4b, 0x06, 0x07] {
            return Err(OsError::Platform(
                "that archive says it is zip64 and its locator is not there".to_string(),
            ));
        }
        let record = u64::from_le_bytes(eight(bytes, locator + 8)?) as usize;
        if bytes.get(record..record + 4) != Some(&[0x50, 0x4b, 0x06, 0x06]) {
            return Err(OsError::Platform(
                "that archive's zip64 directory is not where it says".to_string(),
            ));
        }
        count = u64::from_le_bytes(eight(bytes, record + 32)?);
        start = u64::from_le_bytes(eight(bytes, record + 48)?);
    }

    let mut at = start as usize;
    let mut found = Vec::new();
    for _ in 0..count {
        if bytes.get(at..at + 4) != Some(&[0x50, 0x4b, 0x01, 0x02]) {
            break;
        }
        let method = u16::from_le_bytes([bytes[at + 10], bytes[at + 11]]);
        let stored = u32::from_le_bytes(four(bytes, at + 20)?) as u64;
        let plain = u32::from_le_bytes(four(bytes, at + 24)?) as u64;
        let name_length = u16::from_le_bytes([bytes[at + 28], bytes[at + 29]]) as usize;
        let extra_length = u16::from_le_bytes([bytes[at + 30], bytes[at + 31]]) as usize;
        let comment_length = u16::from_le_bytes([bytes[at + 32], bytes[at + 33]]) as usize;
        let offset = u32::from_le_bytes(four(bytes, at + 42)?) as u64;
        let name = String::from_utf8_lossy(
            bytes
                .get(at + 46..at + 46 + name_length)
                .ok_or_else(|| OsError::Platform("that archive ends mid-name".to_string()))?,
        )
        .to_string();
        found.push(Entry {
            name,
            method,
            stored,
            plain,
            at: offset,
        });
        at += 46 + name_length + extra_length + comment_length;
    }
    Ok(found)
}

/// The bytes of one entry, read through its local header -- which is
/// where the body starts, and whose own name and extra fields are the
/// only thing between the header and the data.
fn body_of(bytes: &[u8], entry: &Entry) -> Result<Vec<u8>, OsError> {
    let at = entry.at as usize;
    if bytes.get(at..at + 4) != Some(&[0x50, 0x4b, 0x03, 0x04]) {
        return Err(OsError::Platform(format!(
            "`{}` is not where that archive's directory says it is",
            entry.name
        )));
    }
    let name_length = u16::from_le_bytes([bytes[at + 26], bytes[at + 27]]) as usize;
    let extra_length = u16::from_le_bytes([bytes[at + 28], bytes[at + 29]]) as usize;
    let from = at + 30 + name_length + extra_length;
    let body = bytes
        .get(from..from + entry.stored as usize)
        .ok_or_else(|| {
            OsError::Platform(format!("`{}` is cut short in that archive", entry.name))
        })?;

    match entry.method {
        0 => Ok(body.to_vec()),
        8 => {
            let mut out = Vec::with_capacity(entry.plain as usize);
            flate2::read::DeflateDecoder::new(body)
                .read_to_end(&mut out)
                .map_err(|error| {
                    OsError::Platform(format!(
                        "`{}` could not be decompressed ({error})",
                        entry.name
                    ))
                })?;
            Ok(out)
        }
        other => Err(OsError::Platform(format!(
            "`{}` is compressed with method {other}, which this reads nothing of. \
             `rustx package` writes stored and deflated entries only",
            entry.name
        ))),
    }
}

/// A name from an archive, as a relative path that cannot leave the
/// directory -- or `None`, which is a refusal and not a correction.
fn safe(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() || name.starts_with('/') || name.starts_with('\\') {
        return None;
    }
    if name.len() >= 2 && name.as_bytes()[1] == b':' {
        // `C:\…`, which `is_absolute` does not catch when the archive
        // was written on another system.
        return None;
    }
    let mut out = PathBuf::new();
    for piece in path.components() {
        match piece {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

fn four(bytes: &[u8], at: usize) -> Result<[u8; 4], OsError> {
    bytes
        .get(at..at + 4)
        .and_then(|piece| piece.try_into().ok())
        .ok_or_else(|| OsError::Platform("that archive ends mid-record".to_string()))
}

fn eight(bytes: &[u8], at: usize) -> Result<[u8; 8], OsError> {
    bytes
        .get(at..at + 8)
        .and_then(|piece| piece.try_into().ok())
        .ok_or_else(|| OsError::Platform("that archive ends mid-record".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check that matters, and the one a test can make without an
    /// archive: a name that climbs out is refused rather than cleaned.
    #[test]
    fn a_name_that_climbs_out_of_the_directory_is_refused() {
        assert!(safe("../outside.exe").is_none());
        assert!(safe("a/../../outside.exe").is_none());
        assert!(safe("/etc/passwd").is_none());
        assert!(safe("C:/Windows/System32/evil.dll").is_none());
        assert!(safe("\\\\server\\share\\x").is_none());
    }

    #[test]
    fn an_ordinary_name_is_a_relative_path() {
        assert_eq!(safe("mon-app.exe"), Some(PathBuf::from("mon-app.exe")));
        assert_eq!(
            safe("locales/fr.pak"),
            Some(PathBuf::from("locales").join("fr.pak"))
        );
        assert_eq!(safe("./a.txt"), Some(PathBuf::from("a.txt")));
    }

    #[test]
    fn something_that_is_not_an_archive_says_so() {
        let said = entries_of(b"not a zip at all").expect_err("refused");
        assert!(
            format!("{said}").contains("not a zip"),
            "the message names what is wrong: {said}"
        );
    }
}
