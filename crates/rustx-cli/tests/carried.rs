//! State crossing a real restart, in real processes.
//!
//! The unit tests next door prove the reading and the writing. What is
//! left to prove is the thing a developer actually sees: a process that
//! is killed and started again, through the file `rustx dev` names in
//! the environment, with the value coming back. So this test starts
//! itself three times — no browser engine, no window, just the two ends
//! of the mechanism and a `kill` in between.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The environment variable the application reads. Set by `rustx dev`
/// and by nothing else in the world.
const CARRIED: &str = "RUSTX_CARRIED";
/// What tells the process it was started as the application.
const AS_THE_APPLICATION: &str = "RUSTX_TEST_AS_THE_APPLICATION";

#[test]
fn a_named_value_crosses_a_real_restart() {
    let file: PathBuf = std::env::temp_dir().join(format!(
        "rustx-carried-across-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_file(&file);

    // Three launches of the same binary, as `rustx dev` would do them:
    // each one reads what the last left, adds one, and is killed.
    for expected in 1..=3 {
        let said = run_once(&file);
        assert!(
            said.contains(&format!("clicks={expected}")),
            "launch {expected} said: {said}"
        );
        assert!(
            said.contains("page=/counter"),
            "the page that was open did not cross: {said}"
        );
    }

    let _ = fs::remove_file(&file);
}

fn run_once(file: &std::path::Path) -> String {
    let exe = std::env::current_exe().expect("this test is a file");
    let output = Command::new(exe)
        .args([
            "--exact",
            "the_application_itself",
            "--nocapture",
            "--include-ignored",
        ])
        .env(CARRIED, file)
        .env(AS_THE_APPLICATION, "1")
        .output()
        .expect("the application should start");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// The application, as far as this test needs one.
///
/// Ignored, so that it only runs when the test above asks for it by
/// name; and a no-op unless the environment says it is the one being
/// started, so that `--include-ignored` on the whole suite does not turn
/// it into a mystery.
#[test]
#[ignore = "started by a_named_value_crosses_a_real_restart"]
fn the_application_itself() {
    if std::env::var(AS_THE_APPLICATION).is_err() {
        return;
    }
    let Some(file) = std::env::var_os(CARRIED) else {
        return;
    };
    rustx_dom::carried::carrying(std::path::Path::new(&file));

    // What the framework does on start: the page that was open, then the
    // page's own `let mut` at the top of the file.
    let page = rustx_dom::carried::page().unwrap_or_else(|| "/counter".to_string());
    rustx_dom::carried::be_at(&page);

    let clicks = rustx_dom::store::shared("clicks", || 0_i32);
    clicks.set(clicks.get() + 1);

    println!("clicks={}", clicks.get());
    println!("page={page}");
}
