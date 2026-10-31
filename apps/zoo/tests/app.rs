//! Many libraries, one page.

mod common;

use std::sync::OnceLock;

use common::{Report, run_zoo_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_zoo_scenario)
}

#[test]
fn icons_are_components_with_numbers_and_strings_for_props() {
    assert_eq!(
        report().get("icons"),
        "3",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("icon_size"),
        "28",
        "a number prop did not reach the library:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("icon_colour"),
        "#a78bfa",
        "a string prop did not reach the library:\n{}",
        report().raw()
    );
}

#[test]
fn a_library_renders_what_the_page_wrote_as_its_children() {
    // `<Markdown>{"# Un titre\n\nEt du **gras**"}</Markdown>`: the
    // content is a Rust string, and what comes out is the library's own
    // elements.
    assert_eq!(
        report().get("markdown_title"),
        "Un titre",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("markdown_bold"),
        "gras",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_library_keeps_its_own_state_inside_the_page() {
    // Radix decides which tab is open. rustx hands over the tree and
    // does not look in, so clicking a trigger is the library's business
    // and the page is not re-rendered for it.
    assert_eq!(
        report().get("panel_one_open"),
        "yes",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("panel_two_after_click"),
        "yes",
        "the library's own state did not change the screen:\n{}",
        report().raw()
    );
    // Radix keeps the panel that is not open mounted and marks it,
    // rather than removing it. What a library does inside its own tree
    // is the library's business: rustx handed the tree over and does
    // not look in.
    assert_eq!(
        report().get("panel_one_state"),
        "inactive",
        "full output:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("panel_two_state"),
        "active",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn the_pages_own_state_works_beside_all_of_them() {
    assert_eq!(
        report().get("clicks"),
        "2",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn a_prop_the_library_calls_reaches_the_page() {
    // Radix calls `onValueChange` when the user picks a tab. The page
    // wrote a Rust closure for it, the closure stayed in Rust, and what
    // crossed was a function that calls it — so a page learns what
    // happened inside a tree it does not own, which is what every real
    // library needs.
    assert_eq!(
        report().get("chosen"),
        "two",
        "the library's callback never reached Rust:\n{}",
        report().raw()
    );
}
