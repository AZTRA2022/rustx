//! A window whose compositor produces no frames.
//!
//! `requestAnimationFrame` is how a page is asked to draw, and on a
//! machine whose GPU process crashes on every start it is never called
//! back: the window exists, the JavaScript runs, and the application
//! draws nothing at all, with nothing in it wrong. A framework whose
//! whole render loop hangs off one browser callback is a framework that
//! looks broken on a virtual machine, a remote desktop, or a laptop
//! whose driver has given up.
//!
//! So the loop has a floor under it: a timer that runs the frame itself
//! whenever none has run for 100ms.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("no_frames"))
}

#[test]
fn frames_keep_coming_when_the_page_is_never_asked_to_draw() {
    let first: u64 = report().get("frames_at_first").parse().unwrap_or_default();
    let later: u64 = report().get("frames_later").parse().unwrap_or_default();
    assert!(
        later > first,
        "the frame loop stopped with no `requestAnimationFrame`: {first} then {later}\n{}",
        report().raw()
    );
    // And at a rate a page can live with, rather than one a minute.
    assert!(
        later - first >= 3,
        "only {} frames in 700ms without `requestAnimationFrame`:\n{}",
        later - first,
        report().raw()
    );
}
