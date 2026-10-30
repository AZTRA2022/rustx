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
