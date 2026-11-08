//! Global shortcuts, on macOS: not yet, and it says so.
//!
//! What this will be: `RegisterEventHotKey` (Carbon, still present and
//! still the way almost every macOS utility grabs a system-wide
//! hotkey, `AppKit`'s "local-only" monitors notwithstanding) or an
//! `NSEvent` global monitor for the combinations that do not need to
//! be registered ahead of time. Both need accessibility permission
//! from the user — a system dialog this crate cannot shell out to
//! `osascript` for, because granting it is a checkbox in System
//! Settings a script cannot click on the user's behalf.
//!
//! The combination's own reading (`shortcut::parse`) is already
//! shared: the key codes are Windows' virtual-key codes, and this is
//! where a table would translate them to macOS's own.

use crate::error::OsError;

pub const AVAILABLE: bool = false;

const WHAT: &str = "global shortcuts";

pub struct Shortcut;

impl Shortcut {
    pub fn listen(_modifiers: u32, _key: u32, _keys: &str) -> Result<Self, OsError> {
        Err(OsError::Unsupported(WHAT))
    }

    pub fn pressed(&self) -> u64 {
        0
    }
}
