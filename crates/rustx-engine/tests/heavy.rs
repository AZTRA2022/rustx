//! A heavy page: a thousand rows, and one of them changing.
//!
//! This is the measurement that decides whether a framework can carry a
//! serious application. A list of a thousand rows is ordinary — a device
//! list, a log, a table — and a change to one row has to cost one
//! correction to the DOM. If it costs a thousand, every list in every
//! application is a stutter nobody can explain from the outside, and no
//! amount of being fast in Rust makes up for it.

mod common;

use std::sync::OnceLock;

use common::{Report, run_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(|| run_scenario("heavy"))
}

fn number(key: &str) -> u64 {
    report()
        .get(key)
        .parse()
        .unwrap_or_else(|_| panic!("{key} should be a number, got {:?}", report().get(key)))
}

#[test]
fn a_thousand_rows_are_on_screen() {
    assert_eq!(number("rows"), 1000, "full output:\n{}", report().raw());
    assert_eq!(
        number("rows_after"),
        1000,
        "the list lost or gained rows:\n{}",
        report().raw()
    );
}

#[test]
fn changing_one_row_of_a_thousand_is_one_correction() {
    // The whole point. The tree is built again — a render is a function
    // of the state, and that is the bargain — but what reaches the DOM
    // is the one text that moved.
    let patches = number("one_row_patches");
    assert!(
        patches <= 2,
        "a one-row change produced {patches} patches:\n{}",
        report().raw()
    );
}

#[test]
fn a_heavy_page_renders_in_a_time_a_frame_can_hold() {
    // The budget a frame has. In a release build that is the real
    // number — sixteen milliseconds is sixty frames a second — and in a
    // debug build it is this same test with the brakes on, kept only so
    // that a change which makes the diff quadratic again is caught by
    // the ordinary suite rather than by a user.
    //
    // It was 26ms before the keyed diff stopped scanning the old
    // children for every new one, and 5.7ms after.
    let ceiling = if cfg!(debug_assertions) {
        40_000
    } else {
        16_000
    };
    let again = number("one_row_micros");
    assert!(
        again < ceiling,
        "changing one row of a thousand took {again}µs, and a frame holds {ceiling}µs:
{}",
        report().raw()
    );
}
