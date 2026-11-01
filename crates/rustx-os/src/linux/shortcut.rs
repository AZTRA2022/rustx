//! Les raccourcis globaux, sous Linux.
//!
//! Pas encore, et ils le disent. Ce que ce sera : le portail
//! `org.freedesktop.portal.GlobalShortcuts`, parce que c'est le seul
//! chemin qui marche sous Wayland — où capter le clavier d'un autre
//! client est précisément ce que le protocole interdit. Sous X11, le
//! repli est `XGrabKey` sur la fenêtre racine.
//!
//! La lecture de la combinaison (`shortcut::parse`) est déjà commune :
//! les codes de touche sont ceux de Windows, et c'est ici qu'une table
//! les traduira en keysyms.

use crate::error::OsError;

pub const AVAILABLE: bool = false;

pub struct Shortcut;

impl Shortcut {
    pub fn listen(_modifiers: u32, _key: u32, _keys: &str) -> Result<Self, OsError> {
        Err(OsError::Unsupported("les raccourcis globaux"))
    }

    pub fn pressed(&self) -> u64 {
        0
    }
}
