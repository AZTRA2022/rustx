//! The editor driven from outside: the real application on `--scenario=nemo`
//! (`assets/scenario.js`), and what the page reported.

mod common;

use common::scenario;

#[test]
fn the_graph_the_tabs_the_history_the_timeline_and_the_export_work() {
    let report = scenario("nemo");
    let get = |key: &str| report.get(key);
    assert_eq!(get("tabs"), "3", "{}", report.raw());
    assert_eq!(get("cards"), "4", "{}", report.raw());
    assert_eq!(get("tabs_after_new"), "4", "{}", report.raw());
    // An idea that names a model of the library changes it, and Undo and Redo walk back and forth.
    assert_eq!(get("fox_after_idea"), "true", "{}", report.raw());
    assert_eq!(get("robot_after_undo"), "true", "{}", report.raw());
    assert_eq!(get("fox_after_redo"), "true", "{}", report.raw());
    // A card moves, a wire goes, a setting is added.
    assert_eq!(get("card_moved"), "true", "{}", report.raw());
    assert_eq!(get("wires"), "2", "{}", report.raw());
    assert_eq!(get("wires_after_remove"), "1", "{}", report.raw());
    assert_eq!(get("light_row"), "true", "{}", report.raw());
    // Play moves the playhead.
    assert_eq!(get("playing"), "true", "{}", report.raw());
    assert_eq!(get("playhead"), "true", "{}", report.raw());
    // The export is recorded for real where the web view can record, and written through the native.
    if !report.raw().contains("no MediaRecorder") {
        assert_eq!(get("export_ext"), "webm", "{}", report.raw());
        assert!(
            get("export_bytes").parse::<u64>().unwrap_or(0) > 10_000,
            "{}",
            report.raw()
        );
    }
    assert_eq!(get("done"), "yes", "{}", report.raw());
}
