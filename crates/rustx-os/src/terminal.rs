//! The shell this machine speaks, and a window to speak it in.
//!
//! Three things an application wants from a terminal: to know which
//! shell it is talking to, to run something in it and read what came
//! back, and to open a real terminal window where the user can carry
//! on by hand. A build tool, a deploy button, a "open a shell here" in
//! a file manager — all three are the same three calls.
//!
//! The shell is the platform's own, because that is the one the user's
//! profile, PATH and aliases belong to: PowerShell on Windows, `zsh`
//! on macOS, `bash` on Linux. None of them is installed by this crate
//! and none is assumed to exist without being looked for — [`shell`]
//! reports what was actually found, and every call that needs one
//! refuses by name when there is none.

use std::path::{Path, PathBuf};

use crate::error::OsError;

/// Which shell this platform speaks, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    /// What to call it in a sentence: `"pwsh"`, `"zsh"`, `"bash"`.
    pub name: String,
    /// The program itself, found on this machine.
    pub program: PathBuf,
    /// What comes before a script on its command line — `-Command` for
    /// PowerShell, `-c` for the Unix shells.
    pub run_flag: String,
}

/// What a command left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// What the program returned. `0` is success everywhere.
    pub code: i32,
    /// Everything it wrote to its standard output, trailing newline and
    /// all — trimming is the caller's business, because a program that
    /// prints a path is a program whose last line matters.
    pub out: String,
    /// And to its standard error. A command can fail with an empty
    /// `err` and succeed with a full one, so this is not the test for
    /// whether it worked: `code` is.
    pub err: String,
}

impl Output {
    /// Did it work?
    pub fn worked(&self) -> bool {
        self.code == 0
    }
}

/// The shell this platform speaks, as found on this machine.
pub fn shell() -> Result<Shell, OsError> {
    crate::platform::terminal::shell()
}

/// Run a script in this platform's shell and wait for it.
///
/// `at` is the directory it runs in; `None` means wherever this process
/// already is. The script is handed to the shell as one argument, so it
/// is the shell's own language — a pipeline, a `&&`, a loop — and not a
/// program name with arguments beside it.
///
/// # Nothing is shown
///
/// No window, no console, nothing on the screen: this is a script being
/// read by a program, not watched by a person. That has to be asked for
/// on Windows — an application is a GUI process and has no console, so
/// Windows gives every console program it starts a new one, and the
/// user sees a black rectangle appear in front of their window, take
/// focus and vanish. `CREATE_NO_WINDOW` is what stops it. [`open`] is
/// the call that deliberately does the opposite.
///
/// # It waits
///
/// On the thread it was called on. A page's handler runs on the thread
/// that draws, so a script that takes two seconds freezes the window
/// for two seconds — which is what `task` exists for:
///
/// ```ignore
/// task(async move {
///     let built = rustx::terminal::run("cargo build --release", Some(&project))?;
///     status.set(if built.worked() { "built" } else { "failed" }.to_string());
/// });
/// ```
///
/// And a script run this way must be one that does not ask: there is
/// nobody at the other end to answer it, so a prompt is a script that
/// never returns. A script that needs a person needs [`open`].
pub fn run(script: &str, at: Option<&Path>) -> Result<Output, OsError> {
    if script.trim().is_empty() {
        return Err(OsError::Platform(
            "there is no script to run — `run` was given an empty string".to_string(),
        ));
    }
    if let Some(directory) = at {
        in_a_directory(directory)?;
    }
    crate::platform::terminal::run(script, at)
}

/// Open a terminal window, in this directory, and leave it to the user.
///
/// This returns as soon as the terminal has been started: it is a
/// window somebody else is now typing in, not a command this program
/// is waiting on.
pub fn open(at: &Path) -> Result<(), OsError> {
    in_a_directory(at)?;
    crate::platform::terminal::open(at)
}

/// A directory, and one that is there.
fn in_a_directory(path: &Path) -> Result<(), OsError> {
    if !path.exists() {
        return Err(OsError::File(format!("{} does not exist", path.display())));
    }
    if !path.is_dir() {
        return Err(OsError::File(format!(
            "{} is a file, and a terminal opens in a directory",
            path.display()
        )));
    }
    Ok(())
}

