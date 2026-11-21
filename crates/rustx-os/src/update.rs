//! Updating the application, from a GitHub release page.
//!
//! The documentation said "not written" where Electron has
//! `autoUpdater` and Tauri has its updater plugin, and a desktop
//! application that cannot update itself is one its users stay three
//! versions behind. What is written here is the shape the asking was
//! for: a repository's releases are the source of truth, the archive
//! `rustx package` wrote is the thing downloaded, and the swap happens
//! while the application is running so that the next start is the new
//! version.
//!
//! # What it does, in order
//!
//! 1. [`latest`] asks `api.github.com` for the newest release of a
//!    repository and picks the asset that matches this machine.
//! 2. [`Release::is_newer_than`] compares it with the version the
//!    running program was built with.
//! 3. [`fetch`] downloads the archive and checks its SHA-256 if the
//!    release published one.
//! 4. [`stage`] unpacks it beside the application, into a folder of its
//!    own, so that a half-downloaded update is never half-installed.
//! 5. [`apply`] moves the new files into place, renaming what is locked
//!    -- which on Windows includes the running `.exe` and `libcef.dll`
//!    -- out of the way rather than failing.
//! 6. [`restart`] starts the new binary and leaves.
//! 7. [`clean`], called at the next start, deletes what was renamed.
//!
//! [`from_github`] is all seven in one call, for an application that
//! wants the ordinary behaviour.
//!
//! # What it deliberately does not do
//!
//! It does not sign anything, and it does not check a signature. The
//! integrity it offers is the digest the release published and HTTPS to
//! GitHub, which is the same trust an installer download has. An
//! application that needs more should check the archive itself between
//! [`fetch`] and [`stage`] -- the path is handed back for exactly that.
//!
//! It also never updates silently while the user is working: nothing
//! here runs on its own. The application decides when to ask, when to
//! download, and when to restart, because only the application knows
//! whether there is unsaved work on the screen.

use std::path::{Path, PathBuf};

use crate::error::OsError;

/// What a release page says about one release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The tag, as the repository wrote it: `v1.4.0`, or `1.4.0`.
    pub tag: String,
    /// The same with any leading `v` taken off, which is what compares
    /// against `env!("CARGO_PKG_VERSION")`.
    pub version: String,
    /// What the release notes say. Shown to the person being asked
    /// whether to update, because "there is an update" is not a reason
    /// and "what changed" is.
    pub notes: String,
    /// The archive for this machine.
    pub asset: String,
    pub url: String,
    pub size: u64,
    /// The digest, when the release notes carry one.
    pub sha256: Option<String>,
    /// A `<archive>.sha256` asset beside the archive, when the release
    /// has one. Fetched by [`fetch`] rather than here: reading it is a
    /// second request, and a caller that only asked whether there is an
    /// update should not pay for it.
    pub digest_url: Option<String>,
}

impl Release {
    /// Is this release newer than the version given?
    ///
    /// Numbers compared as numbers, piece by piece: `1.10.0` is newer
    /// than `1.9.0`, which a string comparison gets wrong. A piece that
    /// is not a number -- `1.0.0-beta.2` -- compares as zero, with the
    /// result that a pre-release of a version is not newer than the
    /// version itself. That is the conservative answer: an updater that
    /// pushes a beta onto somebody who installed a release is a bug
    /// report.
    pub fn is_newer_than(&self, current: &str) -> bool {
        newer(&self.version, current)
    }
}

/// How the application names itself to GitHub, and what it says it is.
///
/// The user agent matters: GitHub refuses a request without one. It is
/// the application's own name rather than the framework's, so that a
/// rate-limit or an abuse report names the program that made the
/// request.
fn agent(app: &str) -> String {
    format!("{app} (rustx updater)")
}

/// The newest release of `owner/repo`, or `None` when the repository
/// has no release with an archive for this machine.
///
/// `repository` is written the way a URL writes it: `aztra/mon-app`.
/// Nothing else is accepted, because a repository name is not a thing
/// to guess at.
pub fn latest(repository: &str, app: &str) -> Result<Option<Release>, OsError> {
    if repository.split('/').count() != 2 || repository.contains(' ') {
        return Err(OsError::Platform(format!(
            "`{repository}` is not a repository. It is written `owner/name`, the way \
             the address bar writes it"
        )));
    }
    // `RUSTX_UPDATE_API` points the check at another server speaking the same
    // JSON: a test over loopback, or a GitHub Enterprise host.
    let api = std::env::var("RUSTX_UPDATE_API").unwrap_or_else(|_| "https://api.github.com".into());
    let url = format!(
        "{}/repos/{repository}/releases/latest",
        api.trim_end_matches('/')
    );
    let body = crate::platform::http::get(&url, &agent(app))?;
    let text = String::from_utf8_lossy(&body).to_string();
    Ok(release_in(&text))
}

