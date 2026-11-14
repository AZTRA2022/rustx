//! Ce que le gestionnaire de fichiers sait faire d'un fichier : le
//! montrer, s'en souvenir, et savoir quelle application l'ouvre.
//!
//! Trois choses qu'une application de documents doit à son système :
//! « Afficher dans le dossier » après un export, sa place dans les
//! documents récents, et le double-clic sur un `.monformat` qui l'ouvre.

use std::path::Path;

use crate::error::OsError;

/// Ouvrir le gestionnaire de fichiers sur le dossier de ce fichier, le
/// fichier sélectionné.
pub fn reveal(path: &Path) -> Result<(), OsError> {
    existing(path)?;
    crate::platform::shell::reveal(path)
}

/// Ajouter ce fichier aux documents récents du système.
pub fn remember_recent(path: &Path) -> Result<(), OsError> {
    existing(path)?;
    crate::platform::shell::remember_recent(path)
}

/// Une association : quelle extension, sous quel nom, ouverte par quel
/// programme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Association {
    /// Sans le point : `"rxdoc"`.
    pub extension: String,
    /// Un identifiant stable, propre à l'application :
    /// `"MonEditeur.Document"`.
    pub id: String,
    /// Ce que l'explorateur affiche comme type : `"Document MonÉditeur"`.
    pub description: String,
    /// Le programme qui ouvre le fichier, à qui le chemin est passé en
    /// premier argument.
    pub program: std::path::PathBuf,
}

impl Association {
    /// Refuser ce qui ne ferait pas une association : une extension avec
    /// un point ou un séparateur, un identifiant vide.
    pub fn checked(&self) -> Result<(), OsError> {
        let bad = |c: char| c == '.' || c == '/' || c == '\\' || c.is_whitespace();
        if self.extension.is_empty() || self.extension.chars().any(bad) {
            return Err(OsError::Platform(format!(
                "`{}` is not an extension — write it without the dot, e.g. `rxdoc`",
                self.extension
            )));
        }
        if self.id.is_empty()
            || self
                .id
                .chars()
                .any(|c| c == '/' || c == '\\' || c.is_whitespace())
        {
            return Err(OsError::Platform(format!(
                "`{}` is not an identifier — letters, digits and dots, e.g. `MyEditor.Document`",
                self.id
            )));
        }
        Ok(())
    }
}

/// Faire de ce programme celui qui ouvre ces fichiers, pour cet
/// utilisateur.
pub fn associate(association: &Association) -> Result<(), OsError> {
    association.checked()?;
    crate::platform::shell::associate(association)
}

/// Défaire ce qu'[`associate`] a fait.
pub fn dissociate(association: &Association) -> Result<(), OsError> {
    association.checked()?;
    crate::platform::shell::dissociate(association)
}

fn existing(path: &Path) -> Result<(), OsError> {
    if path.exists() {
        Ok(())
    } else {
        Err(OsError::File(format!("{} does not exist", path.display())))
    }
}

// Lu par `linux/` seulement.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// Un chemin absolu en URI `file://`, chaque octet hors des caractères
/// sûrs écrit `%XX` — ce que les gestionnaires de fichiers de Linux
/// attendent sur D-Bus.
pub(crate) fn file_uri(path: &str) -> String {
    let mut uri = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_refused_by_name() {
        let said = reveal(Path::new("nowhere/at/all.txt")).expect_err("no such file");
        assert!(said.to_string().contains("all.txt"), "got {said}");
    }

    #[test]
    fn an_extension_is_written_without_its_dot() {
        let mut association = Association {
            extension: ".rxdoc".into(),
            id: "Rustx.Test".into(),
            description: "Test".into(),
            program: "x".into(),
        };
        assert!(association.checked().is_err());
        association.extension = "rxdoc".into();
        assert!(association.checked().is_ok());
        association.id = "has space".into();
        assert!(association.checked().is_err());
    }

    #[test]
    fn a_path_becomes_a_percent_encoded_uri() {
        assert_eq!(
            file_uri("/home/me/a b/é.txt"),
            "file:///home/me/a%20b/%C3%A9.txt"
        );
        assert_eq!(file_uri("/tmp/x-y_z.~"), "file:///tmp/x-y_z.~");
    }
}
