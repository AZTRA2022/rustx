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
use windows::Win32::Foundation::LRESULT;
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, GWL_STYLE, GWLP_WNDPROC, GetWindowLongPtrW, GetWindowRect, HTCAPTION,
    HWND_NOTOPMOST, HWND_TOPMOST, IsIconic, IsWindow, IsZoomed, NCCALCSIZE_PARAMS, SM_CXSCREEN,
    SM_CYSCREEN, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SWP_FRAMECHANGED, SWP_NOMOVE,
    SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, WM_NCCALCSIZE, WM_NCLBUTTONDOWN, WNDPROC, WS_CAPTION, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
};

use crate::error::OsError;
use crate::window::{Dressing, Pane, Spread};

pub const AVAILABLE: bool = true;

// --------------- Ce qu'il faut retenir d'une fenetre ---------------
//
// Deux choses, et les deux parce que Windows ne les retient pas :
//
// 1. **la place qu'elle avait avant le plein ecran.** `ShowWindow` sait
//    revenir d'une fenetre reduite ou agrandie, parce que c'est le
//    systeme qui l'a mise la. Le plein ecran, lui, est une taille qu'on
//    a posee soi-meme : personne d'autre ne sait de quoi en revenir, et
//    sans cela un bouton plein ecran n'agit qu'une fois ;
// 2. **la procedure de fenetre d'origine**, quand on l'a doublee pour
//    rendre au contenu la bande que le systeme garde en haut.

/// La place et le style d'une fenetre avant qu'elle prenne l'ecran.
#[derive(Debug, Clone, Copy)]
struct Placed {
    style: isize,
    rect: RECT,
}

