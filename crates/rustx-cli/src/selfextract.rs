//! The payload of a self-extracting installer.
//!
//! `rustx package` makes a Windows setup without any installer tool: a
//! small stub program (`rustx-setup`) with the application's folder
//! appended to its end. The stub reads itself, so the format is read
//! from the back:
//!
//! ```text
//! [stub .exe] [payload: deflate of entries] [meta text] [u64 payload len] [u32 meta len] [MAGIC]
//! entry = u16 name len, name (UTF-8, `/`), u64 size, bytes; a name length of 0 ends it
//! ```
//!
//! The meta is `key=value` lines: what the installer shows and registers.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

const MAGIC: &[u8; 8] = b"RXSETUP1";

/// What an installer needs to say and register about the application.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    pub name: String,
    pub version: String,
    pub publisher: String,
    /// The `.exe` inside the payload that the shortcut starts.
    pub binary: String,
}

fn one_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

impl Meta {
    fn to_text(&self) -> String {
        format!(
            "name={}\nversion={}\npublisher={}\nbinary={}\n",
            one_line(&self.name),
            one_line(&self.version),
            one_line(&self.publisher),
            one_line(&self.binary)
        )
    }

    fn from_text(text: &str) -> Meta {
        let mut meta = Meta::default();
        for line in text.lines() {
            match line.split_once('=') {
                Some(("name", value)) => meta.name = value.to_string(),
                Some(("version", value)) => meta.version = value.to_string(),
                Some(("publisher", value)) => meta.publisher = value.to_string(),
                Some(("binary", value)) => meta.binary = value.to_string(),
                _ => {}
            }
        }
        meta
    }
}

/// Where a payload sits in an installer file.
#[derive(Debug, Clone, Copy)]
pub struct Located {
    /// The stub's own length: everything before the payload.
    pub stub: u64,
    pub payload: u64,
}

fn walk(root: &Path, here: &Path, into: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(here)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(root, &path, into)?;
        } else if path.is_file() {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            into.push((name, path));
        }
    }
    Ok(())
}

/// `stub` + the folder + the meta, written to `out`.
pub fn pack(stub: &Path, folder: &Path, meta: &Meta, out: &Path) -> Result<(), String> {
    let failed = |what: &str, error: std::io::Error| format!("{what} ({error}).");
    let mut files = Vec::new();
    walk(folder, folder, &mut files)
        .map_err(|error| failed("the folder could not be read", error))?;
    files.sort();

    let mut file = std::fs::File::create(out)
        .map_err(|error| failed(&format!("{} could not be made", out.display()), error))?;
    let mut stub_bytes = std::fs::read(stub)
        .map_err(|error| failed(&format!("{} could not be read", stub.display()), error))?;
    // A stub that is itself an installer must not carry its first payload along.
    if let Some(found) = locate_bytes(&stub_bytes) {
        stub_bytes.truncate(found.stub as usize);
    }
    file.write_all(&stub_bytes)
        .map_err(|error| failed("the stub could not be written", error))?;

    struct Counting<'a>(&'a mut std::fs::File, u64);
    impl Write for Counting<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let n = self.0.write(bytes)?;
            self.1 += n as u64;
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }
    let mut counted = Counting(&mut file, 0);
    {
        let mut encoder = DeflateEncoder::new(&mut counted, Compression::default());
        for (name, path) in &files {
            let size = std::fs::metadata(path)
                .map_err(|error| failed("a file could not be measured", error))?
                .len();
            let name_bytes = name.as_bytes();
            let write = |encoder: &mut DeflateEncoder<&mut Counting>| -> std::io::Result<()> {
                encoder.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
                encoder.write_all(name_bytes)?;
                encoder.write_all(&size.to_le_bytes())?;
                let mut source = std::fs::File::open(path)?;
                std::io::copy(&mut source, encoder)?;
                Ok(())
            };
            write(&mut encoder)
                .map_err(|error| failed(&format!("{name} could not be added"), error))?;
        }
        encoder
            .write_all(&0_u16.to_le_bytes())
            .map_err(|error| failed("the end could not be written", error))?;
        encoder
            .finish()
            .map_err(|error| failed("the payload could not be finished", error))?;
    }
    let payload = counted.1;

    let text = meta.to_text();
    file.write_all(text.as_bytes())
        .and_then(|_| file.write_all(&payload.to_le_bytes()))
        .and_then(|_| file.write_all(&(text.len() as u32).to_le_bytes()))
        .and_then(|_| file.write_all(MAGIC))
        .map_err(|error| failed("the trailer could not be written", error))
}

fn locate_bytes(bytes: &[u8]) -> Option<Located> {
    let end = bytes.len();
    if end < 20 || &bytes[end - 8..] != MAGIC {
        return None;
    }
    let meta_len = u32::from_le_bytes(bytes[end - 12..end - 8].try_into().ok()?) as usize;
    let payload = u64::from_le_bytes(bytes[end - 20..end - 12].try_into().ok()?);
    let stub = (end as u64).checked_sub(20 + meta_len as u64 + payload)?;
    Some(Located { stub, payload })
}

