//! The machine's state, on macOS: what macOS's own tools print.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the documented output of
//! `defaults`, `pmset` and `ioreg`. Never compiled for an Apple target
//! and never run on a Mac; the parsers are tested on every machine
//! against sample output written from Apple's documentation and
//! commonly published captures — not against output this machine has
//! seen. `scale` goes through `AppKit` and has had only the GNUstep
//! type-check the other `AppKit` files had. Verify on a real Mac before
//! trusting any of it.
//!
//! # Refused
//!
//! `locked`: macOS says whether the screen is locked only in a
//! private-looking key of `CGSessionCopyCurrentDictionary`
//! (`CGSSessionScreenIsLocked`), which Apple does not document and
//! which this file will not read on a guess.

use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::system::{Power, Theme, bcp47, zone_from_link};

fn said(program: &str, arguments: &[&str]) -> Option<(bool, String)> {
    let out = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

/// `defaults read -g AppleInterfaceStyle` prints `Dark` in dark mode,
/// and fails — the key does not exist — in light mode. A failure of
/// `defaults` itself (`None`) is not light.
pub(crate) fn theme_from(answer: Option<(bool, String)>) -> Option<Theme> {
    let (worked, text) = answer?;
    Some(if worked && text.eq_ignore_ascii_case("dark") {
        Theme::Dark
    } else {
        Theme::Light
    })
}

pub fn theme() -> Result<Theme, OsError> {
    theme_from(said("defaults", &["read", "-g", "AppleInterfaceStyle"]))
        .ok_or_else(|| OsError::Platform("`defaults` could not be run".to_string()))
}

/// `pmset -g batt`:
///
/// ```text
/// Now drawing from 'Battery Power'
///  -InternalBattery-0 (id=4653155)    87%; discharging; 4:12 remaining present: true
/// ```
pub(crate) fn power_from(batt: &str, low_power: Option<&str>) -> Power {
    let plugged = if batt.contains("'AC Power'") {
        Some(true)
    } else if batt.contains("'Battery Power'") {
        Some(false)
    } else {
        None
    };
    let line = batt.lines().find(|line| line.contains("InternalBattery"));
    let level = line.and_then(|line| {
        let before = line.split('%').next()?;
        before
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()?
            .parse::<u8>()
            .ok()
            .filter(|level| *level <= 100)
    });
    let charging = line.map(|line| line.contains("; charging;") || line.contains("; charged;"));
    // `pmset -g` prints `lowpowermode 1` when Low Power Mode is on, on
    // the Macs that have it.
    let saver = low_power.and_then(|text| {
        text.lines()
            .find(|line| line.trim_start().starts_with("lowpowermode"))
            .map(|line| line.trim_end().ends_with('1'))
    });
    Power {
        plugged,
        level,
        charging,
        saver,
    }
}

pub fn power() -> Result<Power, OsError> {
    let Some((true, batt)) = said("pmset", &["-g", "batt"]) else {
        return Err(OsError::Platform("`pmset -g batt` could not be run".to_string()));
    };
    let settings = said("pmset", &["-g"]).map(|(_, text)| text);
    Ok(power_from(&batt, settings.as_deref()))
}

/// `ioreg -c IOHIDSystem` prints `"HIDIdleTime" = 1234567890`, in
/// nanoseconds.
pub(crate) fn idle_from(ioreg: &str) -> Option<std::time::Duration> {
    let after = ioreg.split("\"HIDIdleTime\"").nth(1)?;
    let digits: String = after
        .trim_start_matches([' ', '='])
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse::<u64>().ok().map(std::time::Duration::from_nanos)
}

pub fn idle() -> Result<std::time::Duration, OsError> {
    let Some((true, text)) = said("ioreg", &["-c", "IOHIDSystem", "-d", "4"]) else {
        return Err(OsError::Platform("`ioreg` could not be run".to_string()));
    };
    idle_from(&text).ok_or_else(|| {
        OsError::Platform("`ioreg` did not report HIDIdleTime".to_string())
    })
}

pub fn locked() -> Result<bool, OsError> {
    Err(OsError::Unsupported(
        "the lock state on macOS (only an undocumented session key says it)",
    ))
}

pub fn locale() -> Result<String, OsError> {
    let Some((true, text)) = said("defaults", &["read", "-g", "AppleLocale"]) else {
        return Err(OsError::Platform("`defaults read -g AppleLocale` failed".to_string()));
    };
    bcp47(&text).ok_or_else(|| OsError::Platform(format!("`{text}` is not a locale")))
}

pub fn timezone() -> Result<String, OsError> {
    let target = std::fs::read_link("/etc/localtime").map_err(|error| {
        OsError::Platform(format!("/etc/localtime is not a link: {error}"))
    })?;
    zone_from_link(&target.to_string_lossy()).ok_or_else(|| {
        OsError::Platform(format!(
            "/etc/localtime points at {}, not into zoneinfo",
            target.display()
        ))
    })
}

#[cfg(target_os = "macos")]
pub fn scale() -> Result<f64, OsError> {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    super::appkit::on_main("the display scale", || {
        // SAFETY: documented `NSScreen` calls, on the main thread.
        unsafe {
            let screen: *mut AnyObject = msg_send![class!(NSScreen), mainScreen];
            if screen.is_null() {
                return None;
            }
            let scale: f64 = msg_send![screen, backingScaleFactor];
            Some(scale)
        }
    })?
    .ok_or_else(|| OsError::Platform("there is no main screen".to_string()))
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn scale() -> Result<f64, OsError> {
    unreachable!("macos/system.rs is never the platform module outside macOS")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_mode_is_a_key_and_light_mode_is_its_absence() {
        assert_eq!(theme_from(Some((true, "Dark".into()))), Some(Theme::Dark));
        assert_eq!(theme_from(Some((false, String::new()))), Some(Theme::Light));
        assert_eq!(theme_from(None), None);
    }

    #[test]
    fn pmset_on_battery() {
        let batt = "Now drawing from 'Battery Power'\n \
                    -InternalBattery-0 (id=4653155)\t87%; discharging; 4:12 remaining present: true";
        let power = power_from(batt, Some(" lowpowermode         1\n sleep 1"));
        assert_eq!(power.plugged, Some(false));
        assert_eq!(power.level, Some(87));
        assert_eq!(power.charging, Some(false));
        assert_eq!(power.saver, Some(true));
    }

    #[test]
    fn pmset_on_a_mac_without_a_battery() {
        let power = power_from("Now drawing from 'AC Power'", None);
        assert_eq!(power.plugged, Some(true));
        assert_eq!(power.level, None);
        assert_eq!(power.charging, None);
        assert_eq!(power.saver, None);
    }

    #[test]
    fn idle_time_is_read_in_nanoseconds() {
        let ioreg = "  |   \"HIDIdleTime\" = 2500000000\n  |   \"Other\" = 1";
        assert_eq!(idle_from(ioreg), Some(std::time::Duration::from_millis(2500)));
        assert_eq!(idle_from("nothing"), None);
    }
}
