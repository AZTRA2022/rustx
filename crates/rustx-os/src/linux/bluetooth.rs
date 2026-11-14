//! Le Bluetooth, sous Linux : `bluetoothctl`, le client de BlueZ.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; la
//! lecture de ce que `bluetoothctl` écrit a ses tests, qui tournent
//! partout. Aucun appareil n'a été listé sur une vraie machine.
//!
//! BlueZ est la pile Bluetooth du noyau Linux, et `bluetoothctl` son
//! client en ligne de commande, livré avec elle : la même porte que
//! `xclip` ou `pactl` ailleurs dans ce crate. Son API D-Bus
//! (`org.bluez`) dirait la même chose, et demanderait une connexion
//! D-Bus que ce crate ne tient pas.

use std::process::{Command, Stdio};

use crate::bluetooth::{Paired, clean_address};
use crate::error::OsError;

fn said(arguments: &[&str]) -> Option<String> {
    let out = Command::new("bluetoothctl")
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// Ce que `bluetoothctl devices Paired` écrit, une ligne par appareil :
///
/// ```text
/// Device AC:DE:48:00:11:22 WH-1000XM4
/// ```
///
/// Un nom peut contenir des espaces, donc tout ce qui suit l'adresse
/// est le nom. Une ligne dont l'adresse n'en est pas une est ignorée :
/// `bluetoothctl` écrit aussi des avertissements sur sa sortie.
pub(crate) fn devices_from(text: &str, connected: &[String]) -> Vec<Paired> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("Device ")?;
            let (address, name) = rest.split_once(' ').unwrap_or((rest, ""));
            let address = clean_address(address)?;
            let name = name.trim();
            Some(Paired {
                connected: connected.contains(&address),
                name: if name.is_empty() {
                    address.clone()
                } else {
                    name.to_string()
                },
                address,
            })
        })
        .collect()
}

/// Les adresses que `bluetoothctl devices Connected` nomme.
pub(crate) fn connected_from(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("Device ")?;
            let address = rest.split_whitespace().next()?;
            clean_address(address)
        })
        .collect()
}

pub fn paired() -> Result<Vec<Paired>, OsError> {
    // `devices Paired` est de BlueZ 5.65 et après ; avant, `paired-devices`
    // faisait la même chose. Les deux sont essayés, dans cet ordre, parce
    // qu'une distribution figée est plus courante qu'un sous-commande
    // manquant.
    let listed = said(&["devices", "Paired"])
        .or_else(|| said(&["paired-devices"]))
        .ok_or(OsError::Unsupported(
            "listing paired Bluetooth devices without `bluetoothctl` (it comes with BlueZ)",
        ))?;
    let connected = said(&["devices", "Connected"])
        .map(|text| connected_from(&text))
        .unwrap_or_default();
    Ok(devices_from(&listed, &connected))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTED: &str = concat!(
        "Device AC:DE:48:00:11:22 WH-1000XM4\n",
        "Device 00:1A:7D:DA:71:13 Magic Mouse 2\n",
        "Device 11:22:33:44:55:66 11-22-33-44-55-66\n"
    );

    #[test]
    fn everything_after_the_address_is_the_name_spaces_included() {
        let all = devices_from(LISTED, &[]);
        assert_eq!(all.len(), 3);
        assert_eq!(all[1].name, "Magic Mouse 2");
        assert_eq!(all[1].address, "00:1A:7D:DA:71:13");
    }

    #[test]
    fn the_connected_list_marks_the_ones_that_are() {
        let connected = connected_from("Device ac:de:48:00:11:22 WH-1000XM4\n");
        assert_eq!(connected, vec!["AC:DE:48:00:11:22"]);
        let all = devices_from(LISTED, &connected);
        assert!(all[0].connected);
        assert!(!all[1].connected);
    }

    #[test]
    fn anything_that_is_not_a_device_line_is_ignored() {
        let noisy = "Waiting to connect to bluetoothd...\nDevice nonsense Name\n[NEW] something\n";
        assert_eq!(devices_from(noisy, &[]), Vec::new());
        assert_eq!(devices_from("", &[]), Vec::new());
    }
}
