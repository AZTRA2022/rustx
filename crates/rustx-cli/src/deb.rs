//! A Debian package, written here.
//!
//! `dpkg-deb` is not on a Windows machine and not on a Fedora one, and
//! a framework that can only package on the distribution it was built
//! on is a framework that cannot package. So this writes the format
//! instead of calling a tool, which is possible because the format is
//! three files in an `ar` archive:
//!
//! | | |
//! |---|---|
//! | `debian-binary` | the four bytes `2.0\n` |
//! | `control.tar.gz` | what the package says it is |
//! | `data.tar.gz` | the files, at the paths they install to |
//!
//! `ar` is a 60-byte header per member and `tar` is a 512-byte header
//! per file, both of them fixed-width text fields from the 1970s. The
//! gzip is [`flate2`], which this crate already has for the `.zip`.
//!
//! What is deliberately not here: `md5sums`, which `dpkg` treats as
//! optional and which would mean carrying an MD5 implementation for a
//! file nothing checks; and a `postinst` script, because a desktop
//! application that runs code as root at install time is one a
//! distribution will not accept.

use std::io::Write;
use std::path::{Path, PathBuf};

/// One file, at the path it installs to.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Absolute on the target system: `/opt/my-app/my-app`.
    pub path: String,
    /// What to put there: a file to copy, or text to write.
    pub from: Source,
    /// The Unix mode. `0o755` for anything executable, `0o644` else.
    pub mode: u32,
}

#[derive(Debug, Clone)]
pub enum Source {
    File(PathBuf),
    Text(String),
    /// A symbolic link to this target.
    Link(String),
}

/// What the package says about itself -- which is what the software
/// centre, `apt show` and the package manager's own panels display.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    /// `Name <email>`, which is the one field `dpkg` is strict about.
    pub maintainer: String,
    /// One line, then indented lines for the rest.
    pub description: String,
    /// `amd64`, `arm64`. Debian's names, not Rust's.
    pub architecture: String,
    /// What has to be installed for this to start: WebKitGTK and GTK, and
    /// a package that does not say so installs and then does not open.
    pub depends: String,
    pub section: String,
    pub homepage: String,
    pub files: Vec<Entry>,
}

/// What a desktop Linux needs to draw the window: WebKitGTK, the engine of
/// the system webview. A package that declares none of it installs and then
/// dies on the first `dlopen`, which looks to a user like the application
/// is broken rather than incomplete.
pub const WEBVIEW_NEEDS: &str = "libc6, libgcc-s1, libstdc++6, libgtk-3-0, libwebkit2gtk-4.1-0";

/// Debian's name for the architecture this build runs on.
pub fn architecture() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "i386",
        "arm" => "armhf",
        "riscv64" => "riscv64",
        other => other,
    }
}

/// Write the package, and say where it landed.
pub fn write(package: &Package, into: &Path) -> Result<PathBuf, String> {
    let file = into.join(format!(
        "{}_{}_{}.deb",
        package.name, package.version, package.architecture
    ));

    let control = tarball(&[
        Entry {
            path: "/control".to_string(),
            from: Source::Text(control_file(package)),
            mode: 0o644,
        },
        Entry {
            path: "/conffiles".to_string(),
            from: Source::Text(String::new()),
            mode: 0o644,
        },
    ])?;
    let data = tarball(&package.files)?;

    let mut out = Vec::new();
    out.extend_from_slice(b"!<arch>\n");
    member(&mut out, "debian-binary", b"2.0\n");
    member(&mut out, "control.tar.gz", &control);
    member(&mut out, "data.tar.gz", &data);

    std::fs::write(&file, &out)
        .map_err(|error| format!("{} could not be written ({error}).", file.display()))?;
    Ok(file)
}

