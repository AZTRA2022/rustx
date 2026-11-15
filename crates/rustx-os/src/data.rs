//! Where an application keeps its own files.
//!
//! The gap this closes was written down in the documentation as "none":
//! Electron has `app.getPath("userData")`, Tauri has its path API, and
//! rustx had the engine's profile folder and a warning not to put
//! anything in it. So an application that wanted to remember a window
//! size, a recent-files list or a signed-in user had to work out
//! `%APPDATA%` for itself, and the honest ones did it differently from
//! each other.
//!
//! Three places, because they are not the same question:
//!
//! - [`directory`] is what the user would be upset to lose: their
//!   settings, their documents, their library. It is the roaming
//!   profile on Windows, which means a domain user finds it on another
//!   machine.
//! - [`cache`] is what can be deleted at any moment without the
//!   application minding: thumbnails, a copy of something that can be
//!   fetched again.
//! - [`logs`] is what somebody reads when the application misbehaves.
//!
//! Each one is created the first time it is asked for, and each one is
//! named after the application rather than after the executable: two
//! builds of the same program are the same program to the person using
//! it.
//!
//! The reads and writes are here rather than left to `std::fs` for one
//! reason, and it is [`write`]'s: a settings file written in place is a
//! settings file that is empty if the machine loses power between the
//! truncate and the flush, and an application that cannot start because
//! its own settings are half a file is a bug report nobody can act on.

use std::path::{Path, PathBuf};

use crate::error::OsError;

/// Where this application keeps what the user would mind losing.
///
/// | | |
/// |---|---|
/// | Windows | `%APPDATA%\<app>` |
/// | macOS | `~/Library/Application Support/<app>` |
/// | Linux | `$XDG_DATA_HOME/<app>`, or `~/.local/share/<app>` |
pub fn directory(app: &str) -> Result<PathBuf, OsError> {
    made(root(Kind::Data)?.join(folder(app)?))
}

/// Where it keeps what it can rebuild.
///
/// | | |
/// |---|---|
/// | Windows | `%LOCALAPPDATA%\<app>\cache` |
/// | macOS | `~/Library/Caches/<app>` |
/// | Linux | `$XDG_CACHE_HOME/<app>`, or `~/.cache/<app>` |
pub fn cache(app: &str) -> Result<PathBuf, OsError> {
    made(root(Kind::Cache)?.join(folder(app)?))
}

/// Where it writes what somebody will read when things go wrong.
///
/// A folder of its own rather than a file in the data directory: logs
/// are the one thing a user is asked to send, and "the whole of this
/// folder" is an instruction that works over a telephone.
pub fn logs(app: &str) -> Result<PathBuf, OsError> {
    made(root(Kind::Logs)?.join(folder(app)?).join("logs"))
}

/// The path of one file inside [`directory`], without reading or
/// writing it.
///
/// Subdirectories are allowed -- `path(app, "profiles/me.json")` is a
/// file in a folder this creates -- and anything that would leave the
/// application's own directory is refused rather than cleaned up: a
/// name that escapes is either a bug or an attack, and both deserve an
/// answer instead of a surprise.
pub fn path(app: &str, name: &str) -> Result<PathBuf, OsError> {
    let directory = directory(app)?;
    let inside = directory.join(checked(name)?);
    if let Some(parent) = inside.parent() {
        made(parent.to_path_buf())?;
    }
    Ok(inside)
}

/// The bytes of a file in [`directory`], or `None` when there is no
/// such file.
///
/// A missing file is not an error, because the first start of every
/// application is a missing file. Anything else -- no permission, a
/// directory where a file was expected -- is.
pub fn read(app: &str, name: &str) -> Result<Option<Vec<u8>>, OsError> {
    let path = path(app, name)?;
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(OsError::Platform(format!(
            "{} could not be read ({error})",
            path.display()
        ))),
    }
}

/// The same, as text, when the file holds text.
pub fn read_text(app: &str, name: &str) -> Result<Option<String>, OsError> {
    let Some(bytes) = read(app, name)? else {
        return Ok(None);
    };
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| OsError::Platform(format!("{name} is not text")))
}

