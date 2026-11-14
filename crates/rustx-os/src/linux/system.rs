//! L'état de la machine, sous Linux.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! Écrit sur une machine Windows : `cargo check --target
//! x86_64-unknown-linux-gnu` l'a vérifié, et les lectures pures (le
//! texte de `gsettings`, les fichiers de `/sys/class/power_supply`, les
//! variables de langue, le lien `/etc/localtime`) ont leurs tests, qui
//! tournent partout. Rien n'a été lu sur un vrai bureau Linux.
//!
//! # D'où chaque réponse vient
//!
//! Ce que toute distribution de bureau a déjà : `gsettings` pour le
//! thème et l'échelle (GNOME, et ce que la plupart des autres bureaux
//! recopient), `/sys/class/power_supply` pour la batterie, l'extension
//! `MIT-SCREEN-SAVER` du serveur X pour l'inactivité, `loginctl` pour le
//! verrouillage, les variables `LC_*`/`LANG` pour la langue, et le lien
//! `/etc/localtime` pour le fuseau. KDE sans `gsettings` n'a pas de
//! thème lisible ici, et c'est refusé en le disant.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::system::{Power, Theme, bcp47, zone_from_link};

/// Ce qu'un programme a écrit, s'il a réussi.
fn said(program: &str, arguments: &[&str]) -> Option<String> {
    let out = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Ce que `gsettings` dit du thème : `color-scheme` d'abord (GNOME 42
/// et après), le nom du thème GTK ensuite (avant, un thème sombre se
/// disait par son nom, `Adwaita-dark`).
pub(crate) fn theme_from(scheme: Option<&str>, gtk_theme: Option<&str>) -> Option<Theme> {
    let unquote = |text: &str| text.trim().trim_matches('\'').to_ascii_lowercase();
    if let Some(scheme) = scheme.map(unquote) {
        if scheme == "prefer-dark" {
            return Some(Theme::Dark);
        }
        if scheme == "prefer-light" {
            return Some(Theme::Light);
        }
    }
    gtk_theme.map(unquote).map(|name| {
        if name.ends_with("-dark") || name.ends_with("_dark") {
            Theme::Dark
        } else {
            Theme::Light
        }
    })
}

pub fn theme() -> Result<Theme, OsError> {
    let scheme = said(
        "gsettings",
        &["get", "org.gnome.desktop.interface", "color-scheme"],
    );
    let gtk = said(
        "gsettings",
        &["get", "org.gnome.desktop.interface", "gtk-theme"],
    );
    theme_from(scheme.as_deref(), gtk.as_deref()).ok_or(OsError::Unsupported(
        "reading the theme without `gsettings` (a GNOME-family desktop)",
    ))
}

/// Une batterie, depuis ce que son répertoire de `/sys` contient.
pub(crate) fn battery_from(
    capacity: Option<&str>,
    status: Option<&str>,
) -> (Option<u8>, Option<bool>) {
    let level = capacity
        .and_then(|text| text.trim().parse::<u8>().ok())
        .filter(|level| *level <= 100);
    let charging = status.map(|text| text.trim() == "Charging");
    (level, charging)
}

pub fn power() -> Result<Power, OsError> {
    let read = |path: &Path| std::fs::read_to_string(path).ok();
    let mut power = Power::default();
    let Ok(supplies) = std::fs::read_dir("/sys/class/power_supply") else {
        return Err(OsError::Platform(
            "/sys/class/power_supply cannot be read".to_string(),
        ));
    };
    for supply in supplies.flatten() {
        let at = supply.path();
        match read(&at.join("type")).as_deref().map(str::trim) {
            Some("Mains") => {
                let online = read(&at.join("online")).map(|text| text.trim() == "1");
                power.plugged = Some(power.plugged.unwrap_or(false) || online.unwrap_or(false));
            }
            Some("Battery") if power.level.is_none() => {
                let (level, charging) = battery_from(
                    read(&at.join("capacity")).as_deref(),
                    read(&at.join("status")).as_deref(),
                );
                power.level = level;
                power.charging = charging;
            }
            _ => {}
        }
    }
    Ok(power)
}

#[cfg(target_os = "linux")]
pub fn idle() -> Result<std::time::Duration, OsError> {
    use x11rb::connection::Connection;
    use x11rb::protocol::screensaver::ConnectionExt as _;

    let (link, screen) = x11rb::connect(None).map_err(|error| {
        OsError::Platform(format!(
            "the X server could not be reached to read the idle time: {error}"
        ))
    })?;
    let root = link.setup().roots[screen].root;
    let info = link
        .screensaver_query_info(root)
        .map_err(|error| OsError::Platform(format!("MIT-SCREEN-SAVER refused: {error}")))?
        .reply()
        .map_err(|error| OsError::Platform(format!("MIT-SCREEN-SAVER refused: {error}")))?;
    Ok(std::time::Duration::from_millis(u64::from(
        info.ms_since_user_input,
    )))
}

/// `x11rb` n'existe que sous Linux ; ce fichier est aussi compilé
/// ailleurs pour être vérifié (voir `lib.rs`), jamais choisi.
#[cfg(not(target_os = "linux"))]
pub fn idle() -> Result<std::time::Duration, OsError> {
    unreachable!("linux/system.rs is never the platform module outside Linux")
}

pub fn locked() -> Result<bool, OsError> {
    let session = std::env::var("XDG_SESSION_ID").unwrap_or_else(|_| "self".to_string());
    let hint = said(
        "loginctl",
        &["show-session", &session, "-p", "LockedHint", "--value"],
    )
    .ok_or(OsError::Unsupported(
        "reading the lock state without `loginctl` (systemd-logind)",
    ))?;
    Ok(hint == "yes")
}

/// La première variable de langue qui en dise une, dans l'ordre où
/// POSIX les lit.
pub(crate) fn locale_from(
    all: Option<&str>,
    messages: Option<&str>,
    lang: Option<&str>,
) -> Option<String> {
    [all, messages, lang].into_iter().flatten().find_map(bcp47)
}

pub fn locale() -> Result<String, OsError> {
    let var = |name: &str| std::env::var(name).ok();
    locale_from(
        var("LC_ALL").as_deref(),
        var("LC_MESSAGES").as_deref(),
        var("LANG").as_deref(),
    )
    .ok_or_else(|| {
        OsError::Platform("no language is set: LC_ALL, LC_MESSAGES and LANG are empty".to_string())
    })
}

pub fn timezone() -> Result<String, OsError> {
    if let Ok(zone) = std::env::var("TZ") {
        let zone = zone.trim_start_matches(':');
        if !zone.is_empty() {
            return Ok(zone.to_string());
        }
    }
    if let Ok(target) = std::fs::read_link("/etc/localtime")
        && let Some(zone) = zone_from_link(&target.to_string_lossy())
    {
        return Ok(zone);
    }
    if let Ok(text) = std::fs::read_to_string("/etc/timezone") {
        let zone = text.trim();
        if !zone.is_empty() {
            return Ok(zone.to_string());
        }
    }
    Err(OsError::Platform(
        "no time zone: $TZ is unset, /etc/localtime is not a link into zoneinfo, and \
         /etc/timezone is absent"
            .to_string(),
    ))
}

/// L'échelle, depuis `GDK_SCALE` ou le `scaling-factor` de GNOME
/// (`uint32 2`). Zéro, pour GNOME, veut dire « automatique » — ce n'est
/// pas une échelle, et ce n'est pas lu comme une.
pub(crate) fn scale_from(gdk: Option<&str>, gnome: Option<&str>) -> Option<f64> {
    let positive = |text: &str| {
        text.trim()
            .trim_start_matches("uint32")
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|scale| *scale > 0.0)
    };
    gdk.and_then(positive).or_else(|| gnome.and_then(positive))
}

