//! Re-rendering: once per frame, and only what changed.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("render"))
}

#[test]
fn clicking_updates_what_is_on_screen() {
    assert_eq!(
        report().get("after_one_click"),
        "1",
        "the counter did not redraw:\n{}",
        report().raw()
    );
}

#[test]
fn a_thousand_mutations_between_two_frames_render_once() {
    // The §7.4 pledge: the developer writes their loop as fast as they
    // like and cannot shoot themselves in the foot.
    let renders: u64 = report()
        .get("renders_for_1000")
        .parse()
        .expect("a number of renders");
    assert!(
        renders <= 2,
        "{renders} renders for 1000 mutations:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("shown_after_1000"),
        "1001",
        "the last value never reached the screen:\n{}",
        report().raw()
    );
}

#[test]
fn an_untouched_component_is_not_re_rendered() {
    assert_eq!(
        report().get("sibling_renders"),
        "1",
        "a component nobody touched was rendered again:\n{}",
        report().raw()
    );
}

#[test]
fn a_mutation_posted_from_a_worker_thread_lands_without_tearing() {
    // Review Focus 4: jobs are drained before the frame callbacks run,
    // so a render never reads a half-written state.
    assert_eq!(
        report().get("worker_final"),
        report().get("worker_shown"),
        "the screen and the state disagree:\n{}",
        report().raw()
    );
}

#[test]
fn only_the_text_that_changed_is_written_to_the_dom() {
    assert_eq!(
        report().get("untouched_sibling_text"),
        "fixed",
        "a node nobody changed was rewritten:\n{}",
        report().raw()
    );
}
