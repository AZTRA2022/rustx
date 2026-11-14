//! Audio, on macOS: `system_profiler` for the list, `osascript` for the
//! volume.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the documented output of
//! `system_profiler SPAudioDataType -json` and from AppleScript's
//! `output volume` / `set volume` commands. Never run on a Mac; both
//! parsers are tested on every machine against sample output written
//! from that documentation, not output this machine has seen.
//!
//! # Why not Core Audio
//!
//! `AudioObjectGetPropertyData` is the real API, and it is the one to
//! reach for the day this crate needs to *open* a stream rather than
//! list and set. It is also a few hundred lines of property-address
//! structs and `CFString` handling that could not be checked here at
//! all, for an answer `system_profiler` already gives — the same
//! reasoning `devices.rs` wrote down for the cameras.
//!
//! # What this costs, said plainly
//!
//! AppleScript's volume is the *system* output volume, and it has no
//! notion of which device: changing it changes whatever is playing.
//! That matches this facade's promise. But `system_profiler`'s JSON
//! has no stable identifier for a device, so `Endpoint::id` is the
//! name — two identically named inputs are indistinguishable here, and
//! Core Audio is what would tell them apart.

use std::process::{Command, Stdio};

use crate::audio::{Endpoint, Flow, Volume, percent};
use crate::error::OsError;

fn said(program: &str, arguments: &[&str]) -> Result<String, OsError> {
    let out = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| OsError::Platform(format!("`{program}` could not be run: {error}")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(OsError::Platform(format!(
            "`{program}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// The audio devices in `system_profiler SPAudioDataType -json`, read
/// by hand the way `devices.rs` reads the cameras.
///
/// Each device is one object with a `_name`, and the keys
/// `coreaudio_device_input` / `coreaudio_device_output` for how many
/// channels it has in each direction — a device with input channels is
/// an input, one with output channels is an output, and an interface
/// with both is both. `coreaudio_default_audio_input_device` (or
/// `_output_`) marks the one the system uses.
pub(crate) fn devices_from(json: &str, flow: Flow) -> Vec<Endpoint> {
    let channels = match flow {
        Flow::Output => "coreaudio_device_output",
        Flow::Input => "coreaudio_device_input",
    };
    let marker = match flow {
        Flow::Output => "coreaudio_default_audio_output_device",
        Flow::Input => "coreaudio_default_audio_input_device",
    };
    // Chaque objet commence à son `_name` : découper là donne un
    // fragment par périphérique, le premier morceau étant l'en-tête.
    let mut found = Vec::new();
    let mut pieces = json.split("\"_name\"");
    let _header = pieces.next();
    for piece in pieces {
        // Le nom est la première chaîne entre guillemets après la clé.
        let Some(name) = quoted_after_colon(piece) else {
            continue;
        };
        // Le fragment s'arrête au périphérique suivant, donc tout ce qui
        // suit appartient à celui-ci.
        if !piece.contains(channels) {
            continue;
        }
        found.push(Endpoint {
            id: name.clone(),
            name,
            flow,
            default: piece.contains(marker),
        });
    }
    found
}

/// The first quoted string after the first colon.
fn quoted_after_colon(text: &str) -> Option<String> {
    let (_, after) = text.split_once(':')?;
    let rest = after.trim_start().strip_prefix('"')?;
    let (value, _) = rest.split_once('"')?;
    (!value.is_empty()).then(|| value.to_string())
}

pub fn endpoints(flow: Flow) -> Result<Vec<Endpoint>, OsError> {
    let json = said("system_profiler", &["SPAudioDataType", "-json"])?;
    Ok(devices_from(&json, flow))
}

/// What `osascript -e 'get volume settings'` writes:
///
/// ```text
/// output volume:70, input volume:33, alert volume:100, output muted:false
/// ```
pub(crate) fn read_volume(settings: &str) -> Option<Volume> {
    let field = |name: &str| {
        settings.split(',').find_map(|part| {
            let (key, value) = part.split_once(':')?;
            (key.trim() == name).then(|| value.trim().to_string())
        })
    };
    let level: u32 = field("output volume")?.parse().ok()?;
    // `output muted` is missing on Macs that cannot report it, and
    // AppleScript then writes `missing value`. A level of zero is the
    // honest reading of "no sound comes out" in that case.
    let muted = match field("output muted").as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => level == 0,
    };
    Some(Volume {
        level: f64::from(level.min(100)) / 100.0,
        muted,
    })
}

pub fn volume() -> Result<Volume, OsError> {
    let settings = said("osascript", &["-e", "get volume settings"])?;
    read_volume(&settings)
        .ok_or_else(|| OsError::Platform(format!("`{}` is not a volume", settings.trim())))
}

pub fn set_volume(level: f64) -> Result<(), OsError> {
    said(
        "osascript",
        &["-e", &format!("set volume output volume {}", percent(level))],
    )
    .map(|_| ())
}

pub fn set_muted(muted: bool) -> Result<(), OsError> {
    let script = format!("set volume output muted {muted}");
    said("osascript", &["-e", &script]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "SPAudioDataType" : [
    {
      "_items" : [
        {
          "_name" : "MacBook Pro Microphone",
          "coreaudio_default_audio_input_device" : "spaudio_yes",
          "coreaudio_device_input" : 1,
          "coreaudio_device_srate" : 48000
        },
        {
          "_name" : "MacBook Pro Speakers",
          "coreaudio_default_audio_output_device" : "spaudio_yes",
          "coreaudio_device_output" : 2
        },
        {
          "_name" : "Focusrite Scarlett",
          "coreaudio_device_input" : 2,
          "coreaudio_device_output" : 2
        }
      ],
      "_name" : "coreaudio_device"
    }
  ]
}"#;

    #[test]
    fn outputs_and_inputs_are_told_apart_by_their_channels() {
        let outputs = devices_from(SAMPLE, Flow::Output);
        let names: Vec<&str> = outputs.iter().map(|each| each.name.as_str()).collect();
        assert!(names.contains(&"MacBook Pro Speakers"), "{names:?}");
        assert!(names.contains(&"Focusrite Scarlett"), "an interface is both: {names:?}");
        assert!(
            !names.contains(&"MacBook Pro Microphone"),
            "a microphone is no output: {names:?}"
        );
    }

    #[test]
    fn the_default_is_the_one_the_marker_names() {
        let inputs = devices_from(SAMPLE, Flow::Input);
        let default: Vec<&str> = inputs
            .iter()
            .filter(|each| each.default)
            .map(|each| each.name.as_str())
            .collect();
        assert_eq!(default, vec!["MacBook Pro Microphone"]);
    }

    #[test]
    fn no_audio_at_all_is_an_empty_list_not_a_failure() {
        assert_eq!(devices_from(r#"{"SPAudioDataType":[]}"#, Flow::Output), Vec::new());
    }

    #[test]
    fn the_volume_settings_line_is_read_field_by_field() {
        let said = "output volume:70, input volume:33, alert volume:100, output muted:false";
        assert_eq!(
            read_volume(said),
            Some(Volume { level: 0.70, muted: false })
        );
        assert_eq!(
            read_volume("output volume:0, output muted:missing value"),
            Some(Volume { level: 0.0, muted: true }),
            "no sound coming out reads as muted when macOS will not say"
        );
        assert_eq!(read_volume("nothing here"), None);
    }
}
