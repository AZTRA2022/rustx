//! L'impression, sous Linux : CUPS, par `lp` et `lpstat`.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; la
//! lecture de ce que `lpstat` écrit a ses tests, qui tournent partout.
//! Rien n'a été imprimé.
//!
//! CUPS est le système d'impression de toutes les distributions, et
//! `lp` son client, livré avec. La bibliothèque `libcups` dirait la
//! même chose en liant du C ; `lp` ne demande rien de plus que ce que
//! la machine a déjà, comme `xclip` ou `pactl` ailleurs ici.
//!
//! # Le dialogue, qui n'existe pas
//!
//! CUPS n'a pas de dialogue d'impression : le dialogue est celui de GTK
//! ou de Qt, dessiné par l'application. [`print`] avec `ask` vrai
//! envoie donc le travail tel quel, comme `silently` — et le dit, parce
//! qu'une application qui croit avoir montré un aperçu à son
//! utilisateur et a imprimé dix pages sans le lui demander a fait pire
//! que de refuser.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::print::Printer;

/// Ce que `lpstat -p -d` écrit :
///
/// ```text
/// printer HP_LaserJet is idle.  enabled since ...
/// printer Brother_DCP disabled since ...
/// system default destination: HP_LaserJet
/// ```
pub(crate) fn printers_from(text: &str) -> Vec<Printer> {
    let default = text.lines().find_map(|line| {
        let (_, name) = line.split_once("system default destination:")?;
        let name = name.trim();
        (!name.is_empty()).then_some(name)
    });
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("printer ")?;
            let name = rest.split_whitespace().next()?;
            Some(Printer {
                name: name.to_string(),
                default: default == Some(name),
            })
        })
        .collect()
}

pub fn printers() -> Result<Vec<Printer>, OsError> {
    let out = Command::new("lpstat")
        .args(["-p", "-d"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            OsError::Platform(format!(
                "`lpstat` could not be run ({error}) — it comes with CUPS"
            ))
        })?;
    // `lpstat` sort en échec quand aucune imprimante n'est installée, et
    // « aucune imprimante » est une réponse.
    Ok(printers_from(&String::from_utf8_lossy(&out.stdout)))
}

pub fn print(path: &Path, _ask: bool) -> Result<(), OsError> {
    let out = Command::new("lp")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            OsError::Platform(format!(
                "`lp` could not be run ({error}) — it comes with CUPS"
            ))
        })?;
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if said.contains("no default destination") {
        return Err(OsError::Platform(
            "there is no default printer: CUPS has no destination set".to_string(),
        ));
    }
    Err(OsError::Platform(format!("`lp` refused the job: {said}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LPSTAT: &str = concat!(
        "printer HP_LaserJet is idle.  enabled since Wed 12 Nov 2026 09:00:00\n",
        "printer Brother_DCP disabled since Tue 11 Nov 2026 18:00:00 -\n",
        "\tPaused\n",
        "system default destination: HP_LaserJet\n"
    );

    #[test]
    fn every_printer_line_is_a_printer_and_the_default_is_marked() {
        let all = printers_from(LPSTAT);
        assert_eq!(all.len(), 2, "got {all:#?}");
        assert_eq!(all[0].name, "HP_LaserJet");
        assert!(all[0].default);
        assert_eq!(all[1].name, "Brother_DCP");
        assert!(!all[1].default, "a disabled printer is still a printer");
    }

    #[test]
    fn no_printer_at_all_is_an_empty_list_not_a_failure() {
        assert_eq!(printers_from("lpstat: No destinations added.\n"), Vec::new());
        assert_eq!(printers_from(""), Vec::new());
    }

    #[test]
    fn a_machine_with_printers_but_no_default_marks_none() {
        let no_default = "printer HP_LaserJet is idle.  enabled since now\n";
        let all = printers_from(no_default);
        assert_eq!(all.len(), 1);
        assert!(!all[0].default);
    }
}
