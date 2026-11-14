//! The About panel, on macOS: `orderFrontStandardAboutPanel:`.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from `NSApplication`'s documentation;
//! only type-checked on Windows against `objc2` built for GNUstep. No
//! panel has ever opened from it. Verify on a real Mac.
//!
//! This is the panel every Mac application's first menu item opens, and
//! the reason this capability exists at all: a Mac user expects it, and
//! expects it to look like everyone else's.
//!
//! # The keys, and the one that is a guess
//!
//! The panel reads a dictionary of option keys. `ApplicationName` and
//! `Version` are plain strings and documented as such. `Copyright` is
//! documented as taking an *attributed* string; a plain one is widely
//! reported to work and is what is sent here, which is the one thing in
//! this file a real Mac might contradict — if it does, the copyright
//! line is the part that will be missing, and the panel will still open.
//!
//! The panel belongs to the application and not to a window, so `pane`
//! is accepted for the facade's sake and deliberately unused.

use crate::about::Credits;
use crate::error::OsError;
use crate::window::Pane;

pub const AVAILABLE: bool = true;

#[cfg(target_os = "macos")]
pub fn show(_pane: Pane, credits: &Credits) -> Result<(), OsError> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    use super::appkit::{application, ns_string, on_main};

    let name = credits.name.clone();
    let version = credits.version.clone();
    let copyright = credits.copyright.clone();
    on_main("the About panel", move || {
        let Some(app) = application() else {
            return;
        };
        // SAFETY: documented `NSMutableDictionary` and `NSApplication`
        // calls, on the main thread. The dictionary retains every value
        // put into it, so the locals may drop after this block.
        unsafe {
            let options: Option<Retained<AnyObject>> =
                msg_send![class!(NSMutableDictionary), dictionary];
            let Some(options) = options else { return };
            let mut put = |key: &str, value: &str| {
                if value.trim().is_empty() {
                    return;
                }
                if let (Some(key), Some(value)) = (ns_string(key), ns_string(value)) {
                    let _: () = msg_send![&*options, setObject: &*value, forKey: &*key];
                }
            };
            put("ApplicationName", &name);
            put("Version", &version);
            put("Copyright", &copyright);
            let _: () = msg_send![&*app, orderFrontStandardAboutPanelWithOptions: &*options];
        }
    })
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn show(_pane: Pane, _credits: &Credits) -> Result<(), OsError> {
    unreachable!("macos/about.rs is never the platform module outside macOS")
}
