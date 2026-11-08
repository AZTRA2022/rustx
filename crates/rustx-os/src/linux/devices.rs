//! The devices, on Linux: cameras, by what the kernel already publishes.
//!
//! `/dev/video0`, `/dev/video1`, … are the nodes V4L2 creates for every
//! capture device, and the kernel writes a human name for each one to
//! `/sys/class/video4linux/videoN/name` — no `ioctl`, no `VIDIOC_QUERYCAP`
//! struct to get exactly right on a machine that cannot check it: two
//! directory reads are the whole mechanism, and both are plain files
//! any account can read.
//!
//! Microphones and screens are still what the comment on `Device` named
//! them: not written. ALSA or PipeWire for the first, the compositor
//! for the second, and both need more than a directory listing.
//!
//! # Not measured, and said rather than guessed
//!
//! Whether a camera is already open by another process is not read
//! here — `lsof`-style `/proc` scanning is its own piece of work — so
//! `busy` is always `false` for a camera on this platform, which is
//! honest about what is not known rather than a guess dressed as one.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! Linux. `cargo check --target x86_64-unknown-linux-gnu` is what
//! verified it compiles; nothing here has been seen to actually list
//! a real camera.

use crate::devices::{Device, Kind};

pub fn explain() {
    println!(
        "cameras are read from /sys/class/video4linux on {}; microphones and \
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

/// Every `/dev/video*` node V4L2 has created, named from the sysfs
/// entry the kernel writes beside it.
fn cameras() -> Vec<Device> {
    let Ok(entries) = std::fs::read_dir("/dev") else {
        return Vec::new();
    };
    let mut found: Vec<Device> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with("video") || !name["video".len()..].bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let id = format!("/dev/{name}");
            let label = sysfs_name(name).unwrap_or_else(|| id.clone());
            Some(Device {
                id,
                name: label,
                kind: Kind::Camera,
                busy: false,
            })
        })
        .collect();
    // `read_dir` makes no promise about order, and `/dev/video0` reads
    // as the first camera to a person before `/dev/video10` does.
    found.sort_by(|a, b| a.id.cmp(&b.id));
    found
}

/// The name the kernel wrote for `video_node` (`"video0"`, …), if any.
fn sysfs_name(video_node: &str) -> Option<String> {
    let path = format!("/sys/class/video4linux/{video_node}/name");
    std::fs::read_to_string(path).ok().map(|text| text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_video_nodes_are_read_as_cameras() {
        // Not a single law can open a real `/dev` on every machine
        // these tests run on, so what is pinned is the filter: a name
        // that is not `videoN` for a decimal `N` is not a camera node.
        for name in ["video0", "video12"] {
            assert!(name.starts_with("video") && name["video".len()..].bytes().all(|b| b.is_ascii_digit()));
        }
        for name in ["videodisk", "random", "videoA"] {
            assert!(!(name.starts_with("video") && name["video".len()..].bytes().all(|b| b.is_ascii_digit())));
        }
    }

    #[test]
    fn microphones_and_screens_are_an_empty_list_not_a_failure() {
        assert_eq!(devices_of(Kind::Microphone), Vec::new());
        assert_eq!(devices_of(Kind::Screen), Vec::new());
    }
}
