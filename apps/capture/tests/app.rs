//! The proof of plan 5: an application that is a directory of files.

mod common;

use std::sync::OnceLock;

use common::{Report, run_capture_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_capture_scenario)
}

#[test]
fn clicking_a_device_opens_its_preview() {
    assert_eq!(
        report().get("preview_title"),
        "Webcam",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn the_navigation_bar_keeps_counting_across_screens() {
    // The bar's thread belongs to the layout, and the layout is above
    // both screens — so it is never stopped and never restarted.
    let ticks: u32 = report()
        .get("ticks_after_navigate")
        .parse()
        .unwrap_or_else(|_| panic!("ticks was not a number:\n{}", report().raw()));
    assert!(ticks >= 3, "the bar counted {ticks} times:\n{}", report().raw());
}

#[test]
fn leaving_a_preview_closes_the_stream_it_opened() {
    assert_eq!(
        report().get("closed_streams"),
        "1",
        "the stream outlived the screen that opened it:\n{}",
        report().raw()
    );
}

#[test]
fn the_two_screens_share_the_selected_device() {
    assert_eq!(
        report().get("selected_in_list"),
        report().get("selected_in_preview"),
        "the two screens disagree about what is selected:\n{}",
        report().raw()
    );
}
