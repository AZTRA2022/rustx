//! Screen capture, on macOS: `screencapture`, the tool macOS ships.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from `screencapture`'s own manual page
//! and from the BMP format. Never run on a Mac; the BMP reading is
//! tested on every machine against bytes this file builds itself, not
//! against a capture a Mac produced.
//!
//! # Why a command and a temporary file
//!
//! `CGWindowListCreateImage` is the API, and it would hand back pixels
//! directly — but the pixels come as a `CGImage`, which means
//! `CGDataProvider`, `CGImageGetDataProvider`, `CFDataGetBytePtr` and a
//! bytes-per-row that is *not* width × 4, all of it unverifiable from
//! here. `screencapture` is Apple's own front end to the same
//! framework, it is on every Mac, and it is what the screenshot keys
//! run. It writes a file, so this reads a file.
//!
//! BMP (`-t bmp`) rather than PNG: a PNG would need a decoder this
//! crate does not carry, and an uncompressed 32-bit BMP is a header
//! and then the pixels, in the BGRA order [`Image`] already uses.
//!
//! # The permission
//!
//! macOS asks the user for Screen Recording in System Settings, once
//! per application. Until it is granted, `screencapture` writes an
//! image of the desktop picture with no windows in it — it does not
//! fail. Nothing here can detect that, and nothing here can grant it:
//! an application that captures should tell its user where the switch
//! is.

use std::process::{Command, Stdio};

use crate::clipboard::Image;
use crate::error::OsError;
use crate::screen::Area;
use crate::window::Pane;

/// A temporary file that removes itself.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        // The process id and a counter: two captures at once in the same
        // process must not collide on one name.
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = format!(
            "rustx-capture-{}-{}.bmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        Self(std::env::temp_dir().join(name))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The pixels of an uncompressed 32-bit BMP, as [`Image`] wants them.
///
/// Only the fields that matter are read: where the pixels start, the
/// size, the bit count, and the sign of the height — a positive height
/// means the rows are stored bottom-up, which is the BMP default and
/// which has to be flipped back.
pub(crate) fn read_bmp(bytes: &[u8]) -> Result<Image, OsError> {
    let short = || OsError::Platform("the capture is not a readable BMP".to_string());
    if bytes.len() < 54 || &bytes[..2] != b"BM" {
        return Err(short());
    }
    let word = |at: usize| -> Result<u32, OsError> {
        bytes
            .get(at..at + 4)
            .and_then(|four| four.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or_else(short)
    };
    let start = word(10)? as usize;
    let width = word(18)? as i32;
    let height = word(22)? as i32;
    let bits = u16::from_le_bytes([bytes[28], bytes[29]]);
    if bits != 32 {
        return Err(OsError::Platform(format!(
            "`screencapture` wrote a {bits}-bit BMP, and only 32-bit ones are read here"
        )));
    }
    let bottom_up = height > 0;
    let width = u32::try_from(width).map_err(|_| short())?;
    let height = u32::try_from(height.abs()).map_err(|_| short())?;
    let needed = crate::capture::bytes_for(width, height)?;
    let pixels = bytes.get(start..start + needed).ok_or_else(short)?;
    if !bottom_up {
        return Ok(Image {
            width,
            height,
            bgra: pixels.to_vec(),
        });
    }
    // Bottom-up: the last stored row is the top one. A 32-bit BMP has no
    // row padding, so a row is exactly width × 4 bytes.
    let stride = width as usize * 4;
    let mut bgra = Vec::with_capacity(needed);
    for row in (0..height as usize).rev() {
        bgra.extend_from_slice(&pixels[row * stride..(row + 1) * stride]);
    }
    Ok(Image {
        width,
        height,
        bgra,
    })
}

fn run(arguments: &[&str], into: &std::path::Path) -> Result<Image, OsError> {
    let done = Command::new("screencapture")
        .args(arguments)
        .arg(into)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            OsError::Platform(format!("`screencapture` could not be run: {error}"))
        })?;
    if !done.status.success() {
        return Err(OsError::Platform(format!(
            "`screencapture` failed: {}",
            String::from_utf8_lossy(&done.stderr).trim()
        )));
    }
    let bytes = std::fs::read(into)
        .map_err(|error| OsError::File(format!("{}: {error}", into.display())))?;
    read_bmp(&bytes)
}

pub fn area(area: Option<Area>) -> Result<Image, OsError> {
    let scratch = Scratch::new();
    match area {
        // `-R x,y,w,h` is the documented way to name a rectangle, and
        // `-x` keeps the shutter sound quiet — a capture an application
        // takes for its own purposes should not click.
        Some(area) => run(
            &["-x", "-t", "bmp", "-R", &format!("{},{},{},{}", area.x, area.y, area.width, area.height)],
            &scratch.0,
        ),
        None => run(&["-x", "-t", "bmp"], &scratch.0),
    }
}

pub fn window(pane: Pane) -> Result<Image, OsError> {
    if !pane.exists() {
        return Err(OsError::Platform(
            "there is no window to capture yet: the handle is still nothing".to_string(),
        ));
    }
    // `screencapture -l` takes a CoreGraphics *window number*, which is
    // not the `NSView` pointer CEF hands over, and not derivable from it
    // without `NSWindow.windowNumber` — a main-thread AppKit call. The
    // window's place on screen is reachable the same way, so the capture
    // goes through the rectangle instead, with the one difference said
    // here: anything covering the window is captured with it.
    let bounds = super::window::frame_of(pane)?;
    self::area(Some(bounds))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BMP of `height` rows of one pixel each, numbered by their blue
    /// channel so the order can be checked.
    fn bmp(height: i32) -> Vec<u8> {
        let rows = height.unsigned_abs() as usize;
        let mut bytes = vec![0u8; 54];
        bytes[..2].copy_from_slice(b"BM");
        bytes[10..14].copy_from_slice(&54u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&1u32.to_le_bytes());
        bytes[22..26].copy_from_slice(&(height as u32).to_le_bytes());
        bytes[28..30].copy_from_slice(&32u16.to_le_bytes());
        for row in 0..rows {
            bytes.extend_from_slice(&[row as u8, 0, 0, 255]);
        }
        bytes
    }

    #[test]
    fn a_bottom_up_bmp_is_turned_the_right_way_up() {
        let image = read_bmp(&bmp(3)).expect("an image");
        assert_eq!((image.width, image.height), (1, 3));
        // Stored 0, 1, 2 from the bottom, so the top row is 2.
        assert_eq!(image.bgra[0], 2);
        assert_eq!(image.bgra[8], 0);
    }

    #[test]
    fn a_top_down_bmp_is_left_alone() {
        let image = read_bmp(&bmp(-3)).expect("an image");
        assert_eq!(image.height, 3);
        assert_eq!(image.bgra[0], 0, "the first stored row is already the top one");
    }

    #[test]
    fn anything_that_is_not_a_32_bit_bmp_is_refused_by_name() {
        assert!(read_bmp(b"not a bmp at all").is_err());
        let mut sixteen = bmp(1);
        sixteen[28..30].copy_from_slice(&16u16.to_le_bytes());
        let said = read_bmp(&sixteen).expect_err("refused");
        assert!(said.to_string().contains("16-bit"), "got {said}");
    }

    #[test]
    fn a_truncated_file_is_a_sentence_and_not_a_panic() {
        let mut cut = bmp(10);
        cut.truncate(60);
        assert!(read_bmp(&cut).is_err());
    }
}
