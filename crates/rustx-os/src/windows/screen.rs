//! Les écrans, sous Windows : `EnumDisplayMonitors`, et le dpi de
//! chacun.
//!
//! `GetDpiForMonitor` plutôt que `GetDpiForSystem` : deux moniteurs
//! peuvent être à deux échelles — c'est le cas courant d'un portable
//! Retina branché sur un écran externe — et l'échelle du système n'est
//! alors celle de personne.

use windows::Win32::Foundation::{LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;
use windows::core::BOOL;

use crate::error::OsError;
use crate::screen::{Area, Screen};

fn area(rect: RECT) -> Area {
    Area::between(rect.left, rect.top, rect.right, rect.bottom)
}

/// Appelé par Windows une fois par moniteur.
unsafe extern "system" fn each(
    monitor: HMONITOR,
    _context: HDC,
    _clip: *mut RECT,
    found: LPARAM,
) -> BOOL {
    let mut information = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: `cbSize` dit la taille de la structure étendue, ce que
    // cette API exige pour remplir aussi `szDevice`.
    let known =
        unsafe { GetMonitorInfoW(monitor, (&raw mut information).cast::<MONITORINFO>()).as_bool() };
    if known {
        let name = {
            let device = &information.szDevice;
            let end = device.iter().position(|&c| c == 0).unwrap_or(device.len());
            String::from_utf16_lossy(&device[..end])
        };
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        // SAFETY: deux `u32` de cette pile ; un échec les laisse à 96,
        // qui est l'échelle 1 et la seule valeur honnête faute de mieux.
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        let screen = Screen {
            name,
            bounds: area(information.monitorInfo.rcMonitor),
            work: area(information.monitorInfo.rcWork),
            scale: f64::from(dpi_x.max(1)) / 96.0,
            primary: information.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        };
        // SAFETY: `found` est le `&mut Vec<Screen>` que `screens` a passé,
        // et cette fonction n'est appelée que depuis elle, sur son fil.
        unsafe { (*(found.0 as *mut Vec<Screen>)).push(screen) };
    }
    // `TRUE` : continuer avec le moniteur suivant.
    TRUE
}

pub fn screens() -> Result<Vec<Screen>, OsError> {
    let mut found: Vec<Screen> = Vec::new();
    // SAFETY: un pointeur vers `found`, qui vit jusqu'au retour de
    // l'appel ; `EnumDisplayMonitors` est synchrone.
    let listed =
        unsafe { EnumDisplayMonitors(None, None, Some(each), LPARAM((&raw mut found) as isize)) };
    if !listed.as_bool() && found.is_empty() {
        return Err(OsError::Platform(format!(
            "the monitors could not be listed: {}",
            windows::core::Error::from_thread()
        )));
    }
    Ok(found)
}
