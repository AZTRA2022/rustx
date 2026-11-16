//! Les écrans, sur la vraie machine qui lance les tests.
//!
//! Rien n'est épinglé en dur — on ne sait pas combien de moniteurs elle
//! a — mais tout ce qui doit être vrai de n'importe quel bureau l'est
//! vérifié : au moins un écran, un principal, des tailles non nulles,
//! une zone de travail dans l'écran, et l'écran sous la souris qui est
//! bien un des écrans listés.

#![cfg(windows)]

use rustx_os::screen::{self, Area};
use rustx_os::window;

#[test]
fn the_machine_lists_at_least_one_screen_and_the_primary_comes_first() {
    let all = screen::screens().expect("at least one screen");
    assert!(!all.is_empty());
    assert!(all[0].primary, "the primary screen comes first: {all:#?}");
    assert_eq!(
        all.iter().filter(|screen| screen.primary).count(),
        1,
        "exactly one screen is primary: {all:#?}"
    );
}

#[test]
fn every_screen_has_a_size_a_scale_and_a_work_area_inside_it() {
    for screen in screen::screens().expect("screens") {
        assert!(!screen.name.is_empty(), "{screen:?}");
        assert!(
            screen.bounds.width > 0 && screen.bounds.height > 0,
            "{screen:?}"
        );
        assert!(screen.scale >= 1.0, "{screen:?}");
        assert!(
            screen.work.x >= screen.bounds.x && screen.work.y >= screen.bounds.y,
            "the work area starts inside the screen: {screen:?}"
        );
        assert!(
            screen.work.x + screen.work.width <= screen.bounds.x + screen.bounds.width,
            "the work area ends inside the screen: {screen:?}"
        );
    }
}

#[test]
fn the_screen_under_the_pointer_is_one_of_the_screens() {
    let (x, y) = window::pointer().expect("the pointer");
    let under = screen::holding(x, y).expect("a screen");
    let all = screen::screens().expect("screens");
    assert!(all.contains(&under), "{under:?} is not in {all:#?}");
    // La souris est sur un écran : celui-là, sauf si elle est dans un
    // trou entre deux moniteurs, où `holding` rend le principal.
    assert!(
        under.bounds.holds(x, y) || under.primary,
        "{under:?} does not hold ({x}, {y}) and is not the primary"
    );
}

#[test]
fn the_primary_screen_is_the_one_the_list_says() {
    let primary = screen::primary().expect("a primary screen");
    assert!(primary.primary);
    assert_eq!(primary, screen::screens().expect("screens")[0]);
    // Le bureau a son origine sur l'écran principal, sous Windows.
    assert_eq!(
        Area::between(primary.bounds.x, primary.bounds.y, 0, 0).width,
        -primary.bounds.x
    );
}