/// The `control` file: one `Key: value` per line, and a description
/// whose continuation lines begin with a space.
pub fn control_file(package: &Package) -> String {
    let mut lines = String::new();
    lines.push_str(&format!("Package: {}\n", package.name));
    lines.push_str(&format!("Version: {}\n", package.version));
    lines.push_str(&format!("Architecture: {}\n", package.architecture));
    lines.push_str(&format!("Maintainer: {}\n", package.maintainer));
    if !package.homepage.trim().is_empty() {
        lines.push_str(&format!("Homepage: {}\n", package.homepage.trim()));
    }
    lines.push_str(&format!("Section: {}\n", package.section));
    lines.push_str("Priority: optional\n");
    if !package.depends.trim().is_empty() {
        lines.push_str(&format!("Depends: {}\n", package.depends.trim()));
    }
    // `Installed-Size` is in kibibytes, and the software centre shows
    // it before the download starts.
    let size: u64 = package
        .files
        .iter()
        .map(|entry| match &entry.from {
            Source::File(path) => std::fs::metadata(path).map(|it| it.len()).unwrap_or(0),
            Source::Text(text) => text.len() as u64,
            Source::Link(_) => 0,
        })
        .sum();
    lines.push_str(&format!("Installed-Size: {}\n", size.div_ceil(1024)));

    // The first line is the summary; every line after it is indented by
    // one space, and a blank line is written as a single dot, which is
    // the format's own way of saying "a paragraph break".
    let mut described = package.description.lines();
    let summary = described.next().unwrap_or("").trim();
    lines.push_str(&format!(
        "Description: {}\n",
        if summary.is_empty() {
            package.name.as_str()
        } else {
            summary
        }
    ));
    for line in described {
        let line = line.trim_end();
        if line.trim().is_empty() {
            lines.push_str(" .\n");
        } else {
            lines.push_str(&format!(" {line}\n"));
        }
    }
    lines
}

/// One `ar` member: a 60-byte text header, the bytes, and a pad byte
/// when the length is odd.
fn member(out: &mut Vec<u8>, name: &str, bytes: &[u8]) {
    // Every field is left-aligned and space-padded, and the lengths are
    // the format's: 16 for the name, 12 for the date, 6 and 6 for the
    // owners, 8 for the mode, 10 for the size.
    out.extend_from_slice(format!("{name:<16}").as_bytes());
    // Zero, not now: a package built twice from the same files is the
    // same package, which is what lets a release be checked.
    out.extend_from_slice(format!("{:<12}", 0).as_bytes());
    out.extend_from_slice(format!("{:<6}", 0).as_bytes());
    out.extend_from_slice(format!("{:<6}", 0).as_bytes());
    out.extend_from_slice(format!("{:<8}", "100644").as_bytes());
    out.extend_from_slice(format!("{:<10}", bytes.len()).as_bytes());
    out.extend_from_slice(b"`\n");
    out.extend_from_slice(bytes);
    if bytes.len() % 2 == 1 {
        out.push(b'\n');
    }
}

/// A gzipped tar of these entries, with every directory above them
/// created first.
fn tarball(entries: &[Entry]) -> Result<Vec<u8>, String> {
    let mut tar = Vec::new();
    let mut made: Vec<String> = Vec::new();

    for entry in entries {
        let name = format!(".{}", entry.path);
        // Every parent, in order, because tar carries no tree and a
        // `dpkg` unpacking a file into a directory it has not been told
        // about leaves it owned by nobody.
        let mut walked = String::from(".");
        let pieces: Vec<&str> = entry.path.trim_matches('/').split('/').collect();
        for piece in &pieces[..pieces.len().saturating_sub(1)] {
            walked.push('/');
            walked.push_str(piece);
            let folder = format!("{walked}/");
            if !made.contains(&folder) {
                header(&mut tar, &folder, 0, 0o755, b'5', "")?;
                made.push(folder);
            }
        }

        match &entry.from {
            Source::Link(to) => header(&mut tar, &name, 0, 0o777, b'2', to)?,
            Source::Text(text) => {
                header(&mut tar, &name, text.len(), entry.mode, b'0', "")?;
                body(&mut tar, text.as_bytes());
            }
            Source::File(path) => {
                let bytes = std::fs::read(path)
                    .map_err(|error| format!("{} could not be read ({error}).", path.display()))?;
                header(&mut tar, &name, bytes.len(), entry.mode, b'0', "")?;
                body(&mut tar, &bytes);
            }
        }
    }

    // Two empty blocks end a tar.
    tar.extend_from_slice(&[0u8; 1024]);

    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(9));
    gzip.write_all(&tar)
        .map_err(|error| format!("the package could not be compressed ({error}）."))?;
    gzip.finish()
        .map_err(|error| format!("the package could not be compressed ({error})."))
}

