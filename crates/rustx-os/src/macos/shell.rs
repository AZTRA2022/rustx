//! The file manager, on macOS: Finder, and `AppKit`'s recent documents.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from Apple's documentation. `reveal`
//! is `open -R`, a documented flag of a command every Mac has;
//! `remember_recent` goes through `NSDocumentController` and has had
//! only the GNUstep type-check the other `AppKit` files had. Neither
//! has been seen to work. Verify on a real Mac before trusting them.
//!
//! # Refused
//!
//! File associations. On macOS the documents an application opens are
//! declared in its bundle's `Info.plist` (`CFBundleDocumentTypes`,
//! `UTExportedTypeDeclarations`), which Launch Services reads when the
//! bundle is installed — that belongs to the command line that writes
//! the bundle, not to code running inside it. `LSSetDefaultRoleHandler
//! ForContentType` can only make an app the default for a type its
//! bundle already declares, so it cannot stand in for the declaration.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::shell::Association;

pub fn reveal(path: &Path) -> Result<(), OsError> {
    let done = Command::new("open")
        .arg("-R")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| OsError::Platform(format!("`open -R` could not run: {error}")))?;
    if done.success() {
        Ok(())
    } else {
        Err(OsError::Platform(format!(
            "`open -R {}` failed ({done})",
            path.display()
        )))
    }
}

#[cfg(target_os = "macos")]
pub fn remember_recent(path: &Path) -> Result<(), OsError> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    let absolute = std::fs::canonicalize(path)
        .map_err(|error| OsError::File(format!("{}: {error}", path.display())))?
        .to_string_lossy()
        .into_owned();
    super::appkit::on_main("the recent documents", move || {
        let name = super::appkit::ns_string(&absolute)?;
        // SAFETY: documented `NSURL` and `NSDocumentController` calls, on
        // the main thread.
        unsafe {
            let url: Option<Retained<AnyObject>> =
                msg_send![class!(NSURL), fileURLWithPath: &*name];
            let url = url?;
            let controller: *mut AnyObject =
                msg_send![class!(NSDocumentController), sharedDocumentController];
            if controller.is_null() {
                return None;
            }
            let _: () = msg_send![controller, noteNewRecentDocumentURL: &*url];
        }
        Some(())
    })?
    .ok_or_else(|| OsError::Platform("AppKit did not take the recent document".to_string()))
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn remember_recent(_path: &Path) -> Result<(), OsError> {
    unreachable!("macos/shell.rs is never the platform module outside macOS")
}

const ASSOCIATIONS: &str = "file associations on macOS (declared in the bundle's Info.plist, \
                            not at run time)";

pub fn associate(_association: &Association) -> Result<(), OsError> {
    Err(OsError::Unsupported(ASSOCIATIONS))
}

pub fn dissociate(_association: &Association) -> Result<(), OsError> {
    Err(OsError::Unsupported(ASSOCIATIONS))
}
