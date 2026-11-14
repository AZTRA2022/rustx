//! Une notification du système.
//!
//! # Par la barre système, et pourquoi
//!
//! Sous Windows, une notification peut être un *toast* WinRT — celles
//! qu'on voit en bas à droite, avec des boutons — ou une bulle de la
//! barre système, que Windows 10 et 11 transforment en toast de
//! toute façon. C'est la seconde qui est écrite ici, parce que la
//! première exige un `AppUserModelID` enregistré dans le menu Démarrer :
//! une application installée en a un, une application lancée par
//! `rustx dev` n'en a pas et n'a pas à en avoir.
//!
//! Ce que cela coûte : pas de boutons dans la notification, et le nom
//! qui s'affiche est celui de l'exécutable. Ce qui le lèvera : un `.exe`
//! installé peut s'enregistrer, et `notify` prendra alors le chemin des
//! toasts sans que la page change d'une ligne.
//!
//! # Elle ne bloque pas
//!
//! Montrer une notification rend la main tout de suite. L'utilisateur
//! la lira, ou ne la lira pas ; une application qui attendrait sa
//! lecture attendrait pour toujours.

use std::path::PathBuf;

use crate::error::OsError;

/// Ce qu'une notification dit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct What {
    /// La ligne en gras.
    pub title: String,
    /// Le texte dessous.
    pub body: String,
    /// L'icône montrée à côté. Sans elle, celle de l'application.
    pub icon: Option<PathBuf>,
}

impl What {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            icon: None,
        }
    }

    /// L'icône à montrer — un `.ico`.
    pub fn icon(mut self, path: impl Into<PathBuf>) -> Self {
        self.icon = Some(path.into());
        self
    }
}

/// Si cette plateforme montre une notification depuis ce code.
pub fn supported() -> bool {
    crate::platform::notify::AVAILABLE
}

/// La montrer.
///
/// L'application n'a pas besoin d'icône de barre système pour cela :
/// s'il n'y en a pas, une icône cachée est posée pour la durée du
/// processus, parce que c'est par une icône que le système fait passer
/// une bulle. Elle part avec le processus, et une application qui tient
/// déjà une [`crate::tray::Icon`] peut passer par elle
/// ([`crate::tray::Icon::notify`]) pour que la notification porte la
/// sienne.
pub fn show(what: What) -> Result<(), OsError> {
    crate::platform::notify::show(&what)
}

/// Un bouton dans la notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Ce que [`ask`] rendra si l'utilisateur le choisit.
    pub id: String,
    /// Ce qu'il lit sur le bouton.
    pub label: String,
}

impl Action {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Si cette plateforme met des boutons dans une notification.
///
/// Demandé *avant* d'en offrir : une notification dont les boutons
/// n'apparaissent pas est une notification que l'utilisateur ne peut
/// pas refermer comme on le lui a dit.
pub fn can_ask() -> bool {
    crate::platform::notify::CAN_ASK
}

/// Montrer une notification avec des boutons, et attendre.
///
/// Rend l'identifiant du bouton choisi, ou `None` si l'utilisateur a
/// fermé la notification ou l'a laissée expirer.
///
/// # Elle bloque, contrairement à [`show`]
///
/// C'est tout l'inverse de `show`, et c'est inévitable : une réponse est
/// demandée. Le fil qui appelle attend donc que l'utilisateur agisse ou
/// que le système laisse tomber la notification — ce qui peut être
/// plusieurs dizaines de secondes, et jamais si son bureau garde les
/// notifications jusqu'au clic. À appeler d'un fil qui peut attendre,
/// pas de celui qui dessine.
pub fn ask(what: What, actions: &[Action]) -> Result<Option<String>, OsError> {
    if actions.is_empty() {
        return Err(OsError::Platform(
            "a notification with no button to choose is `show`, not `ask`".to_string(),
        ));
    }
    if let Some(empty) = actions.iter().find(|action| action.id.is_empty()) {
        return Err(OsError::Platform(format!(
            "the button labelled `{}` has no id, and an id is what `ask` answers with",
            empty.label
        )));
    }
    crate::platform::notify::ask(&what, actions)
}
