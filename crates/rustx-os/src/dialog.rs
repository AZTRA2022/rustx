//! Choisir un fichier, ou dire où en écrire un.
//!
//! Une page ne peut pas faire ça. `<input type="file">` ouvre bien un
//! dialogue, mais ce qu'il rend est un `File` du navigateur, sans
//! chemin : impossible de le rouvrir plus tard, impossible de le passer
//! à une bibliothèque Rust, impossible d'enregistrer à côté. Ce dont un
//! logiciel a besoin est un `PathBuf`, et c'est le dialogue du système
//! qui en donne un.
//!
//! # Celui du système, pas celui de Chromium
//!
//! Chromium en a un — `run_file_dialog` — et il a été écarté. Il
//! n'existe que dans le processus navigateur, alors que tout ce qu'un
//! développeur écrit (l'arbre, l'état, les gestionnaires) vit dans le
//! processus de rendu : l'appeler depuis un `onClick` aurait demandé un
//! pont entre les deux processus, deux messages de Chromium et un
//! registre de questions en attente, pour obtenir au bout du compte le
//! dialogue que le système ouvre de toute façon.
//!
//! Appelé directement, il s'ouvre depuis n'importe quel processus, et la
//! signature redevient celle qu'on veut écrire :
//!
//! ```ignore
//! <button onClick={move |_| {
//!     if let Ok(paths) = dialog::open(Ask::new("Choisir une image")
//!         .filter("Images", ["png", "jpg"])) { ouvrir(&paths[0]); }
//! }}>Ouvrir…</button>
//! ```
//!
//! # Il bloque, et c'est ce qu'on lui demande
//!
//! Un dialogue modal arrête ce qui l'a ouvert jusqu'à ce que
//! l'utilisateur ait répondu — c'est la définition de « modal », et
//! c'est ce qu'attend quelqu'un qui vient de cliquer sur « Ouvrir… ».
//! Le dialogue fait tourner sa propre boucle de messages pendant ce
//! temps, donc la fenêtre reste dessinée et le système ne la croit pas
//! morte.

use std::path::PathBuf;

/// Ce qu'on demande à l'utilisateur.
///
/// Écrit à la main dans une page, donc lisible : un titre, des filtres,
/// un dossier de départ.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ask {
    /// Le titre du dialogue. Vide : celui du système.
    pub title: String,
    /// `("Images", ["png", "jpg"])` — le nom que l'utilisateur lit, et
    /// les extensions qu'il laisse passer.
    pub filters: Vec<(String, Vec<String>)>,
    /// Où le dialogue s'ouvre. `None` : là où le système l'a laissé.
    pub start_in: Option<PathBuf>,
    /// Le nom proposé, pour un enregistrement.
    pub named: Option<String>,
    /// Plusieurs fichiers à la fois. Sans effet sur un enregistrement.
    pub many: bool,
}

impl Ask {
    /// Un dialogue avec ce titre et rien d'autre.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Un filtre de plus, dans l'ordre où il apparaîtra.
    pub fn filter<E: Into<String>>(
        mut self,
        name: impl Into<String>,
        extensions: impl IntoIterator<Item = E>,
    ) -> Self {
        self.filters
            .push((name.into(), extensions.into_iter().map(Into::into).collect()));
        self
    }

    /// Ouvrir le dialogue dans ce dossier.
    pub fn starting_in(mut self, directory: impl Into<PathBuf>) -> Self {
        self.start_in = Some(directory.into());
        self
    }

    /// Le nom proposé à l'enregistrement.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.named = Some(name.into());
        self
    }

    /// Autoriser plusieurs fichiers.
    pub fn many(mut self) -> Self {
        self.many = true;
        self
    }
}

/// Ce qu'un dialogue peut rendre d'autre qu'un chemin.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DialogError {
    /// L'utilisateur a fermé le dialogue. Un cas normal, nommé, et pas
    /// quelque chose à journaliser.
    Cancelled,
    /// Cette plateforme n'ouvre pas encore de dialogue depuis ici.
    Unsupported(&'static str),
    /// Le système a refusé, dans ses mots.
    Platform(String),
}

impl std::fmt::Display for DialogError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(out, "le dialogue a été fermé sans rien choisir"),
            Self::Unsupported(what) => write!(
                out,
                "{what} n'est pas disponible sur cette plateforme ({}) — \
                 `{}/dialog.rs` est le fichier à écrire.",
                std::env::consts::OS,
                std::env::consts::OS
            ),
            Self::Platform(said) => write!(out, "le système a refusé le dialogue : {said}"),
        }
    }
}

impl std::error::Error for DialogError {}

/// Les filtres, dans la forme que les trois systèmes lisent : un nom
/// lisible, et un motif par extension.
///
/// `("Images", ["png", "jpg"])` devient `("Images", "*.png;*.jpg")`.
/// Windows veut exactement cela ; le portail de bureau de Linux et
/// `NSOpenPanel` veulent les extensions seules, qui se relisent de là.
pub fn patterns_for(filters: &[(String, Vec<String>)]) -> Vec<(String, String)> {
    filters
        .iter()
        .map(|(name, extensions)| {
            let patterns: Vec<String> = extensions
                .iter()
                .map(|extension| {
                    let extension = extension.trim().trim_start_matches('*').trim_start_matches('.');
                    format!("*.{extension}")
                })
                .collect();
            (name.clone(), patterns.join(";"))
        })
        .collect()
}

/// Si cette plateforme ouvre un dialogue depuis ici.
pub fn supported() -> bool {
    crate::platform::dialog::AVAILABLE
}

/// Demander un ou plusieurs fichiers existants.
///
/// Bloque jusqu'à ce que l'utilisateur ait répondu. `Cancelled` quand il
/// ferme le dialogue — une application écrit
/// `let Ok(paths) = dialog::open(…) else { return; }` et se comporte
/// juste.
pub fn open(ask: Ask) -> Result<Vec<PathBuf>, DialogError> {
    crate::platform::dialog::open(&ask)
}

/// Demander où écrire un fichier.
pub fn save(ask: Ask) -> Result<PathBuf, DialogError> {
    crate::platform::dialog::save(&ask)
}

/// Demander un dossier.
pub fn folder(ask: Ask) -> Result<PathBuf, DialogError> {
    crate::platform::dialog::folder(&ask)
}