/// Download the archive, check it, and say where it landed.
///
/// The file goes in the application's own cache folder, which is
/// deleted by anybody cleaning up disk space and is not the folder the
/// application is installed in -- an update that fails leaves nothing
/// in the program's directory.
pub fn fetch(
    release: &Release,
    app: &str,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<PathBuf, OsError> {
    let into = crate::data::cache(app)?.join("updates");
    std::fs::create_dir_all(&into).map_err(|error| {
        OsError::Platform(format!("{} could not be made ({error})", into.display()))
    })?;
    let path = into.join(&release.asset);
    let expected = release.size;
    let mut seen = 0_u64;
    crate::platform::http::download(&release.url, &agent(app), &path, &mut |bytes| {
        seen = bytes;
        progress(bytes, expected);
    })?;

    if release.size > 0 && seen != release.size {
        let _ = std::fs::remove_file(&path);
        return Err(OsError::Platform(format!(
            "the download stopped early: {seen} bytes of {}",
            release.size
        )));
    }
    // The notes first, because they are already in hand; the digest
    // asset otherwise, which costs one small request and is what turns
    // "the size was right" into "these are the bytes the release
    // published".
    let expected = match &release.sha256 {
        Some(expected) => Some(expected.clone()),
        None => match &release.digest_url {
            Some(url) => published_digest(url, app)?,
            None => None,
        },
    };
    if let Some(expected) = &expected {
        let bytes = std::fs::read(&path).map_err(|error| {
            OsError::Platform(format!("{} could not be read ({error})", path.display()))
        })?;
        let got = crate::digest::sha256_hex(&bytes);
        if !got.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(&path);
            return Err(OsError::Platform(format!(
                "the download does not match the digest the release published \
                 (expected {expected}, got {got}). Nothing was installed"
            )));
        }
    }
    Ok(path)
}

/// The digest a `.sha256` asset holds.
///
/// A missing or unreadable one is `None` and not an error: the archive
/// is already downloaded and its size already matched, and refusing to
/// install over a file the release page may never have published would
/// be refusing the update because of its paperwork. A file that *is*
/// there and does not match is still an error -- that check happens
/// above.
fn published_digest(url: &str, app: &str) -> Result<Option<String>, OsError> {
    let Ok(bytes) = crate::platform::http::get(url, &agent(app)) else {
        return Ok(None);
    };
    Ok(first_digest(&String::from_utf8_lossy(&bytes)))
}

/// The first digest in a file of them.
///
/// `sha256sum` writes `<digest>  <name>`, and `sha256sum -b` writes
/// `<digest> *<name>`; some release pages write the digest alone.
/// Either way it is the first 64-character hexadecimal word.
fn first_digest(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|word| word.trim_matches(['`', '"', '*']))
        .find(|word| word.len() == 64 && word.chars().all(|l| l.is_ascii_hexdigit()))
        .map(|word| word.to_ascii_lowercase())
}

/// Unpack an archive into a folder of its own, and say which folder.
///
/// The folder is beside the application rather than in a temporary
/// place, because the files are about to be moved into the
/// application's own directory and a move across volumes is a copy that
/// can fail halfway.
pub fn stage(archive: &Path) -> Result<PathBuf, OsError> {
    let beside = beside_the_program()?;
    let into = beside.join(".rustx-update");
    if into.exists() {
        let _ = std::fs::remove_dir_all(&into);
    }
    std::fs::create_dir_all(&into).map_err(|error| {
        OsError::Platform(format!("{} could not be made ({error})", into.display()))
    })?;
    crate::unzip::into(archive, &into)?;

    // `rustx package` puts everything under one folder inside the
    // archive, so that unpacking it by hand does not scatter 300 files.
    // A single directory with nothing beside it is that folder, and
    // what has to be moved is its contents.
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&into)
        .map_err(|error| {
            OsError::Platform(format!("{} could not be read ({error})", into.display()))
        })?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    if entries.len() == 1
        && let Some(only) = entries.pop()
        && only.is_dir()
    {
        return Ok(only);
    }
    Ok(into)
}

