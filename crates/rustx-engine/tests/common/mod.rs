//! Driving the `hello` application from the outside.
//!
//! The application runs a named scenario inside the page when it is given
//! `--scenario=<name>`, and reports what it found through a native function
//! that prints `REPORT <key>=<value>` on stdout — from the render process,
//! which is itself part of what is being proved.

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
pub fn run_scenario(name: &str) -> Report {
    let report_file = std::env::temp_dir().join(format!("rustx-report-{name}.txt"));
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

    std::thread::sleep(Duration::from_secs(10));
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
    let reported = std::fs::read_to_string(&report_file).unwrap_or_default();
    raw.push_str(&reported);

    let values = raw
        .lines()
        .filter_map(|line| line.strip_prefix("REPORT "))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();

    Report { values, raw }
}
