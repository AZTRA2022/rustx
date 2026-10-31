//! Une icône dans la barre système, et son menu.
//!
//! Ce qu'une page ne peut pas avoir, et ce qu'on attend d'un logiciel
//! qui tourne longtemps : une icône à côté de l'horloge, un menu
//! dessus, et une fenêtre qu'on referme sans quitter l'application.
//!
//! # Un fil pour l'icône
//!
//! Une icône de barre système parle par messages : le système en envoie
//! à une fenêtre quand on clique dessus. Elle a donc besoin d'une
//! fenêtre et d'une boucle de messages — mais pas de *la* fenêtre de
//! l'application : une fenêtre invisible, faite pour recevoir, suffit.
//!
//! C'est ce que [`Icon::new`] fait : un fil, une fenêtre invisible
//! dessus, et la boucle. Conséquence, et c'est elle qui compte :
//! l'icône s'obtient d'où la page est écrite, comme le dialogue de
//! fichiers, sans rien demander au processus de la fenêtre.
//!
//! # Un jeton, et ce qu'il retire
//!
//! [`Icon`] est l'icône. Quand elle est lâchée, l'icône quitte la barre
//! et le fil s'arrête — parce qu'une icône qui survit à l'objet qui la
//! portait est une icône morte, que l'utilisateur voit et sur laquelle
//! rien ne répond.
//!
//! ```ignore
//! let icon = Icon::new(Path::new("assets/icon.ico"), "Mon logiciel")?;
//! icon.menu(&[Item::new("ouvrir", "Ouvrir"), Item::new("quitter", "Quitter")])?;
//! // plus tard, dans la boucle de la page :
//! if let Some("quitter") = icon.clicked().as_deref() { app.quit(); }
//! ```

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::OsError;

/// Une ligne du menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Ce que [`Icon::clicked`] rendra si on la choisit.
    pub id: String,
    /// Ce que l'utilisateur lit.
    pub label: String,
    /// `Some(true)` pour une ligne cochée, `None` pour une ligne
    /// ordinaire. Une séparation s'écrit avec un libellé vide.
    pub checked: Option<bool>,
}

impl Item {
    /// Une ligne ordinaire.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: None,
        }
    }

    /// La même, cochée ou décochée.
    pub fn checked(mut self, yes: bool) -> Self {
        self.checked = Some(yes);
        self
    }

    /// Une ligne de séparation.
    pub fn separator() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            checked: None,
        }
    }
}

/// L'identifiant que rend un clic gauche sur l'icône elle-même.
///
/// Réservé, et reconnaissable : un menu dont une ligne s'appellerait
/// `@click` serait un menu qui se fait passer pour un clic.
pub const CLICK: &str = "@click";

/// Ce que l'icône a reçu et que personne n'a encore lu.
///
/// Partagé entre le fil de l'icône, qui y écrit quand on clique, et la
/// page, qui y lit. Une file et non une dernière valeur : deux clics
/// rapprochés sont deux clics, et le premier n'est pas à jeter.
#[derive(Debug, Clone, Default)]
pub struct Chosen {
    queue: Arc<Mutex<VecDeque<String>>>,
}

impl Chosen {
    pub fn new() -> Self {
        Self::default()
    }

    /// Dit par le fil de l'icône.
    pub fn push(&self, id: impl Into<String>) {
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.push_back(id.into());
    }

    /// Lu par la page : le plus ancien non lu, une fois.
    pub fn take(&self) -> Option<String> {
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.pop_front()
    }
}

/// Est-ce un fichier que le système sait charger comme icône ?
///
/// Lu plutôt que deviné : un `.ico` commence par un zéro réservé, le
/// type `1`, et le nombre d'images qu'il contient. Un `.png` renommé en
/// `.ico` est l'erreur que ceci attrape, et il l'attrape avec une
/// phrase au lieu d'une icône qui n'apparaît pas.
pub fn readable(path: &Path) -> Result<(), OsError> {
    let bytes = std::fs::read(path).map_err(|error| {
        OsError::File(format!(
            "l'icône {} n'a pas pu être lue : {error}",
            path.display()
        ))
    })?;
    if bytes.len() < 6 {
        return Err(OsError::File(format!(
            "{} est trop court pour être une icône",
            path.display()
        )));
    }
    let reserved = u16::from_le_bytes([bytes[0], bytes[1]]);
    let kind = u16::from_le_bytes([bytes[2], bytes[3]]);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    if reserved != 0 || kind != 1 || count == 0 {
        return Err(OsError::File(format!(
            "{} is not an icon file — l'icône d'une barre système est un `.ico` ; \
             un `.png` renommé n'en est pas un.",
            path.display()
        )));
    }
    Ok(())
}

/// Si cette plateforme a une barre système que ce code atteint.
pub fn supported() -> bool {
    crate::platform::tray::AVAILABLE
}

/// Combien d'icônes ce processus a dans la barre, maintenant.
///
/// Pour les tests, et pour une application qui veut s'assurer qu'elle
/// n'en a pas laissé deux : ce qui est compté est ce que le système a
/// accepté, pas ce que le programme croit avoir posé.
pub fn shown() -> usize {
    crate::platform::tray::shown()
}

/// Une icône dans la barre système. Lâchée, elle en sort.
pub struct Icon {
    inner: crate::platform::tray::Icon,
}

impl Icon {
    /// Poser l'icône, avec l'infobulle que le système montre dessus.
    pub fn new(icon: &Path, tooltip: &str) -> Result<Self, OsError> {
        readable(icon)?;
        Ok(Self {
            inner: crate::platform::tray::Icon::new(icon, tooltip)?,
        })
    }

    /// Le menu qui s'ouvrira au clic droit, remplaçant le précédent.
    pub fn menu(&self, items: &[Item]) -> Result<(), OsError> {
        self.inner.menu(items)
    }

    /// Ce qui a été choisi depuis la dernière fois, ou rien.
    ///
    /// À lire dans la boucle de la page — un effet, un battement — et
    /// non à attendre : rien ici ne bloque.
    pub fn clicked(&self) -> Option<String> {
        self.inner.clicked()
    }

    /// Une notification, montrée depuis cette icône.
    pub fn notify(&self, what: &crate::notify::What) -> Result<(), OsError> {
        self.inner.notify(what)
    }
}
