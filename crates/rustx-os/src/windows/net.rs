//! Le réseau, sous Windows : le Network List Manager, et le type des
//! cartes.
//!
//! `INetworkListManager::GetConnectivity` est ce que Windows consulte
//! pour sa propre icône de réseau, globe barré compris : il distingue
//! « branché » de « l'Internet répond », ce qu'aucune lecture de carte
//! ne peut faire.
//!
//! Le genre de lien vient de `GetAdaptersAddresses` : le `IfType` de
//! la première carte active et non virtuelle. « Première » est une
//! approximation assumée — une machine avec un câble *et* du Wi-Fi a
//! deux liens, et cette fonction en rend un.

use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses,
    IF_TYPE_ETHERNET_CSMACD, IF_TYPE_IEEE80211, IF_TYPE_SOFTWARE_LOOPBACK, IF_TYPE_TUNNEL,
    IF_TYPE_WWANPP, IF_TYPE_WWANPP2, IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::NetworkListManager::{
    INetworkListManager, NLM_CONNECTIVITY_IPV4_INTERNET, NLM_CONNECTIVITY_IPV4_LOCALNETWORK,
    NLM_CONNECTIVITY_IPV6_INTERNET, NLM_CONNECTIVITY_IPV6_LOCALNETWORK, NetworkListManager,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};

use crate::error::OsError;
use crate::net::{Link, Network, Reach};

/// Ce que les drapeaux de `NLM_CONNECTIVITY` disent, en clair.
///
/// L'Internet par l'une des deux familles d'adresses suffit : une
/// machine en IPv6 seulement est en ligne.
pub(crate) fn reach_from(flags: i32) -> Reach {
    let internet = NLM_CONNECTIVITY_IPV4_INTERNET.0 | NLM_CONNECTIVITY_IPV6_INTERNET.0;
    let local = NLM_CONNECTIVITY_IPV4_LOCALNETWORK.0 | NLM_CONNECTIVITY_IPV6_LOCALNETWORK.0;
    if flags & internet != 0 {
        Reach::Online
    } else if flags & local != 0 {
        Reach::Local
    } else {
        Reach::Offline
    }
}

/// Le genre de lien d'un `IfType` de l'IANA.
pub(crate) fn link_from_type(kind: u32) -> Option<Link> {
    Some(match kind {
        IF_TYPE_IEEE80211 => Link::WiFi,
        IF_TYPE_ETHERNET_CSMACD => Link::Ethernet,
        IF_TYPE_WWANPP | IF_TYPE_WWANPP2 => Link::Cellular,
        // Ni une boucle locale ni un tunnel ne sont le lien par lequel
        // on sort : une machine derrière un VPN a aussi la carte qui le
        // porte, et c'est elle qui compte.
        IF_TYPE_SOFTWARE_LOOPBACK | IF_TYPE_TUNNEL => return None,
        _ => Link::Other,
    })
}

fn reach() -> Result<Reach, OsError> {
    // SAFETY: COM pour ce fil ; déjà initialisé, l'appel le dit et c'est
    // sans conséquence.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // SAFETY: la classe documentée du Network List Manager.
    let flags = unsafe {
        let manager: INetworkListManager =
            CoCreateInstance(&NetworkListManager, None, CLSCTX_INPROC_SERVER).map_err(|error| {
                OsError::Platform(format!("the Network List Manager refused: {error}"))
            })?;
        manager.GetConnectivity().map_err(|error| {
            OsError::Platform(format!("the Network List Manager refused: {error}"))
        })?
    };
    Ok(reach_from(flags.0))
}

fn link() -> Link {
    let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = 0u32;
    // Un premier appel pour la taille : la liste est une chaîne de
    // structures de longueur variable, et c'est Windows qui la mesure.
    // SAFETY: aucun tampon, et la taille demandée en retour.
    let _ = unsafe { GetAdaptersAddresses(0, flags, None, None, &mut size) };
    if size == 0 {
        return Link::Unknown;
    }
    let mut buffer = vec![0u8; size as usize];
    // SAFETY: un tampon de la taille que Windows vient de dire, aligné
    // comme la structure l'exige puisqu'il vient d'un `Vec<u8>` dont on
    // ne lit que ce que Windows y écrit.
    let written = unsafe {
        GetAdaptersAddresses(
            0,
            flags,
            None,
            Some(buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>()),
            &mut size,
        )
    };
    if written != 0 {
        return Link::Unknown;
    }
    let mut at = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !at.is_null() {
        // SAFETY: `at` est une entrée de la chaîne que Windows a écrite
        // dans ce tampon, et `Next` est son chaînage.
        let adapter = unsafe { &*at };
        if adapter.OperStatus == IfOperStatusUp {
            if let Some(link) = link_from_type(adapter.IfType) {
                return link;
            }
        }
        at = adapter.Next;
    }
    Link::Unknown
}

pub fn network() -> Result<Network, OsError> {
    Ok(Network {
        reach: reach()?,
        link: link(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_internet_wins_over_the_local_network() {
        let both = NLM_CONNECTIVITY_IPV4_LOCALNETWORK.0 | NLM_CONNECTIVITY_IPV4_INTERNET.0;
        assert_eq!(reach_from(both), Reach::Online);
        assert_eq!(reach_from(NLM_CONNECTIVITY_IPV6_INTERNET.0), Reach::Online);
        assert_eq!(reach_from(NLM_CONNECTIVITY_IPV4_LOCALNETWORK.0), Reach::Local);
        assert_eq!(reach_from(0), Reach::Offline);
    }

    #[test]
    fn a_loopback_or_a_tunnel_is_not_the_link_we_go_out_by() {
        assert_eq!(link_from_type(IF_TYPE_SOFTWARE_LOOPBACK), None);
        assert_eq!(link_from_type(IF_TYPE_TUNNEL), None);
        assert_eq!(link_from_type(IF_TYPE_IEEE80211), Some(Link::WiFi));
        assert_eq!(link_from_type(IF_TYPE_ETHERNET_CSMACD), Some(Link::Ethernet));
        assert_eq!(link_from_type(IF_TYPE_WWANPP), Some(Link::Cellular));
        assert_eq!(link_from_type(999), Some(Link::Other));
    }
}
