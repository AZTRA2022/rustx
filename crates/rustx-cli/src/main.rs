//! `rustx`.
//!
//! What a developer types. Each command is a function in the library
//! beside this file, so that what the command line does can be tested
//! without a command line.

use std::path::PathBuf;

use rustx_cli::{arguments, new};

fn main() {
    let typed: Vec<String> = std::env::args().skip(1).collect();
    let code = match arguments::read(&typed) {
        arguments::Command::New { name } => make(&name),
        arguments::Command::Help => {
            print!("{}", help());
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
        arguments::Command::Build { release } => make_it(release, false),
        arguments::Command::Run => make_it(false, true),
        // The watching loop, which is its own task of the plan.
        arguments::Command::Dev => {
            eprintln!("`rustx dev` is not wired up yet. `rustx run` builds and runs once.");
            2
        }
    };
    std::process::exit(code);
}

/// `rustx new mon-app`.
fn make(name: &str) -> i32 {
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
        println!("{}", exe.display());
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

fn help() -> String {
    "rustx — native software, written in Rust, drawn by a browser engine.\n\
     \n\
     \x20 rustx new <name>   write a project that compiles the first time\n\
     \x20 rustx dev          watch the files, rebuild, bring the window back\n\
     \x20 rustx build        build, and stamp the .exe with its name and icon\n\
     \x20 rustx run          build and run once, without watching\n\
     \n\
     What `dev` keeps across a restart: the shared state declared at the top\n\
     of a page, and the page that was open. What it does not keep: the cells\n\
     inside a component, whose identity is a place in code that just changed.\n"
        .to_string()
}
