//! The claim the whole framework rests on, pinned by a test.
//!
//! Without this, a refactor could quietly push the application's Rust back
//! into the browser process and nobody would notice until somebody measured
//! disappointing latencies six months later.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("no_boundary"))
}

#[test]
fn rust_called_from_the_dom_runs_in_the_render_process() {
    let rust_pid = report().get("rust_pid");
    let browser_pid = report().get("browser_pid");

    assert_eq!(
        report().get("rust_role"),
        "Renderer",
        "the Rust called from the page did not run in the render process:\n{}",
        report().raw()
    );
    assert_ne!(
        rust_pid,
        browser_pid,
        "the Rust called from the page ran in the browser process, which means a boundary:\n{}",
        report().raw()
    );
}

#[test]
fn rust_sees_dom_state_javascript_wrote_in_the_same_tick() {
    // Asking Rust for its own process id and comparing it to itself proves
    // nothing. This does: JavaScript writes a value nobody could guess into
    // the live document, and Rust reads it back in the same synchronous
    // call, before the tick is over. Across a process boundary that value
    // would have to be serialized and awaited; there is nowhere for an IPC
    // round trip to hide inside one tick.
    let written = report().get("token");
    assert!(!written.is_empty(), "the page wrote no token:\n{}", report().raw());
    assert_eq!(
        report().get("token_seen_by_rust"),
        written,
        "Rust did not see what JavaScript had just written to the DOM:\n{}",
        report().raw()
    );
}

// The two figures below are tripwires, not targets, and the suite runs
// unoptimized. Measured on the development machine:
//
//              bare call   DOM round trip
//   release      1.06 µs         19.2 µs
//   debug        2.68 µs         31.4 µs
//
// An IPC round trip, which is what the framework exists to avoid, costs
// tens to hundreds of microseconds. Crossing either line means a boundary
// came back, not that the code got slightly slower.

#[test]
fn a_dom_rust_dom_round_trip_stays_under_fifty_microseconds() {
    let best_ns: f64 = report()
        .get("dom_round_trip_ns")
        .parse()
        .expect("a number of nanoseconds");

    assert!(
        best_ns < 50_000.0,
        "round trip too slow: {best_ns} ns — a boundary has reappeared"
    );
}

#[test]
fn a_bare_native_call_stays_in_the_microsecond_range() {
    let best_ns: f64 = report()
        .get("call_ns")
        .parse()
        .expect("a number of nanoseconds");

    assert!(
        best_ns < 10_000.0,
        "a plain Rust call from JavaScript took {best_ns} ns, which is IPC territory"
    );
}
