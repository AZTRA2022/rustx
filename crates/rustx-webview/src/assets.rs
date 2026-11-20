//! The application's files, served to its own window.
//!
//! Instead of `file://` pages, which browsers give an origin of their
//! own and which let one local file read another. The window loads
//! `<origin>/index.html`; this answers it from a folder, and from nothing
//! outside it.

use std::path::{Component, Path, PathBuf};

/// One answer: a status, a media type and a body.
pub struct Served {
    pub status: u16,
    pub mime: &'static str,
    pub body: Vec<u8>,
}

/// The file `request_path` names under `root`, or a 404 or a 403.
///
/// `request_path` is the path of the URL, as it came. It is decoded, then
/// refused if it climbs (`..`), is rooted somewhere else, or names a drive
/// -- and, after that, refused again if the file that exists is not under
/// `root` once links are resolved.
pub fn serve(root: &Path, request_path: &str) -> Served {
    let Some(relative) = relative_path(request_path) else {
        return failed(403, "forbidden");
    };
    let mut file = root.join(&relative);
    if file.is_dir() {
        file = file.join("index.html");
    }
    let (Ok(real), Ok(base)) = (file.canonicalize(), root.canonicalize()) else {
        return failed(404, "not found");
    };
    if !real.starts_with(&base) {
        return failed(403, "forbidden");
    }
    match std::fs::read(&real) {
        Ok(body) => Served {
            status: 200,
            mime: mime_of(&real),
            body,
        },
        Err(_) => failed(404, "not found"),
    }
}

fn failed(status: u16, text: &'static str) -> Served {
    Served {
        status,
        mime: "text/plain; charset=utf-8",
        body: text.as_bytes().to_vec(),
    }
}

/// The URL path as a relative file path, or `None` when it tries to leave.
fn relative_path(request_path: &str) -> Option<PathBuf> {
    let path = request_path.split(['?', '#']).next().unwrap_or("");
    let decoded = percent_decoded(path)?;
    let trimmed = decoded.trim_start_matches(['/', '\\']);
    let mut out = PathBuf::new();
    for part in Path::new(trimmed).components() {
        match part {
            Component::Normal(name) => out.push(name),
            Component::CurDir => {}
            // `..`, a drive, a root: none of them stays under `root`.
            _ => return None,
        }
    }
    Some(out)
}

fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let pair = bytes.get(at + 1..at + 3)?;
            let digits = std::str::from_utf8(pair).ok()?;
            out.push(u8::from_str_radix(digits, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    let text = String::from_utf8(out).ok()?;
    // A decoded NUL ends a path in some APIs and not in others.
    (!text.contains('\0')).then_some(text)
}

fn mime_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("wasm") => "application/wasm",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
