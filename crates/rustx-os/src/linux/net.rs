//! Le réseau, sous Linux : la table de routage et `/sys/class/net`.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; les deux
//! lectures (`/proc/net/route` et le nom de l'interface) ont leurs
//! tests, qui tournent partout. Rien n'a été lu sur une vraie machine.
//!
//! # Ce qui est lu, et ce qui ne peut pas l'être
//!
//! Une route par défaut dans `/proc/net/route` veut dire qu'il existe
//! un chemin vers l'extérieur ; son interface donne le genre de lien.
//! C'est tout ce que le noyau sait sans parler à personne.
//!
//! Ce que cela ne dit pas : si l'Internet *répond*. `Reach::Online` est
//! donc rendu dès qu'une route par défaut existe, et un portail captif
//! d'hôtel passera pour « en ligne » ici alors que Windows, lui, le
//! verrait. NetworkManager le sait, mais toutes les distributions ne
//! l'ont pas, et une absence de NetworkManager ne doit pas devenir une
//! absence de réseau.

use crate::error::OsError;
use crate::net::{Link, Network, Reach, link_from_name};

/// L'interface de la route par défaut dans `/proc/net/route`.
///
/// Le format : une ligne d'en-tête, puis une ligne par route — le nom
/// de l'interface, la destination en hexadécimal (`00000000` est la
/// route par défaut), la passerelle, les drapeaux.
pub(crate) fn default_route(table: &str) -> Option<&str> {
    table.lines().skip(1).find_map(|line| {
        let mut fields = line.split_whitespace();
        let interface = fields.next()?;
        let destination = fields.next()?;
        (destination == "00000000").then_some(interface)
    })
}

/// Le genre de lien de cette interface : son nom, corrigé par la
/// présence du répertoire `wireless` que le noyau crée pour toute carte
/// sans fil.
pub(crate) fn link_of(interface: &str, wireless: bool) -> Link {
    if wireless {
        Link::WiFi
    } else {
        link_from_name(interface)
    }
}

/// S'il reste un réseau local : une interface autre que la boucle
/// locale, et montée.
fn local_network() -> bool {
    let Ok(all) = std::fs::read_dir("/sys/class/net") else {
        return false;
    };
    all.flatten().any(|entry| {
        entry.file_name() != "lo"
            && std::fs::read_to_string(entry.path().join("operstate"))
                .is_ok_and(|state| state.trim() == "up")
    })
}

pub fn network() -> Result<Network, OsError> {
    let table = std::fs::read_to_string("/proc/net/route")
        .map_err(|error| OsError::Platform(format!("/proc/net/route: {error}")))?;
    let Some(interface) = default_route(&table) else {
        return Ok(Network {
            reach: if local_network() {
                Reach::Local
            } else {
                Reach::Offline
            },
            link: Link::Unknown,
        });
    };
    let wireless = std::path::Path::new("/sys/class/net")
        .join(interface)
        .join("wireless")
        .exists();
    Ok(Network {
        reach: Reach::Online,
        link: link_of(interface, wireless),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = concat!(
        "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\n",
        "enp3s0\t0000A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\n",
        "wlp2s0\t00000000\t0101A8C0\t0003\t0\t0\t600\t00000000\n"
    );

    #[test]
    fn the_default_route_is_the_one_to_nowhere_in_particular() {
        assert_eq!(default_route(TABLE), Some("wlp2s0"));
        assert_eq!(default_route("Iface\tDestination\n"), None);
        assert_eq!(default_route(""), None, "an empty table has no header either");
    }

    #[test]
    fn the_kernels_wireless_directory_overrules_the_name() {
        // Un pont qui porte en fait du Wi-Fi : seul le répertoire le dit.
        assert_eq!(link_of("br0", true), Link::WiFi);
        assert_eq!(link_of("br0", false), Link::Other);
        assert_eq!(link_of("enp3s0", false), Link::Ethernet);
    }
}
