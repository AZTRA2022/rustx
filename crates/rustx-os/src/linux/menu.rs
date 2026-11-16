//! Refused, with the reason, because the reason is the answer.
//!
//! A menu on Linux is drawn by a toolkit: GTK's `GtkMenu` or Qt's
//! `QMenu`. There is no menu in X11 and none in Wayland -- a popup
//! there is a window a toolkit draws, decorates, keeps the keyboard
//! for, and closes again. Linking GTK to draw one grey rectangle would
//! add a dependency on the whole of GTK, a second event loop beside
//! Chromium's, and a look that matches neither the page nor the desktop
//! on a machine running KDE.
//!
//! So: draw it in the page. A right-click handler and a positioned
//! `<div>` is a menu that matches the application, works the same on
//! all three platforms, and is already what this framework does for the
//! title bar. [`crate::menu`] says the same thing at more length.

use crate::error::OsError;
use crate::tray::Item;
use crate::window::Pane;

pub fn supported() -> bool {
    false
}

pub fn popup(_pane: Pane, _items: &[Item], _at: (i32, i32)) -> Result<Option<String>, OsError> {
    Err(OsError::Unsupported(
        "a menu of the system's own on Linux (there is none: X11 and Wayland have no \
         menu, a toolkit draws one. Draw it in the page -- a right-click handler and a \
         positioned element)",
    ))
}

/// Nothing to install: a menu bar on Linux belongs to a window, and
/// this one belongs to Chromium. See [`crate::menu::application`].
pub fn application() -> Result<(), OsError> {
    Ok(())
}
