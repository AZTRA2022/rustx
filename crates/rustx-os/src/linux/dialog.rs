//! Le dialogue de fichiers, sous Linux.
//!
//! Pas encore, et il le dit. Ce que ce sera : le portail de bureau —
//! `org.freedesktop.portal.FileChooser` sur D-Bus, `OpenFile` et
//! `SaveFile` — parce que c'est le seul qui marche aussi dans un bac à
//! sable (Flatpak, Snap) et qui donne le dialogue du bureau de
//! l'utilisateur, GNOME ou KDE. Sans portail, le repli est `zenity` ou
//! `kdialog`, et l'absence des deux est un refus, pas un plantage.
//!
//! L'API au-dessus ne change pas : les mêmes noms de fonctions, et une
//! constante qui dit si on peut.

use std::path::PathBuf;

use crate::dialog::{Ask, DialogError};

const WHAT: &str = "le dialogue de fichiers";

/// Faux tant que le portail n'est pas écrit : une page lit ceci pour ne
/// pas dessiner un bouton qui refuserait.
pub const AVAILABLE: bool = false;

pub fn open(_ask: &Ask) -> Result<Vec<PathBuf>, DialogError> {
    Err(DialogError::Unsupported(WHAT))
}

pub fn save(_ask: &Ask) -> Result<PathBuf, DialogError> {
    Err(DialogError::Unsupported(WHAT))
}

pub fn folder(_ask: &Ask) -> Result<PathBuf, DialogError> {
    Err(DialogError::Unsupported(WHAT))
}
