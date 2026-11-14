//! Le Bluetooth, sous Windows : `BluetoothFindFirstDevice`.
//!
//! « Appairé » se dit `fRemembered` ou `fAuthenticated` selon le genre
//! d'appareil : un casque passe par un code et est *authentifié* ; une
//! souris sans clavier ni écran s'appaire sans code et n'est que
//! *mémorisée*. Les deux sont demandés, parce que les deux sont dans le
//! panneau Bluetooth de l'utilisateur.
//!
//! `fIssueInquiry` est à faux : une recherche active prend des secondes
//! et allume la radio pour écouter l'air, ce qui n'est pas ce qu'on
//! demande. Ici on lit la liste que Windows tient déjà.

use windows::Win32::Devices::Bluetooth::{
    BLUETOOTH_DEVICE_INFO, BLUETOOTH_DEVICE_SEARCH_PARAMS, BluetoothFindDeviceClose,
    BluetoothFindFirstDevice, BluetoothFindNextDevice, HBLUETOOTH_DEVICE_FIND,
};

use crate::bluetooth::{Paired, address_of};
use crate::error::OsError;

/// Une recherche ouverte, refermée à la sortie.
struct Search(HBLUETOOTH_DEVICE_FIND);

impl Drop for Search {
    fn drop(&mut self) {
        // SAFETY: la poignée que `BluetoothFindFirstDevice` a rendue,
        // refermée une fois.
        let _ = unsafe { BluetoothFindDeviceClose(self.0) };
    }
}

/// L'adresse qu'un `BLUETOOTH_ADDRESS` porte : un entier de 64 bits
/// dont les six octets de poids faible sont l'adresse, le plus
/// significatif d'abord à l'affichage.
pub(crate) fn address_from(packed: u64) -> String {
    let bytes = packed.to_be_bytes();
    address_of([bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]])
}

/// Le nom d'un appareil, terminé par zéro dans un tableau fixe.
fn name_from(name: &[u16; 248], fallback: &str) -> String {
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let text = String::from_utf16_lossy(&name[..end]).trim().to_string();
    if text.is_empty() {
        fallback.to_string()
    } else {
        text
    }
}

fn one(found: &BLUETOOTH_DEVICE_INFO) -> Paired {
    // SAFETY: l'adresse est une union de l'entier et des six octets ;
    // lire l'entier est toujours valide, les deux champs ayant la même
    // taille et aucun bit invalide.
    let packed = unsafe { found.Address.Anonymous.ullLong };
    let address = address_from(packed);
    Paired {
        name: name_from(&found.szName, &address),
        address,
        connected: found.fConnected.as_bool(),
    }
}

pub fn paired() -> Result<Vec<Paired>, OsError> {
    let asked = BLUETOOTH_DEVICE_SEARCH_PARAMS {
        dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
        fReturnAuthenticated: true.into(),
        fReturnRemembered: true.into(),
        fReturnUnknown: false.into(),
        fReturnConnected: true.into(),
        fIssueInquiry: false.into(),
        cTimeoutMultiplier: 0,
        // Toutes les radios : une machine peut avoir un adaptateur
        // interne et une clé USB.
        hRadio: Default::default(),
    };
    let mut found = BLUETOOTH_DEVICE_INFO {
        dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
        ..Default::default()
    };
    // SAFETY: deux structures de cette pile, chacune avec son `dwSize`
    // rempli, ce que ces API exigent.
    let search = match unsafe { BluetoothFindFirstDevice(&asked, &mut found) } {
        Ok(search) => Search(search),
        // Pas de radio, ou rien d'appairé : une liste vide est la
        // réponse juste dans les deux cas, et un refus serait faux.
        Err(_) => return Ok(Vec::new()),
    };
    let mut all = vec![one(&found)];
    loop {
        found = BLUETOOTH_DEVICE_INFO {
            dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            ..Default::default()
        };
        // SAFETY: la recherche est ouverte, et la structure a son
        // `dwSize`.
        if unsafe { BluetoothFindNextDevice(search.0, &mut found) }.is_err() {
            break;
        }
        all.push(one(&found));
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_low_six_bytes_are_the_address_most_significant_first() {
        // 0x0000_ACDE_4800_1122 doit s'écrire AC:DE:48:00:11:22, et non
        // à l'envers : c'est l'erreur que cette fonction existe pour ne
        // pas faire.
        assert_eq!(address_from(0x0000_ACDE_4800_1122), "AC:DE:48:00:11:22");
        assert_eq!(address_from(0), "00:00:00:00:00:00");
    }

    #[test]
    fn a_nameless_device_is_shown_by_its_address() {
        let mut name = [0u16; 248];
        assert_eq!(name_from(&name, "AC:DE:48:00:11:22"), "AC:DE:48:00:11:22");
        for (at, unit) in "Casque".encode_utf16().enumerate() {
            name[at] = unit;
        }
        assert_eq!(name_from(&name, "unused"), "Casque");
    }
}
