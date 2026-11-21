//! The updater against a release page served over loopback: a newer version is
//! found, the download is checked against the digest the page publishes, and a
//! download that does not match is refused and removed.

use std::io::{Read, Write};
use std::net::TcpListener;

use rustx_os::digest::sha256_hex;
use rustx_os::update;

fn serve(listener: TcpListener, archive: Vec<u8>, notes: String) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut head = [0_u8; 2048];
            let n = stream.read(&mut head).unwrap_or(0);
            let request = String::from_utf8_lossy(&head[..n]).to_string();
            let (kind, body): (&str, Vec<u8>) = if request.contains("/releases/latest") {
                let port = stream.local_addr().unwrap().port();
                let json = format!(
                    "{{\"tag_name\":\"v9.0.0\",\"body\":{notes:?},\"assets\":[{{\"name\":\"demo-windows-linux-macos.zip\",\"browser_download_url\":\"http://127.0.0.1:{port}/demo.zip\",\"size\":{}}}]}}",
                    archive.len()
                );
                ("application/json", json.into_bytes())
            } else {
                ("application/zip", archive.clone())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(&body);
        }
    });
}

#[test]
fn a_newer_release_over_loopback_is_found_checked_and_refused_when_wrong() {
    let archive = b"PK-not-really-a-zip-but-bytes".to_vec();
    let good = sha256_hex(&archive);

    for (digest, accepted) in [(good.clone(), true), ("0".repeat(64), false)] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        serve(
            listener,
            archive.clone(),
            format!("What changed.\nsha256 demo-windows-linux-macos.zip: {digest}"),
        );
        // SAFETY: the only test of this file, so nothing else reads the environment.
        unsafe { std::env::set_var("RUSTX_UPDATE_API", format!("http://127.0.0.1:{port}")) };

        let release = update::latest("owner/demo", "rustx-update-loopback")
            .expect("the page answers")
            .expect("it names an archive for this machine");
        assert!(release.is_newer_than("1.0.0"), "{release:?}");
        let fetched = update::fetch(&release, "rustx-update-loopback", &mut |_, _| {});
        match (accepted, fetched) {
            (true, Ok(path)) => {
                assert_eq!(std::fs::read(&path).unwrap(), archive);
                let _ = std::fs::remove_file(path);
            }
            (false, Err(why)) => {
                assert!(why.to_string().contains("does not match"), "{why}");
            }
            (accepted, other) => panic!("accepted={accepted}: {other:?}"),
        }
    }
}
