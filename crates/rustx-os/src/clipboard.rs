//! The clipboard: text, an image, and a format a project invents.
//!
//! The web half of an application cannot have this. A page can read the
//! clipboard after the user presses Ctrl+V and nothing else, by design
//! — so copying a selection when a button is pressed, pasting on a
//! shortcut, or moving a project's own objects between two of its own
//! windows is work for this layer.
//!
//! # One at a time, and briefly
//!
//! The clipboard is one machine-wide object, opened by one program at a
//! time. Every function here opens it, does one thing and closes it:
//! holding it open across anything a developer writes would freeze the
//! copy and paste of every other program on the machine. And because
//! somebody else may be holding it at the moment we ask — a password
//! manager, an editor, the shell — the open is retried for 200 ms
//! before it refuses.

use crate::error::OsError;

/// An image, as the clipboard and the screen both keep one: four bytes
/// a pixel, blue first, rows top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// Whether this platform's clipboard is reachable from here.
///
/// Asked by a page *before* it offers the button, so that a feature
/// that cannot work is a button that was never drawn rather than an
/// error a user has to read. It is the platform's own answer — every
/// `<os>/clipboard.rs` states it — and not a list of platform names
/// kept up to date here.
pub fn supported() -> bool {
    crate::platform::clipboard::AVAILABLE
}

/// How the clipboard names itself in the warning a call made on the
/// thread that draws gets. The open is retried for 200 ms, so a
/// handler that asks while a password manager is holding it stops the
/// window for a fifth of a second — see [`crate::blocking`].
const WAITS: &str = "the clipboard";

/// What the clipboard holds as text, or an empty string when it holds
/// none.
pub fn text() -> Result<String, OsError> {
    crate::blocking::waiting(WAITS);
    crate::platform::clipboard::text()
}

/// Put text on the clipboard, replacing whatever was there.
pub fn set_text(text: &str) -> Result<(), OsError> {
    crate::blocking::waiting(WAITS);
    crate::platform::clipboard::set_text(text)
}

/// The image on the clipboard, if what is there is an image.
pub fn image() -> Result<Image, OsError> {
    crate::blocking::waiting(WAITS);
    crate::platform::clipboard::image()
}

/// Put an image on the clipboard, replacing whatever was there.
pub fn set_image(image: &Image) -> Result<(), OsError> {
    crate::blocking::waiting(WAITS);
    crate::platform::clipboard::set_image(image)
}

/// Bytes under a name a project invents, for its own objects.
pub fn set_custom(format: &str, bytes: &[u8]) -> Result<(), OsError> {
    crate::blocking::waiting(WAITS);
    crate::platform::clipboard::set_custom(format, bytes)
}

/// Read back what `set_custom` wrote under that name.
pub fn custom(format: &str) -> Result<Vec<u8>, OsError> {
    crate::blocking::waiting(WAITS);
    crate::platform::clipboard::custom(format)
}

/// How long a platform retries opening the clipboard, and how long it
/// waits between tries.
///
/// Here rather than in `<os>/clipboard.rs` because it is a decision
/// about what a person will put up with, not about an operating system:
/// 200 ms is below what anybody notices in a button and far above the
/// few milliseconds another program holds the clipboard for. 10 ms
/// between tries — twenty tries, and a thread that is asleep rather
/// than spinning.
pub const PATIENCE: std::time::Duration = std::time::Duration::from_millis(200);
pub const BETWEEN: std::time::Duration = std::time::Duration::from_millis(10);
