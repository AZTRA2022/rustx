//! Le sélecteur de couleur du système.
//!
//! `<input type="color">` existe, mais c'est Chromium qui le dessine,
//! et il n'a pas la pipette ni les couleurs que l'utilisateur a gardées
//! dans le sélecteur de son système. Ceci ouvre celui du système :
//! `ChooseColorW` sous Windows (sans pipette — Windows n'en a pas dans
//! ce dialogue), le panneau de couleurs de macOS (avec la sienne), et
//! celui de `zenity` ou `kdialog` sous Linux.
//!
//! Le dialogue est modal et attend la réponse : on l'appelle depuis un
//! fil qui peut attendre.

use crate::error::OsError;

/// Une couleur, huit bits par canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// `#rrggbb`.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Depuis `#rrggbb` ou `rrggbb`.
    pub fn from_hex(text: &str) -> Option<Self> {
        let digits = text.trim().trim_start_matches('#');
        if digits.len() != 6 || !digits.is_ascii() {
            return None;
        }
        let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
        Some(Self {
            r: channel(0)?,
            g: channel(2)?,
            b: channel(4)?,
        })
    }
}

/// Ouvrir le sélecteur, sur `initial` s'il y en a une.
///
/// `Ok(None)` quand l'utilisateur annule : ce n'est pas une erreur.
pub fn pick(initial: Option<Rgb>) -> Result<Option<Rgb>, OsError> {
    crate::platform::color::pick(initial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_goes_both_ways() {
        let teal = Rgb { r: 0, g: 128, b: 255 };
        assert_eq!(teal.hex(), "#0080ff");
        assert_eq!(Rgb::from_hex("#0080ff"), Some(teal));
        assert_eq!(Rgb::from_hex("0080FF"), Some(teal));
        assert_eq!(Rgb::from_hex("#08f"), None);
        assert_eq!(Rgb::from_hex("#gg0000"), None);
    }
}
