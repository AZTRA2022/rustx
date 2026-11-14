//! Le sélecteur de couleur, sous Windows : `ChooseColorW`.
//!
//! Le dialogue de `comdlg32`, ouvert déployé (`CC_FULLOPEN`) : la
//! palette seule ne laisse choisir que quarante-huit couleurs. Il n'a
//! pas de pipette — ce dialogue n'en a jamais eu.

use windows::Win32::Foundation::COLORREF;
use windows::Win32::UI::Controls::Dialogs::{
    CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW, CommDlgExtendedError,
};

use crate::color::Rgb;
use crate::error::OsError;

/// `COLORREF` est `0x00BBGGRR`.
pub(crate) fn to_colorref(color: Rgb) -> u32 {
    u32::from(color.r) | u32::from(color.g) << 8 | u32::from(color.b) << 16
}

pub(crate) fn from_colorref(value: u32) -> Rgb {
    Rgb {
        r: (value & 0xFF) as u8,
        g: (value >> 8 & 0xFF) as u8,
        b: (value >> 16 & 0xFF) as u8,
    }
}

pub fn pick(initial: Option<Rgb>) -> Result<Option<Rgb>, OsError> {
    // Les seize couleurs « personnalisées » du dialogue : il exige le
    // tableau, et le blanc est ce qu'il montre quand on n'en a pas.
    let mut custom = [COLORREF(0x00FF_FFFF); 16];
    let mut asked = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        rgbResult: COLORREF(initial.map(to_colorref).unwrap_or(0)),
        lpCustColors: custom.as_mut_ptr(),
        Flags: CC_FULLOPEN
            | if initial.is_some() {
                CC_RGBINIT
            } else {
                Default::default()
            },
        ..Default::default()
    };
    // SAFETY: la structure est remplie et son tableau vit jusqu'au retour.
    if unsafe { ChooseColorW(&mut asked) }.as_bool() {
        return Ok(Some(from_colorref(asked.rgbResult.0)));
    }
    // Zéro veut dire « annulé » ; autre chose est une vraie panne.
    // SAFETY: sans argument.
    match unsafe { CommDlgExtendedError() }.0 {
        0 => Ok(None),
        code => Err(OsError::Platform(format!(
            "the color dialog failed (CommDlgExtendedError {code:#x})"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorref_is_blue_green_red() {
        let orange = Rgb {
            r: 0xFF,
            g: 0x80,
            b: 0x10,
        };
        assert_eq!(to_colorref(orange), 0x0010_80FF);
        assert_eq!(from_colorref(0x0010_80FF), orange);
    }
}
