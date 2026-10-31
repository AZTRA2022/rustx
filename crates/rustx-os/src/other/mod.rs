//! Every platform that is neither Windows nor Linux.
//!
//! macOS above all, which will get a directory of its own the day
//! somebody writes it: `AppKit`'s `NSPasteboard` for the clipboard and
//! `AVFoundation` for the devices. Until then this crate compiles there
//! and refuses in a sentence that names the platform, because a program
//! that builds everywhere and silently does nothing on two of the three
//! is worse than one that says where it stands.

pub mod clipboard {
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

    pub const AVAILABLE: bool = false;
}

pub mod devices {
    use crate::devices::{Device, Kind};

    pub fn explain() {
        println!(
            "the device layer is Windows for now; this is {}",
            std::env::consts::OS
        );
    }

    pub fn devices_of(_kind: Kind) -> Vec<Device> {
        Vec::new()
    }
}
