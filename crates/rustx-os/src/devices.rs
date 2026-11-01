//! What is plugged in.
//!
//! A list, with an id that stays the same from one enumeration to the
//! next — because `/device/[id]` is a path in a URL, and an id that
//! changed between the list and the screen it opens would open nothing.
//!
//! A device somebody else is holding is listed, with `busy` set. Hiding
//! it would send a developer looking for a bug in their own code when
//! the answer is that a meeting has the camera.

use crate::error::OsError;

/// What kind of thing it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Camera,
    Microphone,
    Screen,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(match self {
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::Screen => "screen",
        })
    }
}

/// One thing plugged in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// What a program keeps to come back to this device: the Media
    /// Foundation symbolic link for a camera or a microphone, the
    /// display's device name for a screen.
    pub id: String,
    /// What a person reads: "Integrated Webcam", "\\.\DISPLAY1".
    pub name: String,
    pub kind: Kind,
    /// Something else has it open.
    pub busy: bool,
}

/// Everything plugged in, of every kind.
pub fn devices() -> Vec<Device> {
    let mut all = Vec::new();
    for kind in [Kind::Camera, Kind::Microphone, Kind::Screen] {
        all.extend(devices_of(kind));
    }
    all
}

/// Everything of one kind.
///
/// An empty list is an answer: a machine with no webcam is an ordinary
/// machine, not a failure, so nothing here returns an error.
pub fn devices_of(kind: Kind) -> Vec<Device> {
    crate::platform::devices::devices_of(kind)
}

/// Say out loud what the platform answered while enumerating.
///
/// An empty list has two causes — a machine with nothing plugged in, and
/// a subsystem that refused — and they look identical from the outside.
/// This is how a developer, or this project's own example, tells them
/// apart.
pub fn explain() {
    crate::platform::devices::explain();
}

/// The device of this id, as it is now.
pub fn device(id: &str) -> Result<Device, OsError> {
    devices()
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| OsError::NoSuchDevice(id.to_string()))
}

