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
    })
}

/// Build it, then stamp what cargo produced.
pub fn run(release: bool, project: &Path) -> Result<PathBuf, String> {
    let read = read(project)?;

    // A build that cannot win, said before it is started: cargo's own
    // message for this is an access error inside a build script, and it
    // names neither the file nor the window that is holding it.
    if let Some(file) = held(project, release) {
        return Err(says_it_is_held(&file));
    }

    let mut cargo = Command::new("cargo");
    cargo.arg("build").current_dir(project);
    if release {
        cargo.arg("--release");
    }
    let status = cargo
        .status()
        .map_err(|error| format!("cargo could not be started: {error}"))?;
    if !status.success() {
        // Cargo already said what is wrong, in the coordinates of the
        // `.rsx` because that is what the compiler keeps. Saying it
        // again, worse, would only push it off the screen.
        return Err(String::new());
    }

    let Some(exe) = produced(&read.binary, release, project) else {
        return Err(format!(
            "cargo said it was done and there is no `{}{}` under any `target/`. Does \
             the project have a `[[bin]]` of another name?",
            read.binary,
            std::env::consts::EXE_SUFFIX
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

/// Where cargo put the file.
///
/// Its own `target/` for a project on its own; the workspace's for a
/// project that is a member of one. Looked for rather than worked out,
/// because which of the two it is depends on a manifest further up than
/// this reads.
/// Where this project builds.
///
/// `target/` beside the manifest, unless the project said otherwise:
/// `[build] target-dir` in `.cargo/config.toml` is how two projects
/// share one build directory, and a project that shares one must be
/// looked at where it really builds.
pub fn target_directory(project: &Path) -> PathBuf {
    for name in [".cargo/config.toml", ".cargo/config"] {
        let Ok(text) = std::fs::read_to_string(project.join(name)) else {
            continue;
        };
        if let Some(named) = configured(&text) {
            let named = PathBuf::from(named.replace('\\', "/"));
            return if named.is_absolute() {
                named
            } else {
                project.join(named)
            };
        }
    }
    project.join("target")
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

/// The runtime file a program still running holds, if there is one.
///
/// CEF's build script puts `libcef.dll` and its neighbours next to the
/// binary by deleting what is there and copying again. A window still
/// open from an earlier run has that library loaded, so Windows refuses
/// the delete, and cargo reports `Access is denied. (os error 5)` from
/// a build script — which names neither the file nor the window.
///
/// Asked **before** cargo is started, because the answer is a sentence
/// and not a failure: there is nothing to try.
pub fn held(project: &Path, release: bool) -> Option<PathBuf> {
    let profile = if release { "release" } else { "debug" };
    let out = target_directory(project).join(profile);
    // The library is the one that matters: it is the big one, it is
    // always loaded, and the others go the same way when it does.
    for name in ["libcef.dll", "chrome_elf.dll", "libEGL.dll"] {
        let candidate = out.join(name);
        if !candidate.is_file() {
            continue;
        }
        if std::fs::OpenOptions::new()
            .write(true)
            .open(&candidate)
            .is_err()
        {
            return Some(candidate);
        }
    }
    None
}

/// What to say about a file a running program holds.
pub fn says_it_is_held(file: &Path) -> String {
    format!(
        "{} is held by a program that is still running — a window from an \
         earlier run of this application, or a `rustx dev` that was never \
         closed. CEF's build replaces that file on every build, and Windows \
         refuses while it is loaded, so the next build would fail on \
         `Access is denied. (os error 5)` in a build script. Close the \
         window. If there is none on screen, the process is still there: \
         `taskkill /im <the application>.exe /f`.",
        file.display()
    )
}

fn produced(binary: &str, release: bool, project: &Path) -> Option<PathBuf> {
    let file = format!("{binary}{}", std::env::consts::EXE_SUFFIX);
    let profile = if release { "release" } else { "debug" };
    std::iter::once(project)
        .chain(project.ancestors().skip(1))
        .map(|directory| directory.join("target").join(profile).join(&file))
        .find(|candidate| candidate.is_file())
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
