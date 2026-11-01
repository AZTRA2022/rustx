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
