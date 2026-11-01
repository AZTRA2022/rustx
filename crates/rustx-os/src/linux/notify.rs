//! Les notifications, sous Linux.
//!
//! Pas encore, et elles le disent. Ce que ce sera :
//! `org.freedesktop.Notifications.Notify` sur D-Bus — la spécification
//! que GNOME, KDE et les autres implémentent tous, et qui donne le
//! titre, le corps, l'icône et même des actions. Pas de dépendance à
//! `libnotify` : c'est un appel D-Bus, et le portail le relaie en bac à
//! sable.

use crate::error::OsError;
use crate::notify::What;

pub const AVAILABLE: bool = false;

pub fn show(_what: &What) -> Result<(), OsError> {
    Err(OsError::Unsupported("les notifications"))
}
