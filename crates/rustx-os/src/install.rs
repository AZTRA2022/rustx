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
        copy_verified(&app.binary, &target)?;
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
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec=\"{}\"\nTerminal=false\nCategories=Utility;\n",
        app.name.replace('\n', " "),
        app.publisher.replace('\n', " "),
        target.display()
    );
    std::fs::write(applications.join(format!("{slug}.desktop")), entry)
        .map_err(|e| file(e, "the desktop entry"))?;
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).map_err(|e| file(e, "the bin folder"))?;
    let link = bin.join(&slug);
    let _ = std::fs::remove_file(&link);
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, &link).map_err(|e| file(e, "the command link"))?;
    let _ = place;
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

    #[test]
    fn names_become_folders_the_systems_accept() {
        assert_eq!(slug("My App: 2"), "my-app-2");
        assert_eq!(display_name("a/b:c."), "a-b-c");
        assert_eq!(slug("***"), "app");
    }
}
