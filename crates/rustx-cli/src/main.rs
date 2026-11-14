//! `rustx`.
//!
//! What a developer types. Each command is a function in the library
//! beside this file, so that what the command line does can be tested
//! without a command line.
//!
//! The exit codes, said once: `0` it worked, `1` the work failed, `2`
//! what was typed was not a thing to do. A script reads those, so they
//! are a contract — `2` for a mistake at the command line and `1` for a
//! build that did not compile, the way every tool a developer already
//! has does it.

use std::path::PathBuf;

use rustx_cli::{arguments, help, new};

fn main() {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    let code = match arguments::read(&typed) {
        arguments::Command::New { name } => make(&name),
        arguments::Command::Help => {
            print!("{}", help::text());
            0
        }
        arguments::Command::Version => {
            println!("rustx {}", env!("CARGO_PKG_VERSION"));
            0
        }
        arguments::Command::Unknown(word) => {
            eprintln!("`rustx {word}` is not a command.");
            if let Some(near) = arguments::nearest(&word) {
                eprintln!("Did you mean `rustx {near}`?");
            }
            eprintln!("`rustx --help` lists them all.");
            2
        }
        // A command that exists, given something it does not take. The
        // sentence came back with it, because the thing that read the
        // word is the thing that knows what was wrong with it.
        arguments::Command::Misused(said) => {
            eprintln!("{said}");
            2
        }
        arguments::Command::Build { release } => make_it(release, false),
        arguments::Command::Run { release } => make_it(release, true),
        arguments::Command::Dev => {
            let here = match here() {
                Ok(here) => here,
                Err(why) => {
                    eprintln!("{why}");
                    std::process::exit(1);
                }
            };
            match rustx_cli::dev::watch(&here) {
                Ok(()) => 0,
                Err(why) => {
                    if !why.is_empty() {
                        eprintln!("{why}");
                    }
                    1
                }
            }
        }
    };
    std::process::exit(code);
}

/// Where this was run.
///
/// Asked rather than assumed: `.` was what was used before, and a
/// directory that has been deleted under a running shell — or one this
/// process has no right to read — turned every path the command line
/// then built into a relative one, so the project was written somewhere
/// nobody could name and the refusal said nothing about why.
fn here() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| {
        format!(
            "rustx cannot tell which directory it was run in ({error}). The \
             directory may have been deleted or renamed while this terminal was \
             open; `cd` somewhere that exists and try again."
        )
    })
}

/// `rustx new mon-app`.
fn make(name: &str) -> i32 {
    let here = match here() {
        Ok(here) => here,
        Err(why) => {
            eprintln!("{why}");
            return 1;
        }
    };
    match new::write(name, &here) {
        Ok(written) => {
            println!("{name} is ready: {} files.", written.len());
            println!("  cd {name}");
            println!("  rustx dev");
            0
        }
        Err(why) => {
            eprintln!("{why}");
            1
        }
    }
}

/// `rustx build`, and `rustx run` which is the same plus a window.
fn make_it(release: bool, then_run: bool) -> i32 {
    let here = match here() {
        Ok(here) => here,
        Err(why) => {
            eprintln!("{why}");
            return 1;
        }
    };
    let exe = match rustx_cli::build::run(release, &here) {
        Ok(exe) => exe,
        Err(why) => {
            // Cargo has already said what is wrong, in the coordinates
            // of the `.rsx`; an empty complaint means "it said it".
            if !why.is_empty() {
                eprintln!("{why}");
            }
            return 1;
        }
    };
    if !then_run {
        // One path on stdout and nothing else, because a script reads
        // it. What else a release artefact needs goes to stderr, where
        // a person reads it and a pipe does not.
        println!("{}", exe.display());
        if release {
            eprint!("{}", rustx_cli::build::says_what_ships(&exe));
        }
        return 0;
    }
    match std::process::Command::new(&exe).current_dir(&here).status() {
        Ok(status) => status.code().unwrap_or(0),
        Err(error) => {
            eprintln!("{} could not be started: {error}", exe.display());
            1
        }
    }
}
