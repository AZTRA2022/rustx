//! The proof of plan 3: the design document's counter, counting.

mod common;

use std::sync::OnceLock;

use common::{Report, run_counter_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_counter_scenario)
}

#[test]
fn the_counter_of_the_design_document_counts() {
    assert_eq!(
        report().get("after_three_clicks"),
        "3",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn two_counters_side_by_side_count_separately() {
    // Review Focus 1, where a human can see it: one `let mut n = 0;` in
    // the source, two counters on screen, two counts.
    assert_eq!(
        report().get("first"),
        "3",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("second"),
        "0",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn reordering_a_keyed_list_carries_each_items_state_with_it() {
    // Review Focus 3, where a human can see it: the row clicked twice
    // still shows 2 after it has been moved to the front.
    assert_eq!(
        report().get("moved_item_count"),
        "2",
        "the state stayed with the position instead of the row:\n{}",
        report().raw()
    );
}
