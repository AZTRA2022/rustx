//! A `.zip`, written here.
//!
//! A release page holds an archive, so `rustx package` writes one. The
//! format is thirty years old and the part of it this needs is three
//! records: a local header before each file, a central directory
//! entry per file, and one end-of-directory record. Deflate comes from
//! `flate2`, which is already in this workspace's dependency graph, and
//! the CRC-32 the format wants comes with it.
//!
//! Zip64 is written when it is needed — a `libcef.dll` is 280 MB and a
//! package of several is well past the 4 GB a plain zip can describe,
//! so the end record gets its 64-bit form rather than silently
//! wrapping. Each individual file is assumed to be under 4 GB, which
//! `libcef.dll` is by two orders of magnitude.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Zip `from` into `into`, with the folder's own name as the single
/// top-level directory inside the archive.
///
/// That last part is not decoration: an archive that unpacks its files
/// into whatever directory the person was standing in is the one thing
/// everybody has been annoyed by, and a package is unpacked by somebody
/// who is about to run it.
pub fn write(from: &Path, into: &Path) -> Result<(), String> {
    let root = from
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "package".to_string());

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    walk(from, &root, &mut files);
    files.sort();

    if let Some(parent) = into.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} could not be made ({error}).", parent.display()))?;
    }
    let handle = std::fs::File::create(into)
        .map_err(|error| format!("{} could not be written ({error}).", into.display()))?;
    let mut out = std::io::BufWriter::new(handle);
    let mut at: u64 = 0;
    let mut directory: Vec<u8> = Vec::new();
    let count = files.len();

    for (name, path) in files {
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("{} could not be read ({error}).", path.display()))?;
        let crc = crc32(&bytes);
        let plain = bytes.len() as u32;
        let squeezed = deflated(&bytes)?;
        // A file that deflate made bigger is stored as it is. It
        // happens, on a `.pak` that is already compressed, and an
        // archive that is larger than the folder it holds is an
        // archive somebody will not trust.
        let (method, body) = if squeezed.len() < bytes.len() {
            (8_u16, squeezed)
        } else {
            (0_u16, bytes)
        };

        let header = local_header(&name, crc, body.len() as u32, plain, method);
        out.write_all(&header)
            .map_err(|error| format!("{} could not be written ({error}).", into.display()))?;
        out.write_all(&body)
            .map_err(|error| format!("{} could not be written ({error}).", into.display()))?;

        directory.extend_from_slice(&central_entry(
            &name,
            crc,
            body.len() as u32,
            plain,
            method,
            at,
        ));
        at += header.len() as u64 + body.len() as u64;
    }

    let start = at;
    out.write_all(&directory)
        .map_err(|error| format!("{} could not be written ({error}).", into.display()))?;
    out.write_all(&end_of_directory(count, directory.len() as u64, start))
        .map_err(|error| format!("{} could not be written ({error}).", into.display()))?;
    out.flush()
        .map_err(|error| format!("{} could not be finished ({error}).", into.display()))?;
    Ok(())
}

fn walk(directory: &Path, prefix: &str, into: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if path.is_dir() {
            walk(&path, &name, into);
        } else if path.is_file() {
            into.push((name, path));
        }
    }
}

fn deflated(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|error| format!("the archive could not be compressed ({error})."))?;
    encoder
        .finish()
        .map_err(|error| format!("the archive could not be compressed ({error})."))
}

/// CRC-32, the one the format names: the reflected polynomial every
/// zip uses.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = flate2::Crc::new();
    crc.update(bytes);
    crc.sum()
}

const VERSION: u16 = 20;
/// Bit 11: the name is UTF-8. Without it, a path with an accent in it
/// is read in the unpacking machine's code page.
const UTF8: u16 = 1 << 11;

fn local_header(name: &str, crc: u32, stored: u32, plain: u32, method: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(30 + name.len());
    out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&UTF8.to_le_bytes());
    out.extend_from_slice(&method.to_le_bytes());
    // No timestamp. A package built twice from the same tree is the
    // same bytes, which is worth more here than the minute it was
    // built -- the manifest beside it carries the version.
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&stored.to_le_bytes());
    out.extend_from_slice(&plain.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out
}

