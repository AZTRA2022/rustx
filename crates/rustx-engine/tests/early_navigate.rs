//! A navigation asked for before the router has mounted.
//!
//! A scenario of its own, because one CEF application at a time: two
//! windows started from one test binary fight over the same user data
//! directory and the second never initializes.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("early_navigate"))
}

#[test]
fn a_navigation_asked_for_before_the_router_started_is_honoured() {
    // The application's script runs when the JavaScript context is
    // created, and the router mounts on the first frame where its host
    // element exists — later. A `navigate` in between was overwritten by
    // the starting path, so the screen the script asked for never came
    // up and nothing said why.
    assert_eq!(
        report().get("screen"),
        "device 7",
        "the navigation was swallowed by the router's start:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("path"),
        "/device/7",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn going_back_returns_to_the_screen_the_application_started_on() {
    // It was a navigation, not a different starting point: the path the
    // application started on is still behind it.
    assert_eq!(
        report().get("after_back"),
        "home",
        "the starting screen was lost:\n{}",
        report().raw()
    );
}
