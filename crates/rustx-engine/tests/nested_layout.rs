//! Leaving a nested layout takes its DOM with it.
//!
//! Reported from an application being built on the framework:
//! "leaving a nested layout leaves its last page's DOM behind".
//! Navigation worked, nothing crashed and the page reported no error —
//! the old screen simply stayed in the document under the new one, once
//! per navigation, for as long as the window was open.
//!
//! The cause was one line. A mounted subtree was detached with
//! `node.dom.remove()`, and `MountedNode::dom` is `None` for a fragment
//! and for an `Empty`: a fragment has no element of its own. So removing
//! a fragment removed nothing at all, while the bookkeeping that pointed
//! at its children was replaced — leaving elements in the document that
//! no later render could even find. A layout that puts something beside
//! its page is exactly that shape, which is why it took an application
//! rather than a unit test to find.
//!
//! This has to be measured in a real window: what a frame does to the
//! DOM is the half of the engine that needs a V8 context, so the claim
//! is read out of `document.querySelectorAll` in the page itself.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("nested"))
}

#[test]
fn the_screen_under_a_nested_layout_is_on_screen_to_begin_with() {
    assert_eq!(
        report().get("deep_on_screen"),
        "1",
        "the scenario never got as far as its first screen:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("aside_on_screen"),
        "1",
        "the nested layout drew nothing:\n{}",
        report().raw()
    );
}

#[test]
fn leaving_a_nested_layout_takes_its_pages_dom_with_it() {
    assert_eq!(
        report().get("deep_left_behind"),
        "0",
        "the page of the layout that was left is still in the document:\n{}",
        report().raw()
    );
}

#[test]
fn leaving_a_nested_layout_takes_the_layouts_own_dom_with_it() {
    assert_eq!(
        report().get("aside_left_behind"),
        "0",
        "the nested layout itself is still in the document:\n{}",
        report().raw()
    );
}

#[test]
fn the_screen_that_was_navigated_to_is_drawn_once() {
    assert_eq!(
        report().get("flat_on_screen"),
        "1",
        "the new screen was not drawn, or was drawn twice:\n{}",
        report().raw()
    );
}

#[test]
fn two_rounds_in_and_out_leave_nothing_to_accumulate() {
    // The symptom of the defect was accumulation: one round left one
    // copy behind, two rounds left two. A fix that removed only the
    // first copy would pass the tests above and fail this one.
    assert_eq!(
        report().get("deep_after_two_rounds"),
        "0",
        "a second round through the nested layout left a second copy:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("aside_after_two_rounds"),
        "0",
        "a second round left a second copy of the layout:\n{}",
        report().raw()
    );
    assert_eq!(
        report().get("flat_after_two_rounds"),
        "1",
        "the screen outside the layout was drawn more than once:\n{}",
        report().raw()
    );
}
