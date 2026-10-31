//! Les raccourcis globaux : ceux qui marchent quand la fenêtre n'a pas
//! le focus.
//!
//! Tout ici se prouve sans personne devant le clavier, sauf l'appui
//! lui-même : la lecture de ce qui est écrit, le refus quand une autre
//! application tient la combinaison, et les touches rendues quand le
//! jeton est lâché. L'appui se regarde avec
//! `cargo run -p rustx-os --example a_global_shortcut`.

use rustx_os::shortcut::{self, Shortcut};

#[test]
fn a_shortcut_is_read_from_how_it_is_written() {
    // Ce qu'un développeur écrit est ce qu'un utilisateur lit sur un
    // autocollant : `"Ctrl+Shift+R"`. Lu ici, et non deviné.
    let (modifiers, key) = shortcut::parse("Ctrl+Shift+R").expect("une combinaison");
    assert_eq!(modifiers, shortcut::CONTROL | shortcut::SHIFT);
    assert_eq!(key, u32::from(b'R'), "la touche virtuelle d'une lettre est sa majuscule");

    let (modifiers, key) = shortcut::parse("Alt+F4").expect("une combinaison");
    assert_eq!(modifiers, shortcut::ALT);
    assert_eq!(key, 0x73, "VK_F4");

    let (modifiers, key) = shortcut::parse("Win+Space").expect("une combinaison");
    assert_eq!(modifiers, shortcut::WIN);
    assert_eq!(key, 0x20, "VK_SPACE");

    // Les espaces et la casse ne changent rien : c'est écrit à la main.
    assert_eq!(
        shortcut::parse("ctrl + shift + r").expect("la même"),
        shortcut::parse("Ctrl+Shift+R").expect("la même")
    );
}

#[test]
fn a_combination_without_a_key_is_refused_in_words() {
    let said = shortcut::parse("Ctrl+")
        .expect_err("un modificateur seul n'est pas un raccourci")
        .to_string();
    assert!(said.contains("Ctrl+"), "le refus nomme ce qui était écrit : {said}");

    let said = shortcut::parse("Ctrl+Frobnicate")
        .expect_err("cette touche n'existe pas")
        .to_string();
    assert!(said.contains("Frobnicate"), "got {said}");
}

#[test]
fn a_shortcut_somebody_else_has_is_refused_by_name() {
    if !shortcut::supported() {
        return;
    }
    // Une combinaison improbable, pour ne pas voler celle de quelqu'un
    // pendant que les tests tournent.
    let keys = "Ctrl+Shift+F13";
    let first = Shortcut::listen(keys).expect("personne ne devrait l'avoir");
    let second = Shortcut::listen(keys);
    let said = second
        .expect_err("deux fois la même combinaison")
        .to_string();
    assert!(said.contains(keys), "le refus nomme la touche : {said}");
    drop(first);
}

#[test]
fn dropping_a_shortcut_gives_the_keys_back() {
    if !shortcut::supported() {
        return;
    }
    let keys = "Ctrl+Shift+F14";
    {
        let _held = Shortcut::listen(keys).expect("libre");
    }
    // Rendu : sans cela, un raccourci survivrait à l'application et
    // l'utilisateur ne pourrait le récupérer qu'en redémarrant.
    let again = Shortcut::listen(keys);
    assert!(again.is_ok(), "les touches n'ont pas été rendues : {again:?}");
}

#[test]
fn a_shortcut_nobody_pressed_counts_zero() {
    if !shortcut::supported() {
        return;
    }
    let held = Shortcut::listen("Ctrl+Shift+F15").expect("libre");
    assert_eq!(held.pressed(), 0);
    assert_eq!(held.pressed(), 0, "lire ne crée pas un appui");
}

#[test]
fn a_platform_without_global_shortcuts_says_so() {
    assert_eq!(shortcut::supported(), cfg!(windows));
}
