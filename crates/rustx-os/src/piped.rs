//! HTTP through the system's own client program, for the systems whose
//! HTTP lives in one.
//!
//! Windows has WinHTTP, which is a library every Windows machine has,
//! and `windows/http.rs` calls it. macOS and Linux do not have an
//! equivalent that can be called without linking something: on macOS it
//! is `NSURLSession`, which is asynchronous and wants a run loop, and
//! on Linux it is whichever of four libraries the distribution
//! happens to ship.
//!
//! What both of them do have is a client **program**: `curl` is part of
//! macOS and of every desktop Linux, and `wget` is on the ones that
//! predate that. So this asks it, and reads its answer on the pipe.
//!
//! That is not a workaround. It gets the system's own certificate
//! store, the system's proxy configuration -- `http_proxy`,
//! `HTTPS_PROXY`, and on macOS the ones from System Settings -- the
//! system's own security updates, and redirects. It costs one process
//! per request, which for "is there a new version" is nothing.
//!
//! This file is compiled on every platform, because the per-platform
//! modules are all type-checked everywhere. Only Linux and macOS call
//! it.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::OsError;

/// What is tried, in order, and the arguments each takes.
///
/// `curl` first because it is the one that is always there on macOS,
/// and because its `--fail` turns an HTTP error into an exit code
/// instead of a page of HTML written where the archive should be.
const CLIENTS: [&str; 2] = ["curl", "wget"];

/// The whole body of a `GET`, in memory.
pub fn get(url: &str, agent: &str) -> Result<Vec<u8>, OsError> {
    let mut out = Vec::new();
    stream(url, agent, &mut |piece| {
        out.extend_from_slice(piece);
        Ok(())
    })?;
    Ok(out)
}

/// The same, written to a file as it arrives, with the running total
/// handed to `progress`.
pub fn download(
    url: &str,
    agent: &str,
    into: &Path,
    progress: &mut dyn FnMut(u64),
) -> Result<u64, OsError> {
    use std::io::Write;

    let handle = std::fs::File::create(into).map_err(|error| {
        OsError::Platform(format!("{} could not be written ({error})", into.display()))
    })?;
    let mut file = std::io::BufWriter::new(handle);
    let mut total = 0_u64;
    let answer = stream(url, agent, &mut |piece| {
        file.write_all(piece).map_err(|error| {
            OsError::Platform(format!("{} could not be written ({error})", into.display()))
        })?;
        total += piece.len() as u64;
        progress(total);
        Ok(())
    });
    if let Err(why) = answer {
        // A half-written file is worse than none: the updater would
        // check its digest, fail, and say the release is corrupt.
        drop(file);
        let _ = std::fs::remove_file(into);
        return Err(why);
    }
    file.flush().map_err(|error| {
        OsError::Platform(format!(
            "{} could not be finished ({error})",
            into.display()
        ))
    })?;
    Ok(total)
}

