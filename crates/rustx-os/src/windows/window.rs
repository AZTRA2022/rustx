//! La fenêtre, sous Windows.
//!
//! Tout se joue sur deux appels : `SetWindowLongPtrW` change le style
//! de la fenêtre, et `SetWindowPos` avec `SWP_FRAMECHANGED` demande au
//! système de redessiner le cadre. Sans le second, le style a changé
//! et la fenêtre est restée comme elle était — c'est le piège de cette
//! API, et il ne se voit qu'à l'écran.
//!
//! # Ce qui reste du cadre quand on l'enlève
//!
//! `WS_THICKFRAME` est gardé. Il porte deux choses : les bords qu'on
//! attrape pour redimensionner, et l'ombre portée. Une fenêtre sans lui
//! est une fenêtre qu'on ne peut plus retailler et qui a l'air collée
//! au fond de l'écran. Ce qui part, c'est `WS_CAPTION` — la barre de
//! titre et les trois boutons.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetWindowLongPtrW, HTCAPTION, HWND_NOTOPMOST, HWND_TOPMOST,
    IsIconic, IsWindow, IsZoomed, SM_CXSCREEN, SM_CYSCREEN, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
    SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SendMessageW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_NCLBUTTONDOWN, WS_CAPTION, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
};

use crate::error::OsError;
use crate::window::{Dressing, Pane, Spread};

pub const AVAILABLE: bool = true;

/// La poignée, si c'en est une.
///
/// Zéro est la poignée qu'un moteur a avant que la fenêtre existe, et
/// sous Windows un ordre donné à zéro est un ordre donné au bureau :
/// `IsWindow` est la seule chose qui sépare les deux.
fn handle(pane: Pane) -> Result<HWND, OsError> {
    if !pane.exists() {
        return Err(OsError::Platform(
            "there is no window to command yet: the handle is still nothing".to_string(),
        ));
    }
    let window = HWND(pane.0 as *mut std::ffi::c_void);
    // SAFETY: lecture seule, et c'est précisément la question posée :
    // cette poignée désigne-t-elle encore une fenêtre.
    if unsafe { IsWindow(Some(window)) }.as_bool() {
        Ok(window)
    } else {
        Err(OsError::Platform(format!(
            "the handle {:#x} is not a window any more — it was closed",
            pane.0
        )))
    }
}

/// Ce que le système dit quand un appel échoue.
fn complaint(about: &str) -> OsError {
    OsError::Platform(format!(
        "{about} was refused by Windows: {}",
        windows::core::Error::from_thread()
    ))
}

pub fn dress(pane: Pane, dressing: Dressing) -> Result<(), OsError> {
    let window = handle(pane)?;
    // SAFETY: une poignée de fenêtre vivante, et un style qui est un
    // entier.
    let style = unsafe { GetWindowLongPtrW(window, GWL_STYLE) };
    let decorations =
        (WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0) as isize;
    let wanted = match dressing {
        // `WS_THICKFRAME` reste : il porte les bords qu'on attrape et
        // l'ombre portée.
        Dressing::None => (style & !decorations) | WS_THICKFRAME.0 as isize,
        Dressing::System => style | decorations | WS_THICKFRAME.0 as isize,
    };
    if wanted == style {
        return Ok(());
    }
    // SAFETY: idem.
    unsafe { SetWindowLongPtrW(window, GWL_STYLE, wanted) };
    // Le cadre n'est redessiné que si on le demande. Sans ceci, le
    // style a changé et l'écran n'a pas bougé.
    // SAFETY: idem ; aucune taille ni position n'est touchée.
    unsafe {
        SetWindowPos(
            window,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER,
        )
    }
    .map_err(|_| complaint("redrawing the window frame"))
}

pub fn spread(pane: Pane, how: Spread) -> Result<(), OsError> {
    let window = handle(pane)?;
    let command = match how {
        Spread::Normal => SW_RESTORE,
        Spread::Minimized => SW_MINIMIZE,
        Spread::Maximized => SW_MAXIMIZE,
        Spread::Fullscreen => return fullscreen(window),
    };
    // SAFETY: une poignée vivante. `ShowWindow` rend l'état précédent
    // et ne peut pas échouer autrement.
    let _ = unsafe { ShowWindow(window, command) };
    Ok(())
}

