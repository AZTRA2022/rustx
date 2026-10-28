//! Putting an element tree on screen.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("mount"))
}

#[test]
fn the_mounted_tree_matches_what_was_written() {
    // Review Focus 5: nothing lost, nothing reordered, text at depth.
    assert_eq!(
        report().get("html"),
        "<ul><li>a</li><li><b>c</b></li></ul>",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn an_empty_component_mounts_nothing_and_does_not_fail() {
    assert_eq!(
        report().get("empty_children"),
        "0",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_fragment_mounts_its_nodes_side_by_side_without_a_wrapper() {
    assert_eq!(
        report().get("fragment_html"),
        "<i>a</i><i>b</i>",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_click_on_a_mounted_node_runs_the_rust_closure() {
    assert_eq!(
        report().get("clicked"),
        "1",
        "the closure written in Rust never ran:\n{}",
        report().raw()
    );
}