/// Move a staged update into place.
///
/// What makes this work while the application is running: a file
/// Windows has open cannot be deleted or written, but it *can* be
/// renamed. So every file that is in the way is renamed to
/// `<name>.rustx-old` and the new one takes its place. The running
/// program keeps the file it has open -- under its new name, which it
/// does not care about -- and the next start reads the new one.
///
/// [`clean`] deletes the renamed files, and is called at the next
/// start rather than here: this process still has them open.
pub fn apply(staged: &Path) -> Result<usize, OsError> {
    let into = beside_the_program()?;
    let mut moved = 0;
    let mut files = Vec::new();
    walk(staged, staged, &mut files);
    for relative in &files {
        let from = staged.join(relative);
        let to = into.join(relative);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                OsError::Platform(format!("{} could not be made ({error})", parent.display()))
            })?;
        }
        if to.exists() {
            let aside = aside(&to);
            let _ = std::fs::remove_file(&aside);
            std::fs::rename(&to, &aside).map_err(|error| {
                OsError::Platform(format!(
                    "{} could not be moved out of the way ({error}). Is another copy of \
                     this application running?",
                    to.display()
                ))
            })?;
        }
        std::fs::rename(&from, &to).map_err(|error| {
            OsError::Platform(format!(
                "{} could not be put in place ({error})",
                to.display()
            ))
        })?;
        moved += 1;
    }
    let _ = std::fs::remove_dir_all(staged);
    let _ = std::fs::remove_dir_all(into.join(".rustx-update"));
    Ok(moved)
}

/// Delete what the last update renamed out of the way.
///
/// Called at the start of an application, where the files are no longer
/// open. A file that is still held -- a second window of the old
/// version, somebody's antivirus reading it -- is left for the start
/// after that, because an update that refuses to start over a leftover
/// file is worse than a leftover file.
pub fn clean() -> usize {
    let Ok(beside) = beside_the_program() else {
        return 0;
    };
    let mut files = Vec::new();
    walk(&beside, &beside, &mut files);
    let mut gone = 0;
    for relative in files {
        if relative.to_string_lossy().ends_with(".rustx-old")
            && std::fs::remove_file(beside.join(&relative)).is_ok()
        {
            gone += 1;
        }
    }
    gone
}

/// Start this program again, and say so. The caller leaves afterwards.
///
/// The new process is given the same arguments this one was started
/// with, minus the program's own name: an application opened on a
/// document reopens on the same document.
pub fn restart() -> Result<(), OsError> {
    let exe = std::env::current_exe()
        .map_err(|error| OsError::Platform(format!("this program's own path ({error})")))?;
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    std::process::Command::new(&exe)
        .args(arguments)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .spawn()
        .map_err(|error| {
            OsError::Platform(format!("{} could not be started ({error})", exe.display()))
        })?;
    Ok(())
}

/// What one call does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing newer than what is running.
    UpToDate,
    /// There is a newer release and it has been installed; the next
    /// start is the new version. The application decides when that is:
    /// [`restart`] is not called for it.
    Installed(Release),
}

/// Check, download, install -- and stop at the first of those that has
/// nothing to do.
///
/// `current` is the version this build is, which for an application is
/// `env!("CARGO_PKG_VERSION")`. It is a parameter rather than read from
/// the binary because the version of the *application* is in the
/// application's own `Cargo.toml`, and this crate cannot see it.
///
/// It does not restart. An application that wants to restart calls
/// [`restart`] when it is safe to, which is a decision about unsaved
/// work and not about updates.
pub fn from_github(
    repository: &str,
    app: &str,
    current: &str,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Outcome, OsError> {
    let Some(release) = latest(repository, app)? else {
        return Ok(Outcome::UpToDate);
    };
    if !release.is_newer_than(current) {
        return Ok(Outcome::UpToDate);
    }
    let archive = fetch(&release, app, progress)?;
    let staged = stage(&archive)?;
    apply(&staged)?;
    let _ = std::fs::remove_file(&archive);
    Ok(Outcome::Installed(release))
}

/// The directory the running program is in.
fn beside_the_program() -> Result<PathBuf, OsError> {
    let exe = std::env::current_exe()
        .map_err(|error| OsError::Platform(format!("this program's own path ({error})")))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| OsError::Platform("this program is not in a directory".to_string()))
}

