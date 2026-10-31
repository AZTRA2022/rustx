//! What a window is, besides a page: its icon and what it says it is.
//!
//! An application that ships has a logo in its title bar, a logo in the
//! taskbar, and a name and a version somebody can read. None of that is
//! the page's business and all of it is the window's, so it lives here.
//!
//! The icon is set on the window Chromium made, through the platform's
//! own call. The metadata is remembered and handed to the page, because
//! a page that shows its own version should not have to be edited when
//! the version changes. Writing the version into the *binary* — what
//! Windows shows in a file's properties — needs a resource linked at
//! build time, which is the command line's work, not a library's.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// What an application says it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct About {
    /// The window's title, which is also the application's name.
    pub name: String,
    /// `"0.1.0"`, as the application's own crate declares it.
    pub version: String,
    /// One line, for a window that shows one.
    pub description: String,
    /// Who publishes it.
    pub publisher: String,
}

fn state() -> &'static Mutex<(Option<PathBuf>, About)> {
    static STATE: OnceLock<Mutex<(Option<PathBuf>, About)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((None, About::default())))
}

/// Said by the facade before the window opens.
pub fn declare(icon: Option<PathBuf>, about: About) {
    let mut held = state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *held = (icon, about);
}

/// What the application said it is.
pub fn about() -> About {
    state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .1
        .clone()
}

/// The icon the application declared, if it declared one.
pub fn icon() -> Option<PathBuf> {
    state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
        .clone()
}

/// Put the declared icon on a window that has just been created.
///
/// Called with the handle Chromium gives for its own window. A failure
/// is said once and is not fatal: an application with no icon is an
/// application with the system's icon, which is a cosmetic loss and not
/// a reason to refuse to start.
pub fn dress(handle: *mut std::ffi::c_void) {
    let Some(path) = icon() else {
        return;
    };
    if !path.exists() {
        eprintln!(
            "rustx: the window's icon was declared as {} and there is no such file",
            path.display()
        );
        return;
    }
    match load(&path) {
        Ok(()) => {}
        Err(complaint) => eprintln!("rustx: {complaint}"),
    }
    set(handle, &path);
}

/// Is this a file the platform can load as an icon?
///
/// Read rather than guessed: an `.ico` begins with a reserved zero, the
/// type `1`, and how many images it holds. A `.png` renamed to `.ico` is
/// the mistake this catches, and it catches it with a sentence instead
/// of an icon that silently does not appear.
pub fn load(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "the window's icon at {} could not be read: {error}",
            path.display()
        )
    })?;
    if bytes.len() < 6 {
        return Err(format!("{} is too short to be an icon", path.display()));
    }
    let reserved = u16::from_le_bytes([bytes[0], bytes[1]]);
    let kind = u16::from_le_bytes([bytes[2], bytes[3]]);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    if reserved != 0 || kind != 1 || count == 0 {
        return Err(format!(
            "{} is not an icon file. A window's icon is an `.ico`; a `.png` renamed \
             to `.ico` is not one.",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    const IMAGE_ICON: u32 = 1;
    const LR_LOADFROMFILE: u32 = 0x0010;
    const LR_DEFAULTSIZE: u32 = 0x0040;
    const WM_SETICON: u32 = 0x0080;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;

    unsafe extern "system" {
        fn LoadImageW(
            instance: *mut c_void,
            name: *const u16,
            kind: u32,
            width: i32,
            height: i32,
            load: u32,
        ) -> *mut c_void;
        fn SendMessageW(window: *mut c_void, message: u32, high: usize, low: isize) -> isize;
    }

    /// Both sizes: the title bar reads the small one and the taskbar the
    /// big one, and a window given only one of them has a blurred logo
    /// in the other place.
    pub(super) fn set(window: *mut c_void, path: &Path) {
        if window.is_null() {
            return;
        }
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);

        for (which, size) in [(ICON_SMALL, 16), (ICON_BIG, 32)] {
            let icon = unsafe {
                LoadImageW(
                    std::ptr::null_mut(),
                    wide.as_ptr(),
                    IMAGE_ICON,
                    size,
                    size,
                    LR_LOADFROMFILE | LR_DEFAULTSIZE,
                )
            };
            if icon.is_null() {
                eprintln!(
                    "rustx: the window's icon at {} could not be loaded by the system",
                    path.display()
                );
                return;
            }
            unsafe { SendMessageW(window, WM_SETICON, which, icon as isize) };
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::ffi::c_void;
    use std::path::Path;

    /// The other platforms' windows take their icon from the bundle or
    /// from the desktop entry, which the command line writes.
    pub(super) fn set(_window: *mut c_void, _path: &Path) {}
}

fn set(handle: *mut std::ffi::c_void, path: &Path) {
    platform::set(handle, path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ico_is_recognised() {
        let path = std::env::temp_dir().join("rustx-window-icon-good.ico");
        // The header of a one-image icon, which is all `load` reads.
        std::fs::write(&path, [0u8, 0, 1, 0, 1, 0, 32, 32]).expect("a file");
        assert_eq!(load(&path), Ok(()));
    }

    #[test]
    fn a_png_named_ico_is_refused_by_name() {
        let path = std::env::temp_dir().join("rustx-window-icon-bad.ico");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n").expect("a file");
        let said = load(&path).expect_err("should be refused");
        assert!(said.contains("not an icon file"), "got {said}");
        assert!(said.contains(".png"), "got {said}");
    }

    #[test]
    fn a_missing_icon_says_which_file() {
        let said = load(Path::new("nowhere/at/all.ico")).expect_err("should be refused");
        assert!(said.contains("could not be read"), "got {said}");
    }
}
