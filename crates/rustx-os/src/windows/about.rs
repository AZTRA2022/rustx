//! Le panneau « À propos », sous Windows : `ShellAboutW`.
//!
//! La boîte que le système dessine depuis Windows 95, et qui est encore
//! celle de `winver`. Ce qu'elle montre, et qu'on ne choisit pas : sous
//! les deux lignes qu'on lui donne, elle ajoute la version de Windows
//! et la mémoire libre. C'est le panneau du système, avec ce que le
//! système y met.
//!
//! # Le nom et la version, séparés par un dièse
//!
//! Le premier argument porte les deux : ce qui est avant le dièse
//! devient le titre de la boîte, ce qui est après la première ligne en
//! gras. C'est une convention de l'API, et un nom qui contiendrait
//! lui-même un dièse couperait la boîte au mauvais endroit — d'où le
//! remplacement.

use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellAboutW;
use windows::Win32::UI::WindowsAndMessaging::{
    HICON, IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadImageW,
};
use windows::core::PCWSTR;

use crate::about::{Credits, lines};
use crate::error::OsError;
use crate::window::Pane;

pub const AVAILABLE: bool = true;

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Le premier argument de `ShellAboutW` : le titre, un dièse, puis la
/// ligne en gras.
pub(crate) fn titled(credits: &Credits) -> String {
    let safe = |text: &str| text.replace('#', " ");
    let name = safe(&credits.name);
    let version = safe(credits.version.trim());
    if version.is_empty() {
        format!("{name}#{name}")
    } else {
        format!("{name}#{name} {version}")
    }
}

/// Ce qui va sous la ligne en gras : le copyright seul, la version
/// étant déjà montée dans le titre.
pub(crate) fn rest(credits: &Credits) -> String {
    lines(credits)
        .lines()
        .filter(|line| line.trim() != credits.version.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn show(pane: Pane, credits: &Credits) -> Result<(), OsError> {
    let owner = (pane.0 != 0).then(|| HWND(pane.0 as *mut std::ffi::c_void));
    let first = wide(&titled(credits));
    let second = wide(&rest(credits));
    let icon = credits.icon.as_ref().and_then(|path| {
        let file = wide(&path.to_string_lossy());
        // SAFETY: un chemin terminé par zéro qui vit le temps de l'appel ;
        // l'icône chargée depuis un fichier appartient au processus, et
        // Windows la libère à sa fin.
        let loaded = unsafe {
            LoadImageW(
                None,
                PCWSTR(file.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
        };
        loaded.ok().map(|handle| HICON(handle.0))
    });
    // SAFETY: deux chaînes terminées par zéro qui vivent jusqu'au
    // retour, et une poignée de fenêtre ou rien. L'appel est modal : il
    // ne rend la main qu'à la fermeture de la boîte.
    let shown =
        unsafe { ShellAboutW(owner, PCWSTR(first.as_ptr()), PCWSTR(second.as_ptr()), icon) };
    if shown == 0 {
        return Err(OsError::Platform(
            "Windows refused to show the About box".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_separates_the_title_from_the_bold_line() {
        let credits = Credits::new("rustx", "0.2.0");
        assert_eq!(titled(&credits), "rustx#rustx 0.2.0");
    }

    #[test]
    fn a_name_with_a_hash_does_not_cut_the_box_in_the_wrong_place() {
        let credits = Credits::new("C# Studio", "1.0");
        assert_eq!(titled(&credits).matches('#').count(), 1);
        assert_eq!(titled(&credits), "C  Studio#C  Studio 1.0");
    }

    #[test]
    fn the_version_is_not_repeated_under_the_bold_line() {
        let credits = Credits::new("rustx", "0.2.0").copyright("(c) 2026 aztra");
        assert_eq!(rest(&credits), "(c) 2026 aztra");
        assert_eq!(rest(&Credits::new("rustx", "0.2.0")), "");
    }
}
