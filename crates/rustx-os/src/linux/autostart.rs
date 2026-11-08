//! Autostart, on Linux: a `.desktop` file in `$XDG_CONFIG_HOME/autostart/`.
//!
//! This is the file every desktop environment that implements the
//! freedesktop.org autostart specification reads at login — GNOME,
//! KDE, XFCE, and the others. No D-Bus, no portal, no daemon to talk
//! to: writing the file *is* the whole mechanism, which is why this
//! one needed no further research to become real rather than a
//! refusal.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! Linux. `cargo check --target x86_64-unknown-linux-gnu` is what
//! verified it compiles; nothing here has been seen to actually start
//! an application at login.

use std::io::Write;
use std::path::PathBuf;

use crate::error::OsError;

pub const AVAILABLE: bool = true;

/// Where the file for `name` goes.
///
/// `$XDG_CONFIG_HOME`, or `~/.config` when it is unset — the
/// specification's own fallback, not a guess of this crate's.
fn path(name: &str) -> Result<PathBuf, OsError> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => {
            let home = std::env::var_os("HOME").ok_or_else(|| {
                OsError::File(
                    "neither $XDG_CONFIG_HOME nor $HOME is set — there is nowhere to write \
                     an autostart entry"
                        .to_string(),
                )
            })?;
            PathBuf::from(home).join(".config")
        }
    };
    Ok(base.join("autostart").join(format!("{name}.desktop")))
}

/// The file's contents, naming `exe` as what to run.
///
/// `X-GNOME-Autostart-enabled=true` is read by GNOME alone, but
/// writing it costs nothing and is read nowhere as `false` by the
/// others — it only ever helps.
fn entry(name: &str, exe: &std::path::Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={name}\n\
         Exec=\"{}\"\n\
         X-GNOME-Autostart-enabled=true\n",
        exe.display()
    )
}

pub fn set(on: bool, name: &str) -> Result<(), OsError> {
    let path = path(name)?;
    if !on {
        // Removing what is not there is not a failure: the box was
        // already unchecked.
        match std::fs::remove_file(&path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(OsError::File(format!(
                    "{} could not be removed: {error}",
                    path.display()
                )));
            }
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            OsError::File(format!("{} could not be created: {error}", parent.display()))
        })?;
    }
    let exe = std::env::current_exe().map_err(|error| {
        OsError::File(format!("this executable's own path could not be read: {error}"))
    })?;
    let mut file = std::fs::File::create(&path).map_err(|error| {
        OsError::File(format!("{} could not be written: {error}", path.display()))
    })?;
    file.write_all(entry(name, &exe).as_bytes()).map_err(|error| {
        OsError::File(format!("{} could not be written: {error}", path.display()))
    })?;
    Ok(())
}

pub fn is_on(name: &str) -> bool {
    path(name).is_ok_and(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_names_the_executable_in_quotes() {
        let written = entry("mon-app", std::path::Path::new("/usr/bin/mon-app"));
        assert!(written.contains("Exec=\"/usr/bin/mon-app\""), "got {written}");
        assert!(written.contains("Type=Application"), "got {written}");
        assert!(written.contains("Name=mon-app"), "got {written}");
    }
}
