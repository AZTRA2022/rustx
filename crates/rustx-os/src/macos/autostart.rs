//! Autostart, on macOS: a `LaunchAgent` plist, loaded by `launchctl`.
//!
//! `~/Library/LaunchAgents/<name>.plist` with `RunAtLoad` is the
//! documented, no-admin way to start something with the user's own
//! session — the same per-user, no-privilege shape as the Windows
//! `Run` key and the Linux `.desktop` file beside this one.
//! `launchctl load -w`/`unload -w` is the classic pair that reads and
//! forgets it; newer macOS prefers `bootstrap`/`bootout`, but `load`
//! and `unload` still work and are the pair every forum answer and
//! Apple's own older documentation gives first.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! macOS, and there is no Apple target here to even type-check it
//! against — only read over by hand, unlike the Linux files.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::OsError;

pub const AVAILABLE: bool = true;

fn path(name: &str) -> Result<PathBuf, OsError> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        OsError::File("$HOME is not set — there is nowhere to write a LaunchAgent".to_string())
    })?;
    Ok(PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{name}.plist")))
}

/// The plist naming `exe` as what `launchd` should run.
fn plist(name: &str, exe: &std::path::Path) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \x20   <key>Label</key>\n\
         \x20   <string>{name}</string>\n\
         \x20   <key>ProgramArguments</key>\n\
         \x20   <array>\n\
         \x20       <string>{}</string>\n\
         \x20   </array>\n\
         \x20   <key>RunAtLoad</key>\n\
         \x20   <true/>\n\
         </dict>\n\
         </plist>\n",
        exe.display()
    )
}

pub fn set(on: bool, name: &str) -> Result<(), OsError> {
    let path = path(name)?;
    if !on {
        // Told to unload before it is removed: a plist deleted while
        // still loaded leaves launchd holding a job whose file is
        // gone. Failing silently here is right either way — unloading
        // what is not loaded is not a failure, it is an already
        // unchecked box.
        let _ = Command::new("launchctl")
            .arg("unload")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(OsError::File(format!(
                "{} could not be removed: {error}",
                path.display()
            ))),
        };
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
    file.write_all(plist(name, &exe).as_bytes()).map_err(|error| {
        OsError::File(format!("{} could not be written: {error}", path.display()))
    })?;
    let loaded = Command::new("launchctl")
        .arg("load")
        .arg("-w")
        .arg(&path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| OsError::Platform(format!("`launchctl` could not be run: {error}")))?;
    if !loaded.success() {
        return Err(OsError::Platform(format!(
            "`launchctl load` refused {} (code {:?})",
            path.display(),
            loaded.code()
        )));
    }
    Ok(())
}

pub fn is_on(name: &str) -> bool {
    path(name).is_ok_and(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plist_names_the_executable_and_runs_at_load() {
        let written = plist("mon-app", std::path::Path::new("/usr/local/bin/mon-app"));
        assert!(written.contains("<string>/usr/local/bin/mon-app</string>"), "got {written}");
        assert!(written.contains("<string>mon-app</string>"), "got {written}");
        assert!(written.contains("<key>RunAtLoad</key>"), "got {written}");
        assert!(written.contains("<true/>"), "got {written}");
    }
}
