//! What happens when a component leaves, and what props do.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("lifecycle"))
}

#[test]
fn an_unmounted_component_releases_its_state() {
    // Review Focus 5. Cleanup is Rust's own: dropping the cells drops
    // what they hold, and no second mechanism is invented on top.
    assert_eq!(
        report().get("dropped"),
        "1",
        "the component's state was never released:\n{}",
        report().raw()
    );
}

#[test]
fn a_re_render_does_not_drop_anything() {
    assert_eq!(
        report().get("dropped_after_rerender"),
        "0",
        "a re-render destroyed state it should have kept:\n{}",
        report().raw()
    );
}

#[test]
fn an_unmounted_component_leaves_no_state_behind() {
    assert_eq!(
        report().get("live_after_unmount"),
        "0",
        "state outlived the component that owned it:\n{}",
        report().raw()
    );
}

#[test]
fn a_component_takes_props_and_re_renders_when_they_change() {
    assert_eq!(
        report().get("after_prop_change"),
        "Micro",
        "the new prop never reached the screen:\n{}",
        report().raw()
    );
}

#[test]
fn a_prop_is_not_turned_into_state() {
    // Promoting a parameter would freeze it at its first value.
    assert_eq!(
        report().get("prop_after_two_changes"),
        "Écran",
        "the prop stopped changing:\n{}",
        report().raw()
    );
}

#[test]
fn a_row_that_leaves_a_keyed_list_releases_what_it_held() {
    // The cleanup of a keyed row is only reachable from the end of a
    // render. A render loop that never closes a render leaves it
    // unreachable: the list's state grows for every row that ever
    // existed, a row that comes back resurrects its old value, and a
    // row holding a file handle never runs its `Drop`.
    assert_eq!(
        report().get("drops_after_row_left"),
        "1",
        "a row left the list without releasing what it held:\n{}",
        report().raw()
    );
}
