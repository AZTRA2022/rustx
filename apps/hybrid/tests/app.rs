//! One DOM, two owners — and the component React draws comes off npm.

mod common;

use std::sync::OnceLock;

use common::{Report, run_hybrid_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_hybrid_scenario)
}

#[test]
fn the_page_draws_a_library_component_it_wrote_itself() {
    // `<js::recharts::LineChart>` with its axes inside it, written in
    // `app/page.rsx`. What is on screen is recharts' own SVG, with
    // recharts' own classes — nothing in this application drew it and
    // nothing in this application bridges to it.
    assert_eq!(
        report().get("library_drew"),
        "yes",
        "full output:\n{}",
        report().raw()
    );
    // One point in the data, one dot on screen.
    assert_eq!(
        report().get("dots_at_first"),
        "1",
        "the chart was drawn with the wrong data:\n{}",
        report().raw()
    );
}

#[test]
fn a_point_added_in_rust_reaches_the_library() {
    // `points += 1` in a handler, twice. The data is built in Rust,
    // crosses as a list of records, and the library draws it.
    assert_eq!(
        report().get("dots_after_two_points"),
        "3",
        "full output:\n{}",
        report().raw()
    );
    // And three points make a line, which one point could not: the
    // library redrew from the data rather than keeping what it had.
    assert_eq!(
        report().get("curve_drawn"),
        "yes",
        "the chart kept what it had drawn for one point:\n{}",
        report().raw()
    );
}

#[test]
fn a_library_given_new_props_is_updated_and_not_rebuilt() {
    // The same `<svg>` node: one React root per boundary, kept by the
    // node itself, so a new render is a props change.
    assert_eq!(
        report().get("same_svg"),
        "yes",
        "the React root was rebuilt instead of given new props:\n{}",
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
}

#[test]
fn leaving_the_page_takes_what_the_library_drew_with_it() {
    assert_eq!(
        report().get("svg_after_leave"),
        "gone",
        "full output:\n{}",
        report().raw()
    );
}
