//! A page that waits, read off a window that really opened.
//!
//! The unit tests prove that a future runs, that a token abandons it,
//! and that a write posted from one lands on a frame. What they cannot
//! prove is the sentence a developer actually writes: `task(async move
//! { … })` in a `.rsx`, `.await` on a timer, and a `let mut` at the top
//! of the page changing because of it.

mod common;

use std::sync::OnceLock;

use common::{Report, run_hybrid_scenario_named};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_hybrid_scenario_named("attente"))
}

#[test]
fn the_scenario_got_to_the_end() {
    assert_eq!(report().get("done"), "yes", "full output:\n{}", report().raw());
}

#[test]
fn three_awaits_write_three_rows() {
    // `rustx::time::sleep(…).await` three times, and a write to a
    // shared `let mut` after each one. The timer belongs to the
    // runtime, not to the compositor, which is why it fires at all on a
    // machine whose window produces no frames of its own.
    assert_eq!(
        report().get("rows_after_three_awaits"),
        "3",
        "full output:\n{}",
        report().raw()
    );
    // And the screen shows them: a write from a future is a render like
    // any other.
    assert_eq!(
        report().get("rows_drawn"),
        "3",
        "the state changed and the page did not: {}",
        report().raw()
    );
}

#[test]
fn a_thousand_writes_from_a_future_do_not_cost_a_thousand_renders() {
    assert_eq!(
        report().get("burst_total"),
        "1000",
        "some writes were lost:\n{}",
        report().raw()
    );
    // Not "one": a real window draws for its first paint and for each
    // of the three awaits as well. What is asserted is the shape of the
    // contract — the number of renders is the frame loop's, not the
    // loop the developer wrote. A thousand uncoalesced writes would be
    // a thousand.
    let renders: u32 = report()
        .get("renders_for_the_burst")
        .parse()
        .expect("a number");
    assert!(
        renders < 60,
        "a thousand writes cost {renders} renders:\n{}",
        report().raw()
    );
}

#[test]
fn leaving_the_page_stops_the_work_it_started() {
    // The endless task. Nothing in the scenario cancels it by name:
    // leaving drops the cell that holds its token, the token abandons
    // the future, and the counter stops.
    assert_eq!(
        report().get("still_ticking"),
        "no",
        "the page left and its future kept writing (at {} -> {}):\n{}",
        report().get("ticks_at_leaving"),
        report().get("ticks_after_leaving"),
        report().raw()
    );
    // And it really was running before: a test that proves a stopped
    // clock stopped proves nothing.
    let before: u32 = report()
        .get("ticks_before_leaving")
        .parse()
        .expect("a number");
    assert!(
        before > 0,
        "the endless task never ticked at all, so stopping it means nothing:\n{}",
        report().raw()
    );
}
