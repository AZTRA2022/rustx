//! Imprimer un fichier, par la file d'impression du système.
//!
//! # Ce que ce fichier imprime, et ce qu'il n'imprime pas
//!
//! Un **fichier** : un PDF, une image, un texte, que le système envoie
//! à une imprimante avec l'application qu'il associe à ce genre de
//! fichier. C'est ce dont a besoin un logiciel qui a produit un
//! document et veut le sortir sur papier.
//!
//! Ce n'est **pas** l'impression de la page elle-même. Celle-là est à
//! Chromium : il a son propre aperçu, sa mise en page, ses sauts de
//! page CSS, et `BrowserHost::print()` l'ouvre. C'est donc au moteur de
//! l'appeler, pas à ce crate — la frontière est la même que partout
//! ailleurs ici : ce crate ne sait rien du DOM.
//!
//! # Le dialogue, ou la file
//!
//! [`dialog`] ouvre l'aperçu et le choix de l'imprimante, et attend
//! que l'utilisateur décide. [`silently`] envoie à l'imprimante par
//! défaut sans rien demander — ce qu'un logiciel de caisse ou
//! d'étiquettes veut, et ce qu'un traitement de texte ne veut jamais.
//! Les deux rendent la main dès que le système a pris le travail : ce
//! qui se passe ensuite est entre la file et l'imprimante, et personne
//! ici ne le suit.

use std::path::Path;

use crate::error::OsError;

/// Ouvrir le dialogue d'impression du système pour ce fichier.
pub fn dialog(path: &Path) -> Result<(), OsError> {
    existing(path)?;
    crate::blocking::waiting("the print dialog");
    crate::platform::print::print(path, true)
}

/// L'envoyer à l'imprimante par défaut, sans rien demander.
pub fn silently(path: &Path) -> Result<(), OsError> {
    existing(path)?;
    crate::blocking::waiting("the printer");
    crate::platform::print::print(path, false)
}

/// Les imprimantes que le système connaît, celle par défaut en premier.
///
/// Pour une application qui veut dire *où* son document va partir avant
/// de l'y envoyer.
pub fn printers() -> Result<Vec<Printer>, OsError> {
    crate::blocking::waiting("the list of printers");
    let mut all = crate::platform::print::printers()?;
    all.sort_by_key(|printer| !printer.default);
    Ok(all)
}

/// Une imprimante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printer {
    /// Son nom pour le système, qui est aussi celui que l'utilisateur
    /// lit : les trois systèmes n'en ont qu'un.
    pub name: String,
    /// Celle qui reçoit un travail que personne n'a adressé.
    pub default: bool,
}

fn existing(path: &Path) -> Result<(), OsError> {
    if !path.exists() {
        return Err(OsError::File(format!(
            "{} does not exist, so it cannot be printed",
            path.display()
        )));
    }
    if path.is_dir() {
        return Err(OsError::File(format!(
            "{} is a directory, not a document",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_that_is_not_a_file_is_printed() {
        assert!(dialog(Path::new("no/such/document.pdf")).is_err());
        assert!(silently(Path::new("no/such/document.pdf")).is_err());
        let said = dialog(Path::new(".")).expect_err("a directory is not a document");
        assert!(said.to_string().contains("directory"), "got {said}");
    }

    #[test]
    fn this_machine_lists_its_printers_or_refuses_by_name() {
        match printers() {
            Ok(all) => {
                assert!(
                    all.iter().filter(|printer| printer.default).count() <= 1,
                    "at most one default printer: {all:#?}"
                );
                for printer in &all {
                    assert!(!printer.name.is_empty(), "{printer:?}");
                }
                if let Some(first) = all.first() {
                    assert!(
                        first.default || all.iter().all(|each| !each.default),
                        "the default printer comes first: {all:#?}"
                    );
                }
            }
            Err(refused) => assert!(!refused.to_string().is_empty()),
        }
    }
}
