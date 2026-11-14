//! Printing, on macOS: CUPS, through `lp` and `lpstat`.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine. macOS *is* the home of CUPS — Apple
//! maintained it for years — and the two commands are the same ones the
//! Linux file uses, which is why the parser lives there and is called
//! from here rather than written twice. Nothing has been printed from a
//! Mac by this code; the parser's tests run on every machine.
//!
//! # The dialog, which is the application's
//!
//! `NSPrintPanel` is the dialog a Mac user knows, and it belongs to a
//! document in an `NSPrintOperation` — an application printing its own
//! drawn content, not a file handed to the spooler. Printing a file
//! through that panel means `open -a Preview` and letting the user
//! press Command-P, which is not what `dialog` promises. So
//! [`print`] sends the job either way and says so, exactly as on Linux.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::print::Printer;

pub fn printers() -> Result<Vec<Printer>, OsError> {
    let out = Command::new("lpstat")
        .args(["-p", "-d"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| OsError::Platform(format!("`lpstat` could not be run: {error}")))?;
    // No printer installed is an answer, not a failure — `lpstat` exits
    // non-zero for it.
    Ok(printers_from(&String::from_utf8_lossy(&out.stdout)))
}

/// The same reading as Linux's, because it is the same CUPS and the
/// same `lpstat`. Kept in one place so the two cannot drift apart.
fn printers_from(text: &str) -> Vec<Printer> {
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

pub fn print(path: &Path, _ask: bool) -> Result<(), OsError> {
    let out = Command::new("lp")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| OsError::Platform(format!("`lp` could not be run: {error}")))?;
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

    #[test]
    fn the_default_destination_line_marks_one_printer() {
        let said = concat!(
            "printer HP_LaserJet is idle.  enabled since now\n",
            "system default destination: HP_LaserJet\n"
        );
        let all = printers_from(said);
        assert_eq!(all.len(), 1);
        assert!(all[0].default);
    }

    #[test]
    fn no_printer_at_all_is_an_empty_list() {
        assert_eq!(printers_from("lpstat: No destinations added.\n"), Vec::new());
    }
}
