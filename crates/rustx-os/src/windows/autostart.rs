//! Le démarrage automatique de Windows : la clé `Run` de l'utilisateur.
//!
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, une valeur par
//! application, dont le contenu est la ligne de commande à lancer.
//! Aucun droit d'administrateur, parce que c'est la branche de
//! l'utilisateur et non celle de la machine — et parce qu'une
//! application qui demande à être administrateur pour cocher une case
//! est une application qu'on n'installe pas.
//!
//! Le chemin est entre guillemets : sans eux, un exécutable dans
//! `C:\Program Files\…` est lu comme plusieurs mots, et Windows lance
//! `C:\Program`.

use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;

use crate::error::OsError;

/// Windows sait le faire, et ce fichier le fait.
pub const AVAILABLE: bool = true;

/// Là où Windows lit ce qu'il faut lancer à l'ouverture de session.
const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn set(on: bool, name: &str) -> Result<(), OsError> {
    let key = opened(KEY_WRITE)?;
    let value = wide(name);
    let done = if on {
        let exe = std::env::current_exe().map_err(|error| {
            OsError::File(format!(
                "le chemin de cet exécutable n'a pas pu être lu : {error}"
            ))
        })?;
        // Entre guillemets : un chemin avec un espace, sinon, est lu
        // comme plusieurs arguments.
        let line = wide(&format!("\"{}\"", exe.display()));
        unsafe {
            RegSetValueExW(
                key.0,
                PCWSTR(value.as_ptr()),
                None,
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    line.as_ptr() as *const u8,
                    line.len() * 2,
                )),
            )
        }
    } else {
        let removed = unsafe { RegDeleteValueW(key.0, PCWSTR(value.as_ptr())) };
        // Retirer ce qui n'y est pas n'est pas une panne : la case
        // était déjà décochée.
        if removed != ERROR_SUCCESS
            && removed != windows::Win32::Foundation::ERROR_FILE_NOT_FOUND
        {
            removed
        } else {
            ERROR_SUCCESS
        }
    };

    if done != ERROR_SUCCESS {
        return Err(OsError::Platform(format!(
            "le registre a refusé (code {}) — la clé est \
             HKCU\\{RUN}, valeur `{name}`",
            done.0
        )));
    }
    Ok(())
}

pub fn is_on(name: &str) -> bool {
    let Ok(key) = opened(KEY_READ) else {
        return false;
    };
    let value = wide(name);
    let mut size = 0_u32;
    let asked = unsafe {
        RegQueryValueExW(
            key.0,
            PCWSTR(value.as_ptr()),
            None,
            None,
            None,
            Some(&mut size),
        )
    };
    asked == ERROR_SUCCESS
}

/// La clé `Run`, refermée quand on la lâche.
fn opened(rights: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Result<Key, OsError> {
    let path = wide(RUN);
    let mut key = HKEY::default();
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            rights,
            &mut key,
        )
    };
    if opened != ERROR_SUCCESS {
        return Err(OsError::Platform(format!(
            "HKCU\\{RUN} n'a pas pu être ouverte (code {})",
            opened.0
        )));
    }
    Ok(Key(key))
}

/// Une clé de registre ouverte. Refermée par son `Drop`, parce qu'une
/// clé laissée ouverte est un handle que le processus garde pour rien.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

/// Une chaîne Windows, terminée par zéro.
fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
