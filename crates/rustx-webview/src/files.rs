//! Local files, for a page that writes `file://`.
//!
//! The window's page has the application's own origin, and a browser does
//! not let such a page load `file://`. A page of rustx may write one anyway
//! (`<video src="file:///C:/clips/a.mp4">`): the interpreter rewrites the
//! address to [`PATH`] followed by the file's path, and this answers it,
//! with byte ranges, which is what a `<video>` needs to start and to seek.
//!
//! Who may ask: the application's own pages, and nothing a site embedded in
//! them could fetch from elsewhere: a request that says it is cross-site is
//! refused. A file is served only if it is an absolute path, with no `..`.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path};

/// Where the interpreter sends a `file://` address: this, then the path.
pub const PATH: &str = "/__rustx/file/";

/// The most one answer carries. A player asks for more as it needs it.
const CHUNK: u64 = 4 * 1024 * 1024;

pub struct Answer {
    pub status: u16,
    pub mime: &'static str,
    pub body: Vec<u8>,
    /// `bytes first-last/total`, for a partial answer.
    pub content_range: Option<String>,
}

fn refused(status: u16, text: &'static str) -> Answer {
    Answer {
        status,
        mime: "text/plain; charset=utf-8",
        body: text.as_bytes().to_vec(),
        content_range: None,
    }
}

/// `rest` is what follows [`PATH`] in the URL path, still encoded.
pub fn serve(rest: &str, range: Option<&str>, cross_site: bool) -> Answer {
    if cross_site {
        return refused(403, "forbidden");
    }
    let Some(decoded) = crate::assets::percent_decoded(rest) else {
        return refused(400, "bad path");
    };
    // `C:/x` on Windows, `/x` elsewhere: the interpreter strips the slash
    // before a drive letter, so a Unix path arrives without its root.
    let text = if !cfg!(windows) && !decoded.starts_with('/') {
        format!("/{decoded}")
    } else {
        decoded
    };
    let path = Path::new(&text);
    if !path.is_absolute() || path.components().any(|part| matches!(part, Component::ParentDir)) {
        return refused(403, "forbidden");
    }
    if !path.is_file() {
        return refused(404, "not found");
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return refused(404, "not found");
    };
    let Ok(length) = file.metadata().map(|meta| meta.len()) else {
        return refused(404, "not found");
    };
    let mime = crate::assets::mime_of(path);
    if length == 0 {
        return Answer { status: 200, mime, body: Vec::new(), content_range: None };
    }
    let (first, last) = match range.map(|text| byte_range(text, length)) {
        Some(Some(found)) => found,
        Some(None) => {
            return Answer {
                status: 416,
                mime: "text/plain; charset=utf-8",
                body: Vec::new(),
                content_range: Some(format!("bytes */{length}")),
            };
        }
        None => (0, (CHUNK - 1).min(length - 1)),
    };
    let last = last.min(first + CHUNK - 1);
    let mut body = vec![0; (last - first + 1) as usize];
    if file.seek(SeekFrom::Start(first)).and_then(|_| file.read_exact(&mut body)).is_err() {
        return refused(500, "could not read");
    }
    let whole = range.is_none() && last == length - 1;
    Answer {
        status: if whole { 200 } else { 206 },
        mime,
        body,
        content_range: (!whole).then(|| format!("bytes {first}-{last}/{length}")),
    }
}

/// `bytes=a-b`, `bytes=a-` and `bytes=-n`, as first and last byte.
fn byte_range(text: &str, length: u64) -> Option<(u64, u64)> {
    let spec = text.trim().strip_prefix("bytes=")?;
    let (from, to) = spec.split(',').next()?.split_once('-')?;
    let (first, last) = match (from.trim(), to.trim()) {
        ("", n) => {
            let n: u64 = n.parse().ok()?;
            (length.saturating_sub(n), length - 1)
        }
        (a, "") => (a.parse().ok()?, length - 1),
        (a, b) => (a.parse().ok()?, b.parse::<u64>().ok()?.min(length - 1)),
    };
    (first <= last && first < length).then_some((first, last))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_of(bytes: &[u8], name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("rustx-files-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).expect("a file");
        path
    }

    /// The path as the interpreter writes it after `PATH`.
    fn encoded(path: &Path) -> String {
        let text = path.display().to_string().replace('\\', "/");
        let text = text.trim_start_matches('/');
        text.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"/-_.:".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect()
    }

    #[test]
    fn a_range_gets_its_bytes_and_a_content_range() {
        let path = file_of(b"0123456789", "a.mp4");
        let answer = serve(&encoded(&path), Some("bytes=2-5"), false);
        assert_eq!((answer.status, answer.body.as_slice()), (206, &b"2345"[..]));
        assert_eq!(answer.content_range.as_deref(), Some("bytes 2-5/10"));
        assert_eq!(answer.mime, "video/mp4");
        assert_eq!(serve(&encoded(&path), Some("bytes=7-"), false).body, b"789");
        let whole = serve(&encoded(&path), None, false);
        assert_eq!((whole.status, whole.body.len()), (200, 10));
        assert_eq!(serve(&encoded(&path), Some("bytes=50-"), false).status, 416);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn what_is_not_an_absolute_file_or_comes_from_elsewhere_is_refused() {
        let path = file_of(b"x", "b.txt");
        assert_eq!(serve(&encoded(&path), None, true).status, 403);
        assert_eq!(serve("..%2F..%2Fetc%2Fpasswd", None, false).status, 403);
        let _ = std::fs::remove_file(path);
    }
}