fn aside(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".rustx-old");
    PathBuf::from(name)
}

fn walk(root: &Path, directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, into);
        } else if let Ok(relative) = path.strip_prefix(root) {
            into.push(relative.to_path_buf());
        }
    }
}

/// Which of two versions is newer, by their numbers.
fn newer(candidate: &str, current: &str) -> bool {
    let core = |text: &str| -> (Vec<u64>, bool) {
        let text = text.trim().trim_start_matches(['v', 'V']);
        // The release part is what comes before the first `-` or `+`:
        // `1.0.0-beta.2` is release `1.0.0` with something after it,
        // and semantic versioning says that something sorts *before*
        // the release, not after.
        let release = text.split(['-', '+']).next().unwrap_or(text);
        let numbers = release
            .split('.')
            .map(|piece| piece.parse::<u64>().unwrap_or(0))
            .collect();
        (numbers, release.len() != text.len())
    };
    let (theirs, their_prerelease) = core(candidate);
    let (ours, our_prerelease) = core(current);
    for at in 0..theirs.len().max(ours.len()) {
        let theirs = theirs.get(at).copied().unwrap_or(0);
        let ours = ours.get(at).copied().unwrap_or(0);
        if theirs != ours {
            return theirs > ours;
        }
    }
    // The same three numbers. A pre-release of them is older than them
    // -- which is what keeps an updater from pushing `1.0.0-beta.2`
    // onto somebody who installed `1.0.0` -- and the release itself is
    // newer than a pre-release of it, which is how somebody on a beta
    // gets the release.
    our_prerelease && !their_prerelease
}

/// Read the one release out of GitHub's answer.
///
/// A hand-written reader rather than a JSON crate, for the reason the
/// rest of this crate gives for everything it does not depend on: what
/// is needed is five fields of a known shape, and an updater that
/// pulled in a parser and its derive macros to read five fields would
/// be the largest dependency in the framework.
///
/// What it reads: `tag_name`, `body`, and the `assets` array's `name`,
/// `browser_download_url` and `size`. The asset chosen is the one whose
/// name names this platform -- `windows`, `macos`/`darwin`, `linux` --
/// and ends in `.zip`; a release with one archive and no platform in
/// its name is taken as that archive, which is what a project with one
/// platform publishes.
fn release_in(text: &str) -> Option<Release> {
    let tag = string_field(text, "tag_name")?;
    let notes = string_field(text, "body").unwrap_or_default();
    let version = tag.trim_start_matches(['v', 'V']).to_string();

    let assets = after(text, "\"assets\"")?;
    let mut archives: Vec<(String, String, u64)> = Vec::new();
    let mut digests: Vec<(String, String)> = Vec::new();
    for piece in objects_in(assets) {
        let Some(name) = string_field(&piece, "name") else {
            continue;
        };
        let Some(url) = string_field(&piece, "browser_download_url") else {
            continue;
        };
        let size = number_field(&piece, "size").unwrap_or(0);
        let lowered = name.to_ascii_lowercase();
        if lowered.ends_with(".sha256") || lowered.ends_with(".sha256.txt") {
            digests.push((lowered, url));
            continue;
        }
        if lowered.ends_with(".zip") {
            archives.push((name, url, size));
        }
    }
    if archives.is_empty() {
        return None;
    }

    let platform: &[&str] = if cfg!(windows) {
        &["windows", "win64", "win32", "win"]
    } else if cfg!(target_os = "macos") {
        &["macos", "darwin", "osx", "mac"]
    } else {
        &["linux"]
    };
    let chosen = archives
        .iter()
        .find(|(name, _, _)| {
            let lowered = name.to_ascii_lowercase();
            platform.iter().any(|word| lowered.contains(word))
        })
        .or_else(|| {
            // One archive and no platform in its name: a project that
            // publishes for one system. More than one, none of which
            // names this system, is not something to guess at.
            (archives.len() == 1).then(|| &archives[0])
        })?;

    // Two places a release says what the archive should hash to: a
    // `<archive>.sha256` asset beside it, which is the convention, and
    // a line of the notes, which is where a project that writes it by
    // hand usually puts it.
    let wanted = chosen.0.to_ascii_lowercase();
    let digest_url = digests
        .iter()
        .find(|(name, _)| {
            name.as_str() == format!("{wanted}.sha256")
                || name.as_str() == format!("{wanted}.sha256.txt")
        })
        .map(|(_, url)| url.clone());

    Some(Release {
        tag,
        version,
        notes: notes.clone(),
        asset: chosen.0.clone(),
        url: chosen.1.clone(),
        size: chosen.2,
        sha256: sha256_in(&notes, &chosen.0),
        digest_url,
    })
}

