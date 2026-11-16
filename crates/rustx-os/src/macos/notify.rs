//! Notifications, on macOS: `osascript` saying `display notification`.
//!
//! # By AppleScript, and why
//!
//! `display notification "…" with title "…"` is Apple's own,
//! documented way to post a Notification Center banner from a script,
//! and `osascript` — which ships with every macOS install, nothing to
//! check for — is the command line's door into AppleScript. The real
//! API is `UNUserNotificationCenter`, which asks for a signed,
//! bundled application with an identifier to register against; a
//! `cargo run` binary has none of that, the same gap the Windows side
//! has for a WinRT toast and solves the same way: through the plainer
//! door.
//!
//! # What this costs
//!
//! No icon: the simple form of `display notification` takes a title,
//! a subtitle and a sound, not an image, so `What::icon` is read by no
//! platform that reaches this file. No click handler either — a
//! banner posted this way does not report back which of the two
//! sides read the Windows balloon reports today.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! macOS, and there is no Apple target here to even type-check it
//! against — only read over by hand, unlike the Linux files.

use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::notify::What;

pub const AVAILABLE: bool = true;

/// No: AppleScript's `display notification` takes no buttons, and the
/// framework that does — `UNNotificationAction` in
/// `UserNotifications` — only delivers the choice to a *registered
/// delegate of a bundled application*, which a binary run from a
/// terminal is not. See `ask` below.
pub const CAN_ASK: bool = false;

/// The AppleScript line `osascript -e` is given.
///
/// Separated from running it for the same reason as the Linux side's
/// `arguments`: this is the part that can be wrong, and the only part
/// a test on any machine can read.
pub fn script(what: &What) -> String {
    format!(
        "display notification \"{}\" with title \"{}\"",
        escaped(&what.body),
        escaped(&what.title)
    )
}

/// A string safe to sit between the double quotes of an AppleScript
/// literal: its own quotes and backslashes escaped, in that order so
/// a backslash just added in front of a quote is not escaped again.
fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Un enfant qu'on ne fait pas attendre a son appelant, et qui ne
/// reste pas un zombie pour autant.
///
/// Sur Unix, un `Child` lache sans `wait` garde sa place dans la table
/// des processus jusqu'a la sortie du parent. Une machine qui n'a plus
/// de place refuse tous les forks -- c'est `os error 35`, et plus rien
/// ne demarre, pas meme `git`. Le fil ne fait qu'attendre, donc il ne
/// coute rien a celui qui a poste la notification.
pub fn reaped(mut child: std::process::Child) {
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}

pub fn show(what: &What) -> Result<(), OsError> {
    // Does not block the caller: `osascript` posts the banner and exits
    // on its own, and a notification that waited to be read would wait
    // forever. It is still waited for, on a thread of its own: an
    // enfant that nobody waits for stays a zombie and keeps its place in
    // the process table, and a machine with no place left refuses every
    // fork — which is how a loaded runner stops being able to start
    // anything at all.
    Command::new("osascript")
        .arg("-e")
        .arg(script(what))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(reaped)
        .map_err(|error| {
            OsError::Platform(format!(
                "`osascript` could not be run ({error}). It ships with macOS itself, so \
                 this usually means something else is wrong with the environment."
            ))
        })
}

/// Refused, by naming what is missing rather than by substituting
/// something else.
///
/// `display dialog` would put buttons on screen and would answer — and
/// it is a modal window in front of everything, not a notification. An
/// application that asked for a notification and got a dialog has had
/// its user interrupted, which is the opposite of what it asked for.
/// `UNNotificationAction` is the real answer, and it needs the
/// application to be a bundle with a notification delegate registered
/// before launch.
pub fn ask(_what: &What, _actions: &[crate::notify::Action]) -> Result<Option<String>, OsError> {
    Err(OsError::Unsupported(
        "buttons in a notification on macOS (`display notification` takes none, and UserNotifications delivers the choice only to a bundled application's registered delegate)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_and_body_are_both_in_the_script() {
        let what = What::new("Built", "the application compiled");
        let written = script(&what);
        assert!(written.contains("with title \"Built\""), "got {written}");
        assert!(
            written.contains("display notification \"the application compiled\""),
            "got {written}"
        );
    }

    #[test]
    fn a_quote_in_the_text_does_not_end_the_applescript_string_early() {
        let what = What::new("\"quoted\" title", "a \\backslash\\");
        let written = script(&what);
        // Un-escaped, either would close the AppleScript string open
        // and turn the rest of the sentence into a second command.
        assert!(written.contains("\\\"quoted\\\" title"), "got {written}");
        assert!(written.contains("a \\\\backslash\\\\"), "got {written}");
    }
}
