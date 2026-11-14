//! Ce que la machine est en ce moment : son thème, sa batterie, si
//! quelqu'un est devant, sa langue, son heure, sa densité d'écran.
//!
//! Des lectures, toutes : rien ici ne change l'état de la machine. Une
//! page qui veut suivre le thème sombre, se faire discrète sur batterie
//! ou mettre en pause ce que personne ne regarde a besoin de le
//! demander au système — le navigateur n'en dit qu'une partie, et
//! jamais le verrouillage ni l'inactivité.
//!
//! Chaque lecture qu'une plateforme ne sait pas faire est refusée en
//! nommant la plateforme et la lecture, jamais devinée : un thème
//! « clair » rendu par défaut parce qu'on n'a pas su lire est un
//! mensonge que l'application affiche.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use crate::error::OsError;

/// Le thème que l'utilisateur a choisi pour ses applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

/// L'alimentation, telle que le système la rapporte.
///
/// Des `Option` partout où une machine peut ne pas savoir : un poste
/// fixe n'a pas de niveau de batterie, et « 100 % » n'en serait pas un.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Power {
    /// Branchée sur le secteur. `None` si le système ne le dit pas.
    pub plugged: Option<bool>,
    /// Le niveau, de 0 à 100. `None` sans batterie.
    pub level: Option<u8>,
    /// En charge. `None` sans batterie.
    pub charging: Option<bool>,
    /// Le mode économie d'énergie. `None` si la plateforme ne l'expose
    /// pas.
    pub saver: Option<bool>,
}

/// Le thème des applications, maintenant.
pub fn theme() -> Result<Theme, OsError> {
    crate::platform::system::theme()
}

/// L'alimentation, maintenant.
pub fn power() -> Result<Power, OsError> {
    crate::platform::system::power()
}

/// Depuis combien de temps personne n'a touché ni clavier ni souris.
pub fn idle() -> Result<Duration, OsError> {
    crate::platform::system::idle()
}

/// Si la session est verrouillée.
pub fn locked() -> Result<bool, OsError> {
    crate::platform::system::locked()
}

/// La langue de l'utilisateur, en étiquette BCP 47 : `fr-FR`, `en-US`.
pub fn locale() -> Result<String, OsError> {
    crate::platform::system::locale()
}

/// Le fuseau horaire.
///
/// Un nom IANA (`Europe/Paris`) sous Linux et macOS ; sous Windows, le
/// nom que Windows lui donne (`Romance Standard Time`), parce que c'est
/// le seul que le système tienne et qu'une table de correspondance
/// maintenue ici vieillirait plus vite que lui.
pub fn timezone() -> Result<String, OsError> {
    crate::platform::system::timezone()
}

/// Le facteur d'échelle de l'écran principal : `1.0` à 96 dpi, `2.0`
/// sur un écran Retina ou à 200 %.
pub fn scale() -> Result<f64, OsError> {
    crate::platform::system::scale()
}

/// Lire `AppsUseLightTheme` à la main est ce que toutes les
/// plateformes font ; être *prévenu* d'un changement demande, sur
/// chacune, une boucle de messages différente (`WM_SETTINGCHANGE`,
/// une notification distribuée, un signal D-Bus). Ceci relit à
/// intervalle : une seconde de retard sur un changement de thème est
/// invisible, et c'est le même code partout.
pub struct ThemeWatch {
    changed: Arc<Mutex<Option<Theme>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ThemeWatch {
    /// Commencer à surveiller. Refusé si la plateforme ne sait pas lire
    /// le thème du tout.
    pub fn start() -> Result<Self, OsError> {
        Self::every(Duration::from_secs(1))
    }

    /// Idem, à l'intervalle donné.
    pub fn every(interval: Duration) -> Result<Self, OsError> {
        let first = theme()?;
        let changed = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (theirs, halt) = (changed.clone(), stop.clone());
        let thread = std::thread::Builder::new()
            .name("rustx-theme".to_string())
            .spawn(move || {
                let mut last = first;
                while !halt.load(Ordering::Acquire) {
                    std::thread::sleep(interval);
                    if let Ok(now) = theme()
                        && now != last
                    {
                        last = now;
                        *theirs.lock().unwrap_or_else(|p| p.into_inner()) = Some(now);
                    }
                }
            })
            .map_err(|error| {
                OsError::Platform(format!("le fil du thème n'a pas démarré : {error}"))
            })?;
        Ok(Self {
            changed,
            stop,
            thread: Some(thread),
        })
    }

    /// Le nouveau thème, s'il a changé depuis la dernière lecture.
    pub fn changed(&self) -> Option<Theme> {
        self.changed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

impl Drop for ThemeWatch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

// Lus par `linux/` et `macos/` ; Windows a ses propres appels.
#[cfg_attr(windows, allow(dead_code))]
/// `fr_FR.UTF-8`, `fr_FR@euro`, `fr_FR` → `fr-FR`. Ce que POSIX et
/// macOS écrivent, en étiquette BCP 47.
pub(crate) fn bcp47(posix: &str) -> Option<String> {
    let bare = posix.split(['.', '@']).next()?.trim();
    if bare.is_empty() || bare == "C" || bare == "POSIX" {
        return None;
    }
    Some(bare.replace('_', "-"))
}

#[cfg_attr(windows, allow(dead_code))]
/// Le nom IANA dans la cible du lien `/etc/localtime`.
pub(crate) fn zone_from_link(target: &str) -> Option<String> {
    let (_, zone) = target.split_once("zoneinfo/")?;
    let zone = zone.trim_matches('/');
    (!zone.is_empty()).then(|| zone.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_posix_locale_becomes_a_bcp47_tag() {
        assert_eq!(bcp47("fr_FR.UTF-8").as_deref(), Some("fr-FR"));
        assert_eq!(bcp47("de_DE@euro").as_deref(), Some("de-DE"));
        assert_eq!(bcp47("en").as_deref(), Some("en"));
        assert_eq!(bcp47("C.UTF-8"), None, "C is no language at all");
        assert_eq!(bcp47(""), None);
    }

    #[test]
    fn the_zone_is_read_from_the_localtime_link() {
        assert_eq!(
            zone_from_link("/usr/share/zoneinfo/Europe/Paris").as_deref(),
            Some("Europe/Paris")
        );
        assert_eq!(
            zone_from_link("/var/db/timezone/zoneinfo/America/New_York").as_deref(),
            Some("America/New_York")
        );
        assert_eq!(zone_from_link("/etc/something"), None);
    }

    #[test]
    fn this_machine_answers_or_refuses_by_name() {
        // Aucune valeur n'est épinglée — c'est la machine de celui qui
        // lance les tests — mais une réponse est soit une réponse, soit
        // un refus qui dit quoi.
        for answer in [theme().map(|_| ()), power().map(|_| ()), locale().map(|_| ())] {
            if let Err(refused) = answer {
                assert!(!refused.to_string().is_empty());
            }
        }
    }
}
