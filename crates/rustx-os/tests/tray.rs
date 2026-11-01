//! L'icône de barre système, son menu, et les notifications.
//!
//! Ce qui se prouve sans personne devant l'écran : l'icône est bien
//! posée dans la barre et son `Drop` l'en retire, un identifiant choisi
//! est rendu une fois et une seule, une notification rend la main tout
//! de suite, et un fichier qui n'est pas un `.ico` est refusé en une
//! phrase. Ce qui demande un œil — à quoi ressemble le menu, où la
//! bulle apparaît — se regarde avec
//! `cargo run -p rustx-os --example tray_icon`.

use std::path::PathBuf;

use rustx_os::notify;
use rustx_os::tray::{self, Chosen, Icon, Item};

/// Les tests qui comptent les icônes de la barre ne peuvent pas tourner
/// en même temps : ils tournent dans un seul processus, et une icône
/// posée par l'un est comptée par l'autre. Mesuré — le test du `Drop`
/// accusait le `Drop` de ne rien retirer, alors que ce qu'il voyait
/// était l'icône cachée du test des notifications.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Un `.ico` du dépôt, parce qu'un `.ico` écrit à la main dans un test
/// serait un `.ico` dont on ne sait pas s'il est valide.
fn an_icon() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.join("../../apps/hybrid/assets/icon.ico")
        .canonicalize()
        .expect("l'icône de l'application hybride")
}

#[test]
fn an_icon_appears_and_its_drop_removes_it() {
    if !tray::supported() {
        return;
    }
    // Compté à partir de ce qu'il y a déjà : les tests d'un même
    // fichier tournent dans un seul processus et en parallèle, et celui
    // des notifications y pose une icône cachée. Ce qui est prouvé ici
    // est une de plus, puis une de moins.
    let _one_at_a_time = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let before = tray::shown();
    {
        let icon = Icon::new(&an_icon(), "rustx, en test").expect("l'icône est posée");
        assert_eq!(
            tray::shown(),
            before + 1,
            "l'icône n'est pas arrivée dans la barre"
        );
        // Et le menu se pose sans que personne ne clique.
        icon.menu(&[
            Item::new("ouvrir", "Ouvrir la fenêtre"),
            Item::new("quitter", "Quitter"),
        ])
        .expect("le menu est accepté");
    }
    // Hors de la portée : une icône qui survit à son jeton est une
    // icône morte que l'utilisateur ne peut plus enlever.
    assert_eq!(tray::shown(), before, "le Drop n'a pas retiré l'icône");
}

#[test]
fn a_menu_item_that_was_chosen_is_reported_once() {
    // La file entre le thread de l'icône et la page : ce qu'un clic y
    // laisse est lu une fois. Lu deux fois, une application ouvrirait
    // deux fenêtres pour un seul clic.
    let chosen = Chosen::new();
    assert_eq!(chosen.take(), None, "rien n'a encore été choisi");
    chosen.push("ouvrir");
    chosen.push("quitter");
    assert_eq!(chosen.take().as_deref(), Some("ouvrir"));
    assert_eq!(chosen.take().as_deref(), Some("quitter"), "dans l'ordre");
    assert_eq!(chosen.take(), None, "et plus rien");
}

#[test]
fn a_notification_does_not_block() {
    if !notify::supported() {
        return;
    }
    let _one_at_a_time = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let started = std::time::Instant::now();
    let shown = notify::show(
        notify::What::new("rustx", "Une notification de test.").icon(an_icon()),
    );
    let waited = started.elapsed();
    assert!(shown.is_ok(), "la notification a été refusée : {shown:?}");
    assert!(
        waited < std::time::Duration::from_millis(500),
        "elle a mis {waited:?} : une notification ne bloque pas celui qui la montre"
    );
}

#[test]
fn an_icon_file_that_is_not_an_ico_is_refused_in_words() {
    let not_one = std::env::temp_dir().join(format!("rustx-not-an-icon-{}.ico", std::process::id()));
    std::fs::write(&not_one, b"\x89PNG\r\n\x1a\n and the rest").expect("un faux .ico");
    let refused = tray::readable(&not_one);
    let said = refused.expect_err("un PNG renommé n'est pas une icône").to_string();
    assert!(said.contains("icon"), "got {said}");
    let _ = std::fs::remove_file(&not_one);

    // Et le même refus arrive par la porte d'entrée.
    if tray::supported() {
        let refused = Icon::new(&PathBuf::from("rien-du-tout.ico"), "x");
        assert!(refused.is_err());
    }
}

#[test]
fn a_platform_that_has_no_tray_says_so_before_being_asked() {
    assert_eq!(tray::supported(), cfg!(windows));
    assert_eq!(notify::supported(), cfg!(windows));
}
