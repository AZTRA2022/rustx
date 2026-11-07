//! Commander une fenêtre, sur une vraie fenêtre.
//!
//! Il n'y a pas de demi-preuve ici : ce test crée une fenêtre du
//! système, lui enlève son cadre, la réduit, l'agrandit, et relit à
//! chaque fois ce que le système en dit. Un test qui vérifierait que
//! l'ordre a été envoyé ne prouverait rien — le piège de cette API est
//! précisément qu'un style changé sans `SWP_FRAMECHANGED` ne change
//! rien à l'écran.
//!
//! Aucun CEF : une fenêtre est une fenêtre, et celle-ci est faite en
//! trois appels.

#![cfg(windows)]

use rustx_os::window::{self, Dressing, Pane, Spread};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    GWL_STYLE, GetWindowLongPtrW, IsIconic, IsZoomed, RegisterClassW, SW_SHOW, ShowWindow,
    WINDOW_EX_STYLE, WNDCLASSW, WS_CAPTION, WS_OVERLAPPEDWINDOW,
};
use windows::core::PCWSTR;

/// Une fenêtre du système, à nous, et son effacement à la sortie.
struct Own(HWND);

impl Own {
    fn new(class: &str) -> Self {
        let name: Vec<u16> = class.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: le module de ce processus existe toujours.
        let module = unsafe { GetModuleHandleW(None) }.expect("this module");
        let registered = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(procedure),
            hInstance: module.into(),
            lpszClassName: PCWSTR(name.as_ptr()),
            ..Default::default()
        };
        // SAFETY: la classe est remplie, et son nom vit jusqu'à la fin
        // de cette fonction, ce qui suffit : Windows le copie.
        let atom = unsafe { RegisterClassW(&registered) };
        assert_ne!(atom, 0, "the window class was refused");
        // SAFETY: une classe enregistrée à l'instant, et des valeurs
        // par défaut pour le reste.
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(name.as_ptr()),
                PCWSTR(name.as_ptr()),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                320,
                240,
                None,
                None,
                Some(module.into()),
                None,
            )
        }
        .expect("a window");
        // Montrée : `IsIconic` et `IsZoomed` parlent d'une fenêtre que
        // le système a acceptée, et une fenêtre jamais montrée n'est ni
        // l'un ni l'autre quoi qu'on lui demande.
        // SAFETY: une poignée que `CreateWindowExW` vient de rendre.
        let _ = unsafe { ShowWindow(window, SW_SHOW) };
        Self(window)
    }

    fn pane(&self) -> Pane {
        Pane::from_handle(self.0.0.cast())
    }

    fn style(&self) -> isize {
        // SAFETY: lecture seule sur une poignée vivante.
        unsafe { GetWindowLongPtrW(self.0, GWL_STYLE) }
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        // SAFETY: notre propre fenêtre, détruite une seule fois.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

extern "system" fn procedure(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> windows::Win32::Foundation::LRESULT {
    // SAFETY: le traitement par défaut, qui est tout ce que cette
    // fenêtre a à faire.
    unsafe { DefWindowProcW(window, message, w, l) }
}

#[test]
fn this_platform_says_it_commands_its_windows() {
    assert!(
        window::supported(),
        "Windows commande ses fenêtres, et ce test en est la preuve"
    );
}

#[test]
fn taking_the_frame_off_takes_the_title_bar_with_it() {
    let own = Own::new("rustx-window-dress");
    assert_ne!(
        own.style() & WS_CAPTION.0 as isize,
        0,
        "une fenêtre ordinaire a une barre de titre"
    );

    window::dress(own.pane(), Dressing::None).expect("the frame comes off");
    assert_eq!(
        own.style() & WS_CAPTION.0 as isize,
        0,
        "la barre de titre devait partir"
    );

    window::dress(own.pane(), Dressing::System).expect("and goes back on");
    assert_ne!(
        own.style() & WS_CAPTION.0 as isize,
        0,
        "et revenir quand on la redemande"
    );
}

#[test]
fn a_window_is_made_small_then_big_then_itself_again() {
    let own = Own::new("rustx-window-spread");

    window::spread(own.pane(), Spread::Minimized).expect("small");
    // SAFETY: lecture seule.
    assert!(unsafe { IsIconic(own.0) }.as_bool(), "réduite");
    assert_eq!(window::spread_of(own.pane()), Ok(Spread::Minimized));

    window::spread(own.pane(), Spread::Maximized).expect("big");
    // SAFETY: lecture seule.
    assert!(unsafe { IsZoomed(own.0) }.as_bool(), "agrandie");
    assert_eq!(window::spread_of(own.pane()), Ok(Spread::Maximized));

    window::spread(own.pane(), Spread::Normal).expect("itself again");
    assert_eq!(window::spread_of(own.pane()), Ok(Spread::Normal));
}

#[test]
fn a_window_kept_in_front_is_told_so_and_told_back() {
    let own = Own::new("rustx-window-rank");
    window::on_top(own.pane(), true).expect("in front");
    use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, WS_EX_TOPMOST};
    // SAFETY: lecture seule.
    let extended = unsafe { GetWindowLongPtrW(own.0, GWL_EXSTYLE) };
    assert_ne!(
        extended & WS_EX_TOPMOST.0 as isize,
        0,
        "devant les autres, et le système le dit"
    );

    window::on_top(own.pane(), false).expect("and back in the pile");
    // SAFETY: lecture seule.
    let extended = unsafe { GetWindowLongPtrW(own.0, GWL_EXSTYLE) };
    assert_eq!(extended & WS_EX_TOPMOST.0 as isize, 0, "et rendue au rang");
}

#[test]
fn a_handle_that_is_not_a_window_is_refused_in_words() {
    // Ce qu'un moteur a avant que la fenêtre existe, et ce qu'il a
    // après qu'elle est partie. Sous Windows, un ordre donné à zéro est
    // un ordre donné au bureau : refuser est la seule réponse.
    let refused = window::spread(Pane(0), Spread::Maximized).expect_err("nothing to command");
    let said = refused.to_string();
    assert!(
        said.contains("window"),
        "le refus doit dire qu'il n'y a pas de fenêtre : {said}"
    );
}
