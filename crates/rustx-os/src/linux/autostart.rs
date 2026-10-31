//! Le démarrage automatique, sous Linux.
//!
//! Pas encore, et il le dit. Ce que ce sera : un fichier `.desktop`
//! dans `$XDG_CONFIG_HOME/autostart/`, ce que GNOME, KDE, XFCE et les
//! autres lisent tous — et, en bac à sable, le portail
//! `org.freedesktop.portal.Background`, qui demande à l'utilisateur au
//! lieu d'écrire derrière son dos.

use crate::error::OsError;

pub const AVAILABLE: bool = false;

pub fn set(_on: bool, _name: &str) -> Result<(), OsError> {
    Err(OsError::Unsupported("le démarrage automatique"))
}

pub fn is_on(_name: &str) -> bool {
    false
}
