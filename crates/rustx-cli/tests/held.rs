//! A build that cannot win, recognised before it is started.
//!
//! CEF's build script copies `libcef.dll` and its neighbours next to the
//! binary, by deleting what is there and copying again. A window still
//! open from an earlier run holds that file, so the delete is refused —
//! and cargo reports `Access is denied. (os error 5)` from a build
//! script, which says nothing about a window nobody closed.
//!
//! What the command line owes a developer here is the sentence cargo
//! cannot write: *which* file is held, and that closing the window is
//! the cure.

use std::fs;
use std::path::{Path, PathBuf};

use rustx_cli::build::{held, target_directory};

fn somewhere(what: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rustx-held-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&path).expect("a directory to write into");
    path
}

/// A project with a `target/<profile>/` and the runtime files in it.
fn a_project(at: &Path, profile: &str) -> PathBuf {
    let out = at.join("target").join(profile);
    fs::create_dir_all(&out).expect("a target directory");
    fs::write(out.join("libcef.dll"), b"not really a library").expect("the library");
    out
}

#[test]
fn a_file_nobody_holds_is_not_held() {
    let room = somewhere("free");
    a_project(&room, "debug");

    assert_eq!(
        held(&room, false),
        None,
        "nothing is running, so nothing stands in the way of a build"
    );
}

// Only Windows holds a loaded library against its own file; nothing on
// Unix does, and nothing there needs this.
#[cfg(windows)]
#[test]
fn a_file_a_running_program_holds_is_named() {
    let room = somewhere("locked");
    let out = a_project(&room, "debug");
    let library = out.join("libcef.dll");

    // What a loaded DLL does to a file, in the one way a test can do it
    // without loading one: opened for reading, and refused to anybody
    // who wants to delete or replace it.
    let opened = exclusive(&library);

    assert_eq!(
        held(&room, false),
        Some(library.clone()),
        "the file that cannot be replaced is the file the developer is told about"
    );

    drop(opened);
    assert_eq!(
        held(&room, false),
        None,
        "and once the program is gone, the build is possible again"
    );
}

#[test]
fn a_missing_target_directory_is_not_a_problem() {
    let room = somewhere("empty");

    assert_eq!(
        held(&room, false),
        None,
        "a project that has never been built has nothing to hold"
    );
}

#[cfg(windows)]
#[test]
fn the_target_directory_a_project_configured_is_the_one_that_is_looked_at() {
    let room = somewhere("configured");
    let elsewhere = room.join("shared");
    fs::create_dir_all(room.join(".cargo")).expect("a .cargo directory");
    fs::write(
        room.join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"shared\"\n",
    )
    .expect("a cargo configuration");

    assert_eq!(
        target_directory(&room),
        elsewhere,
        "a project that shares a build directory is looked at where it builds, \
         and not where it would have built"
    );

    let out = elsewhere.join("debug");
    fs::create_dir_all(&out).expect("the shared target directory");
    let library = out.join("libcef.dll");
    fs::write(&library, b"not really a library").expect("the library");
    let opened = exclusive(&library);

    assert_eq!(held(&room, false), Some(library));
    drop(opened);
}

/// Open `path` so that nobody may delete or replace it — what Windows
/// does to a library a process has loaded.
#[cfg(windows)]
fn exclusive(path: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    // FILE_SHARE_READ only: another program may read it, and no program
    // may delete or write it.
    fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001)
        .open(path)
        .expect("the file this test just wrote")
}