/// A ustar header: 512 bytes of fixed-width fields, with a checksum
/// over the whole of it computed with the checksum field blank.
fn header(
    tar: &mut Vec<u8>,
    name: &str,
    size: usize,
    mode: u32,
    kind: u8,
    link: &str,
) -> Result<(), String> {
    let mut block = [0u8; 512];

    // `name` is 100 bytes, and `prefix` is 155 more that go in front of
    // it with a `/` between. Anything longer than that would need a GNU
    // extension, and a path that long is a bug rather than a case to
    // support.
    let (prefix, short) = split(name)?;
    put(&mut block[0..100], short.as_bytes());
    put(&mut block[100..108], format!("{mode:07o}").as_bytes());
    put(&mut block[108..116], b"0000000");
    put(&mut block[116..124], b"0000000");
    put(&mut block[124..136], format!("{size:011o}").as_bytes());
    // Epoch, for the same reason the `ar` date is zero.
    put(&mut block[136..148], format!("{:011o}", 0).as_bytes());
    block[148..156].fill(b' ');
    block[156] = kind;
    put(&mut block[157..257], link.as_bytes());
    put(&mut block[257..263], b"ustar\0");
    put(&mut block[263..265], b"00");
    put(&mut block[265..297], b"root");
    put(&mut block[297..329], b"root");
    put(&mut block[345..500], prefix.as_bytes());

    let sum: u32 = block.iter().map(|byte| u32::from(*byte)).sum();
    put(&mut block[148..155], format!("{sum:06o}").as_bytes());
    block[155] = b' ';

    tar.extend_from_slice(&block);
    Ok(())
}

/// The prefix and the name tar splits a long path into.
fn split(name: &str) -> Result<(String, String), String> {
    if name.len() <= 100 {
        return Ok((String::new(), name.to_string()));
    }
    // On a `/`, as late as possible, so that the name half fits.
    let cut = name[..name.len().min(156)]
        .rfind('/')
        .ok_or_else(|| format!("`{name}` is too long for a package and has no directory in it"))?;
    let (prefix, rest) = name.split_at(cut);
    let rest = rest.trim_start_matches('/');
    if prefix.len() > 155 || rest.len() > 100 {
        return Err(format!(
            "`{name}` is {} characters, and a package path is at most 255",
            name.len()
        ));
    }
    Ok((prefix.to_string(), rest.to_string()))
}

fn put(field: &mut [u8], bytes: &[u8]) {
    let taken = bytes.len().min(field.len());
    field[..taken].copy_from_slice(&bytes[..taken]);
}

