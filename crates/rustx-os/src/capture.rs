//! Une image de ce qui est à l'écran : tout un écran, un rectangle, ou
//! une fenêtre.
//!
//! Ce qu'une page ne peut pas faire sans demander : `getDisplayMedia`
//! ouvre un dialogue de permission à chaque fois, rend un flux vidéo et
//! non une image, et ne sait pas viser une zone. Un logiciel qui
//! annote, qui rapporte un bogue ou qui met une capture dans un
//! document a besoin d'une image, maintenant, de l'endroit qu'il nomme.
//!
//! # Ce que cela n'est pas
//!
//! Ni un enregistrement ni un flux : une image, une fois. La capture
//! vidéo est le travail de `devices.rs` et de `media.rs`.
//!
//! # La permission
//!
//! macOS demande l'autorisation « Enregistrement de l'écran » dans les
//! Réglages Système, une fois par application, et c'est l'utilisateur
//! qui la donne ; ni ce crate ni aucun programme ne peut la cocher à sa
//! place. Windows et X11 ne demandent rien. Wayland ne laisse
//! personne lire l'écran d'autrui du tout — c'est un choix de sa
//! conception, pas un manque, et c'est refusé en le disant.

use crate::clipboard::Image;
use crate::error::OsError;
use crate::screen::Area;
use crate::window::Pane;

/// Tout le bureau, tous les écrans d'un coup.
pub fn desktop() -> Result<Image, OsError> {
    crate::blocking::waiting("a capture of the screen");
    crate::platform::capture::area(None)
}

/// Ce rectangle du bureau, en coordonnées d'écran.
pub fn area(area: Area) -> Result<Image, OsError> {
    if area.width <= 0 || area.height <= 0 {
        return Err(OsError::Platform(format!(
            "a capture of {}×{} is not an image",
            area.width, area.height
        )));
    }
    crate::blocking::waiting("a capture of the screen");
    crate::platform::capture::area(Some(area))
}

/// Cette fenêtre, telle qu'elle est à l'écran.
pub fn window(pane: Pane) -> Result<Image, OsError> {
    crate::blocking::waiting("a capture of a window");
    crate::platform::capture::window(pane)
}

/// Combien d'octets une image de cette taille occupe, et le refus quand
/// cela n'a pas de sens.
///
/// Quatre octets par pixel. La borne est volontairement basse par
/// rapport à ce qu'une machine peut allouer — un écran 8K en fait 132
/// Mo — et elle n'est là que pour qu'une taille absurde (un rectangle
/// lu de travers, un `i32` négatif passé en `u32`) devienne une phrase
/// plutôt qu'une allocation de plusieurs gigaoctets.
pub(crate) fn bytes_for(width: u32, height: u32) -> Result<usize, OsError> {
    const LIMIT: u64 = 1024 * 1024 * 1024;
    if width == 0 || height == 0 {
        return Err(OsError::Platform(format!(
            "a capture of {width}×{height} is not an image"
        )));
    }
    // En arithmétique contrôlée : deux `u32` au maximum font déborder un
    // `u64` une fois multipliés par quatre, et un débordement est une
    // panique là où on a promis une phrase.
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .unwrap_or(u64::MAX);
    if bytes > LIMIT {
        return Err(OsError::Platform(format!(
            "a capture of {width}×{height} would take {bytes} bytes, which is more than this \
             crate will allocate for one image"
        )));
    }
    Ok(bytes as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_rectangle_is_refused_before_anything_is_captured() {
        let flat = Area {
            x: 0,
            y: 0,
            width: 0,
            height: 100,
        };
        assert!(area(flat).is_err());
        let backwards = Area {
            x: 0,
            y: 0,
            width: -10,
            height: -10,
        };
        assert!(area(backwards).is_err());
    }

    #[test]
    fn a_size_that_cannot_be_an_image_is_a_sentence_not_an_allocation() {
        assert_eq!(bytes_for(1920, 1080).expect("a size"), 1920 * 1080 * 4);
        assert!(bytes_for(0, 100).is_err());
        assert!(bytes_for(100, 0).is_err());
        let huge = bytes_for(u32::MAX, u32::MAX).expect_err("refused");
        assert!(huge.to_string().contains("more than"), "got {huge}");
    }

    #[test]
    fn a_window_that_does_not_exist_is_not_captured() {
        assert!(window(Pane(0)).is_err());
    }
}
