//! `rustx build`.
//!
//! It does not replace cargo; it calls it, then finishes the job cargo
//! does not do — the `.exe` gets its logo, its name and its version. A
//! developer who prefers `cargo build` gets a binary that works, just
//! one that looks anonymous in an explorer window.
//!
//! What it stamps is read where the developer already wrote it: the
//! version in the manifest, the name and the logo in the `Config` of
//! `main.rs`. A second file to declare them in would be a second file to
//! forget to change.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::resources::{self, About};

/// A project, as much of it as a build needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// What cargo will call the file it produces, without `.exe`.
    pub binary: String,
    pub about: About,
    /// Absolute, so that a stamp does not depend on where it was run.
    pub icon: Option<PathBuf>,
    /// What `.permissions(&[..])` names in the source: they become the macOS usage descriptions.
    pub permissions: Vec<String>,
}

/// Read what the project says about itself.
pub fn read(project: &Path) -> Result<Project, String> {
    let manifest = project.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "there is no Cargo.toml in {} ({error}). `rustx build` is run inside a \
             project; `rustx new` makes one.",
            project.display()
        )
    })?;

    let package = package_of(&text);
    let binary = field(&package, "name").ok_or_else(|| {
        format!(
            "{} has no `name` in its `[package]`, so there is nothing to build.",
            manifest.display()
        )
    })?;
    let version = string(&package, "version", project).unwrap_or_else(|| "0.0.0".to_string());
    let publisher = first_author(&package)
        .or_else(|| {
            inherits(&package, "authors")
                .then(|| workspace_package(project).as_deref().and_then(first_author))
                .flatten()
        })
        .unwrap_or_default();

    let source = sources(project);
    let name = between(&source, "Config::new(\"", '"').unwrap_or_else(|| binary.clone());
    let description = string(&package, "description", project).unwrap_or_default();
    let icon = between(&source, ".icon(\"", '"')
        .map(|named| {
            let named = PathBuf::from(named.replace('\\', "/"));
            if named.is_absolute() {
                named
            } else {
                project.join(named)
            }
        })
        // The same courtesy the window pays: a logo in the obvious place
        // is a logo the framework finds, so the file and the window do
        // not end up wearing different ones.
        .or_else(|| {
            let obvious = project.join("assets").join("icon.ico");
            obvious.is_file().then_some(obvious)
        });

    Ok(Project {
        binary,
        about: About {
            name,
            version,
            description,
            publisher,
        },
        icon,
        permissions: permissions_in(&source),
    })
}

