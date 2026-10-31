//! The devices, on Linux.
//!
//! Not yet, and it says so. What this will be is V4L2 for the cameras
//! (`/dev/video*` and `VIDIOC_QUERYCAP`), ALSA or PipeWire for the
//! microphones, and the compositor for the screens — one file, this
//! one, with the API above it unchanged.

use crate::devices::{Device, Kind};

pub fn explain() {
    println!(
        "the device layer is Windows for now; this is {} — \
         `linux/devices.rs` is the one file that has to be written",
        std::env::consts::OS
    );
}

/// Nothing is listed, so a page that asks gets an empty list rather
/// than an invented one. What is actually *asked of* a device refuses
/// by name, through `OsError::Unsupported`.
pub fn devices_of(_kind: Kind) -> Vec<Device> {
    Vec::new()
}
