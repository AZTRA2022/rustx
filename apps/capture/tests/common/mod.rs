//! Driving the capture application from the outside.

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

pub struct Report {
    values: HashMap<String, String>,
    raw: String,
}

impl Report {
    /// The value reported under `key`, or a panic naming what was captured.
    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or_else(|| {
            panic!(
                "the page never reported '{key}'. Full output:\n{}",
                self.raw()
            )
        })
    }

    pub fn raw(&self) -> String {
        match self.no_driver() {
            false => self.raw.clone(),
            true => format!("{}\n{NO_DRIVER}\n", self.raw),
        }
    }

    /// Did this machine fail to give Chromium a 3D context?
    ///
    /// Half the scenarios wait on frames, and a window that is not being
    /// composited produces none — so the assertion that fails says "a
    /// panicking job killed the frame loop" about a machine that simply
    /// has no driver. The sentence below is appended to every failure so
    /// that nobody debugs the framework for an afternoon over it.
    fn no_driver(&self) -> bool {
        self.raw.contains("Failed to create shared context")
    }
}

/// What to read when the window was never composited.
const NO_DRIVER: &str = "\
    --- rustx: this machine gave Chromium no 3D context (\"Failed to create shared \
    context\"), so the window produced no frames and every scenario that waits on one \
    reports nothing. It is the machine, not the framework: a virtual machine, a remote \
    desktop, or a driver. Run the suite with RUSTX_SOFTWARE=1, which composites on the \
    processor. ---";

/// Run the application on its only scenario and collect what it reported.
pub fn run_capture_scenario() -> Report {
    // A name nobody else can be holding. A fixed one was removed at
    // the start of a run, and on Windows that removal fails silently
    // while a process from the last run still has the file open — so
    // the tests read a stale report and passed in under two seconds.
    let report_file = std::env::temp_dir().join(format!(
        "rustx-report-{}-{}-{}.txt",
        "capture",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&report_file);

    // One application at a time on this machine. Each scenario starts a
    // browser, and this one has a GPU process that crash-loops: two
    // windows at once and both take minutes, which the suite reads as a
    // page that reported nothing. Alone, every one of them passes.
    let _alone = alone();

    let mut child = Command::new(env!("CARGO_BIN_EXE_capture"))
        .arg("--scenario=page")
        .arg(format!("--report={}", report_file.display()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("capture should start");

    let (sender, receiver) = mpsc::channel();
    for stream in [
        Box::new(child.stdout.take().expect("piped stdout")) as Box<dyn Read + Send>,
        Box::new(child.stderr.take().expect("piped stderr")),
    ] {
        let sender = sender.clone();
        std::thread::spawn(move || {
            let mut stream = stream;
            let mut text = String::new();
            let _ = stream.read_to_string(&mut text);
            let _ = sender.send(text);
        });
    }
    drop(sender);

    // Wait for the page to have had its say rather than for a fixed
    // number of seconds: starting Chromium takes much longer on a machine
    // that has just run the engine's own suite, and a flat sleep turns
    // that into a failure that looks like a bug in the application.
    wait_for_report(
        &report_file,
        "closed_after_param_change",
        Duration::from_secs(150),
    );

    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &child.id().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.wait();

    let mut raw = String::new();
    while let Ok(text) = receiver.recv_timeout(Duration::from_secs(10)) {
        raw.push_str(&text);
    }
    raw.push_str(&std::fs::read_to_string(&report_file).unwrap_or_default());

    let values = raw
        .lines()
        .filter_map(|line| line.strip_prefix("REPORT "))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();

    Report { values, raw }
}

/// Block until the report file holds `key`, or until the deadline.
fn wait_for_report(path: &std::path::Path, key: &str, patience: Duration) {
    let needle = format!("REPORT {key}=");
    let deadline = std::time::Instant::now() + patience;
    while std::time::Instant::now() < deadline {
        if std::fs::read_to_string(path).is_ok_and(|text| text.contains(&needle)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Held for as long as one scenario runs, across processes.
///
/// `cargo test` runs one binary per package at the same time, so a
/// workspace of six applications would start six browsers at once. The
/// lock is a file: a directory, in fact, because creating one is the
/// operation every file system refuses to do twice.
pub struct Alone(std::path::PathBuf);

impl Drop for Alone {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.0);
    }
}

/// Wait for every other scenario to be done, then take the lock.
pub fn alone() -> Alone {
    let held = std::env::temp_dir().join("rustx-one-application-at-a-time");
    let since = std::time::Instant::now();
    loop {
        if std::fs::create_dir(&held).is_ok() {
            return Alone(held);
        }
        // A run that was killed leaves its lock behind, and a suite that
        // then waits for ever is worse than two windows at once.
        let stale = std::fs::metadata(&held)
            .and_then(|about| about.modified())
            .map(|when| when.elapsed().unwrap_or_default() > Duration::from_secs(300))
            .unwrap_or(false);
        if stale || since.elapsed() > Duration::from_secs(600) {
            let _ = std::fs::remove_dir(&held);
            continue;
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}
