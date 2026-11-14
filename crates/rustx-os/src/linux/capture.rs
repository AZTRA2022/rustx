//! La capture d'écran, sous Linux : `GetImage` sur la racine X11.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; la
//! conversion des octets que X11 rend vers le BGRA de [`Image`] a ses
//! tests, qui tournent partout. Aucune image n'a été capturée.
//!
//! # Wayland
//!
//! Refusé, et pas par manque de travail : Wayland ne laisse aucun
//! client lire l'écran d'un autre, par conception. La seule voie est le
//! portail `org.freedesktop.portal.Screenshot`, qui montre à
//! l'utilisateur un dialogue de consentement et rend un fichier — une
//! autre fonction, avec un autre contrat (asynchrone, refusable), et
//! non celle-ci. Sous XWayland, `$DISPLAY` existe mais ne voit que les
//! fenêtres X11, pas le bureau : une capture y serait silencieusement
//! fausse, donc elle est refusée aussi quand `$WAYLAND_DISPLAY` est là.

use crate::clipboard::Image;
use crate::error::OsError;
use crate::screen::Area;
use crate::window::Pane;

/// Les octets d'un `GetImage` en 24 ou 32 bits par pixel, vers le BGRA
/// que [`Image`] porte.
///
/// X11 rend ses lignes sans remplissage pour un format `ZPixmap` en 32
/// bits, et chaque pixel y est déjà B, G, R, puis un octet inutilisé —
/// il ne reste qu'à mettre l'alpha à 255, parce que X11 n'en a pas et
/// qu'un alpha à zéro donne une image entièrement transparente.
pub(crate) fn to_bgra(bytes: &[u8], width: u32, height: u32, depth: u8) -> Result<Vec<u8>, OsError> {
    let per_pixel = match depth {
        24 | 32 => 4,
        16 => 2,
        other => {
            return Err(OsError::Unsupported(match other {
                8 => "capturing a screen with an 8-bit palette",
                _ => "capturing a screen at this colour depth",
            }));
        }
    };
    let pixels = (width as usize) * (height as usize);
    if bytes.len() < pixels * per_pixel {
        return Err(OsError::Platform(format!(
            "the X server returned {} bytes for a {width}×{height} image at {depth} bits, \
             and {} were needed",
            bytes.len(),
            pixels * per_pixel
        )));
    }
    let mut bgra = Vec::with_capacity(pixels * 4);
    for index in 0..pixels {
        if per_pixel == 4 {
            let at = index * 4;
            bgra.extend_from_slice(&[bytes[at], bytes[at + 1], bytes[at + 2], 255]);
        } else {
            // 16 bits : cinq bits de rouge, six de vert, cinq de bleu,
            // remis à l'échelle de huit bits.
            let at = index * 2;
            let packed = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
            let red = ((packed >> 11) & 0x1F) as u8;
            let green = ((packed >> 5) & 0x3F) as u8;
            let blue = (packed & 0x1F) as u8;
            bgra.extend_from_slice(&[
                blue << 3 | blue >> 2,
                green << 2 | green >> 4,
                red << 3 | red >> 2,
                255,
            ]);
        }
    }
    Ok(bgra)
}

