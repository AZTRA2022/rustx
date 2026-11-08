//! The file dialog, on Linux: `zenity` or `kdialog`.
//!
//! # By the desktop's own tool, and why
//!
//! The right way is the desktop portal —
//! `org.freedesktop.portal.FileChooser` over D-Bus, `OpenFile` and
//! `SaveFile` — because it is the only one that also works inside a
//! sandbox (Flatpak, Snap) and gives the user's own desktop's dialog,
//! GNOME or KDE, rather than a second one glued on top of it. That
//! needs a D-Bus dependency this crate does not have today (`zbus`
//! and its runtime, or the vtables by hand).
//!
//! `zenity` and `kdialog` are the other end of the same idea: each is
//! the command-line door onto the exact dialog its own desktop would
//! have shown through the portal anyway — GTK's for `zenity`, Qt's
//! for `kdialog` — and both ship with the desktop environment itself
//! on GNOME and KDE respectively, nothing extra to install there.
//! `$XDG_CURRENT_DESKTOP` decides which one is tried, the same
//! environment-reading idea as `clipboard.rs`'s choice between
//! Wayland and X11.
//!
//! # Cancel is not an error
//!
//! Both tools exit with a non-zero status and no output when the
//! user dismisses the dialog, which is read as
//! [`crate::dialog::DialogError::Cancelled`] rather than a platform
//! failure — the distinction `clipboard.rs`'s `read` draws between
//! "empty" and "wrong" has no equivalent here, because a dialog that
//! failed to even open also exits non-zero, so an empty answer is
//! read the more common way first.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! Linux. `cargo check --target x86_64-unknown-linux-gnu` is what
//! verified it compiles; nothing here has been seen to actually open
//! a dialog.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::dialog::{Ask, DialogError};

pub const AVAILABLE: bool = true;

/// The joining character asked of `zenity --separator` and
/// `kdialog --separator`, read back out of several chosen paths.
/// Not `|`, the default of both: a path can contain one.
const SEPARATOR: char = '\u{1f}';

/// Which desktop's own tool to reach for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Zenity,
    Kdialog,
}

/// Read from `$XDG_CURRENT_DESKTOP`, the same variable a desktop
/// portal implementation reads to decide which backend answers it.
/// `zenity` is the default: it is the one GNOME, XFCE, Cinnamon and
/// most others carry, and naming the exception (KDE) is shorter than
/// naming the rule.
pub fn chosen(current_desktop: Option<&str>) -> Tool {
    match current_desktop {
        Some(value) if value.to_uppercase().contains("KDE") => Tool::Kdialog,
        _ => Tool::Zenity,
    }
}

fn package_for(tool: Tool) -> &'static str {
    match tool {
        Tool::Zenity => "`zenity` (GNOME, XFCE, most others)",
        Tool::Kdialog => "`kdialog` (KDE)",
    }
}

/// The program and arguments for `choose`ing an existing file.
pub fn open_arguments(tool: Tool, ask: &Ask) -> (&'static str, Vec<String>) {
    let mut arguments = vec!["--file-selection".to_string(), title(&ask.title)];
    if ask.many {
        arguments.push("--multiple".to_string());
        arguments.push(format!("--separator={SEPARATOR}"));
    }
    arguments.extend(filename(ask));
    arguments.extend(filters(tool, ask));
    (program(tool), arguments)
}

/// The same, for `choose`ing where to write a file.
pub fn save_arguments(tool: Tool, ask: &Ask) -> (&'static str, Vec<String>) {
    let mut arguments = vec![
        "--file-selection".to_string(),
        "--save".to_string(),
        "--confirm-overwrite".to_string(),
        title(&ask.title),
    ];
    arguments.extend(filename(ask));
    arguments.extend(filters(tool, ask));
    (program(tool), arguments)
}

/// The same, for `choose`ing a folder.
pub fn folder_arguments(tool: Tool, ask: &Ask) -> (&'static str, Vec<String>) {
    let mut arguments = vec![
        "--file-selection".to_string(),
        "--directory".to_string(),
        title(&ask.title),
    ];
    arguments.extend(filename(ask));
    (program(tool), arguments)
}

fn program(tool: Tool) -> &'static str {
    match tool {
        Tool::Zenity => "zenity",
        Tool::Kdialog => "kdialog",
    }
}

fn title(text: &str) -> String {
    format!("--title={text}")
}

/// `--filename=` names the starting folder, the proposed name, or
/// both joined by a slash — the one argument `zenity`'s file
/// selection uses for every "where does this open, and what is
/// already typed in the name field" question.
fn filename(ask: &Ask) -> Vec<String> {
    let start_in = ask.start_in.as_ref().map(|path| path.display().to_string());
    let value = match (start_in, &ask.named) {
        (Some(directory), Some(name)) => Some(format!("{directory}/{name}")),
        (Some(directory), None) => Some(format!("{directory}/")),
        (None, Some(name)) => Some(name.clone()),
        (None, None) => None,
    };
    value.map(|value| format!("--filename={value}")).into_iter().collect()
}

