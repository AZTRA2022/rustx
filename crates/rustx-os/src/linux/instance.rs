//! Une seule instance, sous Linux.
//!
//! Pas encore, et elle le dit. Ce que ce sera : un socket de domaine
//! Unix abstrait (`@rustx-instance-<nom>`), qui a la même propriété que
//! le mutex nommé de Windows — le noyau le rend quand le processus
//! s'arrête, même mal — et qui porte en plus les arguments, sans rien
//! d'autre à inventer. Sur un système sans sockets abstraits, un
//! fichier verrou dans `$XDG_RUNTIME_DIR` avec `flock`.

use crate::error::OsError;

const WHAT: &str = "l'instance unique";

pub const AVAILABLE: bool = false;

pub struct Only;

impl Only {
    pub fn one(_name: &str) -> Result<Self, OsError> {
        Err(OsError::Unsupported(WHAT))
    }
}

pub fn hand_over(_name: &str, _arguments: &[String]) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn handed_over() -> Vec<String> {
    Vec::new()
}
