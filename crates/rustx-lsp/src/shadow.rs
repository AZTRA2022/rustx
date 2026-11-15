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
    /// The application, and the directory its pages are generated into:
    /// what an answer's file is told apart from the page's by.
    pub manifest: PathBuf,
    pub directory: PathBuf,
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
    let directory = rustx_compiler::build::generated_directory(&manifest);
    let generated = directory.join(relative).with_extension("rs");

    let mut out = rustx_compiler::transpile::transpile(source).ok()?;
    // At the end, so that no line of the page moves: see `crate_for_pages`.
    out.rust.push_str(crate::crate_for_pages::SCOPE);
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
        manifest,
        directory,
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
    match text.split_at_checked(1) {
        // `file:///c:/…` is a Windows path, and `C:` is how Windows
        // writes it: an editor may send either case, and a path
        // compared as text has to be the one the file system uses.
        Some((drive, after)) if after.starts_with(':') => Some(PathBuf::from(
            format!("{}{after}", drive.to_ascii_uppercase()).replace('/', "\\"),
        )),
        // No drive and no Windows: the `/` this function just stripped
        // is the root of the path, not a separator the URI added, and
        // `/` is already what the file system uses. Putting it back and
        // leaving the separators alone is the whole difference between
        // a path that opens and `tmp` + a backslash.
        _ if !cfg!(windows) => Some(PathBuf::from(format!("/{text}"))),
        _ => Some(PathBuf::from(text.replace('/', "\\"))),
    }
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