/// Tout l'écran, cadre compris.
///
/// L'écran sur lequel la fenêtre se trouve, pas le premier : une
/// application sur le deuxième moniteur qui passerait en plein écran
/// sur le premier aurait disparu, de son point de vue.
fn fullscreen(window: HWND) -> Result<(), OsError> {
    // SAFETY: une poignée vivante ; le moniteur le plus proche existe
    // toujours, c'est ce que `DEFAULTTONEAREST` promet.
    let monitor: HMONITOR = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    let mut information = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `cbSize` est rempli, ce que cette API exige.
    let known = unsafe { GetMonitorInfoW(monitor, &mut information) }.as_bool();
    let screen: RECT = if known {
        information.rcMonitor
    } else {
        // Sans le moniteur, l'écran principal : une fenêtre plein écran
        // au mauvais endroit reste mieux qu'un refus.
        use windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics;
        // SAFETY: deux lectures de métriques système.
        let (width, height) =
            unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        }
    };
    dress(Pane(window.0 as isize), Dressing::None)?;
    // SAFETY: une poignée vivante et un rectangle lu du système.
    unsafe {
        SetWindowPos(
            window,
            None,
            screen.left,
            screen.top,
            screen.right - screen.left,
            screen.bottom - screen.top,
            SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER,
        )
    }
    .map_err(|_| complaint("filling the screen"))
}

pub fn spread_of(pane: Pane) -> Result<Spread, OsError> {
    let window = handle(pane)?;
    // SAFETY: deux lectures sur une poignée vivante.
    let (small, big) = unsafe { (IsIconic(window).as_bool(), IsZoomed(window).as_bool()) };
    Ok(match (small, big) {
        (true, _) => Spread::Minimized,
        (_, true) => Spread::Maximized,
        _ => Spread::Normal,
    })
}

pub fn on_top(pane: Pane, above: bool) -> Result<(), OsError> {
    let window = handle(pane)?;
    let rank = if above { HWND_TOPMOST } else { HWND_NOTOPMOST };
    // SAFETY: une poignée vivante ; ni taille ni position touchées.
    unsafe { SetWindowPos(window, Some(rank), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) }
        .map_err(|_| complaint("changing the window's rank"))
}

pub fn drag(pane: Pane) -> Result<(), OsError> {
    let window = handle(pane)?;
    // La souris est tenue par Chromium, qui a reçu l'appui. Sans la
    // rendre, le système ne verra jamais le déplacement.
    // SAFETY: rend la capture de ce thread, qui est celui de la
    // fenêtre.
    let _ = unsafe { ReleaseCapture() };
    // Et on dit au système que l'appui était sur une barre de titre,
    // même s'il n'y en a pas : à partir de là il déplace la fenêtre
    // lui-même, jusqu'au relâchement.
    //
    // Ce que cela coûte, et qui est voulu : `SendMessageW` ne rend pas
    // la main avant la fin du déplacement, parce que le système tient
    // sa propre boucle pendant ce temps. Ce thread est celui de la
    // fenêtre, donc la fenêtre ne redessine pas tant qu'on la traîne —
    // exactement comme toute fenêtre que l'on traîne par sa barre.
    // SAFETY: un message standard à une fenêtre vivante.
    unsafe {
        SendMessageW(
            window,
            WM_NCLBUTTONDOWN,
            Some(WPARAM(HTCAPTION as usize)),
            Some(LPARAM(0)),
        )
    };
    Ok(())
}

pub fn close(pane: Pane) -> Result<(), OsError> {
    let window = handle(pane)?;
    use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
    // `WM_CLOSE` et non `DestroyWindow` : la fenêtre est celle de
    // Chromium, et c'est Chromium qui doit la défaire — il écoute ce
    // message et rend la main à sa boucle.
    // SAFETY: un message standard à une fenêtre vivante.
    unsafe { SendMessageW(window, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0))) };
    Ok(())
}

/// Là où la souris est, en coordonnées d'écran. Utile à une barre de
/// titre à soi qui veut savoir sur quel moniteur elle se trouve.
pub fn pointer() -> Result<(i32, i32), OsError> {
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut point = POINT::default();
    // SAFETY: écrit dans un `POINT` de cette pile.
    unsafe { GetCursorPos(&mut point) }.map_err(|_| complaint("reading the pointer"))?;
    Ok((point.x, point.y))
}
