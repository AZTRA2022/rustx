//! Un secret rangé là où le système range les siens : le Gestionnaire
//! d'identification de Windows, le trousseau de macOS, le Secret
//! Service de Linux (GNOME Keyring, KWallet).
//!
//! Un jeton d'API ou un mot de passe n'a rien à faire dans un fichier
//! de configuration en clair. Ces trois magasins le chiffrent avec la
//! session de l'utilisateur, et c'est l'utilisateur — pas l'application
//! — qui peut l'y voir et l'en retirer.
//!
//! Un secret est désigné par deux noms : le *service* (l'application,
//! ou le serveur auquel il ouvre) et le *compte*.

use crate::error::OsError;

/// Ranger un secret, en remplaçant celui qui portait ces deux noms.
pub fn store(service: &str, account: &str, secret: &str) -> Result<(), OsError> {
    named(service, account)?;
    crate::blocking::waiting("the system's secret store");
    crate::platform::secret::store(service, account, secret)
}

/// Le secret rangé sous ces deux noms, ou `None` s'il n'y en a pas.
pub fn fetch(service: &str, account: &str) -> Result<Option<String>, OsError> {
    named(service, account)?;
    crate::blocking::waiting("the system's secret store");
    crate::platform::secret::fetch(service, account)
}

/// Retirer le secret. Retirer ce qui n'existe pas n'est pas une erreur.
pub fn forget(service: &str, account: &str) -> Result<(), OsError> {
    named(service, account)?;
    crate::blocking::waiting("the system's secret store");
    crate::platform::secret::forget(service, account)
}

fn named(service: &str, account: &str) -> Result<(), OsError> {
    if service.is_empty() || account.is_empty() {
        return Err(OsError::Platform(
            "a secret is named by a service and an account, and neither may be empty".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_nameless_secret_is_refused() {
        assert!(super::store("", "me", "x").is_err());
        assert!(super::fetch("svc", "").is_err());
    }
}
