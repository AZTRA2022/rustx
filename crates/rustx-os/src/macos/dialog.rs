//! File dialogs, on macOS: `osascript` saying `choose file`.
//!
//! # By AppleScript, and why
//!
//! `choose file`, `choose file name` and `choose folder` are Standard
//! Additions commands — the oldest, most-documented part of
//! AppleScript, and the answer every forum thread about "open a
//! native file picker from a script on macOS" gives first. They open
//! the real system panel and block until it closes, the same
//! contract as `NSOpenPanel`/`NSSavePanel` without binding AppKit:
//! `osascript`, which ships with macOS, is the door.
//!
//! The user cancelling is not an error AppleScript lets a script miss:
//! it raises error **-128**, caught here and turned into
//! [`crate::dialog::DialogError::Cancelled`] rather than a parse
//! failure on empty output.
//!
//! Several chosen files come back joined by the unit separator
//! (`\u{1f}`, the same character `instance::BETWEEN` uses to join
//! handed-over arguments) rather than AppleScript's own `, `, because
//! a path can itself contain a comma and `, ` cannot then be told
//! apart from one that is part of a name.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! macOS, and there is no Apple target here to even type-check it
//! against — only read over by hand, unlike the Linux files. What is
//! tested on any machine is the AppleScript text built for a given
//! [`Ask`], not what macOS does with it.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::dialog::{Ask, DialogError};

pub const AVAILABLE: bool = true;

/// The character chosen paths are joined by, read back out in
/// [`parsed_paths`].
const SEPARATOR: char = '\u{1f}';

/// A string safe to sit between the double quotes of an AppleScript
/// literal.
fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The `of type` clause, when `ask` names any filter — every
/// extension from every filter, flattened: AppleScript's own clause
/// takes one list, not one per named group.
fn of_type(ask: &Ask) -> String {
    if ask.filters.is_empty() {
        return String::new();
    }
    let extensions: Vec<String> = ask
        .filters
        .iter()
        .flat_map(|(_, extensions)| extensions.iter())
        .map(|extension| format!("\"{}\"", escaped(extension.trim_start_matches('.'))))
        .collect();
    format!(" of type {{{}}}", extensions.join(", "))
}

/// The `default location` clause, when `ask` names a starting folder.
fn default_location(ask: &Ask) -> String {
    match &ask.start_in {
        Some(path) => format!(
            " default location (POSIX file \"{}\")",
            escaped(&path.display().to_string())
        ),
        None => String::new(),
    }
}

/// The AppleScript lines for `choose file`.
pub fn open_script(ask: &Ask) -> Vec<String> {
    let many = if ask.many { " with multiple selections allowed" } else { "" };
    vec![
        "try".to_string(),
        format!(
            "set chosen to choose file with prompt \"{}\"{}{}{}",
            escaped(&ask.title),
            of_type(ask),
            default_location(ask),
            many,
        ),
        format!("set AppleScript's text item delimiters to \"{SEPARATOR}\""),
        "if class of chosen is list then".to_string(),
        "set chosen to chosen as text".to_string(),
        "else".to_string(),
        "set chosen to POSIX path of chosen".to_string(),
        "end if".to_string(),
        "chosen".to_string(),
        "on error number -128".to_string(),
        "\"CANCELLED\"".to_string(),
        "end try".to_string(),
    ]
}

/// The AppleScript lines for `choose file name`.
pub fn save_script(ask: &Ask) -> Vec<String> {
    let named = match &ask.named {
        Some(name) => format!(" default name \"{}\"", escaped(name)),
        None => String::new(),
    };
    vec![
        "try".to_string(),
        format!(
            "set chosen to choose file name with prompt \"{}\"{}{}",
            escaped(&ask.title),
            named,
            default_location(ask),
        ),
        "POSIX path of chosen".to_string(),
        "on error number -128".to_string(),
        "\"CANCELLED\"".to_string(),
        "end try".to_string(),
    ]
}

