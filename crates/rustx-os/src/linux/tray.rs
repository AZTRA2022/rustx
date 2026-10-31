//! L'icône de barre système, sous Linux.
//!
//! Pas encore, et elle le dit. Ce que ce sera :
//! `org.kde.StatusNotifierItem` sur D-Bus, que GNOME (avec
//! l'extension AppIndicator) et KDE lisent tous les deux, avec un
//! `com.canonical.dbusmenu` pour le menu. L'ancienne voie — une
//! `GtkStatusIcon` — est retirée depuis GTK 3.14 et ne vaut pas d'être
//! écrite aujourd'hui.
//!
//! L'API au-dessus ne change pas : les mêmes noms, et une constante qui
//! dit si on peut.

use std::path::Path;

use crate::error::OsError;
use crate::notify::What;
use crate::tray::Item;

const WHAT: &str = "l'icône de barre système";

pub const AVAILABLE: bool = false;

pub fn shown() -> usize {
    0
}

pub struct Icon;

impl Icon {
    pub fn new(_icon: &Path, _tooltip: &str) -> Result<Self, OsError> {
        Err(OsError::Unsupported(WHAT))
    }

    pub fn hidden() -> Result<Self, OsError> {
        Err(OsError::Unsupported(WHAT))
    }

    pub fn menu(&self, _items: &[Item]) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }

    pub fn clicked(&self) -> Option<String> {
        None
    }

    pub fn notify(&self, _what: &What) -> Result<(), OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}
