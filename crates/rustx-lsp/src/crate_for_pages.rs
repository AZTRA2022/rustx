//! A crate whose source is the pages, for `rust-analyzer` to read.
//!
//! The pages of an application are generated Rust, and the application
//! includes them — but from inside the expansion of `#[rustx::main]`,
//! which means `rust-analyzer` has no file to attach them to: a hover on
//! a page came back empty, every time, however long one waited. Proved
//! by hand: the very same file, included from ordinary source, answers.
//!
//! So the server writes that ordinary source. A tiny crate in the
//! application's own `target/`, whose `lib.rs` includes the pages the
//! build already generated and whose dependencies are the
//! application's — so a page has the vocabulary it has when it is
//! compiled. `rust-analyzer` is told about it as a linked project, and
//! from then on a page is source like any other.
//!
//! Nothing here is ever compiled by the build: it is a crate that exists
//! to be read.

use std::path::{Path, PathBuf};

/// Make sure the application at `manifest` has its reading crate, and
/// say where its manifest is.
///
/// Rewritten every time: it costs two small files, and an application
/// that gained a dependency since the last time would otherwise have a
/// page that cannot see it.
pub fn written_for(manifest: &Path) -> Option<PathBuf> {
    let pages = rustx_compiler::build::generated_directory(manifest);
    if !pages.join(rustx_compiler::build::PAGES).is_file() {
        // The application has never been built, so there are no pages
        // to read. Mistakes are still underlined; completion starts at
        // the first build.
        return None;
    }

    // The crate is rooted at the pages themselves, and that is the
    // whole trick: `rust-analyzer` loads a crate's own directory, and
    // an `include!` pointing outside it is never read. A crate beside
    // the pages answered nothing, however long one waited; this one
    // answers in seconds.
    let theirs = std::fs::read_to_string(manifest.join("Cargo.toml")).ok()?;
    std::fs::write(pages.join("Cargo.toml"), manifest_for(&theirs, manifest)).ok()?;
    std::fs::write(pages.join(LIBRARY), library_for()).ok()?;

    Some(pages.join("Cargo.toml"))
}

/// The reading crate's source file, beside the pages it includes.
const LIBRARY: &str = "editor.rs";

/// The reading crate's manifest: the application's dependencies, and an
/// empty `[workspace]` so cargo reads it on its own rather than as a
/// member of the workspace it sits inside.
fn manifest_for(theirs: &str, manifest: &Path) -> String {
    let mut text = format!(
        "# Written by rustx-lsp so that an editor can read the pages of\n\
         # this application. Nobody builds it, nobody edits it.\n\
         [package]\n\
         name = \"rustx-editor\"\n\
         version = \"0.0.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\n\
         [workspace]\n\n\
         [lib]\n\
         name = \"rustx_editor\"\n\
         path = \"{LIBRARY}\"\n\n",
    );
    text.push_str(&dependencies_of(theirs, manifest));
    text
}

/// The application's `[dependencies]`, with every path made absolute.
///
/// Absolute because the reading crate sits under the application.s
/// `target/`, so `../../crates/rustx` would point at nothing from
/// there — and a dependency that does not resolve is a page with no
/// vocabulary at all.
fn dependencies_of(theirs: &str, manifest: &Path) -> String {
    let mut out = String::from("[dependencies]\n");
    let mut inside = false;
    for line in theirs.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[dependencies]";
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        out.push_str(&absolute_paths_in(line, manifest));
        out.push('\n');
    }
    // A page always has `rustx` in its vocabulary, even in an
    // application whose manifest does not say so in so many words.
    if !out.contains("rustx ") && !out.contains("rustx=") {
        out.push_str("rustx = { path = \"../../../../crates/rustx\" }\n");
    }
    out
}

fn absolute_paths_in(line: &str, manifest: &Path) -> String {
    let Some(at) = line.find("path = \"") else {
        return line.to_string();
    };
    let from = at + "path = \"".len();
    let Some(length) = line[from..].find('"') else {
        return line.to_string();
    };
    let written = &line[from..from + length];
    // `workspace = true` and a version are left as they are; a relative
    // path is the only thing that moves.
    let absolute = manifest.join(written);
    let absolute = std::fs::canonicalize(&absolute).unwrap_or(absolute);
    let absolute = absolute
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");
    format!("{}{}{}", &line[..from], absolute, &line[from + length..])
}

/// The reading crate's only source file.
///
/// The same vocabulary the attribute macro gives a page — `Element` and
/// `with_key` in scope, the lints the generated shape trips turned off —
/// and then the pages themselves.
fn library_for() -> String {
    format!(
        "// Written by rustx-lsp so that an editor can read the pages of
\r
         // this application. Nobody builds it, nobody edits it.
\r
         #![allow(non_snake_case, unused, clippy::all)]

\r
         pub use rustx::Element;
\r
         pub use rustx::state::with_key;

\r
         include!(\"{}\");
",
        rustx_compiler::build::PAGES
    )
}

/// Every application under `root` that has pages, each with its reading
/// crate written and its manifest named.
///
/// All of them at once, at the handshake, because `rust-analyzer` is
/// told which projects to load when it starts and a project named later
/// means a workspace reloaded under the developer's hands.
///
/// The walk is shallow and skips what cannot hold an application —
/// `target`, `node_modules`, anything hidden — so opening an editor on a
/// large repository does not become a directory crawl.
pub fn for_workspace(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(root, 0, &mut found);
    found
}

fn walk(directory: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    if directory.join("Cargo.toml").is_file()
        && directory.join("app").is_dir()
        && let Some(manifest) = written_for(directory)
    {
        found.push(manifest);
    }
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let skip = path
            .file_name()
            .map(|name| {
                let name = name.to_string_lossy();
                name.starts_with('.') || name == "target" || name == "node_modules"
            })
            .unwrap_or(true);
        if !skip && path.is_dir() {
            walk(&path, depth + 1, found);
        }
    }
}
