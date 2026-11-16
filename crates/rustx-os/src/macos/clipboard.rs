//! The clipboard, on macOS: `pbcopy` and `pbpaste`.
//!
//! # By the tools the system itself ships, and why
//!
//! Every macOS install carries `pbcopy` and `pbpaste` since Mac OS X
//! 10.0 — they are the command-line face of `NSPasteboard`, written by
//! Apple, and there is nothing to install and nothing that can be
//! missing the way `xclip` can be on Linux. What they cost: a process
//! per copy, same as the Linux side.
//!
//! # Only plain text
//!
//! Neither tool takes a pasteboard type to read or write: `pbcopy`
//! always writes `public.utf8-plain-text`, and `pbpaste` without
//! `-Prefer` reads the same. Reading or writing any other type —
//! including an image — needs `NSPasteboard` itself, which is
//! `AppKit`, which this crate does not bind. `image`, `custom` and
//! `set_custom` say so rather than silently handing back plain text
//! under another type's name.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! macOS. There is no Apple target installed here to even type-check
//! it against — unlike the Linux files, which `cargo check --target
//! x86_64-unknown-linux-gnu` does verify, this one has only been read
//! over by hand.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::clipboard::Image;
use crate::error::OsError;

pub const AVAILABLE: bool = true;

const ONLY_PLAIN_TEXT: &str = "a pasteboard type other than plain text (pbcopy/pbpaste only \
                                carry public.utf8-plain-text; NSPasteboard itself would be \
                                needed for anything else)";

fn missing(program: &str, error: &std::io::Error) -> OsError {
    OsError::Platform(format!(
        "`{program}` could not be run ({error}). It ships with macOS itself, so this \
         usually means something else is wrong with the environment."
    ))
}

pub fn text() -> Result<String, OsError> {
    let out = Command::new("pbpaste")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| missing("pbpaste", &error))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn set_text(text: &str) -> Result<(), OsError> {
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| missing("pbcopy", &error))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| OsError::Platform(format!("`pbcopy` refused the bytes: {error}")))?;
    }
    // Waited for the same reason as the Linux side: `pbcopy` must have
    // taken the pasteboard before this returns, or a read right after
    // would see what was there before.
    drop(child.stdin.take());
    child
        .wait()
        .map_err(|error| OsError::Platform(format!("`pbcopy` did not finish: {error}")))?;
    Ok(())
}

pub fn image() -> Result<Image, OsError> {
    Err(OsError::Unsupported(ONLY_PLAIN_TEXT))
}

pub fn set_image(_image: &Image) -> Result<(), OsError> {
    Err(OsError::Unsupported(ONLY_PLAIN_TEXT))
}

pub fn custom(_format: &str) -> Result<Vec<u8>, OsError> {
    Err(OsError::Unsupported(ONLY_PLAIN_TEXT))
}

pub fn set_custom(_format: &str, _bytes: &[u8]) -> Result<(), OsError> {
    Err(OsError::Unsupported(ONLY_PLAIN_TEXT))
}
