//! The proof of plan 2: a page written in `.rsx`, on screen.

mod common;

use std::sync::OnceLock;

use common::{Report, run_catalogue_scenario};

fn report() -> &'static Report {
    static REPORT: OnceLock<Report> = OnceLock::new();
    REPORT.get_or_init(run_catalogue_scenario)
}

#[test]
fn the_page_written_in_rsx_is_on_screen() {
    assert_eq!(
        report().get("html"),
        "<h1>Catalogue</h1><ul class=\"devices\"><li>Webcam</li><li>Micro</li><li>Écran</li></ul><button id=\"shout\">3</button>",
        "full output:\n{}",
        report().raw()
    );
}

#[test]
fn clicking_the_button_runs_the_rust_written_in_the_rsx() {
    assert_eq!(
        report().get("clicked"),
        "true",
        "the closure written in the .rsx never ran:\n{}",
        report().raw()
    );
}
