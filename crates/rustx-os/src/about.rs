//! Le panneau « À propos » que le système dessine.
//!
//! Une convention de macOS avant tout : la première ligne du menu de
//! l'application, et le panneau standard qu'elle ouvre — nom, version,
//! copyright, et le bouton qui mène aux licences. Un utilisateur de Mac
//! s'attend à le trouver là, et à ce qu'il ressemble à celui de toutes
//! ses autres applications ; une fenêtre dessinée par la page ne lui
//! ressemblera jamais tout à fait.
//!
//! Windows a eu le même — `ShellAboutW`, la boîte « À propos de
//! Windows » — et il est encore là. Il est daté : il montre la version
//! de Windows et la mémoire libre sous ce qu'on lui donne. C'est
//! néanmoins le panneau du système, et c'est ce que ce fichier promet.
//!
//! Linux n'en a pas : `GtkAboutDialog` est celui de GTK et n'existe pas
//! chez les autres, et aucun protocole de bureau n'en définit. Refusé en
//! le disant, parce qu'une application Linux dessine son « À propos »
//! elle-même et le sait.

use std::path::PathBuf;

use crate::error::OsError;
use crate::window::Pane;

/// Ce que le panneau dit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Credits {
    /// Le nom de l'application, la ligne en gras.
    pub name: String,
    /// `"0.2.0"`, telle que le crate la déclare.
    pub version: String,
    /// Une ligne de copyright, si on en a une.
    pub copyright: String,
    /// L'icône à montrer — un `.ico` sous Windows, n'importe quelle
    /// image que le système lit sous macOS.
    pub icon: Option<PathBuf>,
}

impl Credits {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            copyright: String::new(),
            icon: None,
        }
    }

    /// La même, avec sa ligne de copyright.
    pub fn copyright(mut self, line: impl Into<String>) -> Self {
        self.copyright = line.into();
        self
    }

    /// La même, avec son icône.
    pub fn icon(mut self, path: impl Into<PathBuf>) -> Self {
        self.icon = Some(path.into());
        self
    }
}

/// Si cette plateforme a un panneau « À propos » que ce code atteint.
pub fn supported() -> bool {
    crate::platform::about::AVAILABLE
}

/// L'ouvrir, devant cette fenêtre.
///
/// `pane` est la fenêtre qui le possède — celle au-dessus de laquelle il
/// s'ouvre, et qu'il bloque tant qu'il est là sous Windows. `Pane(0)`
/// pour une application qui n'en a pas encore.
pub fn show(pane: Pane, credits: &Credits) -> Result<(), OsError> {
    if credits.name.is_empty() {
        return Err(OsError::Platform(
            "an About panel with no application name is an empty box".to_string(),
        ));
    }
    crate::blocking::waiting("the About panel");
    crate::platform::about::show(pane, credits)
}

// Lues par `windows/` et `macos/` ; Linux refuse le panneau.
#[cfg_attr(target_os = "linux", allow(dead_code))]
/// Les deux lignes sous le nom, telles que les deux systèmes les
/// veulent : la version, puis le copyright, chacune seulement si elle
/// existe.
pub(crate) fn lines(credits: &Credits) -> String {
    [credits.version.trim(), credits.copyright.trim()]
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panel_with_no_name_is_refused_before_it_opens() {
        let nothing = Credits::default();
        assert!(show(Pane(0), &nothing).is_err());
    }

    #[test]
    fn the_lines_leave_out_what_was_not_given() {
        let bare = Credits::new("rustx", "0.2.0");
        assert_eq!(lines(&bare), "0.2.0");
        let full = bare.clone().copyright("© 2026 aztra");
        assert_eq!(lines(&full), "0.2.0\n© 2026 aztra");
        let nameless_version = Credits::new("rustx", "").copyright("© 2026 aztra");
        assert_eq!(lines(&nameless_version), "© 2026 aztra");
        assert_eq!(lines(&Credits::new("rustx", "")), "");
    }
}
