//! The host in a real window, driven from outside.
//!
//! Each test starts the `probe` binary, which opens a webview on a folder
//! of files the test wrote, and reads the `REPORT key=value` lines it
//! prints. Needs a desktop session, like the CEF engine's own tests.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Run {
    messages: Vec<String>,
    opened: Vec<String>,
    done: bool,
    stderr: String,
    success: bool,
}

fn folder(name: &str, index: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustx-host-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    std::fs::write(dir.join("index.html"), index).expect("a page");
    dir
}

fn probe(root: &Path, extra: &[&str], envs: &[(&str, &Path)]) -> Run {
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
        BufReader::new(stdout).lines().map_while(Result::ok).collect::<Vec<_>>()
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
        stderr: errors.join().unwrap_or_default(),
        success: status.is_some_and(|status| status.success()),
    };
    for line in lines.join().unwrap_or_default() {
        match line.strip_prefix("REPORT ").and_then(|rest| rest.split_once('=')) {
            Some(("message", value)) => run.messages.push(value.to_string()),
            Some(("opened", value)) => run.opened.push(value.to_string()),
            Some(("done", _)) => run.done = true,
            _ => {}
        }
    }
    run
}

#[test]
fn the_applications_page_loads_from_its_own_origin_and_can_call_the_host() {
    let root = folder(
        "load",
        "<!doctype html><script>window.ipc.postMessage('origin=' + location.origin)</script>",
    );
    let profile = root.join("profile");
    let run = probe(&root, &["--data-dir", profile.to_str().expect("utf8")], &[]);
    let origin = rustx_webview::host::origin("rustx");
    assert_eq!(run.messages, vec![format!("origin={origin}")], "{}", run.stderr);
    assert!(run.done && run.success, "{}", run.stderr);

    // The profile is where it was told to be, and none was made next to
    // the executable, which fails under Program Files.
    assert!(profile.exists(), "no profile at {}", profile.display());
    let beside = format!("{}.WebView2", env!("CARGO_BIN_EXE_probe"));
    assert!(!Path::new(&beside).exists(), "a profile was made at {beside}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_link_to_another_site_opens_in_the_system_browser_and_the_window_stays() {
    let root = folder(
        "external",
        "<!doctype html><script>
           window.ipc.postMessage('ready');
           setTimeout(() => { location.href = 'https://example.invalid/x?token=1'; }, 300);
           setTimeout(() => { window.open('https://example.invalid/new'); }, 700);
           setTimeout(() => window.ipc.postMessage('still-here'), 2500);
         </script>",
    );
    let run = probe(&root, &[], &[]);
    assert!(run.messages.contains(&"still-here".to_string()), "left the page: {}", run.stderr);
    assert!(
        run.opened.contains(&"https://example.invalid/x?token=1".to_string()),
        "navigation: {:?} {}",
        run.opened,
        run.stderr
    );
    assert!(
        run.opened.contains(&"https://example.invalid/new".to_string()),
        "window.open: {:?} {}",
        run.opened,
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_address_that_is_not_a_web_address_is_refused_and_nothing_opens() {
    let root = folder(
        "refused",
        "<!doctype html><script>
           setTimeout(() => { location.href = 'ftp://example.invalid/x'; }, 300);
           setTimeout(() => window.ipc.postMessage('still-here'), 2500);
         </script>",
    );
    let run = probe(&root, &[], &[]);
    assert!(run.messages.contains(&"still-here".to_string()), "{}", run.stderr);
    assert!(run.opened.is_empty(), "opened {:?}", run.opened);
    assert!(run.stderr.contains("refused to navigate"), "{}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_allowed_remote_page_loads_in_the_window_and_cannot_call_the_host() {
    // A web server on this machine stands for the remote site, so the test
    // needs no network.
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = listener.local_addr().expect("an address").port();
    listener.set_nonblocking(true).expect("non blocking");
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let log = Arc::clone(&seen);
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(40);
        while Instant::now() < until {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            };
            let _ = stream.set_nonblocking(false);
            let mut buffer = [0u8; 2048];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let head = String::from_utf8_lossy(&buffer[..read]).to_string();
            let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
            log.lock().expect("lock").push(path.clone());
            let body = match path.as_str() {
                "/" => "<!doctype html><script>
                          try { window.ipc.postMessage('from-remote'); } catch (e) {}
                          fetch('/ran');
                        </script>",
                _ => "",
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });

    let remote = format!("http://127.0.0.1:{port}");
    let root = folder(
        "remote",
        &format!(
            "<!doctype html><script>
               window.ipc.postMessage('ready');
               setTimeout(() => {{ location.href = '{remote}/'; }}, 300);
             </script>"
        ),
    );
    let run = probe(&root, &["--allow", &remote], &[]);
    let paths = seen.lock().expect("lock").clone();
    assert!(paths.contains(&"/".to_string()), "the allowed page never loaded: {paths:?} {}", run.stderr);
    assert!(
        paths.contains(&"/ran".to_string()),
        "the remote script did not run, so the test proves nothing: {paths:?}"
    );
    assert_eq!(run.messages, vec!["ready".to_string()], "the remote page reached the host");
    assert!(run.stderr.contains("ignored a message"), "{}", run.stderr);
    drop(server);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_missing_webview_runtime_is_an_error_and_not_a_crash() {
    let root = folder("noruntime", "<!doctype html><p>x</p>");
    let empty = root.join("no-runtime");
    std::fs::create_dir_all(&empty).expect("a folder");
    let run = probe(&root, &[], &[("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", &empty)]);
    assert!(!run.success, "started without a runtime");
    assert!(!run.done, "ran as if there were a window");
    assert!(run.stderr.contains("WebView2 Runtime"), "the message does not name the runtime: {:?}", run.stderr);
    let _ = std::fs::remove_dir_all(root);
}
