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
        &[rustx_compiler::bundle::SERVED],
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

#[test]
fn what_is_launched_is_a_copy_and_not_the_file_cargo_wrote() {
    // Why there is a copy at all: on Windows a running `.exe` is held by
    // the system, and the next turn of the loop has cargo rewrite that
    // very file -- and `resources::stamp` reopen it to put the logo in.
    // Both are refused, with an access error on every single save, and
    // the refusal is caused by our own rule that the old process is
    // killed last. Launching a copy keeps the rule and frees the file.
    let project = somewhere("copy");
    let exe = project.join("target/debug/mon-app.exe");
    fs::create_dir_all(exe.parent().expect("a target")).expect("a target");
    fs::write(&exe, b"what cargo wrote").expect("a built binary");

    let launched = dev::launchable(&project, &exe).expect("a copy to launch");

    assert_ne!(launched, exe, "the file cargo wrote is the one it must rewrite");
    assert_eq!(
        fs::read(&launched).expect("the copy is readable"),
        b"what cargo wrote",
        "the copy is not the binary that was built"
    );
    assert!(
        launched.starts_with(project.join("target")),
        "{} is outside target/, which is the one directory a project already ignores",
        launched.display()
    );
    assert_eq!(
        launched.extension().and_then(|end| end.to_str()),
        exe.extension().and_then(|end| end.to_str()),
        "a copy that does not end in .exe is not an executable to Windows"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn every_turn_launches_its_own_copy_so_a_held_one_blocks_nothing() {
    // The copy of the turn before is still running when this one is
    // made: it must not be the file being written either.
    let project = somewhere("turns");
    let exe = project.join("target/debug/mon-app.exe");
    fs::create_dir_all(exe.parent().expect("a target")).expect("a target");
    fs::write(&exe, b"one").expect("a built binary");

    let first = dev::launchable(&project, &exe).expect("a first copy");
    let held = fs::File::open(&first).expect("the first copy is open, as a running one is");
    fs::write(&exe, b"two").expect("a second build");
    let second = dev::launchable(&project, &exe).expect("a second copy");

    assert_ne!(first, second, "the second turn wrote over the copy that is running");
    assert_eq!(
        fs::read(&second).expect("the second copy is readable"),
        b"two",
        "the second copy is the first build again"
    );
    drop(held);

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn a_leftover_copy_that_will_not_go_away_is_not_an_error() {
    // Tidying up is a courtesy, not the job: a copy still held by a
    // process that has not quite finished dying must not turn a
    // successful build into a failure.
    // Which is not a hypothesis: this is the leftover of the turn
    // before, with its process still up and therefore holding it.
    let project = somewhere("leftover");
    let exe = project.join("target/debug/mon-app.exe");
    fs::create_dir_all(exe.parent().expect("a target")).expect("a target");
    fs::copy("C:/Windows/System32/PING.EXE", &exe).expect("something that runs");

    let before = dev::launchable(&project, &exe).expect("the copy of the turn before");
    let mut still_up = std::process::Command::new(&before)
        .args(["-n", "30", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("the turn before is still running");

    let launched = dev::launchable(&project, &exe);
    let _ = still_up.kill();
    let _ = still_up.wait();

    let launched = launched.expect("a leftover that will not go away is not a failure");
    assert!(launched.is_file(), "nothing was launched");
    assert!(
        before.is_file(),
        "the copy that was still running was removed under it"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn the_file_the_build_itself_writes_is_not_a_change() {
    // The loop would otherwise feed itself: the bundle is written into
    // `assets/` by every build, `assets/` is watched, and `.js` is a
    // watched extension -- so one saved page became a rebuild every few
    // seconds, for ever. `target/` was already excluded for this exact
    // reason; this is the same door, left open.
    let project = somewhere("generated");
    fs::create_dir_all(project.join("assets")).expect("an assets directory");
    let bundle = project.join("assets").join(rustx_compiler::bundle::SERVED);
    fs::write(&bundle, "// what the build wrote").expect("a bundle");

    let mut watched = Watched::new(
        &[project.join("app"), project.join("assets")],
        &["rsx", "js"],
        &[rustx_compiler::bundle::SERVED],
    );
    assert!(!watched.changed(), "nothing has been touched yet");

    fs::write(&bundle, "// what the next build wrote, a byte longer").expect("a rebuilt bundle");
    assert!(
        !watched.changed(),
        "the bundle the build writes started another build"
    );

    // And the rule is that file, not every `.js`: a script a developer
    // writes is still a reason to rebuild.
    fs::write(project.join("assets").join("a-moi.js"), "console.log(1)").expect("a script");
    assert!(watched.changed(), "a script the developer wrote is a change");

    let _ = fs::remove_dir_all(&project);
}

/// The thing that fails, reproduced: a process that is running holds
/// the file it was started from, and Windows refuses every write to it.
///
/// This is the whole bug. `rustx dev` launched `target/debug/<bin>.exe`
/// and then asked cargo to rewrite that very file -- and
/// `resources::stamp` to reopen it and put the logo in -- while the
/// process it had deliberately not killed yet was still holding it. An
/// access error, on every single save.
#[cfg(windows)]
#[test]
fn a_rebuild_can_write_the_file_cargo_wrote_while_the_application_runs() {
    let project = somewhere("held");
    let exe = project.join("target/debug/mon-app.exe");
    fs::create_dir_all(exe.parent().expect("a target")).expect("a target");
    // A real executable that stays up long enough to hold its file, and
    // one every Windows has.
    fs::copy("C:/Windows/System32/PING.EXE", &exe).expect("something that runs");

    let launched = dev::launchable(&project, &exe).expect("a copy to launch");
    let mut child = std::process::Command::new(&launched)
        .args(["-n", "30", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("the copy runs");

    // The test has teeth only if the lock it is about is real: the file
    // the process was started from cannot be written.
    assert!(
        fs::write(&launched, b"this must be refused").is_err(),
        "nothing is holding {}, so this test proves nothing",
        launched.display()
    );

    // And the point: what cargo writes is free, with the application up.
    let written = fs::write(&exe, b"what the next build wrote");
    let said = written.as_ref().err().map(|error| error.to_string());
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        written.is_ok(),
        "the next build is refused its own output file: {}",
        said.unwrap_or_default()
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn the_copy_keeps_the_name_the_developer_gave_the_application() {
    // What a developer sees in a task manager, and what an application
    // that looks at its own file sees. A copy called
    // `mon-app-10024-7.exe` would be the loop's bookkeeping leaking into
    // the developer's world, so the turn is in the directory and the
    // file keeps its name.
    let project = somewhere("named");
    let exe = project.join("target/debug/mon-app.exe");
    fs::create_dir_all(exe.parent().expect("a target")).expect("a target");
    fs::write(&exe, b"what cargo wrote").expect("a built binary");

    let first = dev::launchable(&project, &exe).expect("a copy to launch");
    let second = dev::launchable(&project, &exe).expect("a second copy");

    for copy in [&first, &second] {
        assert_eq!(
            copy.file_name(),
            exe.file_name(),
            "{} is not the name the developer gave it",
            copy.display()
        );
    }
    assert_ne!(first, second, "two turns, one path");

    let _ = fs::remove_dir_all(&project);
}