/// `--file-filter="Name | *.ext1 *.ext2"`, one per declared filter.
/// `kdialog` reads filters as a single trailing argument instead
/// (`"Name (*.ext1 *.ext2)"`), joined with `\n` for more than one —
/// different enough from `zenity`'s repeated flag that `tool` decides
/// which shape is built.
fn filters(tool: Tool, ask: &Ask) -> Vec<String> {
    if ask.filters.is_empty() {
        return Vec::new();
    }
    match tool {
        Tool::Zenity => ask
            .filters
            .iter()
            .map(|(name, extensions)| {
                let patterns: Vec<String> =
                    extensions.iter().map(|extension| format!("*.{}", extension.trim_start_matches('.'))).collect();
                format!("--file-filter={name} | {}", patterns.join(" "))
            })
            .collect(),
        Tool::Kdialog => {
            let joined: Vec<String> = ask
                .filters
                .iter()
                .map(|(name, extensions)| {
                    let patterns: Vec<String> = extensions
                        .iter()
                        .map(|extension| format!("*.{}", extension.trim_start_matches('.')))
                        .collect();
                    format!("{name} ({})", patterns.join(" "))
                })
                .collect();
            vec![joined.join("\n")]
        }
    }
}

fn missing(tool: Tool, error: &std::io::Error) -> DialogError {
    DialogError::Platform(format!(
        "`{}` could not be run ({error}). It is what opens a file dialog on this \
         desktop — install {}.",
        program(tool),
        package_for(tool)
    ))
}

fn run(tool: Tool, program: &str, arguments: &[String]) -> Result<String, DialogError> {
    let out = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| missing(tool, &error))?;
    if !out.status.success() {
        // Both tools exit non-zero and print nothing when the dialog
        // is cancelled, and a dialog that could not even open does
        // the same — so this reads as `Cancelled` rather than a
        // platform error, the more common of the two by far.
        return Err(DialogError::Cancelled);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end_matches(['\n', '\r']).to_string())
}

pub fn open(ask: &Ask) -> Result<Vec<PathBuf>, DialogError> {
    let tool = chosen(std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref());
    let (program, arguments) = open_arguments(tool, ask);
    let said = run(tool, program, &arguments)?;
    Ok(said.split(SEPARATOR).filter(|piece| !piece.is_empty()).map(PathBuf::from).collect())
}

pub fn save(ask: &Ask) -> Result<PathBuf, DialogError> {
    let tool = chosen(std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref());
    let (program, arguments) = save_arguments(tool, ask);
    Ok(PathBuf::from(run(tool, program, &arguments)?))
}

pub fn folder(ask: &Ask) -> Result<PathBuf, DialogError> {
    let tool = chosen(std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref());
    let (program, arguments) = folder_arguments(tool, ask);
    Ok(PathBuf::from(run(tool, program, &arguments)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kde_reads_kdialog_and_everything_else_reads_zenity() {
        assert_eq!(chosen(Some("KDE")), Tool::Kdialog);
        assert_eq!(chosen(Some("ubuntu:GNOME")), Tool::Zenity);
        assert_eq!(chosen(Some("XFCE")), Tool::Zenity);
        assert_eq!(chosen(None), Tool::Zenity);
    }

    #[test]
    fn a_title_reaches_the_command_line() {
        let ask = Ask::new("Choose an image");
        let (program, arguments) = open_arguments(Tool::Zenity, &ask);
        assert_eq!(program, "zenity");
        assert!(arguments.contains(&"--title=Choose an image".to_string()), "got {arguments:?}");
    }

    #[test]
    fn multiple_selection_is_asked_with_this_crates_own_separator() {
        let ask = Ask::new("Pick").many();
        let (_, arguments) = open_arguments(Tool::Zenity, &ask);
        assert!(arguments.contains(&"--multiple".to_string()));
        assert!(arguments.iter().any(|argument| argument.starts_with("--separator=")));
        assert!(!arguments.iter().any(|argument| argument.contains('|') && argument.starts_with("--separator")));
    }

    #[test]
    fn a_filter_becomes_a_zenity_file_filter_and_a_kdialog_pattern() {
        let ask = Ask::new("Pick").filter("Images", ["png", "jpg"]);
        let (_, zenity_arguments) = open_arguments(Tool::Zenity, &ask);
        assert!(
            zenity_arguments.contains(&"--file-filter=Images | *.png *.jpg".to_string()),
            "got {zenity_arguments:?}"
        );
        let (_, kdialog_arguments) = open_arguments(Tool::Kdialog, &ask);
        assert!(
            kdialog_arguments.iter().any(|argument| argument == "Images (*.png *.jpg)"),
            "got {kdialog_arguments:?}"
        );
    }

    #[test]
    fn a_proposed_name_and_a_starting_folder_join_into_one_filename() {
        let ask = Ask::new("Save").starting_in("/home/me/pictures").named("export.png");
        let (_, arguments) = save_arguments(Tool::Zenity, &ask);
        assert!(
            arguments.contains(&"--filename=/home/me/pictures/export.png".to_string()),
            "got {arguments:?}"
        );
    }
}
