//! The proof of plan 6: one DOM, two owners.

mod common;

use std::sync::OnceLock;

use common::{Report, run_hybrid_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_hybrid_scenario)
}

#[test]
fn a_javascript_component_lives_inside_a_rustx_page() {
    assert_eq!(
        report().get("chart"),
        "2 points",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_rustx_component_lives_inside_a_javascript_panel() {
    assert_eq!(
        report().get("counter_in_panel"),
        "2",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn both_trees_share_one_frame_loop() {
    // One value, two owners, and both showed it: there is no second
    // render loop to fall out of step with.
    assert_eq!(
        report().get("drawn_on_one_frame"),
        "yes",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("js_mounts"),
        "1",
        "the JavaScript component was rebuilt instead of given new props:\n{}",
        report().raw()
    );
}

#[test]
fn leaving_the_page_tells_javascript_before_the_node_goes() {
    assert_eq!(
        report().get("js_unmounted"),
        "yes",
        "full output:\n{}",
        report().raw()
    );
}
