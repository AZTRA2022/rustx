//! The proof of plan 4: a background thread drives the screen, two
//! distant components read the same value, and unmounting the panel that
//! started the thread stops it.

mod common;

use std::sync::OnceLock;

use common::{Report, run_monitor_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_monitor_scenario)
}

#[test]
fn a_background_thread_drives_what_is_on_screen() {
    let ticks: u32 = report()
        .get("ticks")
        .parse()
        .unwrap_or_else(|_| panic!("ticks was not a number:\n{}", report().raw()));
    assert!(
        ticks >= 3,
        "the thread drove the screen {ticks} times:\n{}",
        report().raw()
    );
}

#[test]
fn two_distant_components_show_the_same_value() {
    // Neither is the other's parent, and nothing was handed down.
    assert_eq!(
        report().get("panel"),
        report().get("header"),
        "the two readers of one store disagree:\n{}",
        report().raw()
    );
}

#[test]
fn unmounting_the_panel_stops_the_thread_it_started() {
    assert_eq!(
        report().get("ticking_after_unmount"),
        "no",
        "the panel left but its thread kept going:\n{}",
        report().raw()
    );
}

/// La deuxième fenêtre, mesurée sur une vraie application.
///
/// Les deux fenêtres rapportent dans le même fichier : ce que la
/// deuxième a vu se lit donc d'ici. Ce qui ne se lit pas ici — qu'elles
/// sont deux processus de rendu, donc deux états — est tracé par
/// `RUSTX_TRACE=1` et écrit dans le plan.
mod windows {
    use super::common::{Report, run_window_scenario};
    use std::sync::OnceLock;

    fn report() -> &'static Report {
        static REPORT: OnceLock<Report> = OnceLock::new();
        REPORT.get_or_init(run_window_scenario)
    }

    #[test]
    fn a_second_window_opens_on_the_route_it_was_given() {
        assert_eq!(
            report().get("second_hash"),
            "#/pause",
            "la route d'une deuxième fenêtre voyage dans son URL:\n{}",
            report().raw()
        );
        assert_eq!(
            report().get("second_screen"),
            "le moniteur est arrêté",
            "la deuxième fenêtre n'a pas ouvert l'écran /pause:\n{}",
            report().raw()
        );
    }

    #[test]
    fn opening_a_window_leaves_the_first_one_alone() {
        assert_eq!(
            report().get("first_still_ticking"),
            "yes",
            "la première fenêtre s'est arrêtée quand la deuxième est née:\n{}",
            report().raw()
        );
    }

    #[test]
    fn one_click_asks_for_one_window() {
        assert_eq!(
            report().get("handler_ran"),
            "1",
            "le gestionnaire a tourné plus d'une fois:\n{}",
            report().raw()
        );
    }
}
