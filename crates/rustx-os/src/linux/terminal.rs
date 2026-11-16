//! `bash`, and whichever terminal this desktop happens to have.
//!
//! There is no one terminal on Linux, so there is no one answer: the
//! list below is tried in order, starting with the two that are a
//! *policy* rather than a program — `x-terminal-emulator` is Debian's
//! alternatives link and `gio` asks the desktop itself — and falling
//! through the emulators that are actually installed somewhere.
//! `xterm` is last because it is the one that is always there and the
//! one nobody wants.
//!
//! Compile-checked from a Windows machine and never run on Linux.
//! Everything here is `std::process` and a list of program names, with
//! no X11 or D-Bus call of its own, which is the reason it is written
//! this way: it is the part of the problem that can be got right
//! without the machine in front of you.

use std::path::Path;
use std::process::Command;

use crate::error::OsError;
use crate::terminal::{Output, Shell, first_on_path};

pub fn shell() -> Result<Shell, OsError> {
    let found = first_on_path(&["bash", "zsh", "sh"]).or_else(|| {
        [("bash", "/bin/bash"), ("sh", "/bin/sh")]
            .into_iter()
            .find(|(_, at)| Path::new(at).is_file())
            .map(|(name, at)| (name.to_string(), std::path::PathBuf::from(at)))
    });
    let (name, program) = found.ok_or_else(|| {
        OsError::Platform(
            "no `bash`, `zsh` or `sh` was found, on PATH or in /bin, so there is no \
             shell to run"
                .to_string(),
        )
    })?;
    Ok(Shell {
        name,
        program,
        run_flag: "-c".to_string(),
    })
}

pub fn run(script: &str, at: Option<&Path>) -> Result<Output, OsError> {
    let shell = shell()?;
    let mut command = Command::new(&shell.program);
    command.arg(&shell.run_flag).arg(script);
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
    // Every one of these starts in the directory it is spawned in, so
    // the working directory is the whole of the configuration -- no
    // `--working-directory` flag, which each of them spells
    // differently and two of them do not have.
    for terminal in TERMINALS {
        let Some((_, program)) = first_on_path(&[terminal]) else {
            continue;
        };
        if Command::new(&program).current_dir(at).spawn().is_ok() {
            return Ok(());
        }
    }
    Err(OsError::Platform(format!(
        "no terminal was found to open in {} — none of {} is on PATH",
        at.display(),
        TERMINALS.join(", ")
    )))
}

/// The terminals to try, in the order to try them.
const TERMINALS: &[&str] = &[
    // The desktop's own answer, before any particular program.
    "x-terminal-emulator",
    // Then the ones that come with a desktop environment.
    "gnome-terminal",
    "konsole",
    "xfce4-terminal",
    "mate-terminal",
    "lxterminal",
    "tilix",
    "terminator",
    // Then the ones somebody chose to install.
    "alacritty",
    "kitty",
    "wezterm",
    "foot",
    "urxvt",
    // And the one that is always there.
    "xterm",
];
