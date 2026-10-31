//! Un raccourci qui marche quand la fenêtre n'a pas le focus.
//!
//! Ce qu'une page ne peut pas avoir : elle ne reçoit les touches que
//! quand elle est devant. Un logiciel qui capture l'écran, enregistre
//! le son, ou montre une fenêtre à la demande a besoin de l'inverse —
//! une combinaison que le système lui donne où qu'on soit.
//!
//! # Écrit comme sur un autocollant
//!
//! `"Ctrl+Shift+R"`. C'est ce qu'un utilisateur lit dans une
//! préférence, donc c'est ce qu'un développeur écrit, et [`parse`] le
//! lit — modificateurs et touche. La casse et les espaces ne comptent
//! pas ; ce qui n'est pas une touche connue est refusé en nommant ce
//! qui était écrit, parce qu'un raccourci muet est un défaut qu'on
//! cherche longtemps.
//!
//! # Un jeton, et les touches rendues
//!
//! [`Shortcut`] tient la combinaison. Lâché, il la rend : un raccourci
//! qui survit à l'application est une combinaison que l'utilisateur ne
//! récupère qu'en redémarrant sa session.
//!
//! ```ignore
//! let capture = Shortcut::listen("Ctrl+Shift+R")?;
//! // dans la boucle de la page :
//! if capture.pressed() > 0 { commencer_l_enregistrement(); }
//! ```

use crate::error::OsError;

/// Les modificateurs, tels que le système les compte.
pub const ALT: u32 = 0x0001;
pub const CONTROL: u32 = 0x0002;
pub const SHIFT: u32 = 0x0004;
pub const WIN: u32 = 0x0008;

/// Lire une combinaison écrite à la main.
///
/// Rend les modificateurs et la touche, séparés, pour qu'un test puisse
/// épingler la lecture sans rien enregistrer auprès du système.
pub fn parse(keys: &str) -> Result<(u32, u32), OsError> {
    let mut modifiers = 0;
    let mut key = None;

    for piece in keys.split('+') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        match piece.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= CONTROL,
            "alt" => modifiers |= ALT,
            "shift" => modifiers |= SHIFT,
            "win" | "super" | "cmd" | "meta" => modifiers |= WIN,
            _ => {
                let Some(found) = virtual_key(piece) else {
                    return Err(OsError::Platform(format!(
                        "`{piece}` n'est pas une touche que rustx sait nommer, dans \
                         `{keys}`. Une touche s'écrit `A`…`Z`, `0`…`9`, `F1`…`F24`, \
                         ou l'un de : Space, Tab, Enter, Esc, Backspace, Delete, \
                         Insert, Home, End, PageUp, PageDown, Up, Down, Left, Right."
                    )));
                };
                if key.replace(found).is_some() {
                    return Err(OsError::Platform(format!(
                        "`{keys}` nomme deux touches. Une combinaison porte des \
                         modificateurs et une touche."
                    )));
                }
            }
        }
    }

    let Some(key) = key else {
        return Err(OsError::Platform(format!(
            "`{keys}` n'a pas de touche — seulement des modificateurs. \
             `Ctrl+Shift+R` en est une ; `Ctrl+` n'en est pas une."
        )));
    };
    Ok((modifiers, key))
}

/// La touche virtuelle d'un nom, ou rien.
///
/// Les codes sont ceux de Windows, qui sont aussi ceux que les deux
/// autres plateformes traduiront : une table de correspondance a sa
/// place dans le fichier de la plateforme, pas dans ce qu'un
/// développeur écrit.
fn virtual_key(name: &str) -> Option<u32> {
    let upper = name.to_ascii_uppercase();
    let bytes = upper.as_bytes();

    // Une lettre ou un chiffre : la touche virtuelle est son code ASCII
    // en majuscule, ce qui est une des rares simplicités de cette API.
    if bytes.len() == 1 && (bytes[0].is_ascii_uppercase() || bytes[0].is_ascii_digit()) {
        return Some(u32::from(bytes[0]));
    }

    // `F1` à `F24`.
    if let Some(number) = upper.strip_prefix('F') {
        if let Ok(number) = number.parse::<u32>() {
            if (1..=24).contains(&number) {
                return Some(0x70 + number - 1);
            }
        }
    }

    Some(match upper.as_str() {
        "SPACE" => 0x20,
        "TAB" => 0x09,
        "ENTER" | "RETURN" => 0x0D,
        "ESC" | "ESCAPE" => 0x1B,
        "BACKSPACE" => 0x08,
        "DELETE" | "DEL" => 0x2E,
        "INSERT" | "INS" => 0x2D,
        "HOME" => 0x24,
        "END" => 0x23,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        "LEFT" => 0x25,
        "UP" => 0x26,
        "RIGHT" => 0x27,
        "DOWN" => 0x28,
        _ => return None,
    })
}

/// Si cette plateforme donne des raccourcis globaux depuis ce code.
pub fn supported() -> bool {
    crate::platform::shortcut::AVAILABLE
}

/// Une combinaison tenue. Lâchée, elle est rendue.
///
/// `Debug` dit la combinaison et rien d'autre : ce qui est derrière est
/// un fil et une file de messages, dont il n'y a rien à écrire.
pub struct Shortcut {
    inner: crate::platform::shortcut::Shortcut,
    keys: String,
}

impl Shortcut {
    /// Demander la combinaison au système.
    ///
    /// `OsError::Busy` quand une autre application la tient déjà — ce
    /// qui arrive, et ce qu'une application doit pouvoir dire à son
    /// utilisateur au lieu de ne simplement pas marcher.
    pub fn listen(keys: &str) -> Result<Self, OsError> {
        let (modifiers, key) = parse(keys)?;
        Ok(Self {
            inner: crate::platform::shortcut::Shortcut::listen(modifiers, key, keys)?,
            keys: keys.to_string(),
        })
    }

    /// Combien d'appuis depuis la dernière lecture.
    ///
    /// Un compte et non un drapeau : deux appuis rapprochés sont deux
    /// appuis. Lire remet à zéro, et lire sans appui rend zéro sans
    /// rien attendre.
    pub fn pressed(&self) -> u64 {
        self.inner.pressed()
    }

    /// La combinaison, telle qu'elle a été écrite.
    pub fn keys(&self) -> &str {
        &self.keys
    }
}

impl std::fmt::Debug for Shortcut {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "Shortcut({})", self.keys)
    }
}
