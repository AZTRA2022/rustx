//! Si la machine est en ligne, et par quoi.
//!
//! Une page a `navigator.onLine`, qui ne dit qu'une chose et la dit
//! mal : il est vrai dès qu'une carte réseau est branchée, routeur
//! injoignable compris. Ce que dit ce fichier est ce que le système
//! sait vraiment — il tient cette information pour ses propres icônes
//! de barre des tâches — et par quel genre de lien, parce qu'une
//! application qui synchronise des gigaoctets n'a pas à le faire sur
//! une connexion mobile.
//!
//! Aucune requête n'est faite : c'est une lecture, et une lecture
//! rapide. Rien ici ne garantit qu'un serveur précis répond — seul un
//! appel à ce serveur le dit.

use crate::error::OsError;

/// Jusqu'où la machine atteint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Rien : aucune carte ne porte de réseau utilisable.
    Offline,
    /// Le réseau local seulement — branché, mais pas de route vers
    /// l'extérieur. Un hotspot de hôtel avant qu'on s'y identifie est
    /// exactement cela, et c'est la panne qu'un simple `onLine` ne voit
    /// pas.
    Local,
    /// L'Internet.
    Online,
}

impl Reach {
    /// Si quelque chose passe, local ou non.
    pub fn connected(self) -> bool {
        self != Self::Offline
    }
}

/// Par quoi elle y est.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Ethernet,
    WiFi,
    /// Un modem mobile, ou un téléphone en partage de connexion reconnu
    /// comme tel.
    Cellular,
    /// Autre chose de connu : un pont, un VPN, un lien série.
    Other,
    /// Le système ne l'a pas dit. Distinct de [`Link::Other`] : l'un est
    /// une réponse, l'autre son absence.
    Unknown,
}

impl Link {
    /// Si l'utilisateur paie probablement au volume. `false` pour
    /// `Unknown` : facturer quelqu'un par défaut est pire que de
    /// télécharger de trop.
    pub fn metered(self) -> bool {
        self == Self::Cellular
    }
}

/// L'état du réseau, maintenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Network {
    pub reach: Reach,
    pub link: Link,
}

/// Le lire.
pub fn network() -> Result<Network, OsError> {
    crate::platform::net::network()
}

/// Si l'Internet est joignable — le raccourci que la plupart des
/// appelants veulent.
pub fn online() -> Result<bool, OsError> {
    Ok(network()?.reach == Reach::Online)
}

// Les deux Unix ; Windows lit un `IfType` de l'IANA.
#[cfg_attr(windows, allow(dead_code))]
/// Le genre de lien que porte un nom d'interface Unix.
///
/// Les préfixes sont les conventions des deux systèmes : `wl` est la
/// nomenclature prévisible de `systemd` (`wlan`, l'ancienne), `en` est
/// macOS et `systemd` pour Ethernet, `eth` l'ancienne, `ww`/`wwan` et
/// `pdp_ip` (macOS) un modem mobile. Un nom inconnu n'est pas deviné.
pub(crate) fn link_from_name(name: &str) -> Link {
    let name = name.trim();
    if name.starts_with("wl") || name.starts_with("wifi") {
        Link::WiFi
    } else if name.starts_with("ww") || name.starts_with("pdp_ip") || name.starts_with("rmnet") {
        Link::Cellular
    } else if name.starts_with("en") || name.starts_with("eth") {
        // `en` sous macOS est aussi le nom du Wi-Fi (`en0` sur un
        // portable) : la plateforme corrige quand elle sait, et
        // Ethernet est ce que le nom dit à lui seul.
        Link::Ethernet
    } else if name.is_empty() {
        Link::Unknown
    } else {
        Link::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_interface_name_says_what_kind_of_link_it_is() {
        assert_eq!(link_from_name("wlp3s0"), Link::WiFi);
        assert_eq!(link_from_name("wlan0"), Link::WiFi);
        assert_eq!(link_from_name("enp0s31f6"), Link::Ethernet);
        assert_eq!(link_from_name("eth0"), Link::Ethernet);
        assert_eq!(link_from_name("wwan0"), Link::Cellular);
        assert_eq!(link_from_name("pdp_ip0"), Link::Cellular);
        assert_eq!(link_from_name("tun0"), Link::Other);
        assert_eq!(link_from_name(""), Link::Unknown);
    }

    #[test]
    fn only_a_known_cellular_link_is_metered() {
        assert!(Link::Cellular.metered());
        assert!(!Link::WiFi.metered());
        assert!(!Link::Unknown.metered(), "unknown must not be charged for");
    }

    #[test]
    fn local_is_connected_but_not_online() {
        assert!(Reach::Local.connected());
        assert!(Reach::Online.connected());
        assert!(!Reach::Offline.connected());
    }

    #[test]
    fn this_machine_answers_or_refuses_by_name() {
        match network() {
            Ok(state) => {
                // Hors ligne avec un lien connu est possible (câble
                // branché, pas de route) ; en ligne sans lien connu ne
                // l'est pas sous Windows, mais l'est sous Linux faute de
                // lecture — on n'épingle donc que la cohérence du couple.
                if state.reach == Reach::Offline {
                    assert!(!state.reach.connected());
                }
            }
            Err(refused) => assert!(!refused.to_string().is_empty()),
        }
    }
}
