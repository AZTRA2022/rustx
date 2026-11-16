//! Ce que la barre des tâches ou le Dock montre d'une application qui
//! n'est pas devant : qu'elle demande l'attention, où en est son long
//! travail, combien de choses l'attendent.
//!
//! Trois signaux que chaque système dessine à sa façon, et qu'aucun ne
//! dessine partout : la barre de progression est sur l'icône de la
//! barre des tâches de Windows et nulle part ailleurs sans un composant
//! à soi ; la pastille est une habitude du Dock, que Windows ne connaît
//! que comme une petite icône à dessiner soi-même. Chaque absence est un
//! refus qui le dit.

use crate::error::OsError;
use crate::window::Pane;

/// Faire clignoter l'icône de la fenêtre (Windows), la marquer comme
/// demandant l'attention (Linux), ou faire rebondir le Dock (macOS),
/// jusqu'à ce que l'utilisateur vienne à elle.
pub fn flash(pane: Pane) -> Result<(), OsError> {
    crate::platform::attention::flash(pane)
}

/// La progression sur l'icône de la fenêtre : `Some(0.0..=1.0)`, ou
/// `None` pour la retirer. Une valeur hors de `0..=1` est ramenée dans
/// l'intervalle.
pub fn progress(pane: Pane, done: Option<f64>) -> Result<(), OsError> {
    crate::platform::attention::progress(pane, done.map(|done| done.clamp(0.0, 1.0)))
}

/// La pastille sur l'icône de l'application : un nombre, ou `None` pour
/// l'enlever.
pub fn badge(count: Option<u32>) -> Result<(), OsError> {
    crate::platform::attention::badge(count)
}

// Donnée à Windows seulement ; les deux autres refusent la
// progression.
#[cfg_attr(not(windows), allow(dead_code))]
/// Une fraction en dix-millièmes, l'unité que ce crate donne à Windows :
/// fine assez pour qu'aucun pas ne se voie, et entière.
pub(crate) fn per_ten_thousand(done: f64) -> u64 {
    (done.clamp(0.0, 1.0) * 10_000.0).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_kept_inside_the_bar() {
        assert_eq!(per_ten_thousand(0.5), 5_000);
        assert_eq!(per_ten_thousand(1.7), 10_000);
        assert_eq!(per_ten_thousand(-1.0), 0);
    }

    #[test]
    fn a_window_that_does_not_exist_is_not_flashed() {
        assert!(flash(Pane(0)).is_err());
        assert!(progress(Pane(0), Some(0.5)).is_err());
    }
}
