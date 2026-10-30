//! Driving the `hello` application from the outside.
//!
//! The application runs a named scenario inside the page when it is given
//! `--scenario=<name>`, and reports what it found through a native function
//! that prints `REPORT <key>=<value>` on stdout — from the render process,
//! which is itself part of what is being proved.

#![allow(dead_code)] // each test binary uses its own corner of this module.

use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// What one scenario run reported.
pub struct Report {
    values: HashMap<String, String>,
    raw: String,
}

impl Report {
    /// The value reported under `key`, or a panic naming what was captured.
    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or_else(|| {
            panic!("the page never reported '{key}'. Full output:\n{}", self.raw)
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }
}

/// Runs the application through a scenario and collects what it reported.
///
/// The window stays open until the harness takes it away, which is what a
/// scenario that only observes wants.
pub fn run_scenario(name: &str) -> Report {
    run(name, Ending::Killed).report
}

/// Who is expected to end the run.
enum Ending {
    /// The harness kills the application after it has had its say.
    Killed,
    /// The scenario closes the window, and the process is expected to go.
    ByItself,
}

fn run(name: &str, ending: Ending) -> Completion {
    // A name nobody else can be holding. A fixed one was removed at
    // the start of a run, and on Windows that removal fails silently
    // while a process from the last run still has the file open — so
    // the tests read a stale report and passed in under two seconds.
    let report_file = std::env::temp_dir().join(format!(
        "rustx-report-{}-{}-{}.txt",
        name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_file(&report_file);

    let mut child = Command::new(env!("CARGO_BIN_EXE_hello"))
        .arg(format!("--scenario={name}"))
        .arg(format!("--report={}", report_file.display()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("hello should start");

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

    let exit = match ending {
        Ending::Killed => {
            wait_for_quiet(&report_file, Duration::from_secs(120), Duration::from_secs(8));
            kill(&child);
            let _ = child.wait();
            None
        }
        // Long, because the patience has to cover Chromium's start-up
        // and not only the shutdown being measured: on a machine with
        // no GPU the first frame is most of a minute away.
        Ending::ByItself => wait_for_exit(&mut child, Duration::from_secs(150)),
    };

    let mut raw = String::new();
    while let Ok(text) = receiver.recv_timeout(Duration::from_secs(10)) {
        raw.push_str(&text);
    }
    let reported = std::fs::read_to_string(&report_file).unwrap_or_default();
    raw.push_str(&reported);

    let values = raw
        .lines()
        .filter_map(|line| line.strip_prefix("REPORT "))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();

    Completion {
        report: Report { values, raw },
        exit,
    }
}

/// Waits for the scenario to have had its say, rather than for a fixed
/// number of seconds.
///
/// Starting Chromium takes two seconds on one machine and forty on
/// another — a virtual desktop with no GPU spends most of a minute
/// falling back to software rendering before the first frame. A flat
/// sleep turns that into a suite that fails with every report empty,
/// which looks like a broken engine and is a broken stopwatch. So the
/// report file is watched instead: once it has stopped growing for
/// `quiet`, the scenario is done.
fn wait_for_quiet(report: &std::path::Path, patience: Duration, quiet: Duration) {
    let deadline = std::time::Instant::now() + patience;
    let mut lines = 0;
    let mut last_change = std::time::Instant::now();

    while std::time::Instant::now() < deadline {
        // The browser process writes its pid before the window is even
        // asked for, so that line is not counted: what is being waited
        // for is the page, which runs in the render process.
        let now = std::fs::read_to_string(report)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.starts_with("REPORT browser_pid="))
            .count();
        if now != lines {
            lines = now;
            last_change = std::time::Instant::now();
        } else if lines > 0 && last_change.elapsed() >= quiet {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Waits for the application to go of its own accord, killing it only if
/// it never does — a hang is a result worth reporting, not worth waiting
/// out.
fn wait_for_exit(child: &mut std::process::Child, patience: Duration) -> Option<i32> {
    let deadline = std::time::Instant::now() + patience;
    while std::time::Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.code().unwrap_or(-1)),
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break,
        }
    }
    kill(child);
    let _ = child.wait();
    None
}

/// The whole tree: a CEF application is a family of processes.
fn kill(child: &std::process::Child) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &child.id().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// How a scenario that is meant to end by itself actually ended.
pub struct Completion {
    pub report: Report,
    /// The process's exit code, or `None` if it had to be killed.
    pub exit: Option<i32>,
}

/// Runs a scenario that closes its own window, and waits for the process
/// to go. Unlike [`run_scenario`], the point here is the exit itself: an
/// application that hangs or crashes on the way out is a failure even if
/// everything it reported beforehand was right.
pub fn run_scenario_to_completion(name: &str) -> Completion {
    run(name, Ending::ByItself)
}
