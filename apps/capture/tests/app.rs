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
fn the_navigation_bar_is_not_rebuilt_when_the_screen_under_it_changes() {
    // The bar's thread belongs to the layout, and the layout encloses
    // both screens — so it is never stopped and never restarted. Said
    // with a count rather than a stopwatch: a layout that had been
    // released and built again would have started a second thread.
    assert_eq!(
        report().get("layout_starts"),
        "1",
        "the layout was rebuilt when the screen under it changed:\n{}",
        report().raw()
    );
}

#[test]
fn the_navigation_bar_is_driven_by_its_own_thread() {
    let ticks: u32 = report()
        .get("ticks_after_navigate")
        .parse()
        .unwrap_or_else(|_| panic!("ticks was not a number:\n{}", report().raw()));
    assert!(
        ticks >= 1,
        "the bar never counted at all:\n{}",
        report().raw()
    );
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

#[test]
fn the_preview_follows_the_parameter_it_was_given() {
    // I4 of the plan 5 review, and Review Focus 3 with a proof app
    // behind it at last. `/device/Webcam` and `/device/Micro` are one
    // route, so one component rendered twice — the preview showed the
    // device the first render opened.
    assert_eq!(
        report().get("preview_after_param_change"),
        "Micro",
        "the preview kept the first device:\n{}",
        report().raw()
    );
}

#[test]
fn a_parameter_change_closes_what_the_last_one_opened() {
    // Two streams have been closed by now: the one the first visit
    // opened and left, and the one this visit replaced.
    assert_eq!(
        report().get("closed_after_param_change"),
        "2",
        "the stream of the device that was left stayed open:\n{}",
        report().raw()
    );
}