fn central_entry(
    name: &str,
    crc: u32,
    stored: u32,
    plain: u32,
    method: u16,
    offset: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(46 + name.len());
    out.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
    // Made by: 0x031e is Unix, 3.0. The attributes below are Unix's,
    // which is what makes an unpacked file readable on macOS and Linux
    // rather than mode 000.
    out.extend_from_slice(&0x031e_u16.to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&UTF8.to_le_bytes());
    out.extend_from_slice(&method.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&stored.to_le_bytes());
    out.extend_from_slice(&plain.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    // External attributes: 0o755 for an executable, in the high
    // sixteen bits where Unix keeps its mode. Everything in a package
    // is either the program or something beside it, and the program
    // has to be runnable after unpacking.
    out.extend_from_slice(&0x81ed_0000_u32.to_le_bytes());
    out.extend_from_slice(&(offset.min(u32::MAX as u64) as u32).to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out
}

fn end_of_directory(count: usize, size: u64, start: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(22);
    let many = count > u16::MAX as usize;
    let far = start > u32::MAX as u64 || size > u32::MAX as u64;

    if many || far {
        // Zip64, because a package of Chromium's runtime is not small.
        let record_at = start + size;
        out.extend_from_slice(&0x0606_4b50_u32.to_le_bytes());
        out.extend_from_slice(&44_u64.to_le_bytes());
        out.extend_from_slice(&45_u16.to_le_bytes());
        out.extend_from_slice(&45_u16.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&(count as u64).to_le_bytes());
        out.extend_from_slice(&(count as u64).to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());

        out.extend_from_slice(&0x0706_4b50_u32.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&record_at.to_le_bytes());
        out.extend_from_slice(&1_u32.to_le_bytes());
    }

    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    let counted = count.min(u16::MAX as usize) as u16;
    out.extend_from_slice(&counted.to_le_bytes());
    out.extend_from_slice(&counted.to_le_bytes());
    out.extend_from_slice(&(size.min(u32::MAX as u64) as u32).to_le_bytes());
    out.extend_from_slice(&(start.min(u32::MAX as u64) as u32).to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn somewhere(named: &str) -> PathBuf {
        let at = std::env::temp_dir().join(format!(
            "rustx-zip-{named}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&at);
        std::fs::create_dir_all(&at).expect("a directory");
        at
    }

    #[test]
    fn an_archive_begins_with_the_signature_and_ends_with_the_directory() {
        let from = somewhere("shape");
        std::fs::write(from.join("a.txt"), b"hello").expect("a file");
        let into = from.with_extension("zip");

        write(&from, &into).expect("an archive");
        let bytes = std::fs::read(&into).expect("the archive");

        assert_eq!(&bytes[..4], &[0x50, 0x4b, 0x03, 0x04], "no local header");
        assert!(
            bytes
                .windows(4)
                .any(|four| four == [0x50, 0x4b, 0x01, 0x02]),
            "no central directory"
        );
        assert_eq!(
            &bytes[bytes.len() - 22..bytes.len() - 18],
            &[0x50, 0x4b, 0x05, 0x06],
            "no end record"
        );
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_file(&into);
    }

    /// The folder's name is inside the archive, so unpacking makes one
    /// directory rather than scattering files where the person stood.
    #[test]
    fn every_name_is_under_the_folders_own_name() {
        let from = somewhere("rooted");
        std::fs::create_dir_all(from.join("locales")).expect("a directory");
        std::fs::write(from.join("mon-app.exe"), b"binary").expect("a file");
        std::fs::write(from.join("locales").join("fr.pak"), b"pak").expect("a file");
        let into = from.with_extension("zip");

        write(&from, &into).expect("an archive");
        let bytes = std::fs::read(&into).expect("the archive");
        let text = String::from_utf8_lossy(&bytes);
        let root = from.file_name().unwrap().to_string_lossy().to_string();

        assert!(text.contains(&format!("{root}/mon-app.exe")), "{root}");
        assert!(text.contains(&format!("{root}/locales/fr.pak")), "{root}");
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_file(&into);
    }

    /// Bytes that deflate cannot shrink are stored as they are, so an
    /// archive is never bigger than what it holds.
    #[test]
    fn incompressible_bytes_are_stored_rather_than_grown() {
        let from = somewhere("stored");
        // A run of distinct bytes, repeated once: deflate has nothing
        // to do with the first 256 and would add its own header.
        let awkward: Vec<u8> = (0..=255_u8).collect();
        std::fs::write(from.join("noise.bin"), &awkward).expect("a file");
        let into = from.with_extension("zip");

        write(&from, &into).expect("an archive");
        let bytes = std::fs::read(&into).expect("the archive");
        // Method is at offset 8 of the local header.
        let method = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(method, 0, "incompressible bytes were deflated anyway");
        let _ = std::fs::remove_dir_all(&from);
        let _ = std::fs::remove_file(&into);
    }

    #[test]
    fn the_crc_is_the_one_the_format_names() {
        // The published check value for CRC-32 of "123456789".
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn a_name_is_marked_as_utf8() {
        let header = local_header("café/à.txt", 0, 1, 1, 8);
        let flags = u16::from_le_bytes([header[6], header[7]]);
        assert_eq!(flags & UTF8, UTF8, "a non-ASCII name needs the flag");
    }
}
