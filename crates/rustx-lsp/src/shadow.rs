//! The Rust file that stands in for the page being typed.
//!
//! `rust-analyzer` answers questions about Rust files, and the Rust of a
//! page already exists: `#[rustx::main]` includes it by path, so it is
//! part of the application's crate and shares its whole vocabulary — the
//! application's own modules, `rustx`, every dependency. A question
//! about a `.rsx` becomes a question about that file.
//!
//! So the server writes it. Where the build writes it, deliberately: a
//! second copy somewhere else would be a file nobody compiles, and
//! `rust-analyzer` would answer about a crate that does not exist. The
//! build overwrites it at the next expansion anyway, and what the server
//! writes is only ever a page that compiled.

use std::path::{Path, PathBuf};

use rustx_compiler::map::PositionMap;

/// A page's Rust, on disk, with the way back to the page.
pub struct Shadow {
    /// The generated `.rs`.
    pub path: PathBuf,
    /// Its URI, which is what `rust-analyzer` is told about.
    pub uri: String,
    pub rust: String,
    pub map: PositionMap,
}

/// Compile the text the editor is holding and write it where the build
/// writes it.
///
/// `None` when there is nothing to write: a page that does not compile —
/// half a tag, which is what a `.rsx` is most of the time — or a file
/// that is not under an application's `app/`. In the first case the
/// shadow that is there is left alone, so completion goes on working
/// from the last page that made sense.
pub fn of(uri: &str, source: &str) -> Option<Shadow> {
    let page = path_of(uri)?;
    let (manifest, relative) = application_of(&page)?;
    let generated = rustx_compiler::build::generated_directory(&manifest)
        .join(relative)
        .with_extension("rs");

    let out = rustx_compiler::transpile::transpile(source).ok()?;
    if let Some(parent) = generated.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    std::fs::write(&generated, &out.rust).ok()?;
    // The map goes beside it, under the name the build gives it, so that
    // a `rustc` error from a real build still lands on the right line.
    let _ = std::fs::write(
        generated.with_extension("rsx.map"),
        out.map.to_text().as_bytes(),
    );

    Some(Shadow {
        uri: uri_of(&generated),
        path: generated,
        rust: out.rust,
        map: out.map,
    })
}

/// The application a page belongs to, and where the page sits in its
/// `app/`.
///
/// A `.rsx` is a page of an application, which is a directory holding a
/// `Cargo.toml` and an `app/`. Anything else — a page opened on its own,
/// a scratch file — has no crate and therefore no shadow.
fn application_of(page: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut directory = page.parent()?;
    let mut parts: Vec<&std::ffi::OsStr> = Vec::new();
    loop {
        if directory.file_name() == Some(std::ffi::OsStr::new("app"))
            && let Some(manifest) = directory.parent()
            && manifest.join("Cargo.toml").is_file()
        {
            let mut relative = PathBuf::new();
            for part in parts.iter().rev() {
                relative.push(part);
            }
            relative.push(page.file_name()?);
            return Some((manifest.to_path_buf(), relative));
        }
        parts.push(directory.file_name()?);
        directory = directory.parent()?;
    }
}

/// A `file://` URI for a path, as an editor writes one.
pub fn uri_of(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix('/').unwrap_or(&text);
    format!("file:///{}", escape(text))
}

/// The path a `file://` URI names.
pub fn path_of(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    let text = unescape(rest);
    // `file:///c:/…` is a Windows path, and `C:` is how Windows writes
    // it: an editor may send either case, and a path compared as text
    // has to be the one the file system uses.
    let text = match text.split_at_checked(1) {
        Some((drive, rest)) if rest.starts_with(':') => {
            format!("{}{rest}", drive.to_ascii_uppercase())
        }
        _ => text,
    };
    Some(PathBuf::from(text.replace('/', "\\")))
}

/// The few characters a path may hold that a URI may not.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            letter => out.push(letter),
        }
    }
    out
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut letters = text.chars();
    while let Some(letter) = letters.next() {
        if letter == '%' {
            let digits: String = letters.by_ref().take(2).collect();
            match u32::from_str_radix(&digits, 16)
                .ok()
                .and_then(char::from_u32)
            {
                Some(found) => out.push(found),
                None => out.push('%'),
            }
        } else {
            out.push(letter);
        }
    }
    out
}
