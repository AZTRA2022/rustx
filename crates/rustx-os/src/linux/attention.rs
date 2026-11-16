//! L'attention, sous Linux.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `flash` est `_NET_WM_STATE_DEMANDS_ATTENTION`, par la connexion X11
//! de `window.rs` — compilé, jamais vu clignoter.
//!
//! # Refusé
//!
//! La progression et la pastille. La seule convention que des bureaux
//! Linux lisent pour les deux est `com.canonical.Unity.LauncherEntry`,
//! un signal D-Bus émis par un objet que l'application doit publier et
//! garder vivant, et que seuls Unity, Plasma et l'extension « Dash to
//! Dock » de GNOME affichent. Un `dbus-send` ponctuel ne publie pas
//! d'objet, et ce crate ne tient pas de connexion D-Bus : c'est un
//! travail à part, pas un appel.

use crate::error::OsError;
use crate::window::Pane;

pub fn flash(pane: Pane) -> Result<(), OsError> {
    super::window::demand_attention(pane)
}

pub fn progress(_pane: Pane, _done: Option<f64>) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "taskbar progress on Linux (only Unity's LauncherEntry D-Bus object carries it)",
    ))
}

pub fn badge(_count: Option<u32>) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "a badge on Linux (only Unity's LauncherEntry D-Bus object carries it)",
    ))
}
