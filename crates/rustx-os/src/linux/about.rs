//! Le panneau « À propos », sous Linux : il n'y en a pas, et c'est dit.
//!
//! Ce n'est pas un fichier à remplir plus tard. Aucune spécification de
//! freedesktop.org ne définit de panneau « À propos », et aucun bureau
//! n'en expose : `GtkAboutDialog` est une fenêtre que GTK dessine dans
//! le processus qui lie GTK, et Qt a la sienne, incompatible. Une
//! application Linux dessine donc le sien elle-même — ce qu'une page
//! fait très bien, et mieux que `zenity --info`, qui donnerait une
//! boîte d'information avec du texte dedans et non un panneau de
//! présentation.
//!
//! Le refus nomme la plateforme : une page qui interroge
//! `about::supported()` avant d'offrir la ligne de menu dessinera le
//! sien ici, et laissera le système faire sous Windows et macOS.

use crate::about::Credits;
use crate::error::OsError;
use crate::window::Pane;

pub const AVAILABLE: bool = false;

pub fn show(_pane: Pane, _credits: &Credits) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "a system About panel on Linux (no desktop protocol defines one; GtkAboutDialog \
         belongs to GTK and Qt has its own)",
    ))
}