/// A file's bytes, padded up to the next 512.
fn body(tar: &mut Vec<u8>, bytes: &[u8]) {
    tar.extend_from_slice(bytes);
    let over = bytes.len() % 512;
    if over != 0 {
        tar.extend(std::iter::repeat_n(0u8, 512 - over));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Package {
        Package {
            name: "my-app".to_string(),
            version: "1.2.3".to_string(),
            maintainer: "Someone <someone@example.test>".to_string(),
            description: "Counts things\nA longer line.\n\nAnd another.".to_string(),
            architecture: "amd64".to_string(),
            depends: "libc6, libgtk-3-0".to_string(),
            section: "utils".to_string(),
            homepage: String::new(),
            files: vec![
                Entry {
                    path: "/opt/my-app/my-app".to_string(),
                    from: Source::Text("#!/bin/sh\n".to_string()),
                    mode: 0o755,
                },
                Entry {
                    path: "/usr/bin/my-app".to_string(),
                    from: Source::Link("/opt/my-app/my-app".to_string()),
                    mode: 0o777,
                },
            ],
        }
    }

    #[test]
    fn the_archive_is_an_ar_with_the_three_members_in_order() {
        let into = std::env::temp_dir().join(format!("rustx-deb-{}", std::process::id()));
        std::fs::create_dir_all(&into).expect("a directory");
        let file = write(&sample(), &into).expect("a package");
        let bytes = std::fs::read(&file).expect("readable");
        assert!(bytes.starts_with(b"!<arch>\n"), "not an ar archive");

        // `dpkg` reads the members by position, not by name: the
        // version has to be first and the control before the data.
        let text = String::from_utf8_lossy(&bytes[..512]);
        let version = text.find("debian-binary").expect("no debian-binary");
        let control = text.find("control.tar.gz").expect("no control");
        let data = text.find("data.tar.gz").expect("no data");
        assert!(version < control && control < data, "{text}");
        assert!(
            file.file_name().unwrap().to_string_lossy() == "my-app_1.2.3_amd64.deb",
            "{}",
            file.display()
        );
        let _ = std::fs::remove_dir_all(&into);
    }

    #[test]
    fn the_control_file_indents_the_description_and_dots_the_blank_line() {
        let text = control_file(&sample());
        assert!(text.contains("Package: my-app\n"), "{text}");
        assert!(text.contains("Description: Counts things\n"), "{text}");
        assert!(text.contains("\n A longer line.\n"), "{text}");
        // A blank line inside a description is a single dot, or `dpkg`
        // reads the rest of the file as a new paragraph of fields.
        assert!(text.contains("\n .\n"), "{text}");
        assert!(text.contains("Installed-Size: 1\n"), "{text}");
    }

    /// The checksum is the one thing a reader checks before anything
    /// else, and it is computed over the header with its own field
    /// blanked -- so it is worth asserting rather than trusting.
    #[test]
    fn every_tar_header_carries_a_checksum_that_adds_up() {
        let tar = tarball(&sample().files).expect("a tarball");
        let mut gunzip = flate2::read::GzDecoder::new(&tar[..]);
        let mut plain = Vec::new();
        std::io::Read::read_to_end(&mut gunzip, &mut plain).expect("gzip");

        let mut at = 0;
        let mut seen = 0;
        while at + 512 <= plain.len() {
            let block = &plain[at..at + 512];
            if block.iter().all(|byte| *byte == 0) {
                break;
            }
            let said = std::str::from_utf8(&block[148..155])
                .expect("ascii")
                .trim_end_matches([' ', '\0']);
            let said = u32::from_str_radix(said.trim(), 8).expect("octal");
            let mut blanked = block.to_vec();
            blanked[148..156].fill(b' ');
            let sum: u32 = blanked.iter().map(|byte| u32::from(*byte)).sum();
            assert_eq!(said, sum, "block {seen} does not add up");
            assert_eq!(&block[257..262], b"ustar", "block {seen} is not ustar");

            let size = std::str::from_utf8(&block[124..136]).expect("ascii");
            let size = usize::from_str_radix(size.trim_end_matches(['\0', ' ']).trim(), 8)
                .expect("an octal size");
            at += 512 + size.div_ceil(512) * 512;
            seen += 1;
        }
        // Two directories (`/opt`, `/opt/my-app`), the binary, `/usr`,
        // `/usr/bin`, and the link.
        assert_eq!(seen, 6, "expected six entries, saw {seen}");
    }

    #[test]
    fn a_path_too_long_for_the_format_is_refused_rather_than_cut() {
        let long = format!("/opt/{}/x", "a".repeat(300));
        assert!(split(&long).is_err(), "a 300-character name was accepted");
        let fine = format!("/opt/my-app/{}", "b".repeat(80));
        assert!(split(&fine).is_ok());
    }
}
