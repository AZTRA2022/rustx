//! The host in a real window, driven from outside.
//!
//! Each test starts the `probe` binary, which opens a webview on a folder
//! of files the test wrote, and reads the `REPORT key=value` lines it
//! prints. Needs a desktop session, like the CEF engine's own tests.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{folder, probe};

#[test]
fn the_applications_page_loads_from_its_own_origin_and_can_call_the_host() {
    let root = folder(
        "load",
        "<!doctype html><script>window.ipc.postMessage('origin=' + location.origin)</script>",
    );
    let profile = root.join("profile");
    let run = probe(&root, &["--data-dir", profile.to_str().expect("utf8")], &[]);
    let origin = rustx_webview::host::origin("rustx");
    assert_eq!(
        run.messages,
        vec![format!("origin={origin}")],
        "{}",
        run.stderr
    );
    assert!(run.done && run.success, "{}", run.stderr);

    // The profile is where it was told to be, and none was made next to
    // the executable, which fails under Program Files.
    // WKWebView keeps its data where the system says, not in a folder we name.
    #[cfg(not(target_os = "macos"))]
    assert!(profile.exists(), "no profile at {}", profile.display());
    let beside = format!("{}.WebView2", env!("CARGO_BIN_EXE_probe"));
    assert!(
        !Path::new(&beside).exists(),
        "a profile was made at {beside}"
    );
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
    let run = probe(&root, &["--seconds", "10"], &[]);
    assert!(
        run.messages.contains(&"still-here".to_string()),
        "left the page: {}",
        run.stderr
    );
    assert!(
        run.opened
            .contains(&"https://example.invalid/x?token=1".to_string()),
        "navigation: {:?} {}",
        run.opened,
        run.stderr
    );
    // WebKitGTK refuses a script popup with no user gesture before the host is
    // asked, so only WebView2 reports it here.
    #[cfg(windows)]
    assert!(
        run.opened
            .contains(&"https://example.invalid/new".to_string()),
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
    let run = probe(&root, &["--seconds", "10"], &[]);
    assert!(
        run.messages.contains(&"still-here".to_string()),
        "{}",
        run.stderr
    );
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
                "/" => {
                    "<!doctype html><script>
                          try { window.ipc.postMessage('from-remote'); } catch (e) {}
                          fetch('/ran');
                        </script>"
                }
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
    assert!(
        paths.contains(&"/".to_string()),
        "the allowed page never loaded: {paths:?} {}",
        run.stderr
    );
    assert!(
        paths.contains(&"/ran".to_string()),
        "the remote script did not run, so the test proves nothing: {paths:?}"
    );
    assert_eq!(
        run.messages,
        vec!["ready".to_string()],
        "the remote page reached the host"
    );
    assert!(run.stderr.contains("ignored a message"), "{}", run.stderr);
    drop(server);
    let _ = std::fs::remove_dir_all(root);
}

// The variable that hides the runtime is WebView2's.
#[cfg(windows)]
#[test]
fn a_missing_webview_runtime_is_an_error_and_not_a_crash() {
    let root = folder("noruntime", "<!doctype html><p>x</p>");
    let empty = root.join("no-runtime");
    std::fs::create_dir_all(&empty).expect("a folder");
    let run = probe(
        &root,
        &[],
        &[("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", &empty)],
    );
    assert!(!run.success, "started without a runtime");
    assert!(!run.done, "ran as if there were a window");
    assert!(
        run.stderr.contains("WebView2 Runtime"),
        "the message does not name the runtime: {:?}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_title_the_page_changes_becomes_the_window_title() {
    let root = folder(
        "pagetitle",
        "<!doctype html><title>First</title><script>
           window.ipc.postMessage('ready');
           setTimeout(() => { document.title = 'Second route'; }, 600);
           setTimeout(() => window.ipc.postMessage('late'), 2000);
         </script>",
    );
    let run = probe(
        &root,
        &["--seconds", "8"],
        &[("RUSTX_TRACE", Path::new("1"))],
    );
    assert!(
        run.stderr.contains("window title is now Second route"),
        "{}",
        run.stderr
    );
    // The title the page started with is not a change.
    assert!(
        !run.stderr.contains("window title is now First"),
        "{}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_second_window_is_ordered_over_the_pipe_from_the_first() {
    let root = folder("twowindows", "<!doctype html><p>x</p>");
    let run = probe(
        &root,
        &["--scenario", "twowindows", "--seconds", "9"],
        &[("RUSTX_TRACE", Path::new("1"))],
    );
    assert!(run.stderr.contains("PARENT sent=true"), "{}", run.stderr);
    // The second process received both orders over the pipe.
    assert!(
        run.stderr.contains(r#""order":"size","value":[520,410]"#)
            && run.stderr.contains(r#""order":"title","value":"Retitled""#),
        "{}",
        run.stderr
    );
    // The child was opened at 400x300 and asked for 520x410: it resized (physical pixels, so
    // only the fact is asserted).
    let sizes = run.stderr.matches("CHILD resized").count();
    // Without a window manager (Xvfb) no resize event comes back.
    assert!(
        cfg!(target_os = "linux") || sizes >= 1,
        "the second window never resized: {}",
        run.stderr
    );
    let _ = std::fs::remove_dir_all(root);
}
