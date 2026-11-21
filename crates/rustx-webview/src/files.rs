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
//! refused.
//!
//! What may be asked for: only what the application granted. A path is
//! granted when Rust writes it into the page (an attribute holding a
//! `file://` address, or [`url`]), when a file is dropped on the window, or
//! by [`allow`], which takes a file or a folder. Anything else is a 403, so
//! markup injected into a page cannot read the disk.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Where the interpreter sends a `file://` address: this, then the path.
pub const PATH: &str = "/__rustx/file/";

/// The most one answer carries when a range is asked for or the file is
/// very large. A player asks for more as it needs it.
const CHUNK: u64 = 4 * 1024 * 1024;

/// A file read whole into one answer, when no range is asked for.
const WHOLE: u64 = 256 * 1024 * 1024;

fn grants() -> &'static Mutex<Vec<PathBuf>> {
    static GRANTS: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    GRANTS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Let the page read `path`: a file, or a folder and what is under it.
pub fn allow(path: impl AsRef<Path>) {
    let path = path.as_ref().to_path_buf();
    let mut held = grants()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !held.contains(&path) {
        held.push(path);
    }
}

fn granted(path: &Path) -> bool {
    let Ok(real) = path.canonicalize() else {
        return false;
    };
    let held = grants()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    held.iter()
        .any(|each| each.canonicalize().is_ok_and(|each| real.starts_with(each)))
}

/// The address a page loads `path` from, granted on the way: the one to
/// write in a `src`. Every byte that is not part of a name is encoded, so
/// `#`, `%`, `?`, spaces and accents come out right.
pub fn url(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    allow(path);
    let text = path.display().to_string().replace('\\', "/");
    let encoded: Vec<String> = text
        .trim_start_matches('/')
        .split('/')
        .map(|part| {
            part.bytes()
                .map(
                    |b| match b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                        true => (b as char).to_string(),
                        false => format!("%{b:02X}"),
                    },
                )
                .collect()
        })
        .collect();
    format!("{PATH}{}", encoded.join("/"))
}

/// Grant what an attribute Rust is about to write will make the page load:
/// the address in `src`, `href`, `poster` or `data`, and the `url(...)` of a
/// `style`. Only those, the same places the interpreter rewrites: a `title`,
/// a `data-*`, or the value of a field says what a person or a file wrote,
/// and a path in it is text, not a decision of the application.
pub(crate) fn grant_attribute(name: &str, value: &str) {
    match name {
        "src" | "href" | "poster" | "data" => {
            if value
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("file://")
            {
                grant_urls_in(value.trim_start());
            }
        }
        "style" => {
            let mut rest = value;
            while let Some(at) = rest.to_ascii_lowercase().find("url(") {
                let inside = rest[at + 4..].trim_start().trim_start_matches(['"', '\'']);
                if inside.to_ascii_lowercase().starts_with("file://") {
                    grant_urls_in(inside);
                }
                rest = &rest[at + 4..];
            }
        }
        _ => {}
    }
}

fn grant_urls_in(text: &str) {
    let mut rest = text;
    while let Some(at) = rest.to_ascii_lowercase().find("file://") {
        let after = &rest[at + 7..];
        let end = after
            .find(|c: char| c.is_whitespace() || "\"')<>".contains(c))
            .unwrap_or(after.len());
        let address = after[..end].split(['?', '#']).next().unwrap_or("");
        if let Some(path) = crate::assets::percent_decoded(address.trim_start_matches('/')) {
            allow(rooted(path));
        }
        rest = &after[end..];
    }
}

/// `C:/x` stays, and a Unix path gets its root back.
fn rooted(path: String) -> String {
    if !cfg!(windows) && !path.starts_with('/') {
        format!("/{path}")
    } else {
        path
    }
}

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
    let text = rooted(decoded);
    let path = Path::new(&text);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return refused(403, "forbidden");
    }
    if !granted(path) {
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
        return Answer {
            status: 200,
            mime,
            body: Vec::new(),
            content_range: None,
        };
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
        None if length <= WHOLE => (0, length - 1),
        None => (0, CHUNK - 1),
    };
    let last = if range.is_none() && length <= WHOLE {
        last
    } else {
        last.min(first + CHUNK - 1)
    };
    let mut body = vec![0; (last - first + 1) as usize];
    if file
        .seek(SeekFrom::Start(first))
        .and_then(|_| file.read_exact(&mut body))
        .is_err()
    {
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
        allow(&path);
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
        allow(&path);
        assert_eq!(serve(&encoded(&path), None, true).status, 403);
        assert_eq!(serve("..%2F..%2Fetc%2Fpasswd", None, false).status, 403);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_file_nobody_granted_is_refused_and_a_granted_folder_covers_its_files() {
        let secret = file_of(b"secret", "secret.txt");
        assert_eq!(serve(&encoded(&secret), None, false).status, 403);
        allow(
            secret
                .parent()
                .expect("a folder")
                .join("nothing-granted-here"),
        );
        assert_eq!(serve(&encoded(&secret), None, false).status, 403);
        grant_urls_in(&format!(
            "background: url(\"file:///{}\")",
            encoded(&secret)
        ));
        grant_urls_in(&format!("file:///{}", encoded(&secret)));
        assert_eq!(serve(&encoded(&secret), None, false).status, 200);
        let _ = std::fs::remove_file(secret);
    }

    #[test]
    fn only_the_attributes_that_load_something_grant_a_path() {
        let secret = file_of(b"secret", "from-a-title.txt");
        let address = format!("file:///{}", encoded(&secret));
        for name in ["title", "aria-label", "data-path", "value", "alt"] {
            grant_attribute(name, &address);
        }
        assert_eq!(serve(&encoded(&secret), None, false).status, 403);
        grant_attribute("style", &format!("background:url('{address}')"));
        assert_eq!(serve(&encoded(&secret), None, false).status, 200);
        let _ = std::fs::remove_file(secret);
    }

    #[test]
    fn url_encodes_awkward_names_and_grants_them() {
        let path = file_of(b"x", "a #1 %é&[b].txt");
        let address = url(&path);
        assert!(
            address.starts_with(PATH) && !address.contains('#') && !address.contains(' '),
            "{address}"
        );
        let rest = address.strip_prefix(PATH).expect("the route");
        assert_eq!(serve(rest, None, false).status, 200, "{address}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_large_file_without_a_range_is_whole_and_with_one_is_a_chunk() {
        let big = file_of(&vec![7u8; 5 * 1024 * 1024], "big.bin");
        allow(&big);
        let whole = serve(&encoded(&big), None, false);
        assert_eq!((whole.status, whole.body.len()), (200, 5 * 1024 * 1024));
        let part = serve(&encoded(&big), Some("bytes=0-"), false);
        assert_eq!((part.status, part.body.len() as u64), (206, CHUNK));
        let _ = std::fs::remove_file(big);
    }
}
