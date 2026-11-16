//! Ce que le gestionnaire de fichiers sait faire d'un fichier : le
//! montrer, s'en souvenir, et savoir quelle application l'ouvre.
//!
//! Trois choses qu'une application de documents doit à son système :
//! « Afficher dans le dossier » après un export, sa place dans les
//! documents récents, et le double-clic sur un `.monformat` qui l'ouvre.

use std::path::Path;

use crate::error::OsError;

/// Ouvrir le gestionnaire de fichiers sur le dossier de ce fichier, le
/// fichier sélectionné.
pub fn reveal(path: &Path) -> Result<(), OsError> {
    existing(path)?;
    crate::platform::shell::reveal(path)
}

/// Ajouter ce fichier aux documents récents du système.
pub fn remember_recent(path: &Path) -> Result<(), OsError> {
    existing(path)?;
    crate::platform::shell::remember_recent(path)
}

/// Une association : quelle extension, sous quel nom, ouverte par quel
/// programme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Association {
    /// Sans le point : `"rxdoc"`.
    pub extension: String,
    /// Un identifiant stable, propre à l'application :
    /// `"MonEditeur.Document"`.
    pub id: String,
    /// Ce que l'explorateur affiche comme type : `"Document MonÉditeur"`.
    pub description: String,
    /// Le programme qui ouvre le fichier, à qui le chemin est passé en
    /// premier argument.
    pub program: std::path::PathBuf,
}

impl Association {
    /// Refuser ce qui ne ferait pas une association : une extension avec
    /// un point ou un séparateur, un identifiant vide.
    pub fn checked(&self) -> Result<(), OsError> {
        let bad = |c: char| c == '.' || c == '/' || c == '\\' || c.is_whitespace();
        if self.extension.is_empty() || self.extension.chars().any(bad) {
            return Err(OsError::Platform(format!(
                "`{}` is not an extension — write it without the dot, e.g. `rxdoc`",
                self.extension
            )));
        }
        if self.id.is_empty()
            || self
                .id
                .chars()
                .any(|c| c == '/' || c == '\\' || c.is_whitespace())
        {
            return Err(OsError::Platform(format!(
                "`{}` is not an identifier — letters, digits and dots, e.g. `MyEditor.Document`",
                self.id
            )));
        }
        Ok(())
    }
}

/// A link, opened where the user's links open.
///
/// The gap this closes was written down as "not written, and
/// deliberately: see security". The reason given was that a window
/// which loads remote content is a window that can be attacked -- which
/// is true, and is why this does not load it. It hands the address to
/// the system, which gives it to the browser the user chose. The page
/// is drawn by Chrome or Safari or Firefox, in their process, with
/// their sandbox, and the application is not involved.
///
/// Three schemes and no others: `https`, `http` and `mailto`. The
/// refusal of the rest is the security answer rather than its absence:
/// `file://` would make "open this link" read a local file, and a
/// scheme of somebody else's choosing is an arbitrary program started
/// with an argument from a page. An application that wants to open a
/// file has [`reveal`], and one that wants to start a program has
/// `rustx::terminal`.
///
/// ```ignore
/// rustx::shell::open_url("https://rustx.dev/docs")?;
/// rustx::shell::open_url("mailto:support@example.test?subject=Crash")?;
/// ```
pub fn open_url(url: &str) -> Result<(), OsError> {
    checked_url(url)?;
    crate::platform::shell::open_url(url)
}

/// What [`open_url`] will hand over, and what it will not.
fn checked_url(url: &str) -> Result<(), OsError> {
    let allowed = ["https://", "http://", "mailto:"];
    if !allowed.iter().any(|start| url.starts_with(start)) {
        return Err(OsError::Platform(format!(
            "`{}` is not a link this opens. It takes https, http and mailto, and \
             refuses the rest on purpose: a scheme of somebody else's choosing \
             starts a program of somebody else's choosing",
            first_word(url)
        )));
    }
    if url.contains('\n') || url.contains('\r') || url.contains('\0') {
        return Err(OsError::Platform(
            "that link holds a line break or a zero byte, which no link does".to_string(),
        ));
    }
    Ok(())
}

/// The front of a URL, for a message: the whole of it may be long and
/// may hold a token.
fn first_word(url: &str) -> String {
    let shown: String = url.chars().take(40).collect();
    if url.chars().count() > 40 {
        format!("{shown}...")
    } else {
        shown
    }
}

