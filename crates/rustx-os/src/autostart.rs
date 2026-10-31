//! Démarrer avec la session, ou non.
//!
//! Une préférence que tout logiciel qui tourne en fond propose, et que
//! l'utilisateur doit pouvoir retirer sans chercher : cocher la case la
//! pose, la décocher l'enlève, et rien ne reste derrière.
//!
//! ```ignore
//! autostart::set(true, "mon-logiciel")?;
//! if autostart::is_on("mon-logiciel") { … }
//! ```
//!
//! # Pour cet utilisateur, pas pour la machine
//!
//! Ce qui est écrit l'est pour l'utilisateur qui le demande, et ne
//! demande donc aucun droit d'administrateur. Un démarrage pour toute
//! la machine est une décision d'installateur, pas d'application.

use crate::error::OsError;

/// Si cette plateforme sait démarrer une application avec la session.
pub fn supported() -> bool {
    crate::platform::autostart::AVAILABLE
}

/// Poser ou retirer le démarrage automatique de *cette* application.
///
/// Le chemin enregistré est celui de l'exécutable en cours : c'est lui
/// qu'il faudra lancer, et le demander à l'appelant serait lui demander
/// quelque chose que le système sait déjà.
///
/// Appelé deux fois de suite avec la même valeur, rien ne change et
/// rien n'échoue : une préférence qu'on coche deux fois est cochée.
pub fn set(on: bool, name: &str) -> Result<(), OsError> {
    crate::platform::autostart::set(on, name)
}

/// Si cette application démarre avec la session.
pub fn is_on(name: &str) -> bool {
    crate::platform::autostart::is_on(name)
}
