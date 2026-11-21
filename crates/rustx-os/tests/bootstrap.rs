//! The runtime bootstrapper: a program is downloaded from a loopback server and
//! run, and its exit code is handed back. Windows only: that is the one system
//! where a runtime is installed this way.
#![cfg(windows)]

use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn the_bootstrapper_is_downloaded_and_run() {
    // Any small program stands in for Microsoft's installer; it ignores the flags or rejects them.
    let program = std::fs::read(r"C:\Windows\System32\whoami.exe").expect("whoami.exe");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut head = [0_u8; 1024];
            let _ = stream.read(&mut head);
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                program.len()
            );
            let _ = stream.write_all(&program);
        }
    });
    let url = format!("http://127.0.0.1:{port}/setup.exe");
    let code = rustx_os::bootstrap::run_installer(&url, "rustx-bootstrap-test")
        .expect("downloaded and started");
    // whoami does not know /silent: it ran and said so.
    assert_ne!(code, -1, "the program did not run to an exit code");
}
