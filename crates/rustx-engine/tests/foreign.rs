//! Handing a node to React, and taking it back.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("foreign"))
}

#[test]
fn a_react_component_is_mounted_into_the_container_rustx_built() {
    assert_eq!(
        report().get("chart_text"),
        "chart of a",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn changing_a_prop_reaches_react_without_rebuilding_it() {
    assert_eq!(
        report().get("chart_text_after_prop"),
        "chart of b",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("react_mounts"),
        "1",
        "React was mounted again instead of being given the new props:\n{}",
        report().raw()
    );
}

#[test]
fn rustx_does_not_touch_what_react_put_inside() {
    assert_eq!(
        report().get("react_node_survived"),
        "yes",
        "a rustx re-render took away what React had put in the container:\n{}",
        report().raw()
    );
}

#[test]
fn react_is_told_before_its_container_leaves_the_document() {
    // Otherwise its effects never clean up and its listeners stay on a
    // node nobody can reach.
    assert_eq!(
        report().get("react_unmounted"),
        "yes",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("unmount_came_first"),
        "yes",
        "React was told after the container had already left:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("given_back"),
        "1",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_component_the_page_never_registered_says_so() {
    assert_eq!(
        report().get("unknown_component"),
        "rustx: the page has no React component named 'Nope'",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("unknown_container_empty"),
        "yes",
        "a typo should leave an empty container, not take the window hostage:\n{}",
        report().raw()
    );
}
