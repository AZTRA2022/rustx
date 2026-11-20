//! A window for the tests to drive from outside.
//!
//! Serves a folder, prints what the host sees as `REPORT key=value` lines
//! on stdout, and closes itself after `--seconds`. A test cannot run tao's
//! loop in its own thread, since on Windows and macOS it has to be the main
//! thread, so the loop lives in this process and the test reads its report.
//!
//! `--root DIR` (required), `--seconds N` (default 6), `--allow ORIGIN`
//! (repeatable), `--data-dir DIR`. External links are not opened: they are
//! reported as `opened=<url>`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::time::Duration;

use rustx_webview::host::{Config, Hooks, run};

fn report(key: &str, value: &str) {
    println!("REPORT {key}={}", value.replace(['\n', '\r'], " "));
}

fn main() {
    let mut root = None;
    let mut seconds = 6u64;
    let mut allow = Vec::new();
    let mut data_dir = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || arguments.next().unwrap_or_default();
        match argument.as_str() {
            "--root" => root = Some(PathBuf::from(value())),
            "--seconds" => seconds = value().parse().unwrap_or(seconds),
            "--allow" => allow.push(value()),
            "--data-dir" => data_dir = Some(PathBuf::from(value())),
            other => {
                eprintln!("probe: unknown argument `{other}`");
                std::process::exit(2);
            }
        }
    }
    let Some(root) = root else {
        eprintln!("probe: --root is required");
        std::process::exit(2);
    };

    let mut config = Config::new("probe", root);
    config.allow = allow;
    if data_dir.is_some() {
        config.data_dir = data_dir;
    }
    config.devtools = false;

    let hooks = Hooks {
        // The loop ends when a timer says so, through a proxy that may
        // cross threads.
        on_start: Box::new(move |control| {
            let control = control.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(seconds));
                report("done", "1");
                control.exit();
            });
        }),
        on_message: Box::new(|message, _| report("message", message)),
        open_external: Box::new(|url| report("opened", url)),
    };

    // `run` returns only an error: `Ok` holds an `Infallible`.
    let Err(error) = run(config, hooks);
    eprintln!("probe: {error}");
    std::process::exit(1);
}