/// The AppleScript lines for `choose folder`.
pub fn folder_script(ask: &Ask) -> Vec<String> {
    vec![
        "try".to_string(),
        format!(
            "set chosen to choose folder with prompt \"{}\"{}",
            escaped(&ask.title),
            default_location(ask),
        ),
        "POSIX path of chosen".to_string(),
        "on error number -128".to_string(),
        "\"CANCELLED\"".to_string(),
        "end try".to_string(),
    ]
}

fn run(lines: &[String]) -> Result<String, DialogError> {
    let mut command = Command::new("osascript");
    for line in lines {
        command.arg("-e").arg(line);
    }
    let out = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| {
            DialogError::Platform(format!(
                "`osascript` could not be run ({error}). It ships with macOS itself, so \
                 this usually means something else is wrong with the environment."
            ))
        })?;
    if !out.status.success() {
        return Err(DialogError::Platform(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `output` into the chosen paths, or `Cancelled` when it is the
/// sentinel the script returns for AppleScript's own error -128.
fn parsed_paths(output: &str) -> Result<Vec<PathBuf>, DialogError> {
    if output == "CANCELLED" {
        return Err(DialogError::Cancelled);
    }
    Ok(output
        .split(SEPARATOR)
        .filter(|piece| !piece.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// The same, for a single path — `save` and `folder` never choose
/// more than one.
fn parsed_path(output: &str) -> Result<PathBuf, DialogError> {
    if output == "CANCELLED" {
        return Err(DialogError::Cancelled);
    }
    Ok(PathBuf::from(output))
}

pub fn open(ask: &Ask) -> Result<Vec<PathBuf>, DialogError> {
    parsed_paths(&run(&open_script(ask))?)
}

pub fn save(ask: &Ask) -> Result<PathBuf, DialogError> {
    parsed_path(&run(&save_script(ask))?)
}

pub fn folder(ask: &Ask) -> Result<PathBuf, DialogError> {
    parsed_path(&run(&folder_script(ask))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_is_in_the_script_and_cancel_is_caught_by_number() {
        let ask = Ask::new("Choose an image");
        let script = open_script(&ask).join("\n");
        assert!(script.contains("choose file with prompt \"Choose an image\""), "got {script}");
        assert!(script.contains("on error number -128"), "got {script}");
    }

    #[test]
    fn a_filter_becomes_an_of_type_clause() {
        let ask = Ask::new("Pick").filter("Images", ["png", "jpg"]);
        let script = open_script(&ask).join("\n");
        assert!(script.contains("of type {\"png\", \"jpg\"}"), "got {script}");
    }

    #[test]
    fn a_quote_in_the_title_does_not_end_the_applescript_string_early() {
        let ask = Ask::new("\"Quoted\" title");
        let script = open_script(&ask).join("\n");
        assert!(script.contains("\\\"Quoted\\\" title"), "got {script}");
    }

    #[test]
    fn the_cancelled_sentinel_is_read_as_cancelled_and_nothing_else_is() {
        assert_eq!(parsed_paths("CANCELLED"), Err(DialogError::Cancelled));
        assert_eq!(parsed_path("CANCELLED"), Err(DialogError::Cancelled));
        assert_eq!(
            parsed_paths(&format!("/a/b{SEPARATOR}/c/d{SEPARATOR}")).unwrap(),
            vec![PathBuf::from("/a/b"), PathBuf::from("/c/d")]
        );
    }

    #[test]
    fn a_default_name_reaches_the_save_script() {
        let ask = Ask::new("Save as").named("export.png");
        let script = save_script(&ask).join("\n");
        assert!(script.contains("default name \"export.png\""), "got {script}");
    }

    #[test]
    fn a_starting_folder_reaches_every_script() {
        let ask = Ask::new("Pick").starting_in("/Users/me/Pictures");
        for script in [open_script(&ask), save_script(&ask), folder_script(&ask)] {
            let script = script.join("\n");
            assert!(
                script.contains("default location (POSIX file \"/Users/me/Pictures\")"),
                "got {script}"
            );
        }
    }
}
