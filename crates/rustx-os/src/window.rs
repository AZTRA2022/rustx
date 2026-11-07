//! La fenêtre elle-même : son cadre, sa taille, son rang.
//!
//! # Pourquoi une fenêtre se commande
//!
//! Beaucoup de logiciels n'ont pas la fenêtre que le système dessine :
//! une barre de titre à eux, des boutons à eux, parfois pas de barre du
//! tout. Cela ne s'obtient pas en dessinant par-dessus — il faut
//! enlever le cadre du système, et il faut alors rendre à la page ce
//! que le cadre faisait : déplacer, agrandir, réduire, fermer.
//!
//! C'est ce que ce fichier donne, et rien de plus : une poignée de
//! fenêtre entre, un ordre est donné à la machine. Il ne sait rien du
//! DOM, rien de CEF, rien des composants.
//!
//! # La poignée
//!
//! Une fenêtre est désignée par la poignée que le système lui a donnée
//! — un `HWND` sous Windows — portée ici comme un entier. Ce crate ne
//! la crée jamais : c'est le moteur qui l'a, et c'est le moteur qui la
//! passe.

use crate::error::OsError;

/// La fenêtre, telle que le système la nomme.
///
/// Un entier, parce que c'est ce qu'un `HWND` est, et parce qu'un
/// pointeur dans une API publique obligerait tout le monde au-dessus à
/// écrire `unsafe` pour parler de sa propre fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pane(pub isize);

impl Pane {
    /// Depuis ce que le moteur tient : le `HWND` que Chromium rend.
    pub fn from_handle(handle: *mut std::ffi::c_void) -> Self {
        Self(handle as isize)
    }

    /// S'il y a une fenêtre du tout.
    pub fn exists(&self) -> bool {
        self.0 != 0
    }
}

/// Le cadre que le système dessine autour de la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dressing {
    /// Celui du système : barre de titre, bordures, les trois boutons.
    #[default]
    System,
    /// Aucun. La page dessine tout, et c'est à elle de donner un
    /// endroit où attraper la fenêtre — voir [`drag`].
    None,
}

/// Ce qu'une fenêtre occupe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spread {
    /// Sa taille à elle.
    #[default]
    Normal,
    /// Réduite dans la barre des tâches.
    Minimized,
    /// Toute la place que le bureau laisse.
    Maximized,
    /// Tout l'écran, cadre compris.
    Fullscreen,
}

/// Ce qui se bascule : une fenêtre y est, ou elle revient d'où elle
/// venait.
///
/// Une barre de titre à soi n'a pas un bouton « agrandir » et un bouton
/// « rendre sa taille » : elle en a un, et il bascule. Dire
/// « agrandis-toi » à une fenêtre déjà agrandie ne fait rien — c'est
/// exactement ce qu'un deuxième clic faisait — donc l'ordre qu'un tel
/// bouton donne est celui-ci, et c'est le système qu'on interroge pour
/// savoir dans quel sens il va.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switchable {
    /// Toute la place que le bureau laisse, ou sa taille à elle.
    Maximized,
    /// Tout l'écran, ou sa taille à elle.
    Fullscreen,
}

impl Switchable {
    /// Ce que la fenêtre occupe quand la bascule est dans ce sens-là.
    pub fn spread(self) -> Spread {
        match self {
            Self::Maximized => Spread::Maximized,
            Self::Fullscreen => Spread::Fullscreen,
        }
    }
}

/// Basculer : y aller, ou en revenir si on y est déjà.
pub fn switch(pane: Pane, what: Switchable) -> Result<(), OsError> {
    let wanted = what.spread();
    if spread_of(pane)? == wanted {
        spread(pane, Spread::Normal)
    } else {
        spread(pane, wanted)
    }
}

/// Si cette plateforme commande ses fenêtres depuis ce code.
pub fn supported() -> bool {
    crate::platform::window::AVAILABLE
}

/// Mettre ou enlever le cadre du système.
pub fn dress(pane: Pane, dressing: Dressing) -> Result<(), OsError> {
    crate::platform::window::dress(pane, dressing)
}

/// Réduire, agrandir, revenir, ou prendre tout l'écran.
pub fn spread(pane: Pane, spread: Spread) -> Result<(), OsError> {
    crate::platform::window::spread(pane, spread)
}

/// Ce que la fenêtre occupe en ce moment.
pub fn spread_of(pane: Pane) -> Result<Spread, OsError> {
    crate::platform::window::spread_of(pane)
}

/// La garder devant les autres, ou lui rendre son rang.
pub fn on_top(pane: Pane, above: bool) -> Result<(), OsError> {
    crate::platform::window::on_top(pane, above)
}

