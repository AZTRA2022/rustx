//! Une seule instance, et ce que la deuxième lui passe.
//!
//! Un logiciel qu'on relance alors qu'il tourne déjà ne doit pas
//! s'ouvrir deux fois. Ce qu'on attend, et que tout logiciel du
//! commerce fait : la deuxième instance réveille la première, lui passe
//! ce qu'on lui a donné — le fichier qu'on vient de double-cliquer — et
//! s'en va.
//!
//! ```ignore
//! let Ok(only) = Only::one("mon-logiciel") else {
//!     // Déjà lancé : lui passer nos arguments, et partir.
//!     let _ = instance::hand_over("mon-logiciel", &arguments);
//!     return Ok(());
//! };
//! // … et plus tard, dans la page :
//! for argument in instance::handed_over() { ouvrir(&argument); }
//! ```
//!
//! # Un jeton, et le nom rendu
//!
//! [`Only`] tient le nom. Lâché, il le rend — sans quoi un logiciel qui
//! s'arrête mal resterait « déjà lancé » jusqu'au redémarrage de la
//! machine, ce qui est le défaut que tout le monde a déjà vu quelque
//! part.

use crate::error::OsError;

/// Si cette plateforme sait tenir un nom pour une seule instance.
pub fn supported() -> bool {
    crate::platform::instance::AVAILABLE
}

/// Le nom, tenu. Lâché, il est rendu.
pub struct Only {
    /// Le nom tenu auprès du système. Jamais lu : ce qui compte est
    /// qu'il vive aussi longtemps que ce jeton, et que son `Drop`
    /// rende le nom.
    _held: crate::platform::instance::Only,
    name: String,
}

impl Only {
    /// Réserver ce nom pour cette instance.
    ///
    /// `OsError::Busy(nom)` quand une autre instance le tient déjà —
    /// ce qui est le cas normal et attendu, pas une panne : c'est le
    /// moment d'appeler [`hand_over`] et de partir.
    pub fn one(name: &str) -> Result<Self, OsError> {
        Ok(Self {
            _held: crate::platform::instance::Only::one(name)?,
            name: name.to_string(),
        })
    }

    /// Le nom tenu.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for Only {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "Only({})", self.name)
    }
}

/// Passer des arguments à l'instance qui tourne déjà.
///
/// Appelé par celle qui s'en va. Refusé en nommant l'application quand
/// personne ne tient ce nom — ce qui veut dire que la première instance
/// est partie entre-temps, et que celle-ci devrait rester.
pub fn hand_over(name: &str, arguments: &[String]) -> Result<(), OsError> {
    crate::platform::instance::hand_over(name, arguments)
}

/// Ce qu'une deuxième instance a passé depuis la dernière lecture.
///
/// Pris : lu une fois. Deux lectures ouvriraient deux fois le même
/// fichier.
pub fn handed_over() -> Vec<String> {
    crate::platform::instance::handed_over()
}

/// Ce qui sépare les arguments dans le message, et pourquoi il est
/// écrit ici.
///
/// Les deux côtés — celle qui passe et celle qui reçoit — sont deux
/// *processus différents*, souvent deux versions différentes du même
/// logiciel pendant une mise à jour. Le séparateur fait donc partie du
/// format, au même titre qu'un nom de fichier : un caractère qu'un
/// chemin ne peut pas contenir sur les trois plateformes.
pub const BETWEEN: char = '\u{1f}';