/// A URL scheme this application answers: `monapp://…`.
///
/// What it is for: a link in a browser, an e-mail or another
/// application that opens *this* one, with the rest of the address
/// handed over as an argument. Electron calls it
/// `setAsDefaultProtocolClient`, Tauri calls it a deep link, and the
/// documentation said it was not written.
///
/// The address arrives as the program's first argument. With
/// [`crate::instance::Only`] held, a second start hands it to the copy
/// that is already running instead -- which is what makes a link open
/// the window that is already there rather than a second one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheme {
    /// Without the colon or the slashes: `"monapp"`.
    pub scheme: String,
    /// What the system shows when it asks the user whether to allow
    /// it: `"Mon App"`.
    pub name: String,
    /// The program the address is handed to.
    pub program: std::path::PathBuf,
}

impl Scheme {
    /// Refuse what would not be a scheme. The rules are RFC 3986's: a
    /// letter, then letters, digits, `+`, `-` and `.`.
    pub fn checked(&self) -> Result<(), OsError> {
        let mut letters = self.scheme.chars();
        let first = letters.next();
        let shaped = first.is_some_and(|letter| letter.is_ascii_alphabetic())
            && letters.all(|letter| {
                letter.is_ascii_alphanumeric() || letter == '+' || letter == '-' || letter == '.'
            });
        if !shaped {
            return Err(OsError::Platform(format!(
                "`{}` is not a scheme -- a letter, then letters, digits, `+`, `-` and \
                 `.`, and no colon: `monapp`, not `monapp://`",
                self.scheme
            )));
        }
        // The schemes the machine already answers. Taking one of these
        // would break the browser, the mail client or the file manager
        // for the whole user account.
        const TAKEN: [&str; 10] = [
            "http",
            "https",
            "file",
            "mailto",
            "ftp",
            "data",
            "javascript",
            "about",
            "tel",
            "sms",
        ];
        if TAKEN.contains(&self.scheme.to_ascii_lowercase().as_str()) {
            return Err(OsError::Platform(format!(
                "`{}` belongs to the system, and taking it would break it for every \
                 program this user runs",
                self.scheme
            )));
        }
        if self.name.trim().is_empty() {
            return Err(OsError::Platform(
                "a scheme needs a name: it is what the system shows the user when it \
                 asks whether to allow the link"
                    .to_string(),
            ));
        }
        Ok(())
    }
}

/// Make this program the one that answers `scheme://…`, for this user.
///
/// Per user and never for the machine: writing a system-wide handler
/// needs administrator rights, and a desktop application that asks for
/// them to register a link is an application people stop installing.
pub fn register_scheme(scheme: &Scheme) -> Result<(), OsError> {
    scheme.checked()?;
    crate::platform::shell::register_scheme(scheme)
}

/// Undo it. A scheme that was never registered is not an error.
pub fn unregister_scheme(scheme: &Scheme) -> Result<(), OsError> {
    scheme.checked()?;
    crate::platform::shell::unregister_scheme(scheme)
}

/// Faire de ce programme celui qui ouvre ces fichiers, pour cet
/// utilisateur.
pub fn associate(association: &Association) -> Result<(), OsError> {
    association.checked()?;
    crate::platform::shell::associate(association)
}

/// Défaire ce qu'[`associate`] a fait.
pub fn dissociate(association: &Association) -> Result<(), OsError> {
    association.checked()?;
    crate::platform::shell::dissociate(association)
}

fn existing(path: &Path) -> Result<(), OsError> {
    if path.exists() {
        Ok(())
    } else {
        Err(OsError::File(format!("{} does not exist", path.display())))
    }
}

// Lu par `linux/` seulement.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// Un chemin absolu en URI `file://`, chaque octet hors des caractères
/// sûrs écrit `%XX` — ce que les gestionnaires de fichiers de Linux
/// attendent sur D-Bus.
pub(crate) fn file_uri(path: &str) -> String {
    let mut uri = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_refused_by_name() {
        let said = reveal(Path::new("nowhere/at/all.txt")).expect_err("no such file");
        assert!(said.to_string().contains("all.txt"), "got {said}");
    }

    #[test]
    fn an_extension_is_written_without_its_dot() {
        let mut association = Association {
            extension: ".rxdoc".into(),
            id: "Rustx.Test".into(),
            description: "Test".into(),
            program: "x".into(),
        };
        assert!(association.checked().is_err());
        association.extension = "rxdoc".into();
        assert!(association.checked().is_ok());
        association.id = "has space".into();
        assert!(association.checked().is_err());
    }

    #[test]
    fn a_path_becomes_a_percent_encoded_uri() {
        assert_eq!(
            file_uri("/home/me/a b/é.txt"),
            "file:///home/me/a%20b/%C3%A9.txt"
        );
        assert_eq!(file_uri("/tmp/x-y_z.~"), "file:///tmp/x-y_z.~");
    }
}
