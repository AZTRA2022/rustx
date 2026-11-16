use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use rustx_engine::{Bootstrap, Role, bootstrap, process};

mod common;

#[test]
fn parent_process_is_identified_as_browser() {
    // No sub-process argument is present on a test harness command line.
    assert!(matches!(bootstrap(), Bootstrap::Browser));
    assert_eq!(process::role(), Role::Browser);
}

#[test]
fn application_arguments_survive_bootstrap() {
    // The engine must not swallow the arguments the application was given,
    // and only the browser process may reach the application's own code.
    let run = run_hello(&["--my-option=42"]);

    assert!(
        run.output.contains("--my-option=42"),
        "the application never saw its own argument:\n{}",
        run.output
    );
    assert_eq!(
        run.output.matches("WINDOW_OPENED").count(),
        1,
        "every CEF sub-process ran the application body:\n{}",
        run.output
    );
}

#[test]
fn the_application_survives_startup_without_a_fatal_error() {
    // Every mistake this engine made during its first hours ended the same
    // way: a CEF FATAL and a process gone within a second, with nothing on
    // stderr to explain it. Both halves of that are worth pinning.
    //
    // Deliberately not asserted here: that no sub-process ever crashes. The
    // GPU process is Chromium's business and it dies on legacy drivers
    // (Intel HD 10.18.x, for one) whatever the host does; Chromium restarts
    // it and falls back to software rendering. Failing this test on the
    // developer's graphics card would say nothing about rustx.
    let run = run_hello(&[]);

    assert!(
        run.still_running,
        "the application died during startup:\n{}",
        run.output
    );
    assert!(
        !run.output.contains("FATAL"),
        "CEF reported a fatal error:\n{}",
        run.output
    );
}

/// Runs the `hello` binary with extra arguments, lets it settle, then kills
/// the whole process tree and returns everything it wrote to stdout and
/// stderr. CEF reports sub-process failures on stderr, so both matter.
fn run_hello(args: &[&str]) -> Run {
    // Une fenêtre à la fois sur cette machine. Chromium tient une
    // session par répertoire de cache : deux binaires de test lancés en
    // parallèle par `cargo test` se donnent leurs fenêtres l'un à
    // l'autre, et celui qui n'a pas démarré le premier échoue pour une
    // raison qui n'est pas dans son code.
    let _alone = common::alone();

    let mut child = Command::new(env!("CARGO_BIN_EXE_hello"))
        .args(args)
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

    std::thread::sleep(Duration::from_secs(8));
    let still_running = matches!(child.try_wait(), Ok(None));
    kill_tree(child.id());
    let _ = child.wait();

    // Each reader returns once every process holding its write end is gone;
    // CEF's sub-processes take a moment to follow their parent.
    let deadline = Duration::from_secs(10);
    let mut output = String::new();
    while let Ok(text) = receiver.recv_timeout(deadline) {
        output.push_str(&text);
    }

    Run {
        output,
        still_running,
    }
}

/// What one run of the application told us.
struct Run {
    /// Everything it wrote to stdout and stderr.
    output: String,
    /// Whether it was still alive when we came to shut it down.
    still_running: bool,
}

fn kill_tree(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
