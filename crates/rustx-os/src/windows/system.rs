//! L'état de la machine, sous Windows : des lectures d'API, aucune
//! commande.

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP, OpenInputDesktop,
};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Time::{DYNAMIC_TIME_ZONE_INFORMATION, GetDynamicTimeZoneInformation};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::core::w;

use crate::error::OsError;
use crate::system::{Power, Theme};

pub fn theme() -> Result<Theme, OsError> {
    let mut light: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: un `DWORD` de cette pile, et sa taille.
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut light).cast()),
            Some(&mut size),
        )
    };
    // Absente, la valeur veut dire « clair » : c'est ce que Windows
    // affiche tant que personne n'a touché au réglage (Windows 10 avant
    // 1809 n'en avait pas d'autre).
    if read != ERROR_SUCCESS {
        return Ok(Theme::Light);
    }
    Ok(if light == 0 { Theme::Dark } else { Theme::Light })
}

pub fn power() -> Result<Power, OsError> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: une structure de cette pile.
    unsafe { GetSystemPowerStatus(&mut status) }.map_err(|error| {
        OsError::Platform(format!("l'état de l'alimentation est illisible : {error}"))
    })?;
    Ok(power_from(
        status.ACLineStatus,
        status.BatteryFlag,
        status.BatteryLifePercent,
        status.SystemStatusFlag,
    ))
}

/// Ce que `SYSTEM_POWER_STATUS` dit, en clair. Séparé pour les tests.
///
/// `255` veut dire « inconnu » pour le secteur et le niveau, et le bit
/// `128` de `BatteryFlag` « pas de batterie du tout ».
pub(crate) fn power_from(line: u8, flag: u8, percent: u8, saver: u8) -> Power {
    let battery = flag != 255 && flag & 128 == 0;
    Power {
        plugged: match line {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        level: (battery && percent <= 100).then_some(percent),
        charging: battery.then_some(flag & 8 != 0),
        saver: Some(saver & 1 != 0),
    }
}

pub fn idle() -> Result<std::time::Duration, OsError> {
    let mut last = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `cbSize` rempli, ce que cette API exige.
    if !unsafe { GetLastInputInfo(&mut last) }.as_bool() {
        return Err(OsError::Platform(
            "Windows n'a pas dit quand a eu lieu la dernière saisie".to_string(),
        ));
    }
    // SAFETY: sans argument.
    let now = unsafe { GetTickCount() };
    // `wrapping_sub` : les deux compteurs tournent ensemble tous les
    // 49,7 jours, et l'écart reste juste à travers le tour.
    Ok(std::time::Duration::from_millis(u64::from(
        now.wrapping_sub(last.dwTime),
    )))
}

/// Verrouillée, d'après le bureau qui reçoit les saisies.
///
/// Windows n'a pas d'appel « la session est-elle verrouillée ? ». Ce
/// que tout le monde fait : demander le bureau qui reçoit le clavier.
/// Session verrouillée, c'est le bureau sécurisé (`Winlogon`), qu'un
/// processus ordinaire n'a pas le droit d'ouvrir — le refus *est* la
/// réponse. Une heuristique, dite comme telle : une invite UAC donne la
/// même réponse le temps qu'elle est affichée.
pub fn locked() -> Result<bool, OsError> {
    // SAFETY: aucun pointeur ; le bureau ouvert est refermé aussitôt.
    match unsafe {
        OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_SWITCHDESKTOP)
    } {
        Ok(desktop) => {
            // SAFETY: le bureau qu'on vient d'ouvrir.
            let _ = unsafe { CloseDesktop(desktop) };
            Ok(false)
        }
        Err(_) => Ok(true),
    }
}

pub fn locale() -> Result<String, OsError> {
    let mut name = [0u16; 85];
    // SAFETY: un tampon de `LOCALE_NAME_MAX_LENGTH` caractères.
    let written = unsafe { GetUserDefaultLocaleName(&mut name) };
    if written <= 1 {
        return Err(OsError::Platform(
            "Windows n'a pas rendu la langue de l'utilisateur".to_string(),
        ));
    }
    Ok(String::from_utf16_lossy(&name[..written as usize - 1]))
}

pub fn timezone() -> Result<String, OsError> {
    let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
    // SAFETY: une structure de cette pile.
    let answer = unsafe { GetDynamicTimeZoneInformation(&mut zone) };
    if answer == u32::MAX {
        return Err(OsError::Platform(
            "Windows n'a pas rendu le fuseau horaire".to_string(),
        ));
    }
    let key = &zone.TimeZoneKeyName;
    let end = key.iter().position(|&c| c == 0).unwrap_or(key.len());
    Ok(String::from_utf16_lossy(&key[..end]))
}

/// L'échelle du système, en dpi sur 96.
///
/// Ce que Windows rend dépend de la déclaration du processus : un
/// processus qui ne s'est pas dit « conscient des dpi » se voit répondre
/// 96 quoi qu'il en soit. CEF, lui, se déclare ; un petit programme de
/// test sans manifeste lira `1.0`.
pub fn scale() -> Result<f64, OsError> {
    // SAFETY: sans argument.
    let dpi = unsafe { GetDpiForSystem() };
    if dpi == 0 {
        return Err(OsError::Platform("Windows a rendu 0 dpi".to_string()));
    }
    Ok(f64::from(dpi) / 96.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_desktop_without_a_battery_has_no_level() {
        let fixed = power_from(1, 128, 255, 0);
        assert_eq!(fixed.plugged, Some(true));
        assert_eq!(fixed.level, None);
        assert_eq!(fixed.charging, None);
    }

    #[test]
    fn a_laptop_says_its_level_and_whether_it_charges() {
        let laptop = power_from(0, 1, 87, 1);
        assert_eq!(laptop.plugged, Some(false));
        assert_eq!(laptop.level, Some(87));
        assert_eq!(laptop.charging, Some(false));
        assert_eq!(laptop.saver, Some(true));
        assert_eq!(power_from(1, 8, 40, 0).charging, Some(true));
    }

    #[test]
    fn unknown_is_unknown() {
        let unknown = power_from(255, 255, 255, 0);
        assert_eq!(unknown.plugged, None);
        assert_eq!(unknown.level, None);
    }
}
