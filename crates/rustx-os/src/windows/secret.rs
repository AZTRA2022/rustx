//! Les secrets, sous Windows : le Gestionnaire d'identification
//! (`CredWriteW`), en identifiants génériques.
//!
//! La cible s'écrit `service/compte`, et c'est sous ce nom que
//! l'utilisateur la voit dans « Identifiants Windows ». Le secret est
//! rangé en UTF-8 : c'est ce que les autres plateformes rangent aussi,
//! et ce qu'un outil qui le relirait ailleurs attend.

use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::ERROR_NOT_FOUND;
use windows::Win32::Security::Credentials::{
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW,
    CredWriteW,
};
use windows::core::{PCWSTR, PWSTR};

use crate::error::OsError;

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub(crate) fn target(service: &str, account: &str) -> String {
    format!("{service}/{account}")
}

fn absent(error: &windows::core::Error) -> bool {
    error.code() == ERROR_NOT_FOUND.to_hresult()
}

pub fn store(service: &str, account: &str, secret: &str) -> Result<(), OsError> {
    let mut name = wide(&target(service, account));
    let mut user = wide(account);
    let mut blob = secret.as_bytes().to_vec();
    // 2 560 octets : la limite de `CRED_MAX_CREDENTIAL_BLOB_SIZE`.
    if blob.len() > 5 * 512 {
        return Err(OsError::Platform(format!(
            "a secret is at most 2560 bytes in the Windows Credential Manager; this one is {}",
            blob.len()
        )));
    }
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(name.as_mut_ptr()),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: PWSTR(user.as_mut_ptr()),
        ..Default::default()
    };
    // SAFETY: chaque pointeur de la structure vit jusqu'à la fin de cette
    // fonction ; Windows copie ce qu'il garde.
    unsafe { CredWriteW(&credential, 0) }.map_err(|error| {
        OsError::Platform(format!(
            "the Credential Manager refused the secret: {error}"
        ))
    })
}

pub fn fetch(service: &str, account: &str) -> Result<Option<String>, OsError> {
    let name = wide(&target(service, account));
    let mut found: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: un nom terminé par zéro, et un pointeur que Windows remplit
    // et que `CredFree` libère.
    match unsafe { CredReadW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None, &mut found) } {
        Ok(()) => {}
        Err(error) if absent(&error) => return Ok(None),
        Err(error) => {
            return Err(OsError::Platform(format!(
                "the Credential Manager could not be read: {error}"
            )));
        }
    }
    // SAFETY: `found` est la structure que `CredReadW` vient de rendre ;
    // le blob a la taille qu'elle annonce.
    let secret = unsafe {
        let credential = &*found;
        let bytes = if credential.CredentialBlob.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(
                credential.CredentialBlob,
                credential.CredentialBlobSize as usize,
            )
        };
        let secret = String::from_utf8(bytes.to_vec());
        CredFree(found.cast());
        secret
    };
    secret.map(Some).map_err(|_| {
        OsError::Platform(
            "the stored secret is not UTF-8 — it was not written by rustx".to_string(),
        )
    })
}

pub fn forget(service: &str, account: &str) -> Result<(), OsError> {
    let name = wide(&target(service, account));
    // SAFETY: un nom terminé par zéro.
    match unsafe { CredDeleteW(PCWSTR(name.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(error) if absent(&error) => Ok(()),
        Err(error) => Err(OsError::Platform(format!(
            "the Credential Manager refused to remove the secret: {error}"
        ))),
    }
}
