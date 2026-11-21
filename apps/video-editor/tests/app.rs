//! The editor driven from outside: each test starts the real application on
//! a scenario (`assets/scenario.js`) and reads what the page reported.

mod common;

use common::scenario;

/// A 3 second clip of the given container, or `None` where ffmpeg is absent.
fn clip(name: &str) -> Option<std::path::PathBuf> {
    let place = std::env::temp_dir().join(format!("rustx-editor-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&place).ok()?;
    let file = place.join(name);
    std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=320x240:rate=25:duration=3",
        ])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=3"])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
        .arg(&file)
        .status()
        .ok()?
        .success()
        .then_some(file)
}

#[test]
fn the_timeline_follows_the_pointer_and_zooms_with_the_wheel() {
    let Some(file) = clip("scrub.mp4") else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let report = scenario(&format!("scrub:{}", file.display()));
    let near = |key: &str, expected: f64| {
        let got: f64 = report.get(key).parse().unwrap_or(-1.0);
        assert!(
            (got - expected).abs() < 0.35,
            "{key} was {got}, expected about {expected}:\n{}",
            report.raw()
        );
    };
    near("at_middle", 1.5);
    near("dragged", 2.4);
    near("after_release", 2.4);
    // Three turns of the wheel are one step of a quarter, zoomed once.
    assert_eq!(report.get("zoom"), "1.25", "{}", report.raw());
    let _ = std::fs::remove_dir_all(file.parent().expect("a folder"));
}

#[test]
fn a_radix_slider_owned_by_rust_moves_the_grade() {
    let Some(file) = clip("grade.mp4") else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let report = scenario(&format!("grade:{}", file.display()));
    assert_eq!(report.get("brightness_start"), "0.00", "{}", report.raw());
    // Four steps of a twentieth.
    assert_eq!(report.get("brightness_after"), "0.20", "{}", report.raw());
    assert_eq!(report.get("thumb_after"), "0.2", "{}", report.raw());
    assert!(
        report.get("monitor_css").contains("brightness(1.2"),
        "{}",
        report.raw()
    );
    let _ = std::fs::remove_dir_all(file.parent().expect("a folder"));
}

#[test]
fn a_switch_acts_on_the_clip_its_row_names_after_the_rows_are_reordered() {
    let (Some(a), Some(b)) = (clip("a-first.mp4"), clip("b-second.mp4")) else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let report = scenario(&format!("layers:{}|{}", a.display(), b.display()));
    assert_eq!(
        report.get("rows_before"),
        "a-first.mp4,b-second.mp4",
        "{}",
        report.raw()
    );
    assert_eq!(
        report.get("rows_after_move"),
        "b-second.mp4,a-first.mp4",
        "{}",
        report.raw()
    );
    assert_eq!(
        report.get("states_after_toggle"),
        "b-second.mp4=checked,a-first.mp4=unchecked",
        "{}",
        report.raw()
    );
    for file in [a, b] {
        let _ = std::fs::remove_dir_all(file.parent().expect("a folder"));
    }
}

#[test]
fn an_imported_clip_decodes_and_plays() {
    let Some(file) = clip("clip one.mp4") else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let report = scenario(&format!("import:{}", file.display()));
    assert_eq!(report.get("clip_name"), "clip one.mp4", "{}", report.raw());
    assert_eq!(report.get("decodes"), "true", "{}", report.raw());
    assert_eq!(report.get("duration"), "3.0", "{}", report.raw());
    assert_eq!(report.get("played"), "true", "{}", report.raw());
    let _ = std::fs::remove_dir_all(file.parent().expect("a folder"));
}

#[test]
fn a_burst_of_slider_steps_is_not_collapsed_by_the_round_trip() {
    let Some(file) = clip("burst.mp4") else {
        eprintln!("ffmpeg is not installed: skipped");
        return;
    };
    let report = scenario(&format!("grade_burst:{}", file.display()));
    assert_eq!(report.get("burst_brightness"), "0.20", "{}", report.raw());
    let _ = std::fs::remove_dir_all(file.parent().expect("a folder"));
}
