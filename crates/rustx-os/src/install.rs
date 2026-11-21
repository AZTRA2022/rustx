//! Installing an application from itself: the binary the user was given copies
//! itself where the system keeps applications, registers itself, and can be
//! removed again. No administrator on any system.
//!
//! | | where | registered as |
//! |---|---|---|
//! | Windows | `%LOCALAPPDATA%\Programs\<name>` | a Start menu shortcut and an "Apps & features" entry whose uninstall command is this binary with `--uninstall` |
//! | macOS | `~/Applications/<name>.app` (the whole bundle) | the bundle itself |
//! | Linux | `~/.local/share/<slug>/` | a `.desktop` entry, and a link in `~/.local/bin` |
//!
//! Every path comes from the user's own folders (`LOCALAPPDATA`, `APPDATA`,
//! `HOME`), so a test redirects them and touches nothing real.

use std::path::{Path, PathBuf};

use crate::error::OsError;

/// What is installed: read from the application itself.
#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub version: String,
    pub publisher: String,
    /// The running binary. On macOS the enclosing `.app` is what is copied.
    pub binary: PathBuf,
    /// A PNG for the Linux desktop entry; other systems take the icon from the binary.
    pub icon: Option<PathBuf>,
}

fn file(error: impl std::fmt::Display, what: &str) -> OsError {
    OsError::File(format!("{what}: {error}"))
}

/// A folder-safe name.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for letter in name.chars() {
        if letter.is_ascii_alphanumeric() {
            out.push(letter.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "app".into()
    } else {
        trimmed.into()
    }
}

fn display_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) { '-' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.').to_string();
    if cleaned.is_empty() {
        "App".into()
    } else {
        cleaned
    }
}

fn var(name: &str) -> Result<PathBuf, OsError> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| OsError::Platform(format!("{name} is not set")))
}

/// The folder (Windows, Linux) or bundle (macOS) this application installs to.
pub fn destination(name: &str) -> Result<PathBuf, OsError> {
    if cfg!(windows) {
        Ok(var("LOCALAPPDATA")?
            .join("Programs")
            .join(display_name(name)))
    } else if cfg!(target_os = "macos") {
        Ok(var("HOME")?
            .join("Applications")
            .join(format!("{}.app", display_name(name))))
    } else {
        Ok(var("HOME")?.join(".local/share").join(slug(name)))
    }
}

/// The binary a launch of the installed copy starts.
pub fn installed_binary(app: &App) -> Result<PathBuf, OsError> {
    let place = destination(&app.name)?;
    let name = app
        .binary
        .file_name()
        .ok_or_else(|| OsError::File("the binary has no name".into()))?;
    Ok(if cfg!(target_os = "macos") {
        place.join("Contents/MacOS").join(name)
    } else {
        place.join(name)
    })
}

/// Is this running binary already inside the install location?
pub fn is_installed_here(app: &App) -> bool {
    destination(&app.name)
        .map(|place| {
            let (here, there) = (
                std::fs::canonicalize(&app.binary),
                std::fs::canonicalize(&place),
            );
            matches!((here, there), (Ok(here), Ok(there)) if here.starts_with(&there))
        })
        .unwrap_or(false)
}

/// The version recorded by the install, if one exists.
pub fn installed_version(name: &str) -> Option<String> {
    let place = destination(name).ok()?;
    std::fs::read_to_string(place.join(MARK))
        .ok()
        .map(|text| text.trim().to_string())
}

const MARK: &str = if cfg!(target_os = "macos") {
    "Contents/rustx-install.txt"
} else {
    "rustx-install.txt"
};

