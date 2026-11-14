//! Screens, on macOS: `NSScreen`'s own list.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from `NSScreen`'s documentation; only
//! type-checked on Windows against `objc2` built for GNUstep. No screen
//! has been listed by it. Verify on a real Mac.
//!
//! # The y axis, and why it is flipped here
//!
//! `NSScreen` works in Cocoa's coordinates: the origin is the
//! *bottom* left of the primary screen and y grows upwards. The rest of
//! this crate — and `window::pointer`, and Windows and X11 — uses a
//! top-left origin with y downwards. So every rectangle is flipped
//! against the primary screen's height on the way out, which is also
//! what `CGDisplayBounds` would have given directly. Getting this wrong
//! puts a window on the wrong monitor, so it has a test of its own.

use crate::error::OsError;
use crate::screen::{Area, Screen};

/// A rectangle as `NSScreen` gives it, flipped into this crate's
/// top-left-origin coordinates.
///
/// `primary_height` is the first screen's height — `NSScreen.screens`
/// documents its first element as the one holding the menu bar, which
/// is the origin of Cocoa's global space.
pub(crate) fn flipped(x: f64, y: f64, width: f64, height: f64, primary_height: f64) -> Area {
    Area {
        x: x.round() as i32,
        y: (primary_height - y - height).round() as i32,
        width: width.round() as i32,
        height: height.round() as i32,
    }
}

#[cfg(target_os = "macos")]
pub fn screens() -> Result<Vec<Screen>, OsError> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    use super::appkit::{Rect, on_main};

    on_main("the list of screens", || {
        // SAFETY: documented `NSScreen` and `NSArray` calls, on the main
        // thread.
        unsafe {
            let all: Option<Retained<AnyObject>> = msg_send![class!(NSScreen), screens];
            let all = all?;
            let count: usize = msg_send![&*all, count];
            if count == 0 {
                return None;
            }
            let at = |index: usize| -> Option<Retained<AnyObject>> {
                msg_send![&*all, objectAtIndex: index]
            };
            // The first screen carries the menu bar, and its height is the
            // height Cocoa's y axis is measured from.
            let first = at(0)?;
            let primary_frame: Rect = msg_send![&*first, frame];
            let primary_height = primary_frame.size.height;

            let mut found = Vec::with_capacity(count);
            for index in 0..count {
                let Some(screen) = at(index) else { continue };
                let frame: Rect = msg_send![&*screen, frame];
                let visible: Rect = msg_send![&*screen, visibleFrame];
                let scale: f64 = msg_send![&*screen, backingScaleFactor];
                // `localizedName` is 10.15 and later; the number is what
                // there is to fall back on.
                let name: Option<Retained<AnyObject>> = msg_send![&*screen, localizedName];
                let name = name
                    .and_then(|name| {
                        let utf8: *const std::ffi::c_char = msg_send![&*name, UTF8String];
                        (!utf8.is_null()).then(|| {
                            std::ffi::CStr::from_ptr(utf8)
                                .to_string_lossy()
                                .into_owned()
                        })
                    })
                    .unwrap_or_else(|| format!("screen {}", index + 1));
                found.push(Screen {
                    name,
                    bounds: flipped(
                        frame.origin.x,
                        frame.origin.y,
                        frame.size.width,
                        frame.size.height,
                        primary_height,
                    ),
                    work: flipped(
                        visible.origin.x,
                        visible.origin.y,
                        visible.size.width,
                        visible.size.height,
                        primary_height,
                    ),
                    scale,
                    primary: index == 0,
                });
            }
            Some(found)
        }
    })?
    .ok_or_else(|| OsError::Platform("AppKit reported no screen".to_string()))
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn screens() -> Result<Vec<Screen>, OsError> {
    unreachable!("macos/screen.rs is never the platform module outside macOS")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_primary_screen_keeps_its_origin() {
        // A 1440-high primary screen: Cocoa's (0, 0) is its bottom left,
        // and the whole screen starts at the top left in ours.
        assert_eq!(
            flipped(0.0, 0.0, 2560.0, 1440.0, 1440.0),
            Area {
                x: 0,
                y: 0,
                width: 2560,
                height: 1440
            }
        );
    }

    #[test]
    fn the_menu_bar_is_taken_off_the_top_not_the_bottom() {
        // `visibleFrame` of the primary screen: 25 points shorter, and in
        // Cocoa that shows as a y of 25 at the *bottom*… which must come
        // out as a y of 0 at the top plus a shorter height.
        assert_eq!(
            flipped(0.0, 25.0, 2560.0, 1415.0, 1440.0),
            Area {
                x: 0,
                y: 0,
                width: 2560,
                height: 1415
            }
        );
    }

    #[test]
    fn a_shorter_screen_above_the_primary_gets_a_negative_y() {
        // A 1080-high monitor whose bottom edge sits at the primary's top
        // (Cocoa y = 1440): in our coordinates it is entirely above zero.
        assert_eq!(
            flipped(0.0, 1440.0, 1920.0, 1080.0, 1440.0),
            Area {
                x: 0,
                y: -1080,
                width: 1920,
                height: 1080
            }
        );
    }
}
