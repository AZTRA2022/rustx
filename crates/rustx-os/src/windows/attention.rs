//! L'attention, sous Windows : `FlashWindowEx` et `ITaskbarList3`.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Shell::{ITaskbarList3, TBPF_NOPROGRESS, TBPF_NORMAL, TaskbarList};
use windows::Win32::UI::WindowsAndMessaging::{
    FLASHW_ALL, FLASHW_TIMERNOFG, FLASHWINFO, FlashWindowEx, IsWindow,
};

use crate::attention::per_ten_thousand;
use crate::error::OsError;
use crate::window::Pane;

fn handle(pane: Pane) -> Result<HWND, OsError> {
    let window = HWND(pane.0 as *mut std::ffi::c_void);
    // SAFETY: lecture seule ; c'est la question posée.
    if pane.exists() && unsafe { IsWindow(Some(window)) }.as_bool() {
        Ok(window)
    } else {
        Err(OsError::Platform(format!(
            "the handle {:#x} is not a window",
            pane.0
        )))
    }
}

/// Jusqu'à ce que la fenêtre revienne devant (`FLASHW_TIMERNOFG`) : un
/// nombre de clignements fixé s'arrêterait avant que l'utilisateur ait
/// levé les yeux.
pub fn flash(pane: Pane) -> Result<(), OsError> {
    let window = handle(pane)?;
    let info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd: window,
        dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
        uCount: 0,
        dwTimeout: 0,
    };
    // SAFETY: une structure remplie ; la valeur rendue est l'état
    // d'avant, pas un échec.
    let _ = unsafe { FlashWindowEx(&info) };
    Ok(())
}

pub fn progress(pane: Pane, done: Option<f64>) -> Result<(), OsError> {
    let window = handle(pane)?;
    // SAFETY: COM pour ce fil ; déjà initialisé, l'appel le dit et c'est
    // sans conséquence.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let complaint =
        |error: windows::core::Error| OsError::Platform(format!("the taskbar refused: {error}"));
    // SAFETY: la classe documentée de la barre des tâches, et son
    // initialisation obligatoire avant tout autre appel.
    unsafe {
        let taskbar: ITaskbarList3 =
            CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).map_err(complaint)?;
        taskbar.HrInit().map_err(complaint)?;
        match done {
            None => taskbar
                .SetProgressState(window, TBPF_NOPROGRESS)
                .map_err(complaint),
            Some(done) => {
                taskbar
                    .SetProgressState(window, TBPF_NORMAL)
                    .map_err(complaint)?;
                taskbar
                    .SetProgressValue(window, per_ten_thousand(done), 10_000)
                    .map_err(complaint)
            }
        }
    }
}

pub fn badge(_count: Option<u32>) -> Result<(), OsError> {
    Err(OsError::Unsupported(
        "a badge on Windows (the taskbar shows an overlay icon the application draws itself, \
         not a number)",
    ))
}
