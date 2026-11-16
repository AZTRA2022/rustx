//! Attention, on macOS: the Dock.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from `NSApplication` and `NSDockTile`'s
//! documentation; only type-checked on Windows against `objc2` built
//! for GNUstep. No Dock icon has bounced or worn a badge because of it.
//! Verify on a real Mac before trusting it.
//!
//! `flash` is `requestUserAttention:` with `NSCriticalRequest`, which
//! keeps bouncing until the application is activated — the Mac twin of
//! Windows' `FLASHW_TIMERNOFG`. It is the *application* that bounces:
//! the Dock has one icon per application, not per window, so the pane is
//! only checked, not used.
//!
//! # Refused
//!
//! `progress`: the Dock has no progress bar of its own. Applications
//! that show one draw it into a custom `NSDockTile` content view —
//! drawing code, not a call, and not something to write unseen.

use crate::error::OsError;
use crate::window::Pane;

/// `NSCriticalRequest`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const CRITICAL_REQUEST: usize = 0;

#[cfg(target_os = "macos")]
pub fn flash(pane: Pane) -> Result<(), OsError> {
    use objc2::msg_send;

    if !pane.exists() {
        return Err(OsError::Platform(
            "there is no window to command yet: the handle is still nothing".to_string(),
        ));
    }
    super::appkit::on_main("the Dock", || {
        if let Some(app) = super::appkit::application() {
            // SAFETY: a documented `NSApplication` call, on the main thread.
            let _: isize = unsafe { msg_send![&*app, requestUserAttention: CRITICAL_REQUEST] };
        }
    })
}

#[cfg(target_os = "macos")]
pub fn badge(count: Option<u32>) -> Result<(), OsError> {
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;

    super::appkit::on_main("the Dock badge", move || {
        let Some(app) = super::appkit::application() else {
            return;
        };
        let label = count.and_then(|count| super::appkit::ns_string(&count.to_string()));
        // SAFETY: documented `NSApplication` and `NSDockTile` calls, on the
        // main thread; a `nil` label removes the badge.
        unsafe {
            let tile: Option<Retained<AnyObject>> = msg_send![&*app, dockTile];
            if let Some(tile) = tile {
                let _: () = msg_send![&*tile, setBadgeLabel: label.as_deref()];
            }
        }
    })
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn flash(_pane: Pane) -> Result<(), OsError> {
    unreachable!("macos/attention.rs is never the platform module outside macOS")
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn badge(_count: Option<u32>) -> Result<(), OsError> {
    unreachable!("macos/attention.rs is never the platform module outside macOS")
}

pub fn progress(_pane: Pane, _done: Option<f64>) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "Dock progress on macOS (it is a custom NSDockTile view the application draws)",
    ))
}
