//! The devices, on macOS: cameras, from `system_profiler`'s own JSON.
//!
//! `system_profiler SPCameraDataType -json` is Apple's own
//! introspection tool, already on every Mac, and it is what System
//! Information itself calls to fill its "Camera" page — reading its
//! output is reading the same source System Information does, not a
//! guess at one. `AVCaptureDevice.devices(for: .video)` is the real,
//! live API (and the one to reach for the moment a page needs to
//! *open* a camera and not just list it), but it is `AVFoundation`,
//! which asks for Objective-C bindings this crate does not have, and
//! for the TCC camera permission besides.
//!
//! # Read by hand, like the rest of this crate's JSON
//!
//! `bundle.rs`, in the compiler, already reads a `package.json` by
//! hand rather than carrying a JSON crate into this project — the
//! same reasoning applies here, on a smaller and more regular
//! document: every camera is one flat object with `_name` and
//! `spcamera_unique-id` string fields, so finding each key and taking
//! the quoted text after it is the whole parser.
//!
//! # Not measured, and said rather than guessed
//!
//! `busy` is always `false`, the same honesty as the Linux side's:
//! telling whether a camera is already open by another process is
//! further work, not yet done, and a guess dressed as a measurement
//! is worse than admitting it is not read.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! macOS, and there is no Apple target here to even type-check it
//! against — only read over by hand, unlike the Linux files.

use std::process::{Command, Stdio};

use crate::devices::{Device, Kind};

pub fn explain() {
    println!(
        "cameras are read from `system_profiler SPCameraDataType` on {}; microphones and \
         screens are not written yet",
        std::env::consts::OS
    );
}

pub fn devices_of(kind: Kind) -> Vec<Device> {
    match kind {
        Kind::Camera => cameras(),
        Kind::Microphone | Kind::Screen => Vec::new(),
    }
}

fn cameras() -> Vec<Device> {
    let Ok(out) = Command::new("system_profiler")
        .arg("SPCameraDataType")
        .arg("-json")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    cameras_from(&text)
}

/// The cameras named in `system_profiler`'s own JSON, parsed by hand.
///
/// Separated from running the command for the same reason as every
/// other platform file's split between "what is run" and "what is
/// parsed": this half is what a test on any machine can read.
fn cameras_from(json: &str) -> Vec<Device> {
    let names = quoted_values_after(json, "\"_name\"");
    let ids = quoted_values_after(json, "\"spcamera_unique-id\"");
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let id = ids.get(index).cloned().unwrap_or_else(|| name.clone());
            Device {
                id,
                name,
                kind: Kind::Camera,
                busy: false,
            }
        })
        .collect()
}

/// Every quoted string that follows `key` and a colon, in document
/// order.
fn quoted_values_after(json: &str, key: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = json;
    while let Some(at) = rest.find(key) {
        rest = &rest[at + key.len()..];
        let Some(colon) = rest.find(':') else { break };
        let after_colon = rest[colon + 1..].trim_start();
        let Some(value) = after_colon.strip_prefix('"') else {
            rest = after_colon;
            continue;
        };
        let Some(end) = value.find('"') else { break };
        found.push(value[..end].to_string());
        rest = &value[end + 1..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "SPCameraDataType" : [
    {
      "_name" : "FaceTime HD Camera",
      "spcamera_model-id" : "Built-in",
      "spcamera_unique-id" : "0x1420000005ac8514"
    }
  ]
}"#;

    #[test]
    fn a_camera_is_read_with_its_name_and_its_stable_id() {
        let cameras = cameras_from(SAMPLE);
        assert_eq!(cameras.len(), 1, "got {cameras:?}");
        assert_eq!(cameras[0].name, "FaceTime HD Camera");
        assert_eq!(cameras[0].id, "0x1420000005ac8514");
        assert_eq!(cameras[0].kind, Kind::Camera);
        assert!(!cameras[0].busy);
    }

    #[test]
    fn no_camera_is_an_empty_list_not_a_failure() {
        let empty = r#"{"SPCameraDataType" : []}"#;
        assert_eq!(cameras_from(empty), Vec::new());
    }

    #[test]
    fn microphones_and_screens_are_an_empty_list_not_a_failure() {
        assert_eq!(devices_of(Kind::Microphone), Vec::new());
        assert_eq!(devices_of(Kind::Screen), Vec::new());
    }
}