/// Write a file in [`directory`], whole or not at all.
///
/// The bytes go to a file beside the real one and are then renamed over
/// it, which on every platform this targets is the operation that
/// either happens or does not. A reader sees the old file or the new
/// one, never a truncated one -- which is what makes it safe to keep
/// settings this way, and why this function exists instead of a line of
/// `std::fs`.
pub fn write(app: &str, name: &str, bytes: &[u8]) -> Result<(), OsError> {
    let path = path(app, name)?;
    let temporary = path.with_extension(format!(
        "{}rustx-{}",
        path.extension()
            .map(|it| format!("{}.", it.to_string_lossy()))
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&temporary, bytes).map_err(|error| {
        OsError::Platform(format!(
            "{} could not be written ({error})",
            temporary.display()
        ))
    })?;
    // Windows refuses a rename onto an existing file, so the old one
    // goes first. Between those two lines the file does not exist,
    // which is a worse moment than on the systems where the rename
    // replaces -- it is also the only moment, and it is shorter than
    // the write that used to be done in place.
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    std::fs::rename(&temporary, &path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary);
        OsError::Platform(format!(
            "{} could not be put in place ({error})",
            path.display()
        ))
    })
}

/// The same, from text.
pub fn write_text(app: &str, name: &str, text: &str) -> Result<(), OsError> {
    write(app, name, text.as_bytes())
}

/// Forget a file. A file that was not there is not an error.
pub fn remove(app: &str, name: &str) -> Result<(), OsError> {
    let path = path(app, name)?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(OsError::Platform(format!(
            "{} could not be removed ({error})",
            path.display()
        ))),
    }
}

/// The names of the files directly inside [`directory`], sorted.
///
/// Directories are left out: what this answers is "what has this
/// application saved", and a folder is not a save.
pub fn list(app: &str) -> Result<Vec<String>, OsError> {
    let directory = directory(app)?;
    let entries = std::fs::read_dir(&directory).map_err(|error| {
        OsError::Platform(format!(
            "{} could not be listed ({error})",
            directory.display()
        ))
    })?;
    let mut found: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    found.sort();
    Ok(found)
}

/// Every platform has these three, under different names.
enum Kind {
    Data,
    Cache,
    Logs,
}

#[cfg(windows)]
fn root(kind: Kind) -> Result<PathBuf, OsError> {
    // `%APPDATA%` roams with a domain user and `%LOCALAPPDATA%` does
    // not, which is the whole difference between what a user would mind
    // losing and what they would not notice.
    let variable = match kind {
        Kind::Data => "APPDATA",
        Kind::Cache | Kind::Logs => "LOCALAPPDATA",
    };
    let Some(found) = std::env::var_os(variable) else {
        return Err(OsError::Platform(format!(
            "%{variable}% is not set, so this user has no profile folder to write in"
        )));
    };
    let root = PathBuf::from(found);
    Ok(match kind {
        Kind::Cache => root.join("cache"),
        _ => root,
    })
}

#[cfg(target_os = "macos")]
fn root(kind: Kind) -> Result<PathBuf, OsError> {
    let home = home()?;
    Ok(match kind {
        Kind::Data => home.join("Library").join("Application Support"),
        Kind::Cache => home.join("Library").join("Caches"),
        Kind::Logs => home.join("Library").join("Logs"),
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn root(kind: Kind) -> Result<PathBuf, OsError> {
    // The XDG base directory specification, and its own defaults when
    // the variables are not set -- which they usually are not on a
    // machine somebody has not configured.
    let home = home()?;
    let told = |name: &str| std::env::var_os(name).map(PathBuf::from);
    Ok(match kind {
        Kind::Data => told("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local").join("share")),
        Kind::Cache => told("XDG_CACHE_HOME").unwrap_or_else(|| home.join(".cache")),
        Kind::Logs => told("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local").join("state")),
    })
}

#[cfg(not(any(windows, unix)))]
fn root(_kind: Kind) -> Result<PathBuf, OsError> {
    Err(OsError::Unsupported(
        "an application data folder on this platform (rustx-os/src/data.rs is where \
         it goes)",
    ))
}

#[cfg(unix)]
fn home() -> Result<PathBuf, OsError> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        OsError::Platform("$HOME is not set, so this user has no home directory".to_string())
    })
}

/// The application's name as a folder name.
///
/// Kept as written where it can be -- `Mon App` is a folder called
/// `Mon App`, which is what a user sees in their profile and expects to
/// recognise. What is taken out is what a file system refuses or a path
/// reads as something else, and an empty result is refused rather than
/// turned into a folder called nothing.
fn folder(app: &str) -> Result<String, OsError> {
    let cleaned: String = app
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            other if (other as u32) < 0x20 => '-',
            other => other,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() {
        return Err(OsError::Platform(
            "this application has no name, so there is no folder to keep its files in \
             -- `Config::new(\"…\")` is where the name comes from"
                .to_string(),
        ));
    }
    Ok(cleaned)
}

