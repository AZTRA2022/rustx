//! What a `build.rs` calls.
//!
//! One directory of `.rsx` in, one directory of `.rs` out, each with its
//! position map beside it. Nothing here knows about cargo: a build script
//! is one caller, and the command-line tool of a later plan will be
//! another.

use std::path::{Path, PathBuf};

use crate::error::CompileError;
use crate::transpile::transpile;

/// Translate every `.rsx` under `from` into a `.rs` under `to`, keeping
/// the directory structure, and write `<name>.rsx.map` beside each one.
///
/// Returns the generated files, in a stable order, so that a build script
/// can print `rerun-if-changed` for each.
pub fn compile_directory(from: &Path, to: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let mut sources = Vec::new();
    collect(from, &mut sources)?;
    sources.sort();

    let mut written = Vec::new();
    for source_path in sources {
        let relative = source_path
            .strip_prefix(from)
            .unwrap_or(&source_path)
            .to_path_buf();
        let target = to.join(relative).with_extension("rs");

        let source = read(&source_path)?;
        let out = transpile(&source).map_err(|error| error.in_file(&source_path))?;

        if let Some(parent) = target.parent() {
            create_dir(parent)?;
        }
        write(&target, &out.rust)?;
        write(&target.with_extension("rsx.map"), &out.map.to_text())?;
        written.push(target);
    }
    Ok(written)
}

/// The same, plus the route table — for an application whose screens
/// are its files.
///
/// Routing is asked for rather than assumed: under `compile_directory` a
/// `page.rsx` is just a file, and an application that mounts its
/// components by hand (the counter, the monitor) keeps doing so. Under
/// this one, every `page.rsx` is a screen and must declare `fn Page`,
/// which is what makes a missing declaration a sentence instead of a
/// screen that never appears.
///
/// The table is written beside the pages it names, so its `include!`
/// paths are the paths they were written as, and an application
/// includes one file rather than one per screen.
pub fn compile_app(from: &Path, to: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let mut written = compile_directory(from, to)?;

    let table = to.join("routes.rs");
    write(&table, &crate::routes::table_for(from)?)?;
    written.push(table);

    Ok(written)
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) -> Result<(), CompileError> {
    let entries = std::fs::read_dir(directory).map_err(|error| io(directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io(directory, error))?;
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into)?;
        } else if path.extension().is_some_and(|extension| extension == "rsx") {
            into.push(path);
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<String, CompileError> {
    std::fs::read_to_string(path).map_err(|error| io(path, error))
}

fn write(path: &Path, contents: &str) -> Result<(), CompileError> {
    std::fs::write(path, contents).map_err(|error| io(path, error))
}

fn create_dir(path: &Path) -> Result<(), CompileError> {
    std::fs::create_dir_all(path).map_err(|error| io(path, error))
}

fn io(path: &Path, error: std::io::Error) -> CompileError {
    CompileError {
        offset: 0,
        file: path.to_path_buf(),
        line: 1,
        column: 1,
        message: error.to_string(),
    }
}
