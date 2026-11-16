//! Background work that belongs to a component.
//!
//! A capture thread is the first thing the proof-of-concept software asks
//! for: it opens a stream, it never stops on its own, and it has to be
//! gone when the panel that started it leaves the screen.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("task"))
}

#[test]
fn a_task_stops_when_its_token_is_dropped() {
    // Review Focus 1.
    assert_eq!(
        report().get("task_stopped"),
        "yes",
        "the task kept running after its token was dropped:\n{}",
        report().raw()
    );
}

#[test]
fn a_task_that_panics_does_not_take_the_window_with_it() {
    // Review Focus 2: a capture thread touches hardware; it will panic.
    assert_eq!(
        report().get("alive_after_task_panic"),
        "yes",
        "a panicking task froze the application:\n{}",
        report().raw()
    );
    assert!(
        report().raw().contains("a background task panicked"),
        "the panic was swallowed instead of being reported:\n{}",
        report().raw()
    );
}

#[test]
fn what_a_cancelled_task_posts_afterwards_is_refused() {
    assert_eq!(
        report().get("post_after_cancel"),
        "refused",
        "work from a cancelled task still reached the UI thread:\n{}",
        report().raw()
    );
}

#[test]
fn a_task_kept_by_a_component_stops_when_the_component_goes() {
    assert_eq!(
        report().get("task_after_unmount"),
        "stopped",
        "the component left but its thread stayed:\n{}",
        report().raw()
    );
}
