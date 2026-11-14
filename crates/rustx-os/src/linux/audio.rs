//! Le son, sous Linux : `pactl`, le client de PulseAudio et de
//! PipeWire.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; la
//! lecture de ce que `pactl` écrit a ses tests, qui tournent partout.
//! Aucun volume n'a été réglé sur une vraie machine.
//!
//! # Pourquoi `pactl` et pas ALSA
//!
//! La même raison que `xclip` pour le presse-papiers : c'est le
//! programme que toute distribution de bureau porte déjà. PipeWire,
//! qui a remplacé PulseAudio presque partout, fournit `pactl` et
//! répond au même protocole exprès. ALSA serait plus bas, lierait une
//! bibliothèque C, et verrait des périphériques que l'utilisateur ne
//! reconnaîtrait pas (`front:CARD=PCH,DEV=0`).

use std::process::{Command, Stdio};

use crate::audio::{Endpoint, Flow, Volume, percent};
use crate::error::OsError;

fn pactl(arguments: &[&str]) -> Result<String, OsError> {
    let out = Command::new("pactl")
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            OsError::Platform(format!(
                "`pactl` could not be run ({error}) — it comes with PulseAudio and PipeWire"
            ))
        })?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(OsError::Platform(format!(
            "`pactl {}` failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

fn kind(flow: Flow) -> &'static str {
    match flow {
        Flow::Output => "sinks",
        Flow::Input => "sources",
    }
}

fn one(flow: Flow) -> &'static str {
    match flow {
        Flow::Output => "sink",
        Flow::Input => "source",
    }
}

/// Ce que `pactl list short sinks` écrit : un numéro, le nom, le
/// module, le format, l'état, séparés par des tabulations. Le nom —
/// la deuxième colonne — est la clé que `pactl` reprend partout
/// ailleurs.
pub(crate) fn short_list(text: &str, flow: Flow, default: &str) -> Vec<Endpoint> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let _number = fields.next()?;
            let name = fields.next()?.trim();
            (!name.is_empty()).then(|| Endpoint {
                id: name.to_string(),
                // `pactl list short` ne donne pas de nom lisible : le nom
                // technique est ce qu'il y a, et c'est celui que `pavucontrol`
                // montre aussi en seconde ligne.
                name: name.to_string(),
                flow,
                default: name == default.trim(),
            })
        })
        .collect()
}

pub fn endpoints(flow: Flow) -> Result<Vec<Endpoint>, OsError> {
    let default = pactl(&[&format!("get-default-{}", one(flow))]).unwrap_or_default();
    let listed = pactl(&["list", "short", kind(flow)])?;
    Ok(short_list(&listed, flow, &default))
}

/// Ce que `pactl get-sink-volume @DEFAULT_SINK@` écrit :
///
/// ```text
/// Volume: front-left: 45875 /  70% / -9.33 dB,   front-right: 45875 /  70% / -9.33 dB
/// ```
///
/// Le premier pour-cent est pris : les canaux d'un même périphérique
/// sont au même niveau sauf si quelqu'un les a séparés à la main, et
/// « le volume » est alors une question sans réponse unique.
pub(crate) fn read_level(text: &str) -> Option<f64> {
    let (before, _) = text.split_once('%')?;
    // Les chiffres juste avant le `%`, lus à l'envers puis remis dans le
    // bon sens : ce qui les précède est un niveau brut et un espace.
    let digits: String = before
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let percent: u32 = digits.parse().ok()?;
    Some(f64::from(percent.min(100)) / 100.0)
}

/// Ce que `pactl get-sink-mute` écrit : `Mute: yes` ou `Mute: no`.
pub(crate) fn read_muted(text: &str) -> Option<bool> {
    let (_, value) = text.split_once(':')?;
    match value.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

pub fn volume() -> Result<Volume, OsError> {
    let level = pactl(&["get-sink-volume", "@DEFAULT_SINK@"])?;
    let muted = pactl(&["get-sink-mute", "@DEFAULT_SINK@"])?;
    Ok(Volume {
        level: read_level(&level).ok_or_else(|| {
            OsError::Platform(format!("`{}` is not a volume", level.trim()))
        })?,
        muted: read_muted(&muted).ok_or_else(|| {
            OsError::Platform(format!("`{}` is not a mute state", muted.trim()))
        })?,
    })
}

pub fn set_volume(level: f64) -> Result<(), OsError> {
    pactl(&[
        "set-sink-volume",
        "@DEFAULT_SINK@",
        &format!("{}%", percent(level)),
    ])
    .map(|_| ())
}

pub fn set_muted(muted: bool) -> Result<(), OsError> {
    pactl(&[
        "set-sink-mute",
        "@DEFAULT_SINK@",
        if muted { "1" } else { "0" },
    ])
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINKS: &str = concat!(
        "0\talsa_output.pci-0000_00_1f.3.analog-stereo\tPipeWire\ts16le 2ch 48000Hz\tRUNNING\n",
        "1\talsa_output.usb-Focusrite\tPipeWire\ts32le 2ch 48000Hz\tSUSPENDED\n"
    );

    #[test]
    fn the_second_column_is_the_key_and_the_default_is_marked() {
        let all = short_list(SINKS, Flow::Output, "alsa_output.usb-Focusrite\n");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "alsa_output.pci-0000_00_1f.3.analog-stereo");
        assert!(!all[0].default);
        assert!(all[1].default, "the default sink is the one pactl named");
        assert!(all.iter().all(|endpoint| endpoint.flow == Flow::Output));
    }

    #[test]
    fn an_empty_list_is_no_endpoints_not_a_failure() {
        assert_eq!(short_list("", Flow::Input, ""), Vec::new());
    }

    #[test]
    fn the_volume_is_read_as_the_first_channels_percentage() {
        let said = "Volume: front-left: 45875 /  70% / -9.33 dB,   \
                    front-right: 45875 /  70% / -9.33 dB";
        assert_eq!(read_level(said), Some(0.70));
        assert_eq!(read_level("Volume: mono: 0 / 0% / -inf dB"), Some(0.0));
        assert_eq!(read_level("Volume: mono: 65536 / 100% / 0.00 dB"), Some(1.0));
        assert_eq!(read_level("no percentage here"), None);
    }

    #[test]
    fn the_mute_state_is_yes_or_no_and_nothing_else() {
        assert_eq!(read_muted("Mute: yes"), Some(true));
        assert_eq!(read_muted("Mute: no\n"), Some(false));
        assert_eq!(read_muted("Mute: maybe"), None);
        assert_eq!(read_muted("nonsense"), None);
    }
}
