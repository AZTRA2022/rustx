//! Le presse-papiers, sous Linux.
//!
//! # Par les outils du serveur d'affichage, et pourquoi
//!
//! Le presse-papiers de X11 n'est pas un presse-papiers : c'est une
//! négociation entre deux fenêtres (`XConvertSelection` sur
//! `CLIPBOARD`), qui demande une fenêtre et une boucle d'événements à
//! soi. Sous Wayland, c'est `wl_data_device`, et la même histoire.
//! Les deux sont du travail que le moteur pourrait porter et que ce
//! crate ne peut pas faire seul.
//!
//! `wl-copy` / `wl-paste` et `xclip` sont l'autre bout du même
//! mécanisme : ce sont précisément ces protocoles, dans des programmes
//! que les distributions de bureau installent. Lequel est choisi se lit
//! dans l'environnement — `WAYLAND_DISPLAY` est là ou non — et c'est
//! la même question que se posent tous les logiciels qui ont les deux à
//! gérer.
//!
//! Ce que cela coûte : un processus par copie, et un refus nommé quand
//! l'outil n'est pas installé, avec le paquet à mettre.
//!
//! # Écrit, pas éprouvé
//!
//! Ce fichier a été écrit sur une machine Windows et n'a jamais tourné
//! sous Linux.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::clipboard::Image;
use crate::error::OsError;

pub const AVAILABLE: bool = true;

/// Le serveur d'affichage qui tourne, tel que l'environnement le dit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Wayland,
    X11,
}

/// Lequel des deux, et c'est `WAYLAND_DISPLAY` qui décide : une session
/// Wayland le pose, une session X11 ne le pose pas. Un Xwayland dans
/// une session Wayland verrait les deux, et Wayland est alors le bon —
/// c'est celui qui tient vraiment la sélection.
pub fn display() -> Display {
    match std::env::var_os("WAYLAND_DISPLAY") {
        Some(value) if !value.is_empty() => Display::Wayland,
        _ => Display::X11,
    }
}

/// Le programme et ses arguments pour lire un format donné.
pub fn reading(display: Display, format: &str) -> (&'static str, Vec<String>) {
    match display {
        Display::Wayland => (
            "wl-paste",
            vec!["--no-newline".to_string(), "--type".to_string(), format.to_string()],
        ),
        Display::X11 => (
            "xclip",
            vec![
                "-selection".to_string(),
                "clipboard".to_string(),
                "-target".to_string(),
                format.to_string(),
                "-out".to_string(),
            ],
        ),
    }
}

/// Et pour écrire.
pub fn writing(display: Display, format: &str) -> (&'static str, Vec<String>) {
    match display {
        Display::Wayland => ("wl-copy", vec!["--type".to_string(), format.to_string()]),
        Display::X11 => (
            "xclip",
            vec![
                "-selection".to_string(),
                "clipboard".to_string(),
                "-target".to_string(),
                format.to_string(),
                "-in".to_string(),
            ],
        ),
    }
}

/// Ce qu'il faut installer quand le programme n'est pas là.
fn missing(program: &str, error: &std::io::Error) -> OsError {
    let package = match program {
        "wl-copy" | "wl-paste" => "`wl-clipboard`",
        _ => "`xclip`",
    };
    OsError::Platform(format!(
        "`{program}` could not be run ({error}). It is what holds the clipboard on \
         this session — install {package}."
    ))
}

fn read(format: &str) -> Result<Vec<u8>, OsError> {
    let (program, arguments) = reading(display(), format);
    let out = Command::new(program)
        .args(&arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| missing(program, &error))?;
    // Un presse-papiers qui ne tient pas ce format n'est pas une
    // erreur : c'est un presse-papiers vide pour cette question.
    if !out.status.success() {
        return Ok(Vec::new());
    }
    Ok(out.stdout)
}

fn write(format: &str, bytes: &[u8]) -> Result<(), OsError> {
    let (program, arguments) = writing(display(), format);
    let mut child = Command::new(program)
        .args(&arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| missing(program, &error))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(bytes)
            .map_err(|error| OsError::Platform(format!("`{program}` refused the bytes: {error}")))?;
    }
    // On attend : `xclip` doit avoir pris la sélection avant qu'on
    // rende la main, sinon une lecture faite tout de suite après
    // rendrait ce qu'il y avait avant.
    drop(child.stdin.take());
    child
        .wait()
        .map_err(|error| OsError::Platform(format!("`{program}` did not finish: {error}")))?;
    Ok(())
}

pub fn text() -> Result<String, OsError> {
    let bytes = read("text/plain")?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

pub fn set_text(text: &str) -> Result<(), OsError> {
    write("text/plain", text.as_bytes())
}

pub fn image() -> Result<Image, OsError> {
    // Ce qui manque ici, et qui est dit plutôt que deviné : le
    // presse-papiers rend un PNG, et `Image` est du BGRA. Décoder un
    // PNG demande un décodeur, que ce crate n'a pas — c'est la seule
    // pièce qui manque, et `custom("image/png")` donne les octets à qui
    // en a un.
    Err(OsError::Unsupported(
        "reading an image from the clipboard (the bytes are a PNG; \
         `custom(\"image/png\")` hands them over)",
    ))
}

pub fn set_image(_image: &Image) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "writing an image to the clipboard (it would have to be encoded as a PNG first; \
         `set_custom(\"image/png\", …)` takes one)",
    ))
}

pub fn custom(format: &str) -> Result<Vec<u8>, OsError> {
    read(format)
}

pub fn set_custom(format: &str, bytes: &[u8]) -> Result<(), OsError> {
    write(format, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_and_x11_are_told_apart_by_the_environment() {
        // Pas d'écriture dans l'environnement du processus de test :
        // c'est partagé par tous les tests qui tournent en même temps.
        // La fonction pure est ce qui est épinglé.
        let (program, arguments) = reading(Display::Wayland, "text/plain");
        assert_eq!(program, "wl-paste");
        assert!(arguments.contains(&"text/plain".to_string()));

        let (program, arguments) = reading(Display::X11, "text/plain");
        assert_eq!(program, "xclip");
        // La sélection est nommée : sans `-selection clipboard`, `xclip`
        // parle de la sélection primaire, celle du clic du milieu, qui
        // n'est pas le presse-papiers.
        assert!(arguments.windows(2).any(|pair| pair == ["-selection", "clipboard"]));
        assert!(arguments.contains(&"-out".to_string()));
    }

    #[test]
    fn writing_says_in_and_reading_says_out() {
        let (_, written) = writing(Display::X11, "text/plain");
        assert!(written.contains(&"-in".to_string()));
        let (_, read) = reading(Display::X11, "text/plain");
        assert!(read.contains(&"-out".to_string()));
    }

    #[test]
    fn a_missing_tool_names_the_package_to_install() {
        let error = std::io::Error::from(std::io::ErrorKind::NotFound);
        let said = missing("wl-copy", &error).to_string();
        assert!(said.contains("wl-clipboard"), "got {said}");
        let said = missing("xclip", &error).to_string();
        assert!(said.contains("xclip"), "got {said}");
    }
}