/// A sixty-four-character hexadecimal word in the notes, on a line that
/// also names the archive.
fn sha256_in(notes: &str, asset: &str) -> Option<String> {
    for line in notes.lines() {
        if !line.contains(asset) {
            continue;
        }
        for word in line.split_whitespace() {
            let word = word.trim_matches(['`', '"', '(', ')', ':', ',']);
            if word.len() == 64 && word.chars().all(|letter| letter.is_ascii_hexdigit()) {
                return Some(word.to_ascii_lowercase());
            }
        }
    }
    None
}

fn after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let at = text.find(key)?;
    Some(&text[at + key.len()..])
}

/// `"key": "value"`, with the escapes JSON allows in a string.
fn string_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let rest = after(text, &needle)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut letters = rest.chars();
    while let Some(letter) = letters.next() {
        match letter {
            '"' => return Some(out),
            '\\' => match letters.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let hex: String = letters.by_ref().take(4).collect();
                    if let Ok(number) = u32::from_str_radix(&hex, 16)
                        && let Some(letter) = char::from_u32(number)
                    {
                        out.push(letter);
                    }
                }
                Some(other) => out.push(other),
                None => return Some(out),
            },
            other => out.push(other),
        }
    }
    None
}

fn number_field(text: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let rest = after(text, &needle)?;
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Every `{ … }` at the top level of an array, as text.
fn objects_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes = text.as_bytes();
    let mut depth = 0_i32;
    let mut start = 0_usize;
    let mut inside_text = false;
    let mut escaped = false;
    for (at, byte) in bytes.iter().enumerate() {
        if inside_text {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                inside_text = false;
            }
            continue;
        }
        match byte {
            b'"' => inside_text = true,
            b'{' => {
                if depth == 0 {
                    start = at;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    found.push(text[start..=at].to_string());
                }
                if depth < 0 {
                    break;
                }
            }
            b']' if depth == 0 => break,
            _ => {}
        }
    }
    found
}

#[cfg(test)]
mod tests {
    /// Whatever `sha256sum` wrote, and nothing from a file that holds
    /// no digest -- an HTML error page, which is what a release page
    /// serves for a URL that is gone.
    #[test]
    fn a_digest_file_is_read_whichever_way_it_was_written() {
        let digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        for written in [
            format!("{digest}  counter-1.0.0-windows.zip\n"),
            format!("{digest} *counter-1.0.0-windows.zip\n"),
            format!("{digest}\n"),
            format!("SHA256: `{}`\n", digest.to_uppercase()),
        ] {
            assert_eq!(first_digest(&written).as_deref(), Some(digest), "{written}");
        }
        assert_eq!(first_digest("<html><body>404</body></html>"), None);
        assert_eq!(first_digest(""), None);
    }

    use super::*;

