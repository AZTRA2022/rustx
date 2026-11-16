//! `TrackPopupMenu`, with `TPM_RETURNCMD`.
//!
//! The tray's menu in `tray.rs` builds the same `HMENU` and lets
//! Windows post a `WM_COMMAND` to the hidden window it owns. This one
//! cannot: the window a page is drawn in belongs to Chromium, and its
//! message loop is not ours to add a case to. `TPM_RETURNCMD` is the
//! documented way out -- Windows returns the identifier of the chosen
//! line from the call itself and posts nothing.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    SetForegroundWindow, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TPM_TOPALIGN,
    TrackPopupMenu,
};
use windows::core::PCWSTR;

use crate::error::OsError;
use crate::tray::Item;
use crate::window::Pane;

pub fn supported() -> bool {
    true
}

fn wide(text: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn popup(pane: Pane, items: &[Item], at: (i32, i32)) -> Result<Option<String>, OsError> {
    let window = HWND(pane.0 as *mut std::ffi::c_void);
    if !pane.exists() {
        return Err(OsError::Platform(
            "that window is gone, and a menu needs one to belong to: Windows closes a \
             popup when its owner is destroyed"
                .to_string(),
        ));
    }

    // SAFETY: a menu handle this function owns and destroys below.
    let menu = unsafe { CreatePopupMenu() }
        .map_err(|error| OsError::Platform(format!("the menu could not be made ({error})")))?;

    // The identifiers Windows carries are numbers, and zero is what
    // `TPM_RETURNCMD` returns for "nothing was chosen" -- so they start
    // at one and the number is the index.
    for (at, item) in items.iter().enumerate() {
        let number = at + 1;
        // SAFETY: the menu is live, and the label lives across the call.
        unsafe {
            if item.label.is_empty() {
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
                continue;
            }
            let label = wide(&item.label);
            let mut flags = MF_STRING;
            if let Some(ticked) = item.checked {
                flags |= if ticked { MF_CHECKED } else { MF_UNCHECKED };
            }
            let _ = AppendMenuW(menu, flags, number, PCWSTR(label.as_ptr()));
        }
    }

    // SAFETY: both take a live window; the menu is destroyed whatever
    // the answer is.
    let chosen = unsafe {
        // Windows closes a popup only while its owner is in front. A
        // menu opened from a window that is not would stay on screen
        // after the user clicked elsewhere.
        let _ = SetForegroundWindow(window);
        let chosen = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RETURNCMD | TPM_RIGHTBUTTON,
            at.0,
            at.1,
            Some(0),
            window,
            None,
        );
        let _ = DestroyMenu(menu);
        chosen
    };

    let number = chosen.0;
    if number <= 0 {
        // Zero is "clicked away", and so is a failure: Windows does not
        // distinguish them without `GetLastError`, and neither does a
        // caller who has nothing to do in either case.
        return Ok(None);
    }
    Ok(items.get((number - 1) as usize).map(|item| item.id.clone()))
}

/// Windows has no application menu: a menu bar belongs to a window, and
/// this one belongs to Chromium. See [`crate::menu::application`].
pub fn application() -> Result<(), OsError> {
    Ok(())
}
