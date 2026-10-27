//! Calling Rust from the DOM, with no boundary in between.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

/// One run of the page serves every assertion below: starting a browser
/// costs ten seconds, and the scenario exercises all three behaviours in
/// order, so each test reads its own line out of the same run.
fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("natives"))
}

#[test]
fn a_dom_click_runs_rust_and_returns_its_value() {
    // The counter lives in Rust. Two clicks, two increments.
    assert_eq!(report().get("count"), "2", "full output:\n{}", report().raw());
}

#[test]
fn a_panic_in_a_native_function_does_not_kill_the_process() {
    // A panic unwinding into C is undefined behaviour. It has to become a
    // JavaScript exception instead, and the process has to still be there.
    assert!(
        report().get("panic_error").contains("internal error"),
        "the panic did not surface as a readable exception: {:?}",
        report().get("panic_error")
    );
    assert_eq!(
        report().get("after_panic"),
        "3",
        "the engine did not survive the panic:\n{}",
        report().raw()
    );
}

#[test]
fn badly_typed_arguments_produce_a_readable_js_exception() {
    assert!(
        report().get("bad_args_error").contains("expected 1 argument"),
        "unhelpful message for a bad call: {:?}",
        report().get("bad_args_error")
    );
}

#[test]
fn a_native_function_receives_the_arguments_javascript_passed() {
    assert_eq!(
        report().get("echo"),
        "greet:bonjour/7/true",
        "arguments did not cross intact:\n{}",
        report().raw()
    );
}