fn filling() -> &'static std::sync::Mutex<Vec<(isize, Placed)>> {
    static FILLING: std::sync::OnceLock<std::sync::Mutex<Vec<(isize, Placed)>>> =
        std::sync::OnceLock::new();
    FILLING.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn held<T>(lock: &'static std::sync::Mutex<Vec<T>>) -> std::sync::MutexGuard<'static, Vec<T>> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// La place retenue de cette fenetre, si elle remplit l'ecran.
fn place_of(window: HWND) -> Option<Placed> {
    let key = window.0 as isize;
    held(filling())
        .iter()
        .find(|(each, _)| *each == key)
        .map(|(_, placed)| *placed)
}

/// Retenir ou elle etait, une seule fois : deux `fullscreen` de suite
/// ne doivent pas retenir le plein ecran comme etant « sa place ».
fn remember(window: HWND, placed: Placed) {
    let key = window.0 as isize;
    let mut all = held(filling());
    if all.iter().any(|(each, _)| *each == key) {
        return;
    }
    all.push((key, placed));
}

fn forget(window: HWND) {
    let key = window.0 as isize;
    held(filling()).retain(|(each, _)| *each != key);
}

// --------- La bande que le systeme garde en haut ---------
//
// Enlever `WS_CAPTION` enleve la barre de titre et ses boutons, et ne
// dit rien de la zone non cliente : Windows en garde une bande en haut,
// de l'epaisseur d'une bordure, qu'il peint lui-meme. Dans une
// application sans cadre, cela se voit comme un trait clair au-dessus
// de la barre que la page a dessinee -- et c'est ce que le rapport du
// labo appelait « du blanc au-dessus de la fenetre ».
//
// La seule facon de la reprendre est de repondre a `WM_NCCALCSIZE` :
// laisser le traitement d'origine calculer les bords -- ce sont eux
// qu'on attrape pour retailler, et ce sont eux qui gerent la fenetre
// agrandie -- puis rendre le haut au contenu.

fn procedures() -> &'static std::sync::Mutex<Vec<(isize, isize)>> {
    static DOUBLED: std::sync::OnceLock<std::sync::Mutex<Vec<(isize, isize)>>> =
        std::sync::OnceLock::new();
    DOUBLED.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn procedure_of(window: HWND) -> Option<isize> {
    let key = window.0 as isize;
    held(procedures())
        .iter()
        .find(|(each, _)| *each == key)
        .map(|(_, original)| *original)
}

/// La procedure d'origine, telle que `CallWindowProcW` la prend.
fn original_as_procedure(original: isize) -> WNDPROC {
    // SAFETY: `original` vient de `SetWindowLongPtrW(GWLP_WNDPROC, ...)`,
    // qui rend exactement un pointeur de procedure de fenetre.
    unsafe { std::mem::transmute::<isize, WNDPROC>(original) }
}

/// La procedure que porte une fenetre sans cadre.
unsafe extern "system" fn as_if_there_were_no_frame(
    window: HWND,
    message: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    let Some(original) = procedure_of(window) else {
        // Plus doublee : on ne peut qu'appeler le traitement par
        // defaut, ce qui vaut mieux que de ne pas repondre.
        use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
        // SAFETY: le traitement par defaut d'un message standard.
        return unsafe { DefWindowProcW(window, message, w, l) };
    };
    let keep = original_as_procedure(original);

    if message == WM_NCCALCSIZE && w.0 != 0 {
        let parameters = l.0 as *mut NCCALCSIZE_PARAMS;
        if !parameters.is_null() {
            // SAFETY: Windows passe ici un `NCCALCSIZE_PARAMS` vivant,
            // c'est le contrat de ce message.
            let asked = unsafe { (*parameters).rgrc[0].top };
            // SAFETY: la procedure d'origine de cette fenetre, avec les
            // arguments qu'elle attend.
            let answered = unsafe { CallWindowProcW(keep, window, message, w, l) };
            if answered.0 != 0 {
                return answered;
            }
            // Les bords de gauche, de droite et du bas restent ceux que
            // le systeme a calcules : ils ne se voient pas, et ils
            // portent la prise pour retailler. Le haut revient au
            // contenu, et la bande disparait.
            // SAFETY: idem.
            unsafe { (*parameters).rgrc[0].top = asked };
            return LRESULT(0);
        }
    }

    // SAFETY: idem.
    unsafe { CallWindowProcW(keep, window, message, w, l) }
}

/// Doubler la procedure de cette fenetre, si ce n'est pas deja fait.
fn double(window: HWND) {
    if procedure_of(window).is_some() {
        return;
    }
    // SAFETY: une poignee vivante, et une procedure de fenetre de ce
    // module qui appelle celle qu'elle remplace.
    let original = unsafe {
        SetWindowLongPtrW(
            window,
            GWLP_WNDPROC,
            as_if_there_were_no_frame as *const () as isize,
        )
    };
    if original == 0 {
        return;
    }
    held(procedures()).push((window.0 as isize, original));
}

/// Lui rendre la sienne.
fn undouble(window: HWND) {
    let Some(original) = procedure_of(window) else {
        return;
    };
    // SAFETY: la procedure que cette meme fenetre portait.
    unsafe { SetWindowLongPtrW(window, GWLP_WNDPROC, original) };
    let key = window.0 as isize;
    held(procedures()).retain(|(each, _)| *each != key);
}

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
    // `WS_MINIMIZEBOX` et `WS_MAXIMIZEBOX` restent, meme sans barre ou
    // les dessiner : ce sont eux qui autorisent la reduction, l'Aero
    // Snap et le double-clic sur le bord. Une fenetre sans cadre a qui
    // on les retire est une fenetre que le systeme refuse d'agrandir.
    let decorations = (WS_CAPTION.0 | WS_SYSMENU.0) as isize;
    let boxes = (WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0) as isize;
    let wanted = match dressing {
        // `WS_THICKFRAME` reste : il porte les bords qu'on attrape et
        // l'ombre portée.
        Dressing::None => (style & !decorations) | boxes | WS_THICKFRAME.0 as isize,
        Dressing::System => style | decorations | boxes | WS_THICKFRAME.0 as isize,
    };
    // Doublee avant le changement de style : le premier `WM_NCCALCSIZE`
    // arrive dans le `SetWindowPos` d'en dessous, et c'est deja celui
    // qui decide si la bande est la.
    match dressing {
        Dressing::None => double(window),
        Dressing::System => undouble(window),
    }
    if wanted == style {
        // Le style n'a pas bouge, mais la zone non cliente, elle, est
        // calculee autrement : il faut quand meme le redemander.
        // SAFETY: une poignee vivante ; ni taille ni position touchees.
        return unsafe {
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
        .map_err(|_| complaint("redrawing the window frame"));
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
    if how == Spread::Fullscreen {
        return fullscreen(window);
    }
    // Tout ordre de taille sort d'abord du plein ecran : une fenetre
    // qu'on reduit depuis le plein ecran et qu'on rouvre doit revenir a
    // sa place, pas a l'ecran entier.
    if let Some(placed) = place_of(window) {
        put_back(window, placed)?;
        if how == Spread::Normal {
            return Ok(());
        }
    }
    let command = match how {
        Spread::Normal => SW_RESTORE,
        Spread::Minimized => SW_MINIMIZE,
        Spread::Maximized => SW_MAXIMIZE,
        // Deja traite plus haut ; la branche existe parce que le
        // compilateur demande toutes les variantes.
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
    // Ce qu'elle etait avant, retenu avant d'y toucher : c'est la
    // seule memoire de cette place, et sans elle on ne revient pas.
    let mut was = RECT::default();
    // SAFETY: lecture seule sur une poignee vivante.
    unsafe { GetWindowRect(window, &mut was) }
        .map_err(|_| complaint("reading the window's place"))?;
    // SAFETY: lecture d'un style, qui est un entier.
    let style = unsafe { GetWindowLongPtrW(window, GWL_STYLE) };
    remember(window, Placed { style, rect: was });

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

/// Rendre a une fenetre la place et le style qu'elle avait avant
/// l'ecran entier.
fn put_back(window: HWND, placed: Placed) -> Result<(), OsError> {
    forget(window);
    // Le style d'abord : la zone non cliente est calculee au
    // `SetWindowPos` qui suit, et une fenetre qui retrouve sa barre de
    // titre apres avoir retrouve sa taille serait plus petite d'une
    // barre.
    // SAFETY: une poignee vivante, et le style que cette meme fenetre
    // portait.
    unsafe { SetWindowLongPtrW(window, GWL_STYLE, placed.style) };
    if placed.style & WS_CAPTION.0 as isize != 0 {
        // Elle avait son cadre : la procedure doublee n'a plus de
        // raison d'etre.
        undouble(window);
    }
    // SAFETY: une poignee vivante et le rectangle qu'on avait lu d'elle.
    unsafe {
        SetWindowPos(
            window,
            None,
            placed.rect.left,
            placed.rect.top,
            placed.rect.right - placed.rect.left,
            placed.rect.bottom - placed.rect.top,
            SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER,
        )
    }
    .map_err(|_| complaint("giving the window its place back"))
}

pub fn spread_of(pane: Pane) -> Result<Spread, OsError> {
    let window = handle(pane)?;
    // Le plein ecran n'est pas un etat que Windows tient : c'est une
    // taille qu'on a posee, et la memoire de cette place est la seule
    // chose qui le sache.
    if place_of(window).is_some() {
        return Ok(Spread::Fullscreen);
    }
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
