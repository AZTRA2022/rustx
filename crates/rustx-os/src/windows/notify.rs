//! Les notifications de Windows : la bulle de la barre système.
//!
//! Le système fait passer une bulle *par une icône*, donc il en faut
//! une. Une application qui en tient déjà une passe par elle
//! (`Icon::notify`) et la notification porte son icône ; une
//! application qui n'en tient pas en reçoit une cachée, posée au
//! premier appel et gardée pour la durée du processus.
//!
//! Gardée, et non posée puis retirée aussitôt : une bulle meurt avec
//! l'icône qui l'a montrée, et une notification qui disparaît avant
//! d'avoir été lue ne sert à rien. Elle part avec le processus, qui est
//! le moment où le système retire de toute façon ce qu'un programme
//! disparu avait posé.

use std::sync::{Mutex, OnceLock};

use crate::error::OsError;
use crate::notify::What;

use super::tray::Icon;

/// Windows en montre, et ce fichier les montre.
pub const AVAILABLE: bool = true;

pub fn show(what: &What) -> Result<(), OsError> {
    let held = kept()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let icon = match held.as_ref() {
        Some(icon) => icon,
        None => {
            drop(held);
            let made = Icon::hidden()?;
            let mut held = kept()
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // Deux fils peuvent arriver ici en même temps ; le second
            // garde celle du premier et jette la sienne.
            let icon = held.get_or_insert(made);
            return icon.notify(what);
        }
    };
    icon.notify(what)
}

/// L'icône cachée de ce processus, s'il en a eu besoin.
fn kept() -> &'static Mutex<Option<Icon>> {
    static HIDDEN: OnceLock<Mutex<Option<Icon>>> = OnceLock::new();
    HIDDEN.get_or_init(|| Mutex::new(None))
}
