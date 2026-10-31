//! What `rustx dev` does between two keystrokes.
//!
//! The loop is one thing and what it launches is another, on purpose:
//! these tests ask whether a saved page is noticed, whether ten saves in
//! a breath are one rebuild, and whether a build that fails leaves the
//! window that is on screen alone — without compiling anything, because
//! a test that waits a minute for cargo is a test nobody runs.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use rustx_cli::dev::{self, Outcome};
use rustx_cli::watch::{Coalescing, Watched};

fn somewhere(what: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rustx-dev-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(path.join("app")).expect("a project to watch");
    fs::create_dir_all(path.join("src")).expect("a project to watch");
    fs::write(path.join("app/page.rsx"), "fn Page() -> Element {\n").expect("a page");
    fs::write(path.join("src/main.rs"), "fn main() {}\n").expect("a main");
    path
}

fn watching(project: &std::path::Path) -> Watched {
    Watched::new(
        &[project.join("app"), project.join("src")],
        &["rsx", "rs", "css", "json"],
    )
}

#[test]
fn a_saved_page_is_rebuilt() {
    let project = somewhere("saved");
    let mut watched = watching(&project);
    assert!(
        !watched.changed(),
        "nothing has been touched since it started looking"
    );

    fs::write(
        project.join("app/page.rsx"),
        "fn Page() -> Element {\n    return <main>deux</main>;\n}\n",
    )
    .expect("a saved page");
    assert!(watched.changed(), "a saved page is a change");
    assert!(!watched.changed(), "and it is a change exactly once");

    // A page that appears, and a page that goes away.
    fs::write(project.join("app/about/page.rsx"), "").ok();
    fs::create_dir_all(project.join("app/about")).ok();
    fs::write(project.join("app/about/page.rsx"), "fn Page() {}").expect("a new page");
    assert!(watched.changed(), "a new page is a change");
    fs::remove_file(project.join("app/about/page.rsx")).expect("a page removed");
    assert!(watched.changed(), "a page that goes away is a change too");

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn a_file_that_is_not_a_page_or_a_source_is_ignored() {
    let project = somewhere("noise");
    let mut watched = watching(&project);
    fs::write(project.join("app/debug.log"), "a line of log").expect("a log");
    fs::write(project.join("app/page.rsx.bak"), "an editor's leftover").expect("a backup");
    assert!(
        !watched.changed(),
        "a log and an editor's backup are not a reason to rebuild"
    );
    let _ = fs::remove_dir_all(&project);
}

#[test]
fn ten_saves_in_a_breath_are_one_rebuild() {
    // What an editor that formats on save does. Ten rebuilds would mean
    // nine windows opened and closed in front of somebody typing.
    let start = Instant::now();
    let mut coalescing = Coalescing::new(Duration::from_millis(200));
    let mut rebuilds = 0;

    for tick in 0..10 {
        // A save every 20 ms, which is faster than a rebuild can finish.
        if coalescing.tick(true, start + Duration::from_millis(tick * 20)) {
            rebuilds += 1;
        }
    }
    assert_eq!(
        rebuilds, 0,
        "nothing starts while the saving is still going"
    );

    // The quiet that follows.
    for tick in 10..30 {
        if coalescing.tick(false, start + Duration::from_millis(tick * 20)) {
            rebuilds += 1;
        }
    }
    assert_eq!(rebuilds, 1, "one rebuild, 200 ms after the last save");
}

#[test]
fn a_build_that_fails_leaves_the_window_alone() {
    // The compiler's complaint is already at the coordinates of the
    // `.rsx`. What matters here is that nothing was launched and nothing
    // was killed: the window from before is still on screen, showing the
    // last page that compiled.
    let mut launched: Vec<PathBuf> = Vec::new();
    let outcome = dev::once(
        &mut || Err("app/page.rsx:4:9: `<main>` is never closed".to_string()),
        &mut |exe: &std::path::Path| launched.push(exe.to_path_buf()),
    );
    match outcome {
        Outcome::Failed(said) => assert!(said.contains("page.rsx:4:9"), "got {said}"),
        Outcome::Ran(exe) => panic!("it launched {}", exe.display()),
    }
    assert!(launched.is_empty(), "a failed build launched something");

    // And a build that works does launch, once.
    let outcome = dev::once(
        &mut || Ok(PathBuf::from("target/debug/mon-app.exe")),
        &mut |exe: &std::path::Path| launched.push(exe.to_path_buf()),
    );
    assert!(matches!(outcome, Outcome::Ran(_)));
    assert_eq!(
        launched.len(),
        1,
        "a build that works launches exactly once"
    );
}
