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

#[test]
fn a_radix_dialog_written_in_the_page_opens_and_closes() {
    // What shadcn/ui is made of, written in `.rsx` and nowhere else:
    // Radix for the behaviour, Tailwind for the look. The dialog is in a
    // portal — somewhere else in the document, outside the container
    // rustx handed over — which is exactly what Radix is for and what a
    // boundary that only passed props could never have done.
    assert_eq!(
        report().get("dialog_title"),
        "there",
        "the dialog did not open:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("dialog_is_in_a_portal"),
        "yes",
        "the dialog was not in a portal:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("dialog_closed"),
        "yes",
        "the library's own close button did not close it:\n{}",
        report().raw()
    );
}

#[test]
fn a_tailwind_class_written_in_a_page_reaches_the_screen() {
    // `className="bg-violet-500"` in `app/page.rsx`. Tailwind scanned
    // the `.rsx`, wrote the class, and the framework put the stylesheet
    // in the page — the application has no `.css` of its own and no
    // `<link>`.
    assert_eq!(
        report().get("stylesheet_in_page"),
        "yes",
        "the stylesheet never reached the document:\n{}",
        report().raw()
    );
    // A colour, read on the page's own hero badge.
    assert_eq!(
        report().get("hero_badge_background"),
        "rgba(139, 92, 246, 0.1)",
        "`bg-violet-500/10` did not reach the screen:\n{}",
        report().raw()
    );
    // And a utility on a node Radix rendered, which the page never
    // touched: the padding it asked for is on the library's own button.
    //
    // Read as a padding rather than as a background on purpose. A
    // `<button>` that Chromium paints as a native control reports the
    // control's own background from `getComputedStyle` whatever the
    // stylesheet says — a clone of the very same node, with the same
    // classes and the same parent, reports the violet. The class is
    // applied; what is being read is not the paint.
    assert_eq!(
        report().get("trigger_padding"),
        "20px",
        "`px-5` did not reach the node the library rendered:\n{}",
        report().raw()
    );
    // And a class on a node the library rendered through a portal.
    assert_eq!(
        report().get("dialog_padding"),
        "24px",
        "`p-6` did not reach the dialog:\n{}",
        report().raw()
    );
}

#[test]
fn a_handler_on_an_installed_component_reaches_rust() {
    // `#add` is shadcn's own `Button` — TypeScript that `npx shadcn add`
    // put in `components/ui/`, three components deep inside a `Card`.
    // Its `onClick` is a Rust closure. Until the boundary carried the
    // handlers of the nodes *inside* it, a page of installed components
    // drew perfectly and did nothing at all when clicked.
    assert_eq!(
        report().get("dots_after_two_points"),
        "3",
        "two clicks on an installed component never reached Rust:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("counter_in_panel"),
        "2",
        "full output:\n{}",
        report().raw()
    );
}
