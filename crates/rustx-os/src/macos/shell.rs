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

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::shell::{Association, Scheme};

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

/// `open` sans drapeau : l'adresse part à Launch Services, qui la donne
/// au navigateur que l'utilisateur a choisi.
pub fn open_url(url: &str) -> Result<(), OsError> {
    let done = Command::new("open")
        .arg("--")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| OsError::Platform(format!("`open` could not run: {error}")))?;
    if done.success() {
        Ok(())
    } else {
        Err(OsError::Platform(format!(
            "`open` did not open that link ({done})"
        )))
    }
}

/// Le bundle qui contient ce programme : `…/Mon App.app`.
///
/// Un schéma d'URL sur macOS est une déclaration du bundle, et non un
/// réglage du système : Launch Services lit `CFBundleURLTypes` dans
/// `Info.plist`, et il n'existe aucun appel qui enregistre un schéma
/// pour un exécutable nu.
fn bundle_of(program: &Path) -> Result<PathBuf, OsError> {
    program
        .ancestors()
        .find(|parent| parent.extension().is_some_and(|kind| kind == "app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            OsError::Platform(format!(
                "{} is not inside a `.app` bundle, and a URL scheme on macOS is                  something a bundle declares. `rustx package` makes the bundle; run the                  program from it.",
                program.display()
            ))
        })
}

/// Un ordre de `PlistBuddy`, avec sa sortie.
fn plist(info: &Path, order: &str) -> Result<(bool, String), OsError> {
    let done = Command::new("/usr/libexec/PlistBuddy")
        .arg("-c")
        .arg(order)
        .arg(info)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| OsError::Platform(format!("`PlistBuddy` could not run: {error}")))?;
    Ok((
        done.status.success(),
        String::from_utf8_lossy(&done.stdout).into_owned(),
    ))
}

/// Dire à Launch Services de relire le bundle : sans cela la
/// déclaration est écrite et personne ne l'a lue.
fn relaunch_services(bundle: &Path) {
    let _ = Command::new(
        "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/         Support/lsregister",
    )
    .arg("-f")
    .arg(bundle)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status();
}

/// L'indice de l'entrée qui déclare ce schéma, et le nombre d'entrées.
///
/// `PlistBuddy` ne cherche pas : il faut lire chaque indice jusqu'à ce
/// qu'il n'y en ait plus. Trente-deux est une borne, pas une limite
/// réelle — aucune application n'en déclare autant.
fn entries(info: &Path, scheme: &str) -> Result<(Option<usize>, usize), OsError> {
    let mut found = None;
    let mut count = 0;
    for index in 0..32 {
        let (there, text) = plist(info, &format!("Print :CFBundleURLTypes:{index}"))?;
        if !there {
            break;
        }
        count = index + 1;
        if found.is_none()
            && text
                .lines()
                .any(|line| line.trim().eq_ignore_ascii_case(scheme))
        {
            found = Some(index);
        }
    }
    Ok((found, count))
}

pub fn register_scheme(scheme: &Scheme) -> Result<(), OsError> {
    let bundle = bundle_of(&scheme.program)?;
    let info = bundle.join("Contents/Info.plist");
    if !info.is_file() {
        return Err(OsError::Platform(format!(
            "{} has no Info.plist",
            bundle.display()
        )));
    }
    let (already, count) = entries(&info, &scheme.scheme)?;
    if already.is_some() {
        // Déjà déclaré : il reste à le faire relire, ce qu'un bundle
        // réinstallé demande de toute façon.
        relaunch_services(&bundle);
        return Ok(());
    }
    // La première déclaration crée le tableau ; qu'il existe déjà n'est
    // pas une erreur, et c'est la seule façon de le savoir.
    let _ = plist(&info, "Add :CFBundleURLTypes array")?;
    let at = format!(":CFBundleURLTypes:{count}");
    for order in [
        format!("Add {at} dict"),
        format!("Add {at}:CFBundleURLName string {}", scheme.name),
        format!("Add {at}:CFBundleURLSchemes array"),
        format!("Add {at}:CFBundleURLSchemes:0 string {}", scheme.scheme),
    ] {
        let (done, said) = plist(&info, &order)?;
        if !done {
            return Err(OsError::Platform(format!(
                "{} could not be written (`{order}` said {})",
                info.display(),
                said.trim()
            )));
        }
    }
    relaunch_services(&bundle);
    Ok(())
}

pub fn unregister_scheme(scheme: &Scheme) -> Result<(), OsError> {
    let bundle = bundle_of(&scheme.program)?;
    let info = bundle.join("Contents/Info.plist");
    if !info.is_file() {
        return Ok(());
    }
    let (found, _) = entries(&info, &scheme.scheme)?;
    if let Some(index) = found {
        let (done, said) = plist(&info, &format!("Delete :CFBundleURLTypes:{index}"))?;
        if !done {
            return Err(OsError::Platform(format!(
                "{} could not be written ({})",
                info.display(),
                said.trim()
            )));
        }
    }
    relaunch_services(&bundle);
    Ok(())
}
