//! `zsh`, and Terminal.app to type it in.
//!
//! `zsh` is the login shell on every macOS since Catalina, and `bash`
//! is still there behind it for a machine old enough or configured
//! away from it — so the list is the same shape as the other two
//! platforms', longest-held preference first.
//!
//! Written on Windows and never compiled for a Mac, let alone run.
//! Nothing here is to be trusted until it has been: the `open -a
//! Terminal <dir>` below is the documented way to do this and the
//! `/bin/zsh -c` above it is ordinary POSIX, but "documented" and
//! "observed" are not the same word.

use std::path::Path;
use std::process::Command;

use crate::error::OsError;
use crate::terminal::{Output, Shell, first_on_path};

pub fn shell() -> Result<Shell, OsError> {
    // On PATH first, so a user who installed a newer `zsh` gets theirs.
    // `/bin/zsh` after it, because macOS ships one there whatever PATH
    // happens to say.
    let found = first_on_path(&["zsh", "bash"]).or_else(|| {
        [("zsh", "/bin/zsh"), ("bash", "/bin/bash")]
            .into_iter()
            .find(|(_, at)| Path::new(at).is_file())
            .map(|(name, at)| (name.to_string(), std::path::PathBuf::from(at)))
    });
    let (name, program) = found.ok_or_else(|| {
        OsError::Platform(
            "neither `zsh` nor `bash` was found, on PATH or in /bin, so there is no \
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
    // `open -a` hands the directory to Terminal.app, which opens a
    // window there in the user's own login shell -- which is the point:
    // their profile, their aliases, their prompt.
    Command::new("/usr/bin/open")
        .arg("-a")
        .arg("Terminal")
        .arg(at)
        .spawn()
        .map(super::notify::reaped)
        .map_err(|error| {
            OsError::Platform(format!(
                "Terminal could not be opened in {}: {error}",
                at.display()
            ))
        })
}