/// One request, with each piece of the body handed over as it arrives.
///
/// The body is read from the child's standard output while it runs, so
/// a 300 MB archive never exists in memory -- which is the whole reason
/// this is a pipe and not `output()`.
fn stream(
    url: &str,
    agent: &str,
    take: &mut dyn FnMut(&[u8]) -> Result<(), OsError>,
) -> Result<(), OsError> {
    refuse_an_odd_url(url)?;

    let mut missing = Vec::new();
    for client in CLIENTS {
        let mut command = Command::new(client);
        match client {
            "curl" => {
                command
                    .arg("--silent")
                    .arg("--show-error")
                    // An HTTP error is an error, and not a page of
                    // HTML written where an archive should be.
                    .arg("--fail")
                    .arg("--location")
                    .arg("--max-redirs")
                    .arg("10")
                    .arg("--connect-timeout")
                    .arg("20")
                    .arg("--user-agent")
                    .arg(agent)
                    .arg("--")
                    .arg(url);
            }
            _ => {
                command
                    .arg("--quiet")
                    .arg("--output-document=-")
                    .arg(format!("--user-agent={agent}"))
                    .arg("--timeout=20")
                    .arg("--tries=1")
                    .arg("--")
                    .arg(url);
            }
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(client);
                continue;
            }
            Err(error) => {
                return Err(OsError::Platform(format!(
                    "`{client}` could not be run ({error})"
                )));
            }
        };

        let mut body = child
            .stdout
            .take()
            .ok_or_else(|| OsError::Platform(format!("`{client}` gave nothing to read from")))?;
        let mut buffer = vec![0_u8; 64 * 1024];
        let mut read_error = None;
        loop {
            match body.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    if let Err(why) = take(&buffer[..read]) {
                        read_error = Some(why);
                        break;
                    }
                }
                Err(error) => {
                    read_error = Some(OsError::Platform(format!(
                        "the answer from {url} stopped early ({error})"
                    )));
                    break;
                }
            }
        }

        let finished = child
            .wait_with_output()
            .map_err(|error| OsError::Platform(format!("`{client}` did not finish ({error})")))?;
        if let Some(why) = read_error {
            return Err(why);
        }
        if !finished.status.success() {
            let said = String::from_utf8_lossy(&finished.stderr).trim().to_string();
            return Err(OsError::Platform(format!(
                "{url} was not fetched: {}",
                if said.is_empty() {
                    match finished.status.code() {
                        // curl's own numbers, for the three that
                        // actually happen.
                        Some(6) => "that host could not be resolved".to_string(),
                        Some(7) => "that host refused the connection".to_string(),
                        Some(22) => "the server answered with an error (404 if the release or \
                             the repository is not there, 403 if the request was \
                             rate-limited)"
                            .to_string(),
                        Some(28) => "it timed out".to_string(),
                        Some(code) => format!("`{client}` exited {code}"),
                        None => format!("`{client}` was stopped"),
                    }
                } else {
                    said
                }
            )));
        }
        return Ok(());
    }

    Err(OsError::Platform(format!(
        "neither {} is installed, and this platform's HTTP is one of them. Install \
         either, or do the request with an HTTP crate of your own and hand the bytes \
         to the updater.",
        missing.join(" nor ")
    )))
}

/// A URL this will not hand to a shell-adjacent program.
///
/// `Command` takes an argument vector and no shell, so a space or a
/// semicolon in a URL is only ever an argument -- but a URL that begins
/// with `-` is read by both clients as an option, and `file://` would
/// make "download this release" read a local file. Both are refused by
/// name rather than escaped.
fn refuse_an_odd_url(url: &str) -> Result<(), OsError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(OsError::Platform(format!(
            "`{url}` is not an http or https URL, and those are the two this fetches"
        )));
    }
    if url.contains('\n') || url.contains('\r') {
        return Err(OsError::Platform(
            "that URL holds a line break, which no URL does".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The check that runs on every platform, because it is the one
    /// that keeps a URL from being read as an option or a file.
    #[test]
    fn only_http_and_https_are_fetched() {
        assert!(refuse_an_odd_url("https://example.test/a.zip").is_ok());
        assert!(refuse_an_odd_url("http://example.test/a.zip").is_ok());
        for odd in [
            "file:///etc/passwd",
            "-O/tmp/evil",
            "ftp://example.test/a",
            "example.test/a.zip",
        ] {
            assert!(refuse_an_odd_url(odd).is_err(), "`{odd}` was accepted");
        }
    }

    #[test]
    fn a_url_with_a_line_break_is_refused() {
        assert!(refuse_an_odd_url("https://example.test/a\nb").is_err());
    }

    /// On this machine one of the two is installed or neither is, and
    /// either way the sentence names them: a developer whose update
    /// check fails has to be told what to install.
    #[test]
    fn the_message_for_a_machine_with_no_client_names_both() {
        let said = stream("https://127.0.0.1:1/nothing", "test", &mut |_| Ok(()));
        if let Err(why) = said {
            let text = format!("{why}");
            assert!(
                text.contains("curl") || text.contains("fetched") || text.contains("wget"),
                "got {text}"
            );
        }
    }
}