/// Copy a file, then check the copy by size and digest: a half-copied binary
/// registered with the system is worse than a failed install.
pub fn copy_verified(from: &Path, to: &Path) -> Result<(), OsError> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| file(e, &parent.display().to_string()))?;
    }
    std::fs::copy(from, to).map_err(|e| file(e, &format!("copying to {}", to.display())))?;
    let (a, b) = (
        std::fs::read(from).map_err(|e| file(e, "reading the source"))?,
        std::fs::read(to).map_err(|e| file(e, "reading the copy"))?,
    );
    if a.len() != b.len() || crate::digest::sha256(&a) != crate::digest::sha256(&b) {
        let _ = std::fs::remove_file(to);
        return Err(OsError::File(format!(
            "the copy at {} does not match its source",
            to.display()
        )));
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), OsError> {
    std::fs::create_dir_all(to).map_err(|e| file(e, &to.display().to_string()))?;
    for entry in std::fs::read_dir(from)
        .map_err(|e| file(e, &from.display().to_string()))?
        .flatten()
    {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            copy_verified(&entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Copy this application into place and register it. `progress` gets 0 to 100.
/// Returns the installed binary.
pub fn install(app: &App, progress: &mut dyn FnMut(u8)) -> Result<PathBuf, OsError> {
    let place = destination(&app.name)?;
    progress(5);
    let target = installed_binary(app)?;
    if cfg!(target_os = "macos") {
        let bundle = app
            .binary
            .ancestors()
            .find(|path| path.extension().is_some_and(|e| e == "app"))
            .ok_or_else(|| OsError::Platform("this binary is not inside an .app bundle".into()))?;
        let _ = std::fs::remove_dir_all(&place);
        copy_tree(bundle, &place)?;
    } else {
        // An update over a running copy: a running binary cannot be written to
        // (Windows locks it, Linux says "text file busy") but it can be renamed.
        let old = target.with_extension("old");
        if target.exists() {
            let _ = std::fs::remove_file(&old);
            if std::fs::rename(&target, &old).is_err() {
                return Err(OsError::File(format!(
                    "{} is running and cannot be replaced: close it, then try again",
                    target.display()
                )));
            }
        }
        if let Err(why) = copy_verified(&app.binary, &target) {
            let _ = std::fs::rename(&old, &target);
            return Err(why);
        }
        let _ = std::fs::remove_file(&old);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
        }
    }
    progress(70);
    std::fs::write(place.join(MARK), &app.version).map_err(|e| file(e, "the install record"))?;
    register(app, &place, &target)?;
    progress(100);
    Ok(target)
}

#[cfg(windows)]
fn register(app: &App, place: &Path, target: &Path) -> Result<(), OsError> {
    use std::os::windows::process::CommandExt;
    let hidden = |program: &str| {
        let mut command = std::process::Command::new(program);
        command.creation_flags(0x0800_0000);
        command
    };
    let link = var("APPDATA")?
        .join(r"Microsoft\Windows\Start Menu\Programs")
        .join(format!("{}.lnk", display_name(&app.name)));
    if let Some(parent) = link.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let script = "$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:RX_LINK); \
                  $s.TargetPath=$env:RX_TARGET; $s.WorkingDirectory=$env:RX_DIR; $s.Save()";
    let made = hidden("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            script,
        ])
        .env("RX_LINK", &link)
        .env("RX_TARGET", target)
        .env("RX_DIR", place)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !made {
        return Err(OsError::Platform(
            "the Start menu shortcut could not be made".into(),
        ));
    }
    let key = format!(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
        display_name(&app.name)
    );
    let uninstall = format!("\"{}\" --uninstall", target.display());
    let (location, icon) = (place.display().to_string(), target.display().to_string());
    for (name, value) in [
        ("DisplayName", app.name.as_str()),
        ("DisplayVersion", app.version.as_str()),
        ("Publisher", app.publisher.as_str()),
        ("InstallLocation", location.as_str()),
        ("DisplayIcon", icon.as_str()),
        ("UninstallString", uninstall.as_str()),
    ] {
        let _ = hidden("reg")
            .args(["add", &key, "/v", name, "/t", "REG_SZ", "/d", value, "/f"])
            .status();
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn register(_: &App, _: &Path, _: &Path) -> Result<(), OsError> {
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn register(app: &App, place: &Path, target: &Path) -> Result<(), OsError> {
    let home = var("HOME")?;
    let slug = slug(&app.name);
    let applications = home.join(".local/share/applications");
    std::fs::create_dir_all(&applications).map_err(|e| file(e, "the applications folder"))?;
    // The icon is copied beside the binary so the entry never names a temp folder.
    let icon = app
        .icon
        .as_ref()
        .filter(|from| from.is_file())
        .and_then(|from| {
            let to = place.join("icon.png");
            std::fs::copy(from, &to).ok().map(|_| to)
        });
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec=\"{}\"\n{}Terminal=false\nCategories=Utility;\n",
        app.name.replace('\n', " "),
        app.publisher.replace('\n', " "),
        target.display(),
        icon.map(|path| format!("Icon={}\n", path.display()))
            .unwrap_or_default()
    );
    std::fs::write(applications.join(format!("{slug}.desktop")), entry)
        .map_err(|e| file(e, "the desktop entry"))?;
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).map_err(|e| file(e, "the bin folder"))?;
    let link = bin.join(&slug);
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, &link).map_err(|e| file(e, "the command link"))?;
    Ok(())
}

/// Unregister and remove. Where the running binary lives in the folder it
/// removes (Windows), the folder goes when this process has ended.
pub fn uninstall(name: &str) -> Result<(), OsError> {
    let place = destination(name)?;
    unregister(name);
    if cfg!(windows) {
        delete_when_this_ends(&place)
    } else {
        let removed = if place.is_dir() {
            std::fs::remove_dir_all(&place)
        } else {
            Ok(())
        };
        removed.map_err(|e| file(e, &place.display().to_string()))
    }
}

fn unregister(name: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(appdata) = var("APPDATA") {
            let _ = std::fs::remove_file(
                appdata
                    .join(r"Microsoft\Windows\Start Menu\Programs")
                    .join(format!("{}.lnk", display_name(name))),
            );
        }
        let key = format!(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{}",
            display_name(name)
        );
        let _ = std::process::Command::new("reg")
            .creation_flags(0x0800_0000)
            .args(["delete", &key, "/f"])
            .status();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    if let Ok(home) = var("HOME") {
        let slug = slug(name);
        let _ = std::fs::remove_file(
            home.join(".local/share/applications")
                .join(format!("{slug}.desktop")),
        );
        let _ = std::fs::remove_file(home.join(".local/bin").join(&slug));
    }
    let _ = name;
}

/// A second process waits for this one to end (up to a minute) and deletes
/// the folder.
#[cfg(windows)]
fn delete_when_this_ends(place: &Path) -> Result<(), OsError> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("cmd")
        .creation_flags(0x0800_0000)
        .raw_arg(
            r#"/C for /L %i in (1,1,60) do @(ping -n 2 127.0.0.1 >nul & rmdir /s /q "%RX_DIR%" 2>nul & if not exist "%RX_DIR%" exit)"#,
        )
        .env("RX_DIR", place)
        .spawn()
        .map(|_| ())
        .map_err(|e| file(e, "the delayed delete"))
}

