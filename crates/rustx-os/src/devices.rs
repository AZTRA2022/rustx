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
    platform::devices_of(kind)
}

/// Say out loud what the platform answered while enumerating.
///
/// An empty list has two causes — a machine with nothing plugged in, and
/// a subsystem that refused — and they look identical from the outside.
/// This is how a developer, or this project's own example, tells them
/// apart.
pub fn explain() {
    platform::explain();
}

/// The device of this id, as it is now.
pub fn device(id: &str) -> Result<Device, OsError> {
    devices()
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| OsError::NoSuchDevice(id.to_string()))
}

#[cfg(windows)]
mod platform {
    use super::{Device, Kind};
    use windows::Win32::Graphics::Gdi::{
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICEW, EnumDisplayDevicesW,
    };
    use windows::Win32::Media::MediaFoundation::{
        IMFActivate, MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE,
        MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_ENDPOINT_ID,
        MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_GUID,
        MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID,
        MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, MFCreateAttributes,
        MFEnumDeviceSources,
    };

    pub fn devices_of(kind: Kind) -> Vec<Device> {
        match kind {
            Kind::Camera | Kind::Microphone => media_devices(kind),
            Kind::Screen => screens(),
        }
    }

    /// The cameras or the microphones, through Media Foundation.
    ///
    /// Media Foundation is started and stopped around the enumeration
    /// rather than for the life of the process: a list of devices is
    /// something a program asks for a few times, and leaving the
    /// platform started for it would make every program that merely
    /// *lists* pay for the whole subsystem.
    fn media_devices(kind: Kind) -> Vec<Device> {
        let _started = crate::windows_media::started();
        let mut found = Vec::new();

        unsafe {
            let mut asked = None;
            if MFCreateAttributes(&mut asked, 1).is_err() {
                return found;
            }
            let Some(asked) = asked else {
                return found;
            };
            let wanted = if kind == Kind::Camera {
                MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID
            } else {
                MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_GUID
            };
            if asked
                .SetGUID(&MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, &wanted)
                .is_err()
            {
                return found;
            }

            let mut sources: *mut Option<IMFActivate> = std::ptr::null_mut();
            let mut count: u32 = 0;
            if MFEnumDeviceSources(&asked, &mut sources, &mut count).is_err() {
                return found;
            }
            if sources.is_null() {
                return found;
            }

            let listed = std::slice::from_raw_parts(sources, count as usize);
            for source in listed.iter().flatten() {
                let name = string_of(source, &MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME);
                let id = string_of(
                    source,
                    if kind == Kind::Camera {
                        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK
                    } else {
                        &MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_ENDPOINT_ID
                    },
                );
                let (Some(id), Some(name)) = (id, name) else {
                    // A device that will not say what it is called is a
                    // device nothing could open twice. Left out rather
                    // than listed as half of one.
                    continue;
                };
                // Whether anybody else has it: the only way to know is
                // to try, so the activation is made and dropped at once.
                let busy = source
                    .ActivateObject::<windows::Win32::Media::MediaFoundation::IMFMediaSource>()
                    .is_err();
                found.push(Device {
                    id,
                    name,
                    kind,
                    busy,
                });
                let _ = source.ShutdownObject();
            }
            windows::Win32::System::Com::CoTaskMemFree(Some(sources as *const _));
        }
        found
    }

    /// One `Device` per screen attached to the desktop.
    fn screens() -> Vec<Device> {
        let mut found = Vec::new();
        let mut which = 0_u32;
        loop {
            let mut about = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            let more = unsafe { EnumDisplayDevicesW(None, which, &mut about, 0) };
            if !more.as_bool() {
                break;
            }
            which += 1;
            if about.StateFlags.0 & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP.0 == 0 {
                continue;
            }
            let id = from_wide(&about.DeviceName);
            if id.is_empty() {
                continue;
            }
            let name = {
                let friendly = from_wide(&about.DeviceString);
                if friendly.is_empty() {
                    id.clone()
                } else {
                    friendly
                }
            };
            found.push(Device {
                id,
                name,
                kind: Kind::Screen,
                // A screen is never held: everybody draws on it at once.
                busy: false,
            });
        }
        found
    }

    /// What each step of the enumeration answered.
    pub fn explain() {
        let _started = crate::windows_media::started();
        for kind in [Kind::Camera, Kind::Microphone] {
            unsafe {
                let mut asked = None;
                let made = MFCreateAttributes(&mut asked, 1);
                println!("{kind}: MFCreateAttributes -> {made:?}");
                let Some(asked) = asked else { continue };
                let wanted = if kind == Kind::Camera {
                    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID
                } else {
                    MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_AUDCAP_GUID
                };
                let set = asked.SetGUID(&MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, &wanted);
                println!("{kind}: SetGUID -> {set:?}");
                let mut sources: *mut Option<IMFActivate> = std::ptr::null_mut();
                let mut count: u32 = 0;
                let listed = MFEnumDeviceSources(&asked, &mut sources, &mut count);
                println!("{kind}: MFEnumDeviceSources -> {listed:?}, {count} found");
                if !sources.is_null() {
                    windows::Win32::System::Com::CoTaskMemFree(Some(sources as *const _));
                }
            }
        }
    }

    /// One string attribute of an activation.
    fn string_of(source: &IMFActivate, which: &windows::core::GUID) -> Option<String> {
        unsafe {
            let mut length = 0_u32;
            let mut text = [0_u16; 512];
            source.GetString(which, &mut text, Some(&mut length)).ok()?;
            let said = String::from_utf16_lossy(&text[..length as usize]);
            (!said.trim().is_empty()).then_some(said)
        }
    }

    /// A fixed-width Windows string, up to its first nothing.
    fn from_wide(text: &[u16]) -> String {
        let end = text
            .iter()
            .position(|letter| *letter == 0)
            .unwrap_or(text.len());
        String::from_utf16_lossy(&text[..end]).trim().to_string()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{Device, Kind};

    pub fn explain() {
        println!(
            "the capture layer is Windows for now; this is {}",
            std::env::consts::OS
        );
    }

    /// Elsewhere: nothing is listed, and the capability refuses by name
    /// wherever something is actually asked of a device.
    pub fn devices_of(_kind: Kind) -> Vec<Device> {
        Vec::new()
    }
}