/// The names inside `.permissions(&["camera", ...])`, wherever the source says it.
fn permissions_in(source: &str) -> Vec<String> {
    let Some(at) = source.find(".permissions(&[") else {
        return Vec::new();
    };
    let rest = &source[at + ".permissions(&[".len()..];
    let list = rest.split(']').next().unwrap_or_default();
    list.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The cargo to run.
///
/// `CARGO` first, which is the one thing in the environment that names a
/// cargo for certain: cargo sets it for everything it starts, so a
/// `rustx build` run from a cargo subcommand, a build script or a test
/// uses the toolchain that is already in play rather than whichever
/// `cargo` the `PATH` of that process happens to reach first. Otherwise
/// the name, looked up on the `PATH` the way a developer typing it would
/// have it looked up.
///
/// This is the command line's one hidden dependency on the developer's
/// machine, and it is the one dependency it may legitimately have — the
/// framework is a Rust framework. What it owes in exchange is a refusal
/// that says so, which [`says_no_cargo`] is.
fn cargo_command() -> Command {
    match std::env::var_os("CARGO").filter(|named| !named.is_empty()) {
        Some(named) => Command::new(named),
        None => Command::new("cargo"),
    }
}

/// What to say when cargo is not there.
///
/// `cargo could not be started: program not found` named neither what
/// was looked for nor where, and a developer whose `PATH` is missing
/// `~/.cargo/bin` — a fresh terminal after an install, a CI image, a
/// GUI-launched editor on macOS — read it as a bug in `rustx`.
pub fn says_no_cargo(error: &std::io::Error) -> String {
    let looked_for = std::env::var_os("CARGO")
        .filter(|named| !named.is_empty())
        .map(|named| named.to_string_lossy().to_string())
        .unwrap_or_else(|| "cargo, on the PATH".to_string());
    format!(
        "`{looked_for}` could not be started: {error}.\n\
         `rustx build` does not replace cargo, it calls it: a Rust toolchain has to \
         be on this machine and on this terminal's PATH. `cargo --version` says \
         whether it is; https://rustup.rs installs one. If cargo is installed and \
         only this terminal cannot see it, the PATH of a terminal opened before the \
         install is the usual reason — open a new one."
    )
}

/// Build it, then stamp what cargo produced.
pub fn run(release: bool, project: &Path) -> Result<PathBuf, String> {
    let read = read(project)?;

    let mut cargo = cargo_command();
    cargo.arg("build").current_dir(project);
    if release {
        cargo.arg("--release");
    }
    let status = cargo.status().map_err(|error| says_no_cargo(&error))?;
    if !status.success() {
        // Cargo already said what is wrong, in the coordinates of the
        // `.rsx` because that is what the compiler keeps. Saying it
        // again, worse, would only push it off the screen.
        return Err(String::new());
    }

    let Some(exe) = produced(&read.binary, release, project) else {
        return Err(format!(
            "cargo said it was done and there is no `{}{}` in {}. Does the project \
             have a `[[bin]]` of another name?",
            read.binary,
            std::env::consts::EXE_SUFFIX,
            target_directory(project)
                .join(if release { "release" } else { "debug" })
                .display()
        ));
    };

    if let Some(icon) = read.icon.as_deref()
        && !icon.is_file()
    {
        // Not fatal: the binary is built and it runs. Said once,
        // because a declared logo that is not there is a typo.
        eprintln!(
            "rustx: the logo was declared as {} and there is no such file, so the \
                 .exe keeps the system's icon.",
            icon.display()
        );
        resources::stamp(&exe, &read.about, None)?;
        return Ok(exe);
    }
    resources::stamp(&exe, &read.about, read.icon.as_deref())?;
    Ok(exe)
}

/// Where cargo builds this project, in the same four steps cargo takes.
///
/// One function, because the command line has four reasons to ask —
/// where the binary is, which runtime files a window is holding, where
/// the state that crosses a restart lives, and where the copies
/// `rustx dev` launches go — and four answers that disagree is the
/// defect this replaced. `rustx build` looked in `target/` beside the
/// manifest while cargo had been told to build elsewhere, and said
/// `cargo said it was done and there is no mon-app.exe under any
/// target/` about a file cargo had just written.
///
/// The order is cargo's own:
///
/// 1. `CARGO_TARGET_DIR` — how one build directory is shared between
///    projects, and how every CI cache is wired.
/// 2. `CARGO_BUILD_TARGET_DIR`, the same thing said as a configuration
///    key.
/// 3. `[build] target-dir` in the nearest `.cargo/config.toml` at or
///    above the project, because cargo walks up from where it was run.
/// 4. `target/` at the root of the workspace, which for a project on
///    its own is the project itself.
pub fn target_directory(project: &Path) -> PathBuf {
    target_directory_told(project, target_from_environment().as_deref())
}

/// What the environment says about where cargo builds, if anything.
///
/// Read here and handed to [`target_directory_told`] rather than read
/// inside it, so that a test can say what the environment says without
/// setting a variable: tests of one crate share one process, and a
/// variable set in one is set in all of them — which is why writing one
/// is `unsafe` since the 2024 edition.
pub fn target_from_environment() -> Option<String> {
    for name in ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"] {
        if let Some(said) = std::env::var_os(name) {
            let said = said.to_string_lossy().to_string();
            // An exported-but-empty variable is what a shell script
            // leaves behind, and cargo ignores it. Taking it at its word
            // would put the binary at the root of the filesystem.
            if !said.trim().is_empty() {
                return Some(said);
            }
        }
    }
    None
}

