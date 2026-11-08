//! The system tray icon, on macOS: not yet, and it says so.
//!
//! What this will be: an `NSStatusItem`, held by the status bar for
//! as long as the application keeps a reference to it — there is no
//! separate "show" call the way Windows' `Shell_NotifyIcon` has, the
//! item simply exists or does not. The menu is an `NSMenu` handed to
//! the item directly, which is simpler than the two other platforms'
//! own tray menu (no custom protocol, no `com.canonical.dbusmenu`),
//! but it is still `AppKit`, which this crate does not bind.

use std::path::Path;

use crate::error::OsError;
use crate::notify::What;
use crate::tray::Item;

const WHAT: &str = "the system tray icon";

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
