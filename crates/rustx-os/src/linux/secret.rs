//! Les secrets, sous Linux : le Secret Service, par `secret-tool`.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement. Aucun
//! secret n'a été rangé dans un vrai trousseau GNOME ou KWallet depuis
//! ce fichier.
//!
//! `secret-tool` (paquet `libsecret-tools`) parle au Secret Service —
//! GNOME Keyring, ou KWallet depuis la version 5.97 — et lit le secret
//! sur son entrée standard : il ne passe jamais par la ligne de
//! commande, où `ps` le montrerait à tout l'ordinateur.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::OsError;

fn missing(error: std::io::Error) -> OsError {
    OsError::Platform(format!(
        "`secret-tool` could not be run ({error}) — it is in the `libsecret-tools` package"
    ))
}

pub fn store(service: &str, account: &str, secret: &str) -> Result<(), OsError> {
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            &format!("--label={service} ({account})"),
            "service",
            service,
            "account",
            account,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(missing)?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(secret.as_bytes())
            .map_err(|error| OsError::Platform(format!("writing to secret-tool: {error}")))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|error| OsError::Platform(format!("secret-tool: {error}")))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(OsError::Platform(format!(
            "the Secret Service refused the secret: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

pub fn fetch(service: &str, account: &str) -> Result<Option<String>, OsError> {
    let out = Command::new("secret-tool")
        .args(["lookup", "service", service, "account", account])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(missing)?;
    // `lookup` sort en échec, sans rien écrire, quand rien ne porte ces
    // noms : c'est l'absence, pas une panne.
    if !out.status.success() && out.stdout.is_empty() {
        return Ok(None);
    }
    String::from_utf8(out.stdout)
        .map(Some)
        .map_err(|_| OsError::Platform("the stored secret is not UTF-8".to_string()))
}

pub fn forget(service: &str, account: &str) -> Result<(), OsError> {
    let status = Command::new("secret-tool")
        .args(["clear", "service", service, "account", account])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(missing)?;
    // `clear` réussit aussi quand il n'y avait rien.
    if status.success() {
        Ok(())
    } else {
        Err(OsError::Platform(format!(
            "secret-tool clear failed ({status})"
        )))
    }
}
