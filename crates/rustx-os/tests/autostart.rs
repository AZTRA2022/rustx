//! Démarrer avec la session, ou non.
//!
//! Le test écrit vraiment dans le registre de l'utilisateur, parce
//! qu'une écriture simulée ne prouverait rien — et il l'efface en
//! partant, sous un nom qui ne peut être celui d'aucune vraie
//! application.

use rustx_os::autostart;

fn a_name() -> String {
    format!("rustx-test-{}", std::process::id())
}

#[test]
fn what_is_set_is_on_and_what_is_unset_is_gone() {
    if !autostart::supported() {
        return;
    }
    let name = a_name();
    assert!(!autostart::is_on(&name), "there should be nothing there");

    autostart::set(true, &name).expect("le registre accepte");
    assert!(autostart::is_on(&name), "rien n'a été écrit");

    autostart::set(false, &name).expect("le registre accepte");
    assert!(
        !autostart::is_on(&name),
        "the application would still start with the session"
    );
}

#[test]
fn asking_twice_is_not_an_error() {
    if !autostart::supported() {
        return;
    }
    // Une préférence qu'on coche deux fois est cochée, pas cassée. Et
    // la décocher alors qu'elle ne l'était pas n'est pas une panne.
    let name = format!("{}-twice", a_name());
    autostart::set(true, &name).expect("une fois");
    autostart::set(true, &name).expect("deux fois");
    assert!(autostart::is_on(&name));
    autostart::set(false, &name).expect("décoché");
    autostart::set(false, &name).expect("décoché deux fois");
    assert!(!autostart::is_on(&name));
}

/// Les trois plateformes l'ont : le registre sur Windows, un
/// `.desktop` sur Linux, un agent de session sur macOS. Le jour ou une
/// quatrieme arrive sans, c'est ici qu'elle le dira.
#[test]
fn every_platform_here_has_this() {
    assert!(autostart::supported());
}