#[cfg(not(windows))]
fn delete_when_this_ends(_: &Path) -> Result<(), OsError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_that_differs_from_its_source_is_refused_and_removed() {
        let root = std::env::temp_dir().join(format!("rustx-install-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (from, to) = (root.join("a.bin"), root.join("out/b.bin"));
        std::fs::write(&from, vec![7_u8; 4096]).unwrap();
        copy_verified(&from, &to).unwrap();
        assert_eq!(std::fs::read(&to).unwrap().len(), 4096);
        // A source that vanishes between the copy and the check is an error, not a registration.
        assert!(copy_verified(&root.join("missing"), &root.join("c.bin")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// What signtool does to a stamped exe: pad to 8 bytes, point the security
    /// directory at the end, append a certificate table.
    #[cfg(windows)]
    #[test]
    fn the_stamp_survives_a_certificate_table() {
        use std::io::{Seek, SeekFrom, Write};
        let root = std::env::temp_dir().join(format!("rustx-sign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let exe = root.join("signed.exe");
        std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
        let stamp = Stamp {
            name: "Demo".into(),
            version: "1.2.3".into(),
            publisher: "Me".into(),
            license: "Line one\nC:\\dir\\n".into(),
        };
        write_stamp(&exe, &stamp).unwrap();
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&exe)
            .unwrap();
        let len = f.seek(SeekFrom::End(0)).unwrap();
        let start = len.div_ceil(8) * 8;
        f.write_all(&vec![0; (start - len) as usize]).unwrap();
        f.write_all(&[7; 64]).unwrap();
        let mut head = [0_u8; 64];
        f.seek(SeekFrom::Start(0)).unwrap();
        std::io::Read::read_exact(&mut f, &mut head).unwrap();
        let pe = u32::from_le_bytes(head[60..64].try_into().unwrap()) as u64;
        let mut magic = [0_u8; 2];
        f.seek(SeekFrom::Start(pe + 24)).unwrap();
        std::io::Read::read_exact(&mut f, &mut magic).unwrap();
        let directories = if u16::from_le_bytes(magic) == 0x20b {
            112
        } else {
            96
        };
        f.seek(SeekFrom::Start(pe + 24 + directories + 32)).unwrap();
        f.write_all(&(start as u32).to_le_bytes()).unwrap();
        f.write_all(&64_u32.to_le_bytes()).unwrap();
        drop(f);
        assert_eq!(read_stamp_of(&exe), Some(stamp));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn names_become_folders_the_systems_accept() {
        assert_eq!(slug("My App: 2"), "my-app-2");
        assert_eq!(display_name("a/b:c."), "a-b-c");
        assert_eq!(slug("***"), "app");
    }
}

// ---------------------------------------------------------------- the stamp

/// What `rustx package` writes into a binary to say it may install itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    pub name: String,
    pub version: String,
    pub publisher: String,
    /// The licence text to accept, empty when the project has none.
    pub license: String,
}

const TRAILER: &[u8; 10] = b"RXINSTALL1";

/// Append the stamp to a binary (Windows, Linux). Before signing, never after.
pub fn write_stamp(binary: &Path, stamp: &Stamp) -> Result<(), OsError> {
    use std::io::Write;
    let line = |text: &str| text.replace(['\n', '\r'], " ");
    let body = format!(
        "name={}\nversion={}\npublisher={}\nlicense={}\n",
        line(&stamp.name),
        line(&stamp.version),
        line(&stamp.publisher),
        // One line: backslash and newline are escaped.
        stamp
            .license
            .replace('\\', "\\\\")
            .replace('\r', "")
            .replace('\n', "\\n")
    );
    let mut out = std::fs::OpenOptions::new()
        .append(true)
        .open(binary)
        .map_err(|e| file(e, &binary.display().to_string()))?;
    out.write_all(body.as_bytes())
        .and_then(|_| out.write_all(&(body.len() as u32).to_le_bytes()))
        .and_then(|_| out.write_all(TRAILER))
        .map_err(|e| file(e, "the install stamp"))
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn stamp_from(text: &str) -> Stamp {
    let mut stamp = Stamp {
        name: String::new(),
        version: String::new(),
        publisher: String::new(),
        license: String::new(),
    };
    for line in text.lines() {
        match line.split_once('=') {
            Some(("name", v)) => stamp.name = v.into(),
            Some(("version", v)) => stamp.version = v.into(),
            Some(("publisher", v)) => stamp.publisher = v.into(),
            Some(("license", v)) => stamp.license = unescape(v),
            _ => {}
        }
    }
    stamp
}

/// The stamp of this running binary, if `rustx package` made it installable.
pub fn read_stamp() -> Option<Stamp> {
    let binary = std::env::current_exe().ok()?;
    read_stamp_of(&binary)
}

/// Where the Authenticode certificate table of a signed PE file starts.
fn certificate_start(file: &mut std::fs::File) -> Option<u64> {
    use std::io::{Read, Seek, SeekFrom};
    let mut head = [0_u8; 64];
    file.seek(SeekFrom::Start(0)).ok()?;
    file.read_exact(&mut head).ok()?;
    if &head[..2] != b"MZ" {
        return None;
    }
    let pe = u32::from_le_bytes(head[60..64].try_into().ok()?) as u64;
    let mut optional = [0_u8; 2];
    file.seek(SeekFrom::Start(pe + 24)).ok()?;
    file.read_exact(&mut optional).ok()?;
    // Data directories begin at 96 (PE32) or 112 (PE32+) into the optional header; entry 4 is security.
    let directories = match u16::from_le_bytes(optional) {
        0x10b => 96,
        0x20b => 112,
        _ => return None,
    };
    let mut entry = [0_u8; 8];
    file.seek(SeekFrom::Start(pe + 24 + directories + 4 * 8))
        .ok()?;
    file.read_exact(&mut entry).ok()?;
    let offset = u32::from_le_bytes(entry[..4].try_into().ok()?) as u64;
    (offset != 0).then_some(offset)
}

/// The stamp of a given binary.
pub fn read_stamp_of(binary: &Path) -> Option<Stamp> {
    use std::io::{Read, Seek, SeekFrom};
    if cfg!(target_os = "macos") {
        // A signed Mach-O cannot be appended to: the bundle's Info.plist says it.
        let plist = binary.parent()?.parent()?.join("Info.plist");
        let text = std::fs::read_to_string(plist).ok()?;
        if !text.contains("<key>RustxInstallable</key>") {
            return None;
        }
        let value = |key: &str| {
            let at = text.find(&format!("<key>{key}</key>"))?;
            let rest = &text[at..];
            let open = rest.find("<string>")? + "<string>".len();
            let close = rest.find("</string>")?;
            Some(rest[open..close].to_string())
        };
        return Some(Stamp {
            name: value("CFBundleName")?,
            version: value("CFBundleShortVersionString").unwrap_or_default(),
            publisher: value("RustxPublisher").unwrap_or_default(),
            // The licence of a bundle is an asset beside the binary.
            license: String::new(),
        });
    }
    let mut file = std::fs::File::open(binary).ok()?;
    let mut end = file.seek(SeekFrom::End(0)).ok()?;
    // Signing appends a certificate table after the stamp: read up to it.
    if let Some(cert) = certificate_start(&mut file) {
        end = cert;
        // The table starts on an 8-byte boundary, so up to 7 bytes of padding follow the stamp.
        let mut back = 0;
        while back < 8 && end > 14 {
            let mut tail = [0_u8; 10];
            file.seek(SeekFrom::Start(end - 10)).ok()?;
            file.read_exact(&mut tail).ok()?;
            if &tail == TRAILER {
                break;
            }
            end -= 1;
            back += 1;
        }
    }
    if end < 14 {
        return None;
    }
    let mut tail = [0_u8; 14];
    file.seek(SeekFrom::Start(end - 14)).ok()?;
    file.read_exact(&mut tail).ok()?;
    if &tail[4..] != TRAILER {
        return None;
    }
    let length = u32::from_le_bytes(tail[..4].try_into().ok()?) as u64;
    let mut body = vec![0_u8; length as usize];
    file.seek(SeekFrom::Start(end.checked_sub(14 + length)?))
        .ok()?;
    file.read_exact(&mut body).ok()?;
    Some(stamp_from(&String::from_utf8_lossy(&body)))
}

#[cfg(test)]
mod stamp_tests {
    use super::*;

    #[test]
    fn a_stamp_is_read_back_from_the_end_of_a_binary_and_a_plain_one_has_none() {
        if cfg!(target_os = "macos") {
            return;
        }
        let path = std::env::temp_dir().join(format!("rustx-stamp-{}", std::process::id()));
        std::fs::write(&path, vec![1_u8; 500]).unwrap();
        assert_eq!(read_stamp_of(&path), None);
        let stamp = Stamp {
            name: "My App".into(),
            version: "1.0.0".into(),
            publisher: "me".into(),
            license: "Be kind.\nLine \\two".into(),
        };
        write_stamp(&path, &stamp).unwrap();
        assert_eq!(read_stamp_of(&path), Some(stamp));
        let _ = std::fs::remove_file(path);
    }
}
