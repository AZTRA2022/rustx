//! The clipboard, on Linux.
//!
//! Not yet, and it says so. What this will be is X11's selections
//! (`XConvertSelection` on `CLIPBOARD`, with `UTF8_STRING`,
//! `image/png` and a project's own target atom) and, under Wayland,
//! `wl_data_device`. Both need a window and an event loop, which the
//! engine has and this crate does not yet reach — so the work is one
//! file, this one, and the API above it does not change.
//!
//! Refusing in a sentence that names the platform is deliberate: a
//! program that builds on Linux and silently copies nothing is worse
//! than one that says where it stands.

use crate::clipboard::Image;
use crate::error::OsError;

const WHAT: &str = "the clipboard";

pub fn text() -> Result<String, OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn set_text(_text: &str) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn image() -> Result<Image, OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn set_image(_image: &Image) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn custom(_format: &str) -> Result<Vec<u8>, OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn set_custom(_format: &str, _bytes: &[u8]) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

/// Whether this platform's clipboard is reachable from here. `false`
/// until the two selections above are written.
pub const AVAILABLE: bool = false;