pub fn scale() -> Result<f64, OsError> {
    let gnome = said(
        "gsettings",
        &["get", "org.gnome.desktop.interface", "scaling-factor"],
    );
    scale_from(std::env::var("GDK_SCALE").ok().as_deref(), gnome.as_deref()).ok_or_else(|| {
        OsError::Platform(
            "the scale is not set: no GDK_SCALE, and GNOME's scaling-factor is automatic or \
             absent"
                .to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnome_42_says_dark_by_its_color_scheme() {
        assert_eq!(
            theme_from(Some("'prefer-dark'"), Some("'Adwaita'")),
            Some(Theme::Dark)
        );
        assert_eq!(theme_from(Some("'prefer-light'"), None), Some(Theme::Light));
    }

    #[test]
    fn older_desktops_say_dark_by_the_theme_name() {
        assert_eq!(
            theme_from(Some("'default'"), Some("'Adwaita-dark'")),
            Some(Theme::Dark)
        );
        assert_eq!(theme_from(None, Some("'Yaru'")), Some(Theme::Light));
        assert_eq!(theme_from(None, None), None, "nothing read is not light");
    }

    #[test]
    fn a_battery_is_read_from_its_sys_files() {
        assert_eq!(
            battery_from(Some("87\n"), Some("Discharging\n")),
            (Some(87), Some(false))
        );
        assert_eq!(
            battery_from(Some("40"), Some("Charging")),
            (Some(40), Some(true))
        );
        assert_eq!(battery_from(Some("garbage"), None), (None, None));
    }

    #[test]
    fn the_language_follows_posix_precedence() {
        assert_eq!(
            locale_from(None, Some("de_DE.UTF-8"), Some("fr_FR.UTF-8")).as_deref(),
            Some("de-DE")
        );
        assert_eq!(
            locale_from(Some("C"), None, Some("fr_FR")).as_deref(),
            Some("fr-FR")
        );
        assert_eq!(locale_from(None, None, None), None);
    }

    #[test]
    fn automatic_scaling_is_not_a_scale() {
        assert_eq!(scale_from(Some("2"), None), Some(2.0));
        assert_eq!(scale_from(None, Some("uint32 2")), Some(2.0));
        assert_eq!(scale_from(None, Some("uint32 0")), None);
        assert_eq!(scale_from(None, None), None);
    }
}