/// A file name inside the application's own directory, checked.
fn checked(name: &str) -> Result<PathBuf, OsError> {
    let path = Path::new(name);
    if name.is_empty() {
        return Err(OsError::Platform("a file needs a name".to_string()));
    }
    if path.is_absolute() {
        return Err(OsError::Platform(format!(
            "`{name}` is an absolute path. This writes inside the application's own \
             folder; a path of your own is `std::fs`'s business"
        )));
    }
    for piece in path.components() {
        match piece {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(OsError::Platform(format!(
                    "`{name}` leaves the application's own folder, which is the one \
                     place this writes"
                )));
            }
        }
    }
    Ok(path.to_path_buf())
}

fn made(directory: PathBuf) -> Result<PathBuf, OsError> {
    std::fs::create_dir_all(&directory).map_err(|error| {
        OsError::Platform(format!(
            "{} could not be made ({error})",
            directory.display()
        ))
    })?;
    Ok(directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name of its own per test, because these touch the real profile
    /// folder -- which is the only way to find out that it works.
    fn named(what: &str) -> String {
        format!("rustx-test-{what}-{}", std::process::id())
    }

    fn cleaned(app: &str) {
        if let Ok(directory) = directory(app) {
            let _ = std::fs::remove_dir_all(directory);
        }
    }

    #[test]
    fn a_file_written_is_a_file_read_back() {
        let app = named("roundtrip");
        write_text(&app, "settings.json", "{\"size\":900}").expect("written");
        assert_eq!(
            read_text(&app, "settings.json").expect("read"),
            Some("{\"size\":900}".to_string())
        );
        cleaned(&app);
    }

    #[test]
    fn a_file_that_was_never_written_is_none_and_not_an_error() {
        let app = named("absent");
        assert_eq!(read(&app, "nothing.bin").expect("no error"), None);
        cleaned(&app);
    }

    #[test]
    fn a_second_write_replaces_the_first_whole() {
        let app = named("replace");
        write_text(&app, "a.txt", "first").expect("written");
        write_text(&app, "a.txt", "second").expect("written");
        assert_eq!(
            read_text(&app, "a.txt").expect("read"),
            Some("second".to_string())
        );
        // And nothing is left beside it: the temporary file is the
        // mechanism, not a file the application has to know about.
        let files = list(&app).expect("listed");
        assert_eq!(files, vec!["a.txt".to_string()], "{files:?}");
        cleaned(&app);
    }

    #[test]
    fn a_file_in_a_folder_is_written_and_listed_as_its_folder() {
        let app = named("nested");
        write_text(&app, "profiles/me.json", "{}").expect("written");
        assert!(path(&app, "profiles/me.json").expect("a path").is_file());
        // `list` answers "what has this application saved" at the top
        // level, so a folder is not in it.
        assert!(list(&app).expect("listed").is_empty());
        cleaned(&app);
    }

    #[test]
    fn removing_a_file_that_is_not_there_is_not_an_error() {
        let app = named("remove");
        remove(&app, "never.txt").expect("no error");
        write_text(&app, "once.txt", "x").expect("written");
        remove(&app, "once.txt").expect("removed");
        assert_eq!(read(&app, "once.txt").expect("read"), None);
        cleaned(&app);
    }

    /// The check that makes this safe to hand a name from a page.
    #[test]
    fn a_name_that_leaves_the_folder_is_refused() {
        let app = named("escape");
        for name in ["../outside.txt", "a/../../outside.txt", "./../outside.txt"] {
            assert!(
                path(&app, name).is_err(),
                "`{name}` was allowed out of the folder"
            );
        }
        assert!(path(&app, "").is_err(), "an empty name was allowed");
        cleaned(&app);
    }

    #[test]
    fn the_three_folders_are_three_different_places() {
        let app = named("three");
        let data = directory(&app).expect("data");
        let cache = cache(&app).expect("cache");
        let logs = logs(&app).expect("logs");
        assert_ne!(data, cache);
        assert_ne!(data, logs);
        assert_ne!(cache, logs);
        let _ = std::fs::remove_dir_all(&logs);
        let _ = std::fs::remove_dir_all(&cache);
        cleaned(&app);
    }

    #[test]
    fn an_application_with_no_name_is_refused_rather_than_given_a_folder() {
        assert!(folder("").is_err());
        assert!(folder("   ").is_err());
        assert!(folder("...").is_err());
    }

    #[test]
    fn a_name_a_file_system_refuses_is_cleaned_and_not_rejected() {
        assert_eq!(folder("Mon App").expect("a folder"), "Mon App");
        assert_eq!(folder("a/b:c").expect("a folder"), "a-b-c");
    }
}
