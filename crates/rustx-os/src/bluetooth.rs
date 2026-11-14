//! Les appareils Bluetooth que l'utilisateur a appairés.
//!
//! Lire, et rien de plus : la liste de ce qui est appairé, et lequel est
//! connecté en ce moment. Ce qu'une application en fait, c'est nommer le
//! casque, la manette ou la balance dans sa propre interface au lieu de
//! montrer une adresse hexadécimale.
//!
//! # Ce que ce fichier ne fait pas, et pourquoi
//!
//! Ni appairer, ni connecter, ni échanger des données. Appairer est un
//! dialogue entre l'utilisateur, le système et l'appareil — un code à
//! confirmer des deux côtés — et c'est le panneau du système qui le
//! mène ; une application qui le refait mal rend l'appareil
//! inutilisable ailleurs. Échanger des données veut dire GATT ou RFCOMM,
//! un protocole par appareil, et c'est une bibliothèque, pas une
//! fonction.
//!
//! Le Bluetooth est le seul endroit de ce crate où l'absence de matériel
//! et l'absence de réponse se confondent : une machine sans radio
//! Bluetooth rend une liste vide, pas un refus, parce que « aucun
//! appareil appairé » est la réponse juste dans les deux cas.

use crate::error::OsError;

/// Un appareil appairé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paired {
    /// L'adresse de l'appareil, `AA:BB:CC:DD:EE:FF` en majuscules : la
    /// seule clé stable, et la même sur les trois systèmes.
    pub address: String,
    /// Ce que l'utilisateur lit, ou l'adresse si l'appareil n'a pas
    /// donné de nom.
    pub name: String,
    /// Connecté en ce moment.
    pub connected: bool,
}

/// Les appareils appairés. Une liste vide est une réponse.
pub fn paired() -> Result<Vec<Paired>, OsError> {
    crate::blocking::waiting("the list of paired devices");
    crate::platform::bluetooth::paired()
}

/// Les six octets d'une adresse Bluetooth, écrits comme le système les
/// montre.
///
/// L'ordre est voulu : Windows tient l'adresse dans un entier de 64
/// bits dont l'octet de poids faible est le *dernier* du nom affiché,
/// et c'est là que se trompe tout code qui lit cet entier à l'envers.
pub(crate) fn address_of(bytes: [u8; 6]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

// Lue par `linux/` et `macos/`, qui lisent du texte ; Windows a six
// octets.
#[cfg_attr(windows, allow(dead_code))]
/// Une adresse telle qu'un outil l'a écrite, normalisée.
///
/// `None` pour ce qui n'en est pas une : six paires hexadécimales
/// séparées par des deux-points ou des tirets, et rien d'autre.
pub(crate) fn clean_address(text: &str) -> Option<String> {
    let pieces: Vec<&str> = text.trim().split([':', '-']).collect();
    if pieces.len() != 6 {
        return None;
    }
    let mut bytes = [0u8; 6];
    for (at, piece) in pieces.iter().enumerate() {
        if piece.len() != 2 {
            return None;
        }
        bytes[at] = u8::from_str_radix(piece, 16).ok()?;
    }
    Some(address_of(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_is_written_most_significant_byte_first() {
        assert_eq!(
            address_of([0xAC, 0xDE, 0x48, 0x00, 0x11, 0x22]),
            "AC:DE:48:00:11:22"
        );
        assert_eq!(address_of([0; 6]), "00:00:00:00:00:00");
    }

    #[test]
    fn an_address_is_normalised_whatever_separator_and_case_it_came_in() {
        assert_eq!(clean_address("ac:de:48:00:11:22").as_deref(), Some("AC:DE:48:00:11:22"));
        assert_eq!(clean_address("AC-DE-48-00-11-22").as_deref(), Some("AC:DE:48:00:11:22"));
        assert_eq!(clean_address(" ac:de:48:00:11:22 \n").as_deref(), Some("AC:DE:48:00:11:22"));
    }

    #[test]
    fn what_is_not_an_address_is_not_one() {
        assert_eq!(clean_address("AC:DE:48:00:11"), None, "five bytes is not an address");
        assert_eq!(clean_address("AC:DE:48:00:11:22:33"), None);
        assert_eq!(clean_address("ZZ:DE:48:00:11:22"), None);
        assert_eq!(clean_address("A:B:C:D:E:F"), None, "a byte is two digits");
        assert_eq!(clean_address(""), None);
    }

    #[test]
    fn this_machine_lists_its_paired_devices_or_refuses_by_name() {
        match paired() {
            Ok(all) => {
                for device in &all {
                    assert_eq!(
                        clean_address(&device.address).as_deref(),
                        Some(device.address.as_str()),
                        "an address comes out normalised: {device:?}"
                    );
                    assert!(!device.name.is_empty(), "{device:?}");
                }
            }
            Err(refused) => assert!(!refused.to_string().is_empty()),
        }
    }
}
