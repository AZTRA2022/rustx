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

#[test]
fn react_can_mount_a_rustx_component_into_a_node_it_owns() {
    assert_eq!(
        report().get("rustx_inside_react"),
        "0",
        "the page could not mount a rustx component in a node React owns:\n{}",
        report().raw()
    );
}

#[test]
fn that_component_is_live_and_not_a_snapshot() {
    // The same frame loop as everything else: a component inside a node
    // React owns is a root of its own, re-rendered by the same tick.
    assert_eq!(
        report().get("rustx_inside_react_after_click"),
        "1",
        "the component was a snapshot, not a component:\n{}",
        report().raw()
    );
}

#[test]
fn releasing_the_node_releases_the_component_and_its_tasks() {
    assert_eq!(
        report().get("released_inside_react"),
        "yes",
        "what the component held was never dropped:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("roots_after_release"),
        report().get("roots_before_mount"),
        "the root stayed on the page after being released:\n{}",
        report().raw()
    );
}

#[test]
fn mounting_an_unregistered_rustx_component_is_an_error_the_page_can_catch() {
    assert!(
        report().get("unknown_rustx_component").contains("Nope"),
        "got {}:\n{}",
        report().get("unknown_rustx_component"),
        report().raw()
    );
}

#[test]
fn a_boundary_rendered_before_the_bridge_exists_is_handed_over_when_it_appears() {
    // A page installs its half of the boundary in a script, and rustx
    // renders as soon as the host element exists. A container that was
    // handed to nobody must not stay empty for good — otherwise a
    // `<script>` written after the host is an empty box nobody can
    // explain.
    assert_eq!(
        report().get("empty_while_no_bridge"),
        "yes",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("handed_over_late"),
        "yes",
        "the container was never handed over once the bridge appeared:\n{}",
        report().raw()
    );
}

#[test]
fn a_foreign_child_is_handed_over_with_its_parent() {
    // A library's components are written inside one another — a chart
    // holds its axes — and the whole tree crosses at once.
    assert_eq!(
        report().get("chart_inside"),
        r#"Axis({"dataKey":"at"})"#,
        "what was written inside the boundary did not cross:\n{}",
        report().raw()
    );
}

#[test]
fn a_childs_props_reach_react_without_the_parent_changing() {
    // The boundary is the node and everything under it: an axis whose
    // key changed is a change React has to be told about, even though
    // the chart's own props are the same.
    assert_eq!(
        report().get("chart_inside_after_child_prop"),
        r#"Axis({"dataKey":"value"})"#,
        "a change inside the boundary never reached React:\n{}",
        report().raw()
    );
}