/// Commencer un déplacement à la souris.
///
/// Ce qu'une barre de titre à soi appelle quand on appuie dessus : le
/// système prend la suite et déplace la fenêtre jusqu'à ce que le
/// bouton soit relâché, exactement comme sur sa propre barre. Sans
/// cela, une fenêtre sans cadre ne se déplace pas.
pub fn drag(pane: Pane) -> Result<(), OsError> {
    crate::platform::window::drag(pane)
}

/// La fermer.
pub fn close(pane: Pane) -> Result<(), OsError> {
    crate::platform::window::close(pane)
}

/// Où la souris se trouve, en coordonnées d'écran.
///
/// Ce qu'une barre de titre à soi demande quand elle veut savoir sur
/// quel moniteur elle est, ou de combien la fenêtre a suivi.
pub fn pointer() -> Result<(i32, i32), OsError> {
    crate::platform::window::pointer()
}

/// Le mot qu'un pont porte, et ce qu'il veut dire.
///
/// Écrit ici et pas dans le moteur pour une raison simple : la liste
/// des ordres et la liste des fonctions sont la même liste, et deux
/// listes séparées finissent par ne plus l'être.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Dress(Dressing),
    Spread(Spread),
    /// Y aller, ou en revenir : ce qu'un seul bouton de barre fait.
    Switch(Switchable),
    OnTop(bool),
    Drag,
    Close,
}

impl Order {
    /// L'ordre que ce mot désigne, ou rien si ce n'en est pas un.
    pub fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "frameless" => Self::Dress(Dressing::None),
            "framed" => Self::Dress(Dressing::System),
            "minimize" => Self::Spread(Spread::Minimized),
            "maximize" => Self::Spread(Spread::Maximized),
            "restore" => Self::Spread(Spread::Normal),
            "fullscreen" => Self::Spread(Spread::Fullscreen),
            "maximize-or-restore" => Self::Switch(Switchable::Maximized),
            "fullscreen-or-restore" => Self::Switch(Switchable::Fullscreen),
            "on-top" => Self::OnTop(true),
            "not-on-top" => Self::OnTop(false),
            "drag" => Self::Drag,
            "close" => Self::Close,
            _ => return None,
        })
    }

    /// Le mot de cet ordre, celui que `from_word` relit.
    pub fn word(&self) -> &'static str {
        match self {
            Self::Dress(Dressing::None) => "frameless",
            Self::Dress(Dressing::System) => "framed",
            Self::Spread(Spread::Minimized) => "minimize",
            Self::Spread(Spread::Maximized) => "maximize",
            Self::Spread(Spread::Normal) => "restore",
            Self::Spread(Spread::Fullscreen) => "fullscreen",
            Self::Switch(Switchable::Maximized) => "maximize-or-restore",
            Self::Switch(Switchable::Fullscreen) => "fullscreen-or-restore",
            Self::OnTop(true) => "on-top",
            Self::OnTop(false) => "not-on-top",
            Self::Drag => "drag",
            Self::Close => "close",
        }
    }

    /// L'exécuter sur cette fenêtre.
    pub fn given(&self, pane: Pane) -> Result<(), OsError> {
        match *self {
            Self::Dress(dressing) => dress(pane, dressing),
            Self::Spread(how) => spread(pane, how),
            Self::Switch(what) => switch(pane, what),
            Self::OnTop(above) => on_top(pane, above),
            Self::Drag => drag(pane),
            Self::Close => close(pane),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_order_is_read_back_from_the_word_it_writes() {
        // Le pont porte le mot, et c'est l'autre bout qui le relit : si
        // les deux listes s'écartent d'un seul mot, un bouton de la
        // page ne fait plus rien et personne ne dit pourquoi.
        for order in [
            Order::Dress(Dressing::None),
            Order::Dress(Dressing::System),
            Order::Spread(Spread::Minimized),
            Order::Spread(Spread::Maximized),
            Order::Spread(Spread::Normal),
            Order::Spread(Spread::Fullscreen),
            Order::Switch(Switchable::Maximized),
            Order::Switch(Switchable::Fullscreen),
            Order::OnTop(true),
            Order::OnTop(false),
            Order::Drag,
            Order::Close,
        ] {
            assert_eq!(
                Order::from_word(order.word()),
                Some(order),
                "the word `{}` did not come back",
                order.word()
            );
        }
    }

    #[test]
    fn a_word_that_is_not_an_order_is_not_one() {
        assert_eq!(Order::from_word("explode"), None);
        assert_eq!(Order::from_word(""), None);
    }

    #[test]
    fn a_window_that_does_not_exist_is_not_commanded() {
        // La poignée nulle est ce qu'un moteur a avant que Chromium ait
        // fait la fenêtre. Refuser est la seule réponse : donner un
        // ordre à zéro, sous Windows, c'est le donner au bureau.
        let nothing = Pane(0);
        assert!(!nothing.exists());
        assert!(matches!(
            super::spread(nothing, Spread::Minimized),
            Err(OsError::Unsupported(_)) | Err(OsError::Platform(_))
        ));
    }
}
