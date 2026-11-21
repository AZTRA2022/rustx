//! Driving the probe binary from outside: a folder of files, a window, and the
//! REPORT key=value lines it prints. Needs a desktop session.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Run {
    pub messages: Vec<String>,
    pub opened: Vec<String>,
    pub done: bool,
    /// Every other `REPORT key=value`, in order.
    pub reports: Vec<(String, String)>,
    pub stderr: String,
    pub success: bool,
}

pub fn folder(name: &str, index: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustx-host-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    std::fs::write(dir.join("index.html"), index).expect("a page");
    dir
}

pub fn probe(root: &Path, extra: &[&str], envs: &[(&str, &Path)]) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_probe"));
    command
        .arg("--root")
        .arg(root)
        .args(["--seconds", "5"])
        .args(extra)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("the probe starts");
    let stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let lines = std::thread::spawn(move || {
        BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
            .collect::<Vec<_>>()
    });
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("waits") {
            break Some(status);
        }
        if started.elapsed() > Duration::from_secs(45) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let mut run = Run {
        messages: Vec::new(),
        opened: Vec::new(),
        done: false,
        reports: Vec::new(),
        // The system's own chatter on a runner with no desktop (GTK's
        // accessibility bus, Mesa's DRI3) is not the application's.
        stderr: errors
            .join()
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.contains("dbind-WARNING") && !line.contains("libEGL warning"))
            .collect::<Vec<_>>()
            .join("
"),
        success: status.is_some_and(|status| status.success()),
    };
    for line in lines.join().unwrap_or_default() {
        match line
            .strip_prefix("REPORT ")
            .and_then(|rest| rest.split_once('='))
        {
            Some(("message", value)) => run.messages.push(value.to_string()),
            Some(("opened", value)) => run.opened.push(value.to_string()),
            Some(("done", _)) => run.done = true,
            Some((key, value)) => run.reports.push((key.to_string(), value.to_string())),
            _ => {}
        }
    }
    run
}
