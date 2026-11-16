//! Bluetooth, on macOS: `system_profiler SPBluetoothDataType`.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the shape of `system_profiler
//! SPBluetoothDataType -json`. Never run on a Mac; the parser is tested
//! on every machine against sample output written from that shape, not
//! from output this machine has seen — and this is the file most likely
//! of all of them to be wrong, because Apple has changed this report's
//! key names between macOS versions (`device_title` and
//! `device_connected` on Monterey and later; a flat `device_*` list
//! before). The parser is written to tolerate both, and only a real Mac
//! can say whether it does.
//!
//! `IOBluetooth`'s `IOBluetoothDevice.pairedDevices()` is the API, and
//! the honest reason it is not used is the same as `devices.rs`'s for
//! the cameras: it is an Objective-C framework call returning an
//! `NSArray` of objects whose properties would all have to be read
//! through `msg_send!`, unverifiable from here, for a list
//! `system_profiler` already prints.

use std::process::{Command, Stdio};

use crate::bluetooth::{Paired, clean_address};
use crate::error::OsError;

/// The paired devices in `system_profiler SPBluetoothDataType -json`.
///
/// Each device is a key whose name is the device's own name, holding an
/// object with `device_address` and `device_connected` (`"attrib_Yes"`
/// or `"attrib_No"`). Read the way `devices.rs` reads the cameras: find
/// each address, then look backwards for the name the object hangs
/// under, and forwards inside the object for whether it is connected.
pub(crate) fn devices_from(json: &str) -> Vec<Paired> {
    let mut found = Vec::new();
    let mut rest = json;
    while let Some(at) = rest.find("\"device_address\"") {
        let before = &rest[..at];
        rest = &rest[at + "\"device_address\"".len()..];
        let Some(address) = quoted_after_colon(rest).and_then(|text| clean_address(&text)) else {
            continue;
        };
        // The name is the last quoted key before this object opened.
        let name = before
            .rsplit_once('{')
            .and_then(|(outside, _)| last_quoted(outside))
            .filter(|name| !name.starts_with("device_") && !name.starts_with("SP"))
            .unwrap_or_else(|| address.clone());
        // `device_connected` belongs to this object, so it is whichever
        // comes before the next address.
        let inside = match rest.find("\"device_address\"") {
            Some(next) => &rest[..next],
            None => rest,
        };
        let connected = inside
            .find("\"device_connected\"")
            .and_then(|at| quoted_after_colon(&inside[at + "\"device_connected\"".len()..]))
            .is_some_and(|value| value.eq_ignore_ascii_case("attrib_Yes"));
        found.push(Paired {
            address,
            name,
            connected,
        });
    }
    found
}

/// The first quoted string after the first colon.
fn quoted_after_colon(text: &str) -> Option<String> {
    let (_, after) = text.split_once(':')?;
    let rest = after.trim_start().strip_prefix('"')?;
    let (value, _) = rest.split_once('"')?;
    Some(value.to_string())
}

/// The last quoted string in this text.
fn last_quoted(text: &str) -> Option<String> {
    let (before, _) = text.trim_end().rsplit_once('"')?;
    let (_, value) = before.rsplit_once('"')?;
    (!value.is_empty()).then(|| value.to_string())
}

pub fn paired() -> Result<Vec<Paired>, OsError> {
    let out = Command::new("system_profiler")
        .args(["SPBluetoothDataType", "-json"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            OsError::Platform(format!("`system_profiler` could not be run: {error}"))
        })?;
    if !out.status.success() {
        // A Mac with no Bluetooth radio: an empty list, not a refusal.
        return Ok(Vec::new());
    }
    Ok(devices_from(&String::from_utf8_lossy(&out.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "SPBluetoothDataType" : [
    {
      "device_connected" : [
        {
          "WH-1000XM4" : {
            "device_address" : "ac-de-48-00-11-22",
            "device_connected" : "attrib_Yes",
            "device_minorType" : "attrib_Headphones"
          }
        }
      ],
      "device_not_connected" : [
        {
          "Magic Mouse" : {
            "device_address" : "00-1a-7d-da-71-13",
            "device_connected" : "attrib_No"
          }
        }
      ]
    }
  ]
}"#;

    #[test]
    fn a_device_is_named_by_the_key_it_hangs_under() {
        let all = devices_from(SAMPLE);
        assert_eq!(all.len(), 2, "got {all:#?}");
        assert_eq!(all[0].name, "WH-1000XM4");
        assert_eq!(all[0].address, "AC:DE:48:00:11:22");
        assert_eq!(all[1].name, "Magic Mouse");
    }

    #[test]
    fn connected_is_read_per_device_and_not_from_the_section_above() {
        let all = devices_from(SAMPLE);
        assert!(all[0].connected, "{:#?}", all[0]);
        assert!(!all[1].connected, "{:#?}", all[1]);
    }

    #[test]
    fn no_bluetooth_is_an_empty_list_not_a_failure() {
        assert_eq!(devices_from(r#"{"SPBluetoothDataType":[]}"#), Vec::new());
        assert_eq!(devices_from(""), Vec::new());
    }
}