/// The first of these programs that is on this machine.
///
/// Read by every platform's own file: each has a list of the shells or
/// the terminals it would like, in the order it would like them, and
/// this is the part of the question that is the same everywhere.
#[cfg_attr(
    not(any(target_os = "windows", target_os = "linux", target_os = "macos")),
    allow(dead_code)
)]
pub(crate) fn first_on_path(wanted: &[&str]) -> Option<(String, PathBuf)> {
    let path = std::env::var_os("PATH")?;
    for name in wanted {
        for directory in std::env::split_paths(&path) {
            for candidate in with_extensions(directory.join(name)) {
                if candidate.is_file() {
                    return Some(((*name).to_string(), candidate));
                }
            }
        }
    }
    None
}

/// The names one program can have on this platform.
///
/// On Windows a program on `PATH` is `pwsh.exe`, not `pwsh`, and the
/// list of what counts comes from `PATHEXT` — which is why this is not
/// simply `.exe`: a shell installed as a `.cmd` shim is how several
/// package managers install one.
fn with_extensions(base: PathBuf) -> Vec<PathBuf> {
    if !cfg!(windows) {
        return vec![base];
    }
    let mut tried = vec![base.clone()];
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string());
    for extension in extensions.split(';').filter(|each| !each.is_empty()) {
        let mut named = base.clone().into_os_string();
        named.push(extension);
        tried.push(PathBuf::from(named));
    }
    tried
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_that_is_not_there_is_refused_by_name() {
        let said = open(Path::new("nowhere/at/all")).expect_err("no such directory");
        assert!(said.to_string().contains("all"), "got {said}");
    }

    #[test]
    fn a_file_is_not_a_directory_to_open_a_terminal_in() {
        let file = std::env::temp_dir().join(format!("rustx-terminal-{}.txt", std::process::id()));
        std::fs::write(&file, "x").expect("a file");
        let said = open(&file).expect_err("a file is not a directory");
        assert!(said.to_string().contains("directory"), "got {said}");
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn an_empty_script_is_refused_rather_than_run() {
        let said = run("   ", None).expect_err("nothing to run");
        assert!(said.to_string().contains("empty"), "got {said}");
    }

    #[test]
    fn the_shell_this_machine_speaks_is_a_program_that_is_there() {
        // Every machine this is built on has one of the three. If it
        // does not, the refusal names it rather than panicking, and
        // that is the case being asserted.
        match shell() {
            Ok(found) => {
                assert!(
                    found.program.is_file(),
                    "`{}` is not a program on this machine",
                    found.program.display()
                );
                assert!(!found.name.is_empty());
                assert!(!found.run_flag.is_empty());
            }
            Err(said) => {
                let said = said.to_string();
                assert!(
                    said.contains("shell") || said.contains("platform"),
                    "a refusal should say what is missing: {said}"
                );
            }
        }
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    #[test]
    fn a_script_runs_and_hands_back_what_it_printed() {
        let Ok(_) = shell() else {
            return; // Said by the test above; nothing to run it with.
        };
        let found = run("echo rustx-was-here", None).expect("a shell to run it");
        assert!(found.worked(), "the script failed: {found:?}");
        assert!(
            found.out.contains("rustx-was-here"),
            "nothing came back: {found:?}"
        );
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    #[test]
    fn a_script_that_fails_says_so_in_its_code_and_not_by_an_error() {
        let Ok(_) = shell() else { return };
        // A command nothing has. The shell itself ran, so this is not an
        // `Err` -- it is an `Output` whose code is not zero, which is
        // the distinction the whole type exists to keep.
        let found = run("rustx-no-such-command-anywhere", None).expect("the shell ran");
        assert!(!found.worked(), "that should not have worked: {found:?}");
    }

    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    #[test]
    fn a_script_runs_where_it_was_told_to() {
        let Ok(_) = shell() else { return };
        let directory = std::env::temp_dir();
        let found = run(WORKING_DIRECTORY, Some(&directory)).expect("a shell");
        assert!(found.worked(), "the script failed: {found:?}");
        let said = found.out.trim().to_lowercase();
        let wanted = directory.to_string_lossy().to_lowercase();
        let wanted = wanted.trim_end_matches(['/', '\\']);
        assert!(
            said.contains(wanted) || wanted.contains(said.trim_end_matches(['/', '\\'])),
            "ran in {said}, not {wanted}"
        );
    }

    #[cfg(target_os = "windows")]
    const WORKING_DIRECTORY: &str = "Write-Output (Get-Location).Path";
    #[cfg(not(target_os = "windows"))]
    const WORKING_DIRECTORY: &str = "pwd";
}
