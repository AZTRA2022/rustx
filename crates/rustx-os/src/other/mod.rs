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

pub mod notify {
    use crate::error::OsError;
    use crate::notify::What;

    pub const AVAILABLE: bool = false;

    pub fn show(_what: &What) -> Result<(), OsError> {
        Err(OsError::Unsupported("les notifications"))
    }
}

pub mod shortcut {
    use crate::error::OsError;

    pub const AVAILABLE: bool = false;

    pub struct Shortcut;

    impl Shortcut {
        pub fn listen(_modifiers: u32, _key: u32, _keys: &str) -> Result<Self, OsError> {
            Err(OsError::Unsupported("les raccourcis globaux"))
        }

        pub fn pressed(&self) -> u64 {
            0
        }
    }
}

pub mod tray {
    use std::path::Path;

    use crate::error::OsError;
    use crate::notify::What;
    use crate::tray::Item;

    const WHAT: &str = "l'icône de barre système";

    pub const AVAILABLE: bool = false;

    pub fn shown() -> usize {
        0
    }

    pub struct Icon;

    impl Icon {
        pub fn new(_icon: &Path, _tooltip: &str) -> Result<Self, OsError> {
            Err(OsError::Unsupported(WHAT))
        }

        pub fn hidden() -> Result<Self, OsError> {
            Err(OsError::Unsupported(WHAT))
        }

        pub fn menu(&self, _items: &[Item]) -> Result<(), OsError> {
            Err(OsError::Unsupported(WHAT))
        }

        pub fn clicked(&self) -> Option<String> {
            None
        }

        pub fn notify(&self, _what: &What) -> Result<(), OsError> {
            Err(OsError::Unsupported(WHAT))
        }
    }
}

pub mod dialog {
    use std::path::PathBuf;

    use crate::dialog::{Ask, DialogError};

    const WHAT: &str = "le dialogue de fichiers";

    pub const AVAILABLE: bool = false;

    pub fn open(_ask: &Ask) -> Result<Vec<PathBuf>, DialogError> {
        Err(DialogError::Unsupported(WHAT))
    }

    pub fn save(_ask: &Ask) -> Result<PathBuf, DialogError> {
        Err(DialogError::Unsupported(WHAT))
    }

    pub fn folder(_ask: &Ask) -> Result<PathBuf, DialogError> {
        Err(DialogError::Unsupported(WHAT))
    }
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
