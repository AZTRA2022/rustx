//! Le gestionnaire de fichiers, sous Linux : D-Bus pour montrer un
//! fichier, les fichiers de freedesktop.org pour les associations.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; le texte
//! des fichiers `.desktop` et du type MIME a ses tests, qui tournent
//! partout. Aucun gestionnaire de fichiers n'a été vu s'ouvrir, aucun
//! double-clic n'a ouvert l'application. À essayer sur un vrai bureau.
//!
//! # Refusé
//!
//! Les documents récents : ils vivent dans
//! `~/.local/share/recently-used.xbel`, un fichier XML que GTK réécrit
//! en entier et que plusieurs applications ouvertes tiennent en même
//! temps. L'écrire à la main, c'est risquer de perdre l'historique de
//! toutes les autres ; la seule façon sûre est `GtkRecentManager`, que
//! ce crate ne lie pas.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::shell::{Association, file_uri};

fn ran(program: &str, arguments: &[&str]) -> bool {
    Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// `org.freedesktop.FileManager1.ShowItems`, que Nautilus, Dolphin,
/// Nemo, Thunar et Caja exposent tous — le seul moyen de *sélectionner*
/// le fichier. Sans lui, le dossier s'ouvre, sans sélection, et c'est
/// mieux que rien.
pub fn reveal(path: &Path) -> Result<(), OsError> {
    let absolute = std::fs::canonicalize(path)
        .map_err(|error| OsError::File(format!("{}: {error}", path.display())))?;
    let uri = file_uri(&absolute.to_string_lossy());
    if ran(
        "dbus-send",
        &[
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
            &format!("array:string:{uri}"),
            "string:",
        ],
    ) {
        return Ok(());
    }
    let folder = absolute.parent().unwrap_or(&absolute);
    if ran("xdg-open", &[&folder.to_string_lossy()]) {
        return Ok(());
    }
    Err(OsError::Platform(
        "neither a file manager on D-Bus (org.freedesktop.FileManager1) nor `xdg-open` \
         answered"
            .to_string(),
    ))
}

pub fn remember_recent(_path: &Path) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "recent documents on Linux (GTK's recently-used.xbel is only safe to write through \
         GtkRecentManager)",
    ))
}

/// Le type MIME que ce crate donne à l'extension : `application/x-` et
/// l'identifiant, en minuscules, les points devenus des tirets.
pub(crate) fn mime_of(association: &Association) -> String {
    format!(
        "application/x-{}",
        association.id.to_ascii_lowercase().replace('.', "-")
    )
}

fn data_home() -> Result<PathBuf, OsError> {
    if let Some(home) = std::env::var_os("XDG_DATA_HOME").filter(|home| !home.is_empty()) {
        return Ok(PathBuf::from(home));
    }
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share"))
        .ok_or_else(|| OsError::Platform("neither XDG_DATA_HOME nor HOME is set".to_string()))
}

/// Le paquet de types MIME : l'extension, le type, et sa description.
pub(crate) fn mime_package(association: &Association) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n  \
         <mime-type type=\"{}\">\n    <comment>{}</comment>\n    \
         <glob pattern=\"*.{}\"/>\n  </mime-type>\n</mime-info>\n",
        mime_of(association),
        escape(&association.description),
        association.extension
    )
}

/// L'entrée de bureau qui dit « ce programme ouvre ce type ».
pub(crate) fn desktop_entry(association: &Association) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec=\"{}\" %f\nMimeType={};\n\
         NoDisplay=true\n",
        association.description,
        association.program.display(),
        mime_of(association)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn files(association: &Association) -> Result<(PathBuf, PathBuf, String), OsError> {
    let home = data_home()?;
    let name = association.id.to_ascii_lowercase();
    Ok((
        home.join("mime/packages").join(format!("{name}.xml")),
        home.join("applications").join(format!("{name}.desktop")),
        format!("{name}.desktop"),
    ))
}

fn write(path: &Path, text: &str) -> Result<(), OsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| OsError::File(format!("{}: {error}", parent.display())))?;
    }
    std::fs::write(path, text).map_err(|error| OsError::File(format!("{}: {error}", path.display())))
}

pub fn associate(association: &Association) -> Result<(), OsError> {
    let (package, entry, entry_name) = files(association)?;
    write(&package, &mime_package(association))?;
    write(&entry, &desktop_entry(association))?;
    let home = data_home()?;
    // Les bases que les bureaux lisent : sans leur mise à jour, les deux
    // fichiers existent et personne ne les a lus.
    let mime_dir = home.join("mime");
    let _ = ran("update-mime-database", &[&mime_dir.to_string_lossy()]);
    let _ = ran(
        "update-desktop-database",
        &[&home.join("applications").to_string_lossy()],
    );
    if ran("xdg-mime", &["default", &entry_name, &mime_of(association)]) {
        Ok(())
    } else {
        Err(OsError::Platform(
            "the files are written, but `xdg-mime default` did not run — the association \
             will not be the default until it does"
                .to_string(),
        ))
    }
}

pub fn dissociate(association: &Association) -> Result<(), OsError> {
    let (package, entry, _) = files(association)?;
    for path in [&package, &entry] {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(OsError::File(format!("{}: {error}", path.display()))),
        }
    }
    let home = data_home()?;
    let _ = ran("update-mime-database", &[&home.join("mime").to_string_lossy()]);
    let _ = ran(
        "update-desktop-database",
        &[&home.join("applications").to_string_lossy()],
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Association {
        Association {
            extension: "rxdoc".into(),
            id: "My.Editor".into(),
            description: "Docs & <notes>".into(),
            program: "/opt/my editor/bin/editor".into(),
        }
    }

    #[test]
    fn the_mime_type_is_derived_from_the_identifier() {
        assert_eq!(mime_of(&sample()), "application/x-my-editor");
    }

    #[test]
    fn the_package_globs_the_extension_and_escapes_the_comment() {
        let package = mime_package(&sample());
        assert!(package.contains("<glob pattern=\"*.rxdoc\"/>"), "{package}");
        assert!(package.contains("Docs &amp; &lt;notes&gt;"), "{package}");
        assert!(package.contains("type=\"application/x-my-editor\""), "{package}");
    }

    #[test]
    fn the_desktop_entry_quotes_the_program_and_names_the_type() {
        let entry = desktop_entry(&sample());
        assert!(entry.contains("Exec=\"/opt/my editor/bin/editor\" %f"), "{entry}");
        assert!(entry.contains("MimeType=application/x-my-editor;"), "{entry}");
    }
}
