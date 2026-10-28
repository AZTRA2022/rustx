//! The frame loop, and what worker threads are allowed to do.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

/// The scenario starts a worker thread that posts ten thousand state
/// updates as fast as it can, then lets the frame loop settle.
fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("frame"))
}

#[test]
fn every_posted_update_is_applied() {
    assert_eq!(
        report().get("counter"),
        "10000",
        "updates were dropped:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("shown"),
        "10000",
        "the page does not show the final state:\n{}",
        report().raw()
    );
}

#[test]
fn ten_thousand_updates_do_not_cause_ten_thousand_renders() {
    // The point of the design: mutating state marks the component dirty,
    // and the DOM is written once per frame. A capture thread pushing
    // faster than the screen refreshes must not be able to drown rendering.
    let writes: u64 = report()
        .get("dom_writes")
        .parse()
        .expect("a number of DOM writes");

    assert!(
        writes < 200,
        "{writes} DOM writes for 10000 state updates — the frame loop is not coalescing"
    );
    assert!(
        writes >= 1,
        "nothing was ever rendered:\n{}",
        report().raw()
    );
}

#[test]
fn worker_threads_never_touch_the_dom_themselves() {
    // A posted job runs on the thread that owns the DOM, never on the
    // thread that posted it.
    assert_eq!(
        report().get("job_thread"),
        report().get("ui_thread"),
        "a posted job ran off the UI thread:\n{}",
        report().raw()
    );
    assert_ne!(
        report().get("worker_thread"),
        report().get("ui_thread"),
        "the worker never actually ran on another thread:\n{}",
        report().raw()
    );
}

#[test]
fn a_panic_in_a_posted_job_does_not_stop_the_frame_loop() {
    // The proof application touches hardware, so jobs will panic. If one
    // panic ended the loop, the window would freeze with no error anywhere
    // and the queue would grow without bound behind it.
    assert_eq!(
        report().get("after_panicking_job"),
        "alive",
        "a panicking job killed the frame loop:\n{}",
        report().raw()
    );
}

#[test]
fn a_panic_in_a_frame_callback_does_not_unregister_the_others() {
    assert_eq!(
        report().get("callbacks_after_panic"),
        "alive",
        "a panicking frame callback took the others with it:\n{}",
        report().raw()
    );
}

#[test]
fn posting_after_the_window_closes_returns_an_error() {
    // A capture thread outliving the window must be told to stop, not
    // pile up work that will never run, and not bring the process down.
    assert_eq!(
        report().get("post_after_close"),
        "WindowClosed",
        "posting after shutdown did not report the window was gone:\n{}",
        report().raw()
    );
}
