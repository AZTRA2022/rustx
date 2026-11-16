//! Menus: the one that opens where the pointer is, and the one macOS
//! puts at the top of the screen.
//!
//! The documentation's table said "application menu, context menu:
//! not written", against Electron's `Menu` and Tauri's menu API. These
//! are the two halves of that row, and they are not the same kind of
//! thing:
//!
//! - [`popup`] is a context menu. It opens at a point, it blocks until
//!   the user picks a line or clicks away, and it gives back the
//!   `id` of what was picked. That is a thing the system draws, and
//!   this draws it: `TrackPopupMenu` on Windows, `NSMenu` on macOS.
//! - [`application`] is the menu bar. It is a macOS idea: there, the
//!   menu belongs to the *application* and sits at the top of the
//!   screen, and without one `Cmd+Q`, `Cmd+C` and `Cmd+W` do nothing at
//!   all -- which is why it takes no items and installs the standard
//!   set. On Windows and Linux a menu bar belongs to the window, and
//!   the window here is Chromium's: its message loop, not ours. Draw
//!   that bar in the page, where the rest of this framework's chrome is
//!   already drawn.
//!
//! # A menu drawn in the page is often the better answer
//!
//! A `TrackPopupMenu` over a page is a grey Windows menu over whatever
//! the page looks like, and it will not match it. A right-click handler
//! that shows an absolutely positioned `<div>` matches, animates, works
//! the same on all three platforms, and needs nothing from this module.
//! Use [`popup`] when you want the system's menu *because* it is the
//! system's -- a tray-like menu, or a menu a screen reader should read
//! as one.
//!
//! ```ignore
//! let items = [
//!     rustx::menu::Item::new("copy", "Copy"),
//!     rustx::menu::Item::new("paste", "Paste").checked(false),
//!     rustx::menu::Item::separator(),
//!     rustx::menu::Item::new("about", "About"),
//! ];
//! if let Some(chosen) = rustx::menu::popup(pane, &items, None)? {
//!     // `chosen` is one of the ids above.
//! }
//! ```

use crate::error::OsError;
use crate::window::Pane;

/// One line of a menu -- the same type the tray's menu uses, because it
/// is the same list: an id, a label, and whether it carries a tick.
/// [`Item::separator`] is the dividing line.
pub use crate::tray::Item;

/// Whether this platform draws a menu of the system's own.
pub fn supported() -> bool {
    crate::platform::menu::supported()
}

/// Open a context menu and wait for the answer.
///
/// `at` is in screen coordinates; `None` means where the pointer is,
/// which is what a right-click wants. The answer is the `id` of the
/// line that was chosen, or `None` when the user clicked away -- which
/// is not an error and is the common case.
///
/// It blocks. The system runs its own loop while a menu is open, so the
/// window keeps drawing, but the line after this one does not run until
/// the menu is gone. Call it from the thread that owns the window.
pub fn popup(
    pane: Pane,
    items: &[Item],
    at: Option<(i32, i32)>,
) -> Result<Option<String>, OsError> {
    checked(items)?;
    let at = match at {
        Some(at) => at,
        None => crate::window::pointer()?,
    };
    crate::platform::menu::popup(pane, items, at)
}

/// What a menu cannot be.
fn checked(items: &[Item]) -> Result<(), OsError> {
    if items.iter().all(|item| item.label.is_empty()) {
        return Err(OsError::Platform(
            "a menu of nothing but separators, or of nothing at all, has no line to \
             choose: it would open and close again with no answer"
                .to_string(),
        ));
    }
    if let Some(odd) = items
        .iter()
        .find(|item| !item.label.is_empty() && item.id.is_empty())
    {
        return Err(OsError::Platform(format!(
            "the line `{}` has no id, so choosing it could not be told apart from \
             clicking away. `Item::separator()` is how a dividing line is written.",
            odd.label
        )));
    }
    Ok(())
}

/// Give the application the menu bar macOS requires.
///
/// The standard one, and no choice of items, because the items that
/// matter are not the application's: `Quit`, `Hide`, `Cut`, `Copy`,
/// `Paste`, `Select All`, `Minimise` and `Close` are AppKit's own
/// actions, and what this buys is that their keyboard shortcuts work.
/// Without a menu bar macOS has nowhere to send `Cmd+C`, and a text
/// field in the page cannot be copied out of.
///
/// Call it once, at start, from the thread that runs the window. On
/// Windows and Linux it does nothing and says so by returning `Ok`:
/// there is nothing of the sort to install, and a caller that had to
/// test the platform first would be writing this module's `cfg` for it.
pub fn application() -> Result<(), OsError> {
    crate::platform::menu::application()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_menu_with_no_line_to_choose_is_refused() {
        assert!(checked(&[]).is_err());
        assert!(checked(&[Item::separator(), Item::separator()]).is_err());
    }

    #[test]
    fn a_line_without_an_id_is_refused_and_the_message_names_it() {
        let mut item = Item::new("", "Copy");
        item.id = String::new();
        let why = format!("{}", checked(&[item]).unwrap_err());
        assert!(why.contains("Copy"), "{why}");
    }

    #[test]
    fn a_separator_beside_a_real_line_is_fine() {
        assert!(checked(&[Item::new("a", "A"), Item::separator()]).is_ok());
    }
}
