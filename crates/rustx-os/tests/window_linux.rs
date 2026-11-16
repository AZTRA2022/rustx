//! Commander une fenêtre, sous Linux — ce qui se prouve sans écran.
//!
//! Le test Windows (`window.rs`) crée une vraie fenêtre et relit ce que
//! le système en dit. Ici, rien de tel n'a encore tourné : ce crate a
//! été écrit et vérifié à la compilation sur une machine sans serveur
//! X. Ce qui est épinglé est ce qui ne dépend pas d'un écran — que la
//! plateforme dit qu'elle commande ses fenêtres, et qu'une poignée nulle
//! est refusée avant toute connexion. Les calculs purs (indications
//! Motif, messages EWMH, lecture de l'état) ont leurs tests dans
//! `src/linux/window.rs`.
//!
//! Le test qui manque, et qu'il faudra écrire sur une machine Linux avec
//! un serveur (Xvfb et un gestionnaire de fenêtres suffisent) : créer
//! une fenêtre X11 avec `x11rb`, la déshabiller, la réduire, l'agrandir,
//! et relire `_MOTIF_WM_HINTS` et `_NET_WM_STATE` à chaque fois, comme
//! le test Windows relit `GWL_STYLE`.

#![cfg(target_os = "linux")]

use rustx_os::OsError;
use rustx_os::window::{self, Pane, Spread};

#[test]
fn linux_says_it_commands_its_windows() {
    assert!(window::supported());
}

#[test]
fn a_window_that_does_not_exist_is_refused_before_any_connection() {
    // Lue avant d'ouvrir la connexion : une poignée nulle est refusée
    // pour ce qu'elle est, même sans `$DISPLAY`.
    let refused = window::spread(Pane(0), Spread::Minimized).expect_err("no window");
    assert!(
        matches!(&refused, OsError::Platform(said) if said.contains("still nothing")),
        "got {refused}"
    );
}