/// [`target_directory`], with the environment said out loud.
pub fn target_directory_told(project: &Path, environment: Option<&str>) -> PathBuf {
    if let Some(named) = environment.filter(|said| !said.trim().is_empty()) {
        // Cargo resolves a relative one against the directory it was run
        // in, and this command line runs it in the project.
        return beside(project, named);
    }
    for directory in project.ancestors() {
        for name in [".cargo/config.toml", ".cargo/config"] {
            let Ok(text) = std::fs::read_to_string(directory.join(name)) else {
                continue;
            };
            if let Some(named) = configured(&text) {
                // A relative `target-dir` is relative to the directory
                // that holds the `.cargo` it was written in.
                return beside(directory, &named);
            }
        }
    }
    workspace_root(project).join("target")
}

/// `named`, made absolute against `directory` if it is not already.
fn beside(directory: &Path, named: &str) -> PathBuf {
    let named = PathBuf::from(named.trim().replace('\\', "/"));
    if named.is_absolute() {
        named
    } else {
        directory.join(named)
    }
}

/// The directory whose manifest declares the workspace this project is
/// in, or the project itself when it is in none.
///
/// Cargo builds every member of a workspace into one `target/` at the
/// workspace's root. A member looked at where it would have built on
/// its own is a member whose binary is never found.
fn workspace_root(project: &Path) -> PathBuf {
    for directory in project.ancestors() {
        let Ok(text) = std::fs::read_to_string(directory.join("Cargo.toml")) else {
            continue;
        };
        if text.lines().any(|line| {
            let line = line.trim();
            line == "[workspace]" || line.starts_with("[workspace.")
        }) {
            return directory.to_path_buf();
        }
    }
    project.to_path_buf()
}

/// `target-dir = "…"` under `[build]`, the value, unquoted.
fn configured(text: &str) -> Option<String> {
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[build]";
            continue;
        }
        if !inside {
            continue;
        }
        let Some((named, value)) = trimmed.split_once('=') else {
            continue;
        };
        if named.trim() != "target-dir" {
            continue;
        }
        return Some(value.trim().trim_matches('"').to_string());
    }
    None
}

/// Is this, beside the binary, part of the application rather than the
/// build? On the system webview that is the binary and nothing else: the
/// engine is the system's, so there is no library or resource to carry.
pub fn runtime_file(name: &str, binary: &str) -> bool {
    name.eq_ignore_ascii_case(binary)
}

/// The binary cargo wrote, where cargo wrote it.
pub fn produced(binary: &str, release: bool, project: &Path) -> Option<PathBuf> {
    produced_told(
        binary,
        release,
        project,
        target_from_environment().as_deref(),
    )
}

/// [`produced`], with the environment said out loud — see
/// [`target_from_environment`] for why that is a parameter.
///
/// One place, and not a search: a search found a stale binary in a
/// `target/` cargo had stopped writing to, and handed it back as the
/// build that had just succeeded.
pub fn produced_told(
    binary: &str,
    release: bool,
    project: &Path,
    environment: Option<&str>,
) -> Option<PathBuf> {
    let file = format!("{binary}{}", std::env::consts::EXE_SUFFIX);
    let profile = if release { "release" } else { "debug" };
    let candidate = target_directory_told(project, environment)
        .join(profile)
        .join(&file);
    candidate.is_file().then_some(candidate)
}

