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
    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or_else(|| {
            panic!(
                "the page never reported '{key}'. Full output:\n{}",
                self.raw
            )
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }
}

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
