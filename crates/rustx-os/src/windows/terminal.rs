//! PowerShell, and a window to type it in.
//!
//! `pwsh` first, `powershell` after it: PowerShell 7 is the one being
//! installed on new machines and the one whose behaviour matches the
//! other two platforms' shells most closely, but Windows PowerShell
//! 5.1 is on every Windows there is, so a machine without the first
//! still has a shell.
//!
//! The window is Windows Terminal (`wt.exe`) when it is there, which
//! it is by default on Windows 11, and the console host otherwise.

use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::error::OsError;
use crate::terminal::{Output, Shell, first_on_path};

pub fn shell() -> Result<Shell, OsError> {
    let (name, program) = first_on_path(&["pwsh", "powershell"]).ok_or_else(|| {
        OsError::Platform(
            "neither `pwsh` nor `powershell` is on PATH, so there is no shell to run".to_string(),
        )
    })?;
    Ok(Shell {
        name,
        program,
        // `-Command` rather than `-File`: what is being handed over is
        // a script written inline, not a path to one.
        run_flag: "-Command".to_string(),
    })
}

pub fn run(script: &str, at: Option<&Path>) -> Result<Output, OsError> {
    let shell = shell()?;
    let mut command = Command::new(&shell.program);
    command
        // No profile and no prompt: a profile can print a banner, ask a
        // question or take a second to load, and what is wanted here is
        // the script's own output and nothing else.
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg(&shell.run_flag)
        .arg(script)
        // And no window. A rustx application is a GUI process, which
        // has no console of its own, so Windows gives every console
        // program it starts a brand new one — a black rectangle that
        // appears in front of the user's window, takes focus and
        // disappears again. On a build button pressed once that is a
        // flicker; on a script run every few seconds it makes the
        // application unusable. `CREATE_NO_WINDOW` is what says the
        // child is being read by a program rather than watched by a
        // person. `open` deliberately does the opposite.
        .creation_flags(CREATE_NO_WINDOW);
    if let Some(directory) = at {
        command.current_dir(directory);
    }
    let finished = command
        .output()
        .map_err(|error| OsError::Platform(format!("{} could not be run: {error}", shell.name)))?;
    Ok(Output {
        code: finished.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&finished.stdout).into_owned(),
        err: String::from_utf8_lossy(&finished.stderr).into_owned(),
    })
}

pub fn open(at: &Path) -> Result<(), OsError> {
    let shell = shell()?;

    // Windows Terminal, which takes the directory as its own argument
    // and opens a tab in a window that may already be there.
    if let Some((_, terminal)) = first_on_path(&["wt"]) {
        let started = Command::new(&terminal)
            .arg("-d")
            .arg(at)
            .arg(&shell.program)
            .spawn();
        if started.is_ok() {
            return Ok(());
        }
    }

    // Otherwise the console host, which needs to be told to make a
    // window: a child of this process would share this one's console,
    // and this process has none to share.
    Command::new(&shell.program)
        .arg("-NoExit")
        .current_dir(at)
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            OsError::Platform(format!(
                "{} could not be opened in {}: {error}",
                shell.name,
                at.display()
            ))
        })
}

/// `CREATE_NEW_CONSOLE`, for the window a user is meant to see, and
/// `CREATE_NO_WINDOW`, for the one they are not. Named here rather than
/// reached for through the `windows` crate, because they are two
/// numbers and this is the only place in the crate that wants them.
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
