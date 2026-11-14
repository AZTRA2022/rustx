//! Le sélecteur de couleur, sous Linux : `zenity`, ou `kdialog` sous
//! KDE — la même porte que les dialogues de fichiers (`dialog.rs`).
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; la
//! lecture de ce que les deux outils écrivent a ses tests. Aucun
//! sélecteur n'a été vu s'ouvrir.

use std::process::{Command, Stdio};

use crate::color::Rgb;
use crate::error::OsError;

/// Ce que l'un ou l'autre outil écrit : `rgb(255,128,0)` ou
/// `rgba(…, 1)` pour `zenity`, `#ff8000` pour `kdialog`.
pub(crate) fn read_color(text: &str) -> Option<Rgb> {
    let text = text.trim();
    if let Some(color) = Rgb::from_hex(text) {
        return Some(color);
    }
    let inside = text
        .strip_prefix("rgba(")
        .or_else(|| text.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let mut channels = inside.split(',').map(|part| part.trim().parse::<u8>().ok());
    Some(Rgb {
        r: channels.next()??,
        g: channels.next()??,
        b: channels.next()??,
    })
}

fn kde() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|desktop| desktop.contains("KDE"))
}

pub fn pick(initial: Option<Rgb>) -> Result<Option<Rgb>, OsError> {
    let mut command = if kde() {
        let mut command = Command::new("kdialog");
        command.arg("--getcolor");
        if let Some(color) = initial {
            command.args(["--default", &color.hex()]);
        }
        command
    } else {
        let mut command = Command::new("zenity");
        command.arg("--color-selection");
        if let Some(color) = initial {
            command.arg(format!("--color={}", color.hex()));
        }
        command
    };
    let out = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            OsError::Platform(format!(
                "neither `zenity` nor `kdialog` could be run for the color picker: {error}"
            ))
        })?;
    // Les deux sortent en échec, sans rien écrire, quand on annule.
    if !out.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    read_color(&text)
        .map(Some)
        .ok_or_else(|| OsError::Platform(format!("`{}` is not a color", text.trim())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_tools_are_read() {
        let orange = Some(Rgb { r: 255, g: 128, b: 0 });
        assert_eq!(read_color("rgb(255,128,0)\n"), orange);
        assert_eq!(read_color("rgba(255, 128, 0, 1)"), orange);
        assert_eq!(read_color("#ff8000"), orange);
        assert_eq!(read_color("rgb(300,0,0)"), None);
        assert_eq!(read_color(""), None);
    }
}
