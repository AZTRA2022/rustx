//! The color picker, on macOS: `osascript`'s `choose color`, which is
//! the system color panel — eyedropper included.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the AppleScript documentation of
//! `choose color` (it answers `{r, g, b}` with each channel from 0 to
//! 65535, and fails with error -128 when cancelled). Never run on a
//! Mac; the reading of its answer is tested on every machine. The same
//! door `dialog.rs` uses for files.

use std::process::{Command, Stdio};

use crate::color::Rgb;
use crate::error::OsError;

/// `choose color`'s 16-bit channels, down to 8.
pub(crate) fn read_color(text: &str) -> Option<Rgb> {
    let mut channels = text.trim().trim_matches(['{', '}']).split(',').map(|part| {
        part.trim()
            .parse::<u32>()
            .ok()
            .filter(|value| *value <= 65535)
            .map(|value| ((value * 255 + 32767) / 65535) as u8)
    });
    let color = Rgb {
        r: channels.next()??,
        g: channels.next()??,
        b: channels.next()??,
    };
    channels.next().is_none().then_some(color)
}

/// The script, with the starting color in 16-bit channels.
pub(crate) fn script(initial: Option<Rgb>) -> String {
    match initial {
        Some(color) => format!(
            "choose color default color {{{}, {}, {}}}",
            u32::from(color.r) * 257,
            u32::from(color.g) * 257,
            u32::from(color.b) * 257
        ),
        None => "choose color".to_string(),
    }
}

pub fn pick(initial: Option<Rgb>) -> Result<Option<Rgb>, OsError> {
    let out = Command::new("osascript")
        .args(["-e", &script(initial)])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| OsError::Platform(format!("`osascript` could not run: {error}")))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        // -128 is "User canceled".
        if said.contains("-128") {
            return Ok(None);
        }
        return Err(OsError::Platform(format!(
            "`choose color` failed: {}",
            said.trim()
        )));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    read_color(&text)
        .map(Some)
        .ok_or_else(|| OsError::Platform(format!("`{}` is not a color", text.trim())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_bit_channels_come_down_to_eight() {
        assert_eq!(
            read_color("65535, 32896, 0\n"),
            Some(Rgb { r: 255, g: 128, b: 0 })
        );
        assert_eq!(read_color("{0, 0, 0}"), Some(Rgb { r: 0, g: 0, b: 0 }));
        assert_eq!(read_color("1, 2"), None);
        assert_eq!(read_color("70000, 0, 0"), None);
    }

    #[test]
    fn the_starting_color_is_written_in_sixteen_bits() {
        assert_eq!(
            script(Some(Rgb { r: 255, g: 0, b: 1 })),
            "choose color default color {65535, 0, 257}"
        );
        assert_eq!(script(None), "choose color");
    }
}
