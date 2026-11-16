//! The network, on macOS: the routing table, and the hardware ports.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the documented output of `route`
//! and `networksetup`. Never run on a Mac; both parsers are tested on
//! every machine against sample output written from that documentation,
//! not from output this machine has seen.
//!
//! # Why not `SCNetworkReachability`
//!
//! It is the right API, and it answers with the authority Windows'
//! Network List Manager has. It is also `CFRunLoop`-shaped (a callback
//! and a scheduled source) for anything beyond a flags read, and the
//! synchronous read is the one Apple documents as not to use. Reading
//! the routing table is a smaller, honest answer, and its limit is
//! said here: it does not say whether the Internet *answers*. A
//! captive portal has a default route too.

use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::net::{Link, Network, Reach, link_from_name};

fn said(program: &str, arguments: &[&str]) -> Option<String> {
    let out = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

/// The interface of the default route, from `route -n get default`,
/// whose output is a block of `key: value` lines including
/// `interface: en0`.
pub(crate) fn default_interface(route: &str) -> Option<&str> {
    route.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim() != "interface" {
            return None;
        }
        let name = value.trim();
        (!name.is_empty()).then_some(name)
    })
}

/// What `networksetup -listallhardwareports` says this device is: the
/// output pairs a `Hardware Port:` line with the `Device:` line under
/// it.
///
/// Needed because `en0` is Wi-Fi on a laptop and Ethernet on a desktop:
/// the name alone cannot tell, and this table can.
pub(crate) fn port_of(ports: &str, device: &str) -> Option<Link> {
    let mut port: Option<&str> = None;
    for line in ports.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "Hardware Port" => port = Some(value.trim()),
            "Device" if value.trim() == device => {
                let name = port?.to_ascii_lowercase();
                return Some(if name.contains("wi-fi") || name.contains("airport") {
                    Link::WiFi
                } else if name.contains("ethernet") || name.contains("lan") {
                    Link::Ethernet
                } else if name.contains("iphone") || name.contains("modem") {
                    Link::Cellular
                } else {
                    Link::Other
                });
            }
            _ => {}
        }
    }
    None
}

pub fn network() -> Result<Network, OsError> {
    let route = said("route", &["-n", "get", "default"]).unwrap_or_default();
    let Some(interface) = default_interface(&route) else {
        return Ok(Network {
            reach: Reach::Offline,
            link: Link::Unknown,
        });
    };
    let link = said("networksetup", &["-listallhardwareports"])
        .and_then(|ports| port_of(&ports, interface))
        .unwrap_or_else(|| link_from_name(interface));
    Ok(Network {
        reach: Reach::Online,
        link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_is_read_out_of_the_route() {
        let route = "   route to: default\ndestination: default\n   gateway: 192.168.1.1\n  \
                     interface: en0\n";
        assert_eq!(default_interface(route), Some("en0"));
        assert_eq!(default_interface(""), None);
        assert_eq!(default_interface("route: writing to routing socket"), None);
    }

    #[test]
    fn en0_is_wifi_or_ethernet_by_the_table_not_by_its_name() {
        let ports = concat!(
            "Hardware Port: Wi-Fi\nDevice: en0\nEthernet Address: 00:00\n\n",
            "Hardware Port: Thunderbolt Ethernet\nDevice: en5\nEthernet Address: 00:01\n"
        );
        assert_eq!(port_of(ports, "en0"), Some(Link::WiFi));
        assert_eq!(port_of(ports, "en5"), Some(Link::Ethernet));
        assert_eq!(
            port_of(ports, "en9"),
            None,
            "a device not listed is not guessed"
        );
        // And the name alone would have been wrong:
        assert_eq!(link_from_name("en0"), Link::Ethernet);
    }
}
