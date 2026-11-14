//! Le gestionnaire de fichiers, sous Windows : l'Explorateur, les
//! documents récents de `shell32`, et les associations sous
//! `HKCU\Software\Classes`.
//!
//! # Pour cet utilisateur, et sans droits d'administrateur
//!
//! Les associations vont dans `HKCU\Software\Classes`, que Windows
//! fusionne avec `HKLM` pour former `HKEY_CLASSES_ROOT` : l'utilisateur
//! qui installe l'application l'obtient, sans élévation. Depuis
//! Windows 8, un utilisateur qui a déjà *choisi* une autre application
//! pour cette extension garde son choix — Windows protège ce choix par
//! un hachage qu'aucun programme n'est censé écrire, et ce fichier ne
//! le fait pas. L'association devient alors une proposition dans
//! « Ouvrir avec », ce qui est le comportement voulu par Windows.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegDeleteTreeW, RegGetValueW,
    RegSetKeyValueW,
};
use windows::Win32::UI::Shell::{
    ILCreateFromPathW, ILFree, SHARD_PATHW, SHAddToRecentDocs, SHCNE_ASSOCCHANGED, SHCNF_IDLIST,
    SHChangeNotify, SHOpenFolderAndSelectItems,
};
use windows::core::PCWSTR;

use crate::error::OsError;
use crate::shell::Association;

fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

fn wide_str(text: &str) -> Vec<u16> {
    wide(std::ffi::OsStr::new(text))
}

/// `SHOpenFolderAndSelectItems`, et non `explorer /select,` : la ligne
/// de commande de l'Explorateur a ses propres règles de guillemets, qui
/// ne sont pas celles que `Command` écrit, et un chemin avec une virgule
/// la casse.
pub fn reveal(path: &Path) -> Result<(), OsError> {
    let absolute = std::fs::canonicalize(path)
        .map_err(|error| OsError::File(format!("{}: {error}", path.display())))?;
    // `canonicalize` rend un chemin `\\?\C:\…`, que le Shell ne lit pas.
    let shown = absolute.to_string_lossy();
    let shown = shown.strip_prefix(r"\\?\").unwrap_or(&shown);
    let name = wide_str(shown);
    // SAFETY: COM pour ce fil ; déjà initialisé, l'appel le dit et c'est
    // sans conséquence.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // SAFETY: une chaîne terminée par zéro qui vit le temps de l'appel.
    let item = unsafe { ILCreateFromPathW(PCWSTR(name.as_ptr())) };
    if item.is_null() {
        return Err(OsError::Platform(format!(
            "the Shell could not name {}",
            path.display()
        )));
    }
    // SAFETY: l'élément vient d'être créé, et il est libéré juste après.
    let shown = unsafe { SHOpenFolderAndSelectItems(item, None, 0) };
    // SAFETY: idem.
    unsafe { ILFree(Some(item)) };
    shown.map_err(|error| {
        OsError::Platform(format!("the Explorer did not open on {}: {error}", path.display()))
    })
}

pub fn remember_recent(path: &Path) -> Result<(), OsError> {
    let absolute = std::fs::canonicalize(path)
        .map_err(|error| OsError::File(format!("{}: {error}", path.display())))?;
    let text = absolute.to_string_lossy();
    let name = wide_str(text.strip_prefix(r"\\?\").unwrap_or(&text));
    // SAFETY: une chaîne terminée par zéro, ce que `SHARD_PATHW` attend.
    unsafe { SHAddToRecentDocs(SHARD_PATHW.0 as u32, Some(name.as_ptr().cast())) };
    Ok(())
}

fn classes(below: &str) -> Vec<u16> {
    wide_str(&format!(r"Software\Classes\{below}"))
}

fn written(result: WIN32_ERROR, what: &str) -> Result<(), OsError> {
    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(OsError::Platform(format!(
            "{what} was refused by the registry: {:?}",
            windows::core::Error::from(result.to_hresult())
        )))
    }
}

/// La valeur par défaut de cette clé, en texte.
fn set_default(key: &str, value: &str) -> Result<(), OsError> {
    let key_name = classes(key);
    let data = wide_str(value);
    // SAFETY: une clé et une valeur terminées par zéro ; la taille est
    // celle des données, zéro compris.
    let result = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key_name.as_ptr()),
            PCWSTR::null(),
            REG_SZ.0,
            Some(data.as_ptr().cast()),
            (data.len() * 2) as u32,
        )
    };
    written(result, &format!(r"writing HKCU\Software\Classes\{key}"))
}

/// La valeur par défaut d'une clé, si elle existe.
pub(crate) fn default_of(key: &str) -> Option<String> {
    let key_name = classes(key);
    let mut buffer = [0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: un tampon de cette pile, et sa taille en octets.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key_name.as_ptr()),
            PCWSTR::null(),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS {
        return None;
    }
    let length = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..length]))
}

/// La commande qui ouvre un fichier : le programme et `%1`, chacun entre
/// guillemets — un chemin avec une espace en vaut deux sinon.
pub(crate) fn command_for(program: &Path) -> String {
    format!("\"{}\" \"%1\"", program.display())
}

/// Dire à l'Explorateur que les associations ont changé : sans cela, il
/// garde l'ancienne icône et l'ancien programme jusqu'à sa relance.
fn announce() {
    // SAFETY: un événement global, sans élément.
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

pub fn associate(association: &Association) -> Result<(), OsError> {
    let id = &association.id;
    set_default(&format!(".{}", association.extension), id)?;
    set_default(id, &association.description)?;
    set_default(
        &format!(r"{id}\shell\open\command"),
        &command_for(&association.program),
    )?;
    announce();
    Ok(())
}

pub fn dissociate(association: &Association) -> Result<(), OsError> {
    let id = &association.id;
    let program = classes(id);
    // SAFETY: une clé terminée par zéro.
    let result = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(program.as_ptr())) };
    if result != ERROR_FILE_NOT_FOUND {
        written(result, &format!(r"removing HKCU\Software\Classes\{id}"))?;
    }
    // L'extension n'est rendue que si elle pointe encore vers nous : une
    // autre application a pu la prendre depuis, et ce n'est pas à nous de
    // la lui retirer.
    let extension = format!(".{}", association.extension);
    if default_of(&extension).as_deref() == Some(id.as_str()) {
        let key = classes(&extension);
        // SAFETY: une clé terminée par zéro ; la valeur nulle est la
        // valeur par défaut.
        let result =
            unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(key.as_ptr()), PCWSTR::null()) };
        written(result, &format!(r"removing HKCU\Software\Classes\{extension}"))?;
    }
    announce();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_open_command_quotes_both_halves() {
        assert_eq!(
            command_for(Path::new(r"C:\Program Files\App\app.exe")),
            r#""C:\Program Files\App\app.exe" "%1""#
        );
    }
}
