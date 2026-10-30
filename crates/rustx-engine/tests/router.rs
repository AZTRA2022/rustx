//! One screen at a time, chosen by a path.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("router"))
}

#[test]
fn the_first_screen_is_the_route_the_application_started_on() {
    assert_eq!(
        report().get("first_screen"),
        "home",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn navigating_shows_the_other_screen() {
    assert_eq!(
        report().get("after_navigate"),
        "device 7",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_layout_that_stays_on_screen_keeps_its_state() {
    // Review Focus 1: the comfort of Next.js is exactly this. The bar
    // was clicked three times before the navigation and still says 3.
    assert_eq!(
        report().get("layout_state_after_navigate"),
        "3",
        "the layout was rebuilt instead of kept:\n{}",
        report().raw()
    );
}

#[test]
fn the_page_that_was_left_is_released_and_its_task_stops() {
    assert_eq!(
        report().get("left_page_released"),
        "yes",
        "the page that was left kept its state:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("left_page_task"),
        "stopped",
        "the page left the screen but its thread stayed:\n{}",
        report().raw()
    );
}

#[test]
fn changing_a_parameter_re_renders_the_same_page_and_re_runs_its_effect() {
    // Review Focus 3.
    assert_eq!(
        report().get("after_param_change"),
        "device 8",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("effect_order"),
        "open 7,close 7,open 8",
        "an effect watching the id did not close before opening:\n{}",
        report().raw()
    );
}

#[test]
fn a_path_nobody_declared_shows_the_not_found_page() {
    // Review Focus 4.
    assert_eq!(
        report().get("unknown_path"),
        "not found",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn going_back_returns_to_the_previous_screen() {
    assert_eq!(
        report().get("after_back"),
        "device 8",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_navigation_posted_from_a_worker_thread_lands_on_the_next_frame() {
    assert_eq!(
        report().get("after_posted_navigate"),
        "home",
        "a navigation from a worker thread never reached the screen:\n{}",
        report().raw()
    );
}