/// The `[package]` table, as text.
///
/// Read by hand like the rest of the project reads its tokens: what is
/// wanted is four strings out of a file whose shape is known, and a TOML
/// parser would be more code to read than this is.
fn package_of(manifest: &str) -> String {
    let mut inside = false;
    let mut out = String::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[package]";
            continue;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// `name = "mon-app"` — the value, unquoted.
fn field(package: &str, key: &str) -> Option<String> {
    for line in package.lines() {
        let Some((named, value)) = line.split_once('=') else {
            continue;
        };
        if named.trim() != key {
            continue;
        }
        let value = value.trim();
        // `version.workspace = true` and friends: a value this does not
        // know how to read is a value it does not claim to have read.
        let value = value.strip_prefix('"')?.strip_suffix('"')?;
        return Some(value.to_string());
    }
    None
}

/// A field of the package, wherever it really lives.
///
/// `version.workspace = true` is how half the projects in the world
/// write their version, this one included: the value is one directory up,
/// in the workspace's own `[workspace.package]`. A `.exe` stamped
/// `0.0.0` because of that would be a bug a developer could not see the
/// cause of.
fn string(package: &str, key: &str, project: &Path) -> Option<String> {
    field(package, key).or_else(|| {
        inherits(package, key)
            .then(|| {
                workspace_package(project)
                    .as_deref()
                    .and_then(|table| field(table, key))
            })
            .flatten()
    })
}

/// Does the package say this field is the workspace's to say?
fn inherits(package: &str, key: &str) -> bool {
    package.lines().any(|line| {
        let line = line.trim();
        line.starts_with(&format!("{key}.workspace"))
            || (line.starts_with(key) && line.contains("workspace") && line.contains('{'))
    })
}

/// The `[workspace.package]` of whichever manifest above this declares a
/// workspace.
fn workspace_package(project: &Path) -> Option<String> {
    for directory in project.ancestors() {
        let Ok(text) = std::fs::read_to_string(directory.join("Cargo.toml")) else {
            continue;
        };
        if !text.contains("[workspace") {
            continue;
        }
        let mut inside = false;
        let mut out = String::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                inside = trimmed == "[workspace.package]";
                continue;
            }
            if inside {
                out.push_str(line);
                out.push('\n');
            }
        }
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn first_author(package: &str) -> Option<String> {
    let line = package
        .lines()
        .find(|line| line.trim_start().starts_with("authors"))?;
    let inside = line.split_once('[')?.1;
    between(inside, "\"", '"')
}

/// Every `.rs` of `src/`, run together.
///
/// The `Config` is usually in `main.rs`, and is sometimes one call away
/// in another file. What is being looked for is a literal string, and a
/// literal string is where it is written.
fn sources(project: &Path) -> String {
    let mut out = String::new();
    let mut left = vec![project.join("src")];
    while let Some(directory) = left.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                left.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                out.push_str(&text);
                out.push('\n');
            }
        }
    }
    out
}

/// What comes after `opening`, up to `closing`.
fn between(text: &str, opening: &str, closing: char) -> Option<String> {
    let at = text.find(opening)? + opening.len();
    let rest = &text[at..];
    let end = rest.find(closing)?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_are_read_from_the_source() {
        let source = "Config::new(\"x\").permissions(&[\"camera\", \"files\"]).run()";
        assert_eq!(permissions_in(source), ["camera", "files"]);
        assert!(permissions_in("Config::new(\"x\")").is_empty());
    }

    #[test]
    fn a_manifest_is_read_for_its_four_strings() {
        let manifest = "[package]\nname = \"mon-app\"\nversion = \"2.0.1\"\n\
                        authors = [\"aztra <a@b.c>\"]\ndescription = \"Une app\"\n\n\
                        [dependencies]\nname = \"not this one\"\n";
        let package = package_of(manifest);
        assert_eq!(field(&package, "name"), Some("mon-app".to_string()));
        assert_eq!(field(&package, "version"), Some("2.0.1".to_string()));
        assert_eq!(
            first_author(&package),
            Some("aztra <a@b.c>".to_string()),
            "the publisher is whoever the manifest names first"
        );
    }

    #[test]
    fn a_field_this_cannot_read_is_not_claimed() {
        let package = package_of("[package]\nversion.workspace = true\n");
        assert_eq!(field(&package, "version"), None);
    }
}