/// S'il y a un serveur X dont la capture veut dire quelque chose.
fn x11_only() -> Result<(), OsError> {
    let set = |name: &str| std::env::var(name).is_ok_and(|value| !value.is_empty());
    if set("WAYLAND_DISPLAY") {
        return Err(OsError::Unsupported(
            "capturing the screen on Wayland (no client may read another's pixels; the \
             org.freedesktop.portal.Screenshot portal, with its own consent dialog, is the \
             only way and is a different function)",
        ));
    }
    if !set("DISPLAY") {
        return Err(OsError::Platform(
            "there is no display to capture: $DISPLAY is not set".to_string(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn grab(area: Option<Area>, from: Option<u32>) -> Result<Image, OsError> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat};

    x11_only()?;
    let (link, number) = x11rb::connect(None).map_err(|error| {
        OsError::Platform(format!("the X server could not be reached: {error}"))
    })?;
    let screen = &link.setup().roots[number];
    let drawable = from.unwrap_or(screen.root);
    let geometry = link
        .get_geometry(drawable)
        .map_err(|error| OsError::Platform(format!("the geometry is unreadable: {error}")))?
        .reply()
        .map_err(|error| OsError::Platform(format!("the geometry is unreadable: {error}")))?;
    // Un rectangle donné est en coordonnées d'écran, et la racine a son
    // origine à (0, 0) : l'un est l'autre.
    let (x, y, width, height) = match area {
        Some(area) => (
            area.x as i16,
            area.y as i16,
            u16::try_from(area.width).unwrap_or(0),
            u16::try_from(area.height).unwrap_or(0),
        ),
        None => (0, 0, geometry.width, geometry.height),
    };
    crate::capture::bytes_for(u32::from(width), u32::from(height))?;
    let image = link
        .get_image(
            ImageFormat::Z_PIXMAP,
            drawable,
            x,
            y,
            width,
            height,
            // Tous les plans : un masque de bits à un.
            !0,
        )
        .map_err(|error| OsError::Platform(format!("the capture was refused: {error}")))?
        .reply()
        .map_err(|error| OsError::Platform(format!("the capture was refused: {error}")))?;
    Ok(Image {
        width: u32::from(width),
        height: u32::from(height),
        bgra: to_bgra(&image.data, u32::from(width), u32::from(height), image.depth)?,
    })
}

#[cfg(target_os = "linux")]
pub fn area(area: Option<Area>) -> Result<Image, OsError> {
    grab(area, None)
}

#[cfg(target_os = "linux")]
pub fn window(pane: Pane) -> Result<Image, OsError> {
    let xid = super::window::xid(pane)?;
    grab(None, Some(xid))
}

/// Jamais la plateforme choisie hors de Linux — voir `lib.rs`.
#[cfg(not(target_os = "linux"))]
pub fn area(_area: Option<Area>) -> Result<Image, OsError> {
    unreachable!("linux/capture.rs is never the platform module outside Linux")
}

/// Jamais la plateforme choisie hors de Linux — voir `lib.rs`.
#[cfg(not(target_os = "linux"))]
pub fn window(_pane: Pane) -> Result<Image, OsError> {
    unreachable!("linux/capture.rs is never the platform module outside Linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_32_bit_pixel_keeps_its_channels_and_gains_an_alpha() {
        // Deux pixels : bleu pur, puis rouge pur, en B G R X.
        let bytes = [255, 0, 0, 0, 0, 0, 255, 0];
        let bgra = to_bgra(&bytes, 2, 1, 24).expect("an image");
        assert_eq!(bgra, vec![255, 0, 0, 255, 0, 0, 255, 255]);
    }

    #[test]
    fn x11_has_no_alpha_so_the_image_is_never_transparent() {
        let bytes = [1, 2, 3, 0];
        let bgra = to_bgra(&bytes, 1, 1, 32).expect("an image");
        assert_eq!(bgra[3], 255, "an alpha of zero would be an invisible capture");
    }

    #[test]
    fn a_sixteen_bit_screen_is_spread_back_over_eight_bits() {
        // Blanc en 5-6-5 : tous les bits à un.
        let bytes = u16::MAX.to_le_bytes();
        assert_eq!(to_bgra(&bytes, 1, 1, 16).expect("an image"), vec![255, 255, 255, 255]);
        // Noir reste noir.
        assert_eq!(to_bgra(&[0, 0], 1, 1, 16).expect("an image"), vec![0, 0, 0, 255]);
    }

    #[test]
    fn too_few_bytes_is_a_sentence_and_not_a_panic() {
        let said = to_bgra(&[0, 0, 0], 10, 10, 24).expect_err("refused");
        assert!(said.to_string().contains("were needed"), "got {said}");
    }

    #[test]
    fn a_palette_screen_is_refused_by_name() {
        let said = to_bgra(&[0; 64], 8, 8, 8).expect_err("refused");
        assert!(said.to_string().contains("8-bit palette"), "got {said}");
    }
}