/// Read the trailer of `exe`: the meta, and where the payload is.
pub fn locate(exe: &Path) -> Option<(Meta, Located)> {
    use std::io::{Seek, SeekFrom};
    let mut file = std::fs::File::open(exe).ok()?;
    let end = file.seek(SeekFrom::End(0)).ok()?;
    if end < 20 {
        return None;
    }
    let mut tail = [0_u8; 20];
    file.seek(SeekFrom::Start(end - 20)).ok()?;
    file.read_exact(&mut tail).ok()?;
    if &tail[12..] != MAGIC {
        return None;
    }
    let payload = u64::from_le_bytes(tail[..8].try_into().ok()?);
    let meta_len = u32::from_le_bytes(tail[8..12].try_into().ok()?) as u64;
    let stub = end.checked_sub(20 + meta_len + payload)?;
    let mut text = vec![0_u8; meta_len as usize];
    file.seek(SeekFrom::Start(end - 20 - meta_len)).ok()?;
    file.read_exact(&mut text).ok()?;
    Some((
        Meta::from_text(&String::from_utf8_lossy(&text)),
        Located { stub, payload },
    ))
}

/// A name from a payload is a path under the destination or nothing: no
/// `..`, no root, no drive.
fn safe(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    let clean = path
        .components()
        .all(|part| matches!(part, Component::Normal(_)));
    (clean && !name.contains(':') && !name.contains('\\')).then(|| path.to_path_buf())
}

/// Write the payload of `exe` under `into`. Returns the files written.
pub fn unpack(exe: &Path, located: Located, into: &Path) -> Result<usize, String> {
    use std::io::{Seek, SeekFrom};
    let failed = |what: String, error: std::io::Error| format!("{what} ({error}).");
    let mut file = std::fs::File::open(exe)
        .map_err(|error| failed(format!("{} could not be opened", exe.display()), error))?;
    file.seek(SeekFrom::Start(located.stub))
        .map_err(|error| failed("the payload could not be reached".into(), error))?;
    let mut stream = DeflateDecoder::new(file.take(located.payload));
    let mut count = 0;
    loop {
        let mut length = [0_u8; 2];
        stream
            .read_exact(&mut length)
            .map_err(|error| failed("the payload is cut short".into(), error))?;
        let length = u16::from_le_bytes(length) as usize;
        if length == 0 {
            return Ok(count);
        }
        let mut name = vec![0_u8; length];
        stream
            .read_exact(&mut name)
            .map_err(|error| failed("the payload is cut short".into(), error))?;
        let name = String::from_utf8(name)
            .map_err(|_| "a name in the payload is not text.".to_string())?;
        let relative = safe(&name).ok_or_else(|| {
            format!("{name} would leave the install folder; the installer is refused.")
        })?;
        let mut size = [0_u8; 8];
        stream
            .read_exact(&mut size)
            .map_err(|error| failed("the payload is cut short".into(), error))?;
        let destination = into.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                failed(format!("{} could not be made", parent.display()), error)
            })?;
        }
        let mut out = std::fs::File::create(&destination).map_err(|error| {
            failed(
                format!(
                    "{} could not be written (is the application running?)",
                    destination.display()
                ),
                error,
            )
        })?;
        let wanted = u64::from_le_bytes(size);
        let copied = std::io::copy(&mut (&mut stream).take(wanted), &mut out)
            .map_err(|error| failed(format!("{name} could not be extracted"), error))?;
        if copied != wanted {
            return Err(format!("{name} is cut short in the payload."));
        }
        count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("rustx-selfextract-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn a_folder_comes_back_out_of_an_installer_and_the_stub_survives_a_second_pack() {
        let root = temp("round");
        let folder = root.join("app");
        std::fs::create_dir_all(folder.join("assets")).unwrap();
        std::fs::write(folder.join("app.exe"), vec![7_u8; 100_000]).unwrap();
        std::fs::write(folder.join("assets/index.html"), "été").unwrap();
        std::fs::write(root.join("stub.exe"), b"STUB").unwrap();
        let meta = Meta {
            name: "My App".into(),
            version: "1.2.3".into(),
            publisher: "me".into(),
            binary: "app.exe".into(),
        };

        let setup = root.join("setup.exe");
        pack(&root.join("stub.exe"), &folder, &meta, &setup).unwrap();
        let (read, located) = locate(&setup).expect("a trailer");
        assert_eq!(read, meta);
        assert_eq!(located.stub, 4);

        let out = root.join("out");
        assert_eq!(unpack(&setup, located, &out).unwrap(), 2);
        assert_eq!(std::fs::read(out.join("app.exe")).unwrap().len(), 100_000);
        assert_eq!(
            std::fs::read_to_string(out.join("assets/index.html")).unwrap(),
            "été"
        );

        // The installer used as a stub: its payload is not carried along.
        let again = root.join("again.exe");
        pack(&setup, &folder, &meta, &again).unwrap();
        assert_eq!(locate(&again).unwrap().1.stub, 4);
        assert!(locate(&root.join("stub.exe")).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_name_that_leaves_the_install_folder_is_refused() {
        for bad in ["../x", "/x", "a/../../x", "C:/x", "a\\b"] {
            assert!(safe(bad).is_none(), "{bad}");
        }
        assert!(safe("assets/a b.txt").is_some());
    }
}