    const ANSWER: &str = r#"{
      "tag_name": "v1.10.0",
      "name": "1.10.0",
      "body": "Faster start.\nmon-app-1.10.0-windows.zip 9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08\nmon-app-1.10.0-linux.zip 9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08\nmon-app-1.10.0-macos.zip 9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08",
      "assets": [
        { "name": "mon-app-1.10.0-linux.zip", "size": 11, "browser_download_url": "https://example.test/linux.zip" },
        { "name": "mon-app-1.10.0-windows.zip", "size": 22, "browser_download_url": "https://example.test/windows.zip" },
        { "name": "mon-app-1.10.0-macos.zip", "size": 33, "browser_download_url": "https://example.test/macos.zip" }
      ]
    }"#;

    #[test]
    fn the_release_is_read_out_of_githubs_answer() {
        let release = release_in(ANSWER).expect("a release");
        assert_eq!(release.tag, "v1.10.0");
        assert_eq!(release.version, "1.10.0");
        assert!(release.notes.starts_with("Faster start."), "{release:?}");
        assert!(release.notes.contains('\n'), "the escape was not read");
    }

    /// The asset chosen is this machine's, and the sizes are there to
    /// prove which one was picked rather than the first.
    #[test]
    fn the_asset_for_this_machine_is_the_one_chosen() {
        let release = release_in(ANSWER).expect("a release");
        if cfg!(windows) {
            assert_eq!(release.asset, "mon-app-1.10.0-windows.zip");
            assert_eq!(release.size, 22);
        } else if cfg!(target_os = "macos") {
            assert_eq!(release.asset, "mon-app-1.10.0-macos.zip");
            assert_eq!(release.size, 33);
        } else {
            assert_eq!(release.asset, "mon-app-1.10.0-linux.zip");
            assert_eq!(release.size, 11);
        }
    }

    #[test]
    fn a_digest_in_the_notes_is_read_and_lowercased() {
        let release = release_in(ANSWER).expect("a release");
        assert_eq!(
            release.sha256.as_deref(),
            Some("9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
        );
    }

    #[test]
    fn one_archive_with_no_platform_in_its_name_is_taken() {
        let answer = r#"{"tag_name":"2.0.0","assets":[
            {"name":"mon-app.zip","size":5,"browser_download_url":"https://example.test/a.zip"}]}"#;
        let release = release_in(answer).expect("a release");
        assert_eq!(release.asset, "mon-app.zip");
    }

    #[test]
    fn a_release_with_no_archive_is_no_release() {
        let answer = r#"{"tag_name":"2.0.0","assets":[
            {"name":"source.tar.gz","size":5,"browser_download_url":"https://example.test/a"}]}"#;
        assert!(release_in(answer).is_none());
    }

    #[test]
    fn several_archives_and_none_for_this_machine_is_not_guessed_at() {
        let answer = r#"{"tag_name":"2.0.0","assets":[
            {"name":"mon-app-aix.zip","size":5,"browser_download_url":"https://example.test/a.zip"},
            {"name":"mon-app-haiku.zip","size":6,"browser_download_url":"https://example.test/b.zip"}]}"#;
        assert!(release_in(answer).is_none());
    }

    /// The comparison a string would get wrong, which is the one that
    /// matters: ten is not less than nine.
    #[test]
    fn versions_compare_as_numbers() {
        assert!(newer("1.10.0", "1.9.0"));
        assert!(!newer("1.9.0", "1.10.0"));
        assert!(newer("2.0.0", "1.999.999"));
        assert!(!newer("1.0.0", "1.0.0"));
        assert!(newer("1.0.1", "1.0.0"));
        assert!(newer("v1.0.1", "1.0.0"), "a leading v is not a version");
    }

    /// A pre-release is not newer than the release it precedes. An
    /// updater that pushes a beta onto somebody who installed a release
    /// is a bug report.
    #[test]
    fn a_prerelease_is_not_newer_than_its_release() {
        assert!(!newer("1.0.0-beta.2", "1.0.0"));
        assert!(newer("1.0.0-beta.2", "0.9.0"));
        // And the other way: somebody running a beta is offered the
        // release.
        assert!(newer("1.0.0", "1.0.0-beta.2"));
        // Two pre-releases of the same version are not compared by
        // their suffix. Whoever is on a beta gets the next release,
        // not the next beta -- an updater is not a release channel.
        assert!(!newer("1.0.0-beta.3", "1.0.0-beta.2"));
    }

    #[test]
    fn a_repository_that_is_not_one_is_refused_before_a_request_is_made() {
        let said = latest("not a repo", "test").expect_err("refused");
        assert!(format!("{said}").contains("owner/name"), "{said}");
        let said = latest("https://github.com/a/b", "test").expect_err("refused");
        assert!(format!("{said}").contains("owner/name"), "{said}");
    }

    #[test]
    fn a_renamed_file_is_named_so_the_next_start_can_find_it() {
        let path = Path::new("C:/apps/mon-app/libcef.dll");
        assert!(
            aside(path).to_string_lossy().ends_with(".dll.rustx-old"),
            "{:?}",
            aside(path)
        );
    }

    #[test]
    fn objects_are_read_one_by_one_and_a_brace_in_a_string_is_text() {
        let found = objects_in(r#"[{"a":"}"},{"b":"{"}]"#);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].contains("\"a\""), "{found:?}");
    }
}