/// An answer from the downstream server, with every file it names put
/// back where the developer wrote it.
///
/// The downstream server answers in the files it holds — the shadow of
/// this page, the shadow of another page, the application's `src/` seen
/// through the reading crate — and a definition that lands there lands in
/// `target/`, in text the developer never wrote. Done by *path*, and
/// never by comparing strings: the server writes `file:///c%3A/…` where
/// this one wrote `file:///C:/…`, and a replace that does not match is a
/// definition that quietly goes to the generated file. One function,
/// applied to every answer that carries a location.
pub fn retargeted(answer: &str, page_uri: &str, shadow: &Shadow) -> String {
    let mut out = String::with_capacity(answer.len());
    let mut rest = answer;
    while let Some(at) = rest.find("file://") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find(['"', '\\']).unwrap_or(tail.len());
        let found = &tail[..end];
        match source_of(found, page_uri, shadow) {
            Some(back) => out.push_str(&crate::json::escape(&back)),
            None => out.push_str(found),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// Where the file a `uri` names is written, if it is generated.
fn source_of(uri: &str, page_uri: &str, shadow: &Shadow) -> Option<String> {
    let path = path_of(uri)?;
    let relative = under(&path, &shadow.directory)?;
    if relative.is_empty() {
        return None;
    }
    if under(&path, &shadow.path).is_some_and(|rest| rest.is_empty()) {
        return Some(page_uri.to_string());
    }
    let relative = PathBuf::from(relative);
    if relative.starts_with("src") {
        return Some(uri_of(&shadow.manifest.join(relative)));
    }
    let page = shadow
        .manifest
        .join("app")
        .join(relative)
        .with_extension("rsx");
    page.is_file().then(|| uri_of(&page))
}

/// What is left of `path` below `root`, or `None` when it is not below
/// it. Windows names the same file in any case.
fn under(path: &Path, root: &Path) -> Option<String> {
    let text = path.to_string_lossy().replace('\\', "/");
    let base = root.to_string_lossy().replace('\\', "/");
    let base = base.trim_end_matches('/');
    let (a, b) = if cfg!(windows) {
        (text.to_ascii_lowercase(), base.to_ascii_lowercase())
    } else {
        (text.clone(), base.to_string())
    };
    if a == b {
        return Some(String::new());
    }
    a.strip_prefix(&format!("{b}/"))?;
    Some(text[base.len() + 1..].to_string())
}

/// `retargeted`, and then every place it left in this page's own file put
/// where the page really has it.
pub fn back(answer: &str, page_uri: &str, shadow: &Shadow, source: &str) -> String {
    placed(
        &retargeted(answer, page_uri, shadow),
        page_uri,
        shadow,
        source,
    )
}

/// A location in the page, told in the coordinates of the generated file,
/// put where the developer wrote it.
///
/// Lines are the same in both files, so most places need only the map. A
/// state value is the exception: it is declared once at the top of the
/// page and the generated file declares it again inside the component, on
/// a line that answers to some other line of the page. There the *name*
/// is what survives, so the declaration is looked for by name.
fn placed(answer: &str, page_uri: &str, shadow: &Shadow, source: &str) -> String {
    let marker = format!(
        "\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":",
        crate::json::escape(page_uri)
    );
    let mut out = String::with_capacity(answer.len());
    let mut rest = answer;
    while let Some(at) = rest.find(&marker) {
        let head = at + marker.len();
        out.push_str(&rest[..head]);
        let tail = &rest[head..];
        let Some((range, used)) = a_range(tail) else {
            rest = tail;
            continue;
        };
        let (line, column, end_line, end_column) =
            place_in_page(range, shadow, source).unwrap_or(range);
        out.push_str(&format!(
            "{line},\"character\":{column}}},\"end\":{{\"line\":{end_line},\"character\":{end_column}"
        ));
        rest = &tail[used..];
    }
    out.push_str(rest);
    out
}

/// `A,"character":B},"end":{"line":C,"character":D` read off the front of
/// `text`, and how much of it that was.
fn a_range(text: &str) -> Option<((usize, usize, usize, usize), usize)> {
    fn number(text: &str, from: &mut usize, then: &str) -> Option<usize> {
        let digits = text[*from..]
            .chars()
            .take_while(char::is_ascii_digit)
            .count();
        let value = text[*from..*from + digits].parse().ok()?;
        *from += digits;
        text[*from..]
            .starts_with(then)
            .then(|| *from += then.len())?;
        Some(value)
    }
    let mut at = 0;
    let line = number(text, &mut at, ",\"character\":")?;
    let column = number(text, &mut at, "},\"end\":{\"line\":")?;
    let end_line = number(text, &mut at, ",\"character\":")?;
    let end_column = number(text, &mut at, "")?;
    Some(((line, column, end_line, end_column), at))
}

/// Zero-based, in the generated file's terms, to zero-based in the page's.
fn place_in_page(
    (line, column, end_line, end_column): (usize, usize, usize, usize),
    shadow: &Shadow,
    source: &str,
) -> Option<(usize, usize, usize, usize)> {
    if let (Some((a, b)), Some((c, d))) = (
        shadow.map.to_rsx_position(line + 1, column + 1),
        shadow.map.to_rsx_position(end_line + 1, end_column + 1),
    ) {
        return Some((a - 1, b - 1, c - 1, d - 1));
    }
    if line != end_line {
        return None;
    }
    let generated = shadow.rust.lines().nth(line)?;
    let name: String = generated
        .chars()
        .skip(column)
        .take(end_column.saturating_sub(column))
        .collect();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    declaration_of(&name, source).map(|(l, c)| (l, c, l, c + name.chars().count()))
}

/// The first line of the page that declares `name`, and its column.
fn declaration_of(name: &str, source: &str) -> Option<(usize, usize)> {
    const KEYWORDS: [&str; 9] = [
        "let mut ", "let ", "fn ", "struct ", "enum ", "const ", "static ", "type ", "trait ",
    ];
    for (number, text) in source.lines().enumerate() {
        for keyword in KEYWORDS {
            let Some(at) = text.find(&format!("{keyword}{name}")) else {
                continue;
            };
            let after = text[at + keyword.len() + name.len()..].chars().next();
            let before = text[..at].chars().next_back();
            if after.is_some_and(|c| c.is_alphanumeric() || c == '_')
                || before.is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            return Some((number, text[..at + keyword.len()].chars().count()));
        }
    }
    None
}

/// What a shadow is handed to a downstream server as.
///
/// `didOpen` carries the language and the text; `didChange` carries the
/// whole text again, because `textDocumentSync: 1` is what this server
/// asked the editor for and what it asks of its own server.
pub fn told(shadow: &Shadow, method: &str) -> String {
    let text = crate::json::escape(&shadow.rust);
    if method.ends_with("didOpen") {
        format!(
            "{{\"textDocument\":{{\"uri\":\"{}\",\"languageId\":\"rust\",\"version\":1,\
             \"text\":\"{text}\"}}}}",
            crate::json::escape(&shadow.uri)
        )
    } else {
        format!(
            "{{\"textDocument\":{{\"uri\":\"{}\",\"version\":{}}},\
             \"contentChanges\":[{{\"text\":\"{text}\"}}]}}",
            crate::json::escape(&shadow.uri),
            next_version()
        )
    }
}

/// A `didChange` carrying `text` for the file at `uri`.
///
/// Used for a shadow's real text, and for the file built to answer one
/// question about one expression.
pub fn changed(uri: &str, text: &str) -> String {
    format!(
        "{{\"textDocument\":{{\"uri\":\"{}\",\"version\":{}}},         \"contentChanges\":[{{\"text\":\"{}\"}}]}}",
        crate::json::escape(uri),
        next_version(),
        crate::json::escape(text)
    )
}

/// The version number of the next change sent downstream.
///
/// A version that does not move is a change `rust-analyzer` is entitled
/// to ignore, and it does: the file built for one question went out
/// under the same version as the file before it, so the question was
/// answered about the wrong text — or, more often, about a text the
/// server had already decided it knew, and answered `null`. Counted
/// here, for the whole process, because one editor session is one
/// sequence of changes however many pages it opens.
fn next_version() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static VERSION: AtomicUsize = AtomicUsize::new(2);
    VERSION.fetch_add(1, Ordering::Relaxed)
}
