//! The editor driven from outside: each test starts the real application on
//! a scenario (`assets/scenario.js`) and reads what the page reported.

mod common;

use common::scenario;

/// A 3 second clip of the given container, or `None` where ffmpeg is absent.
fn clip(name: &str) -> Option<std::path::PathBuf> {
    let place = std::env::temp_dir().join(format!("rustx-editor-{}", std::process::id()));
    std::fs::create_dir_all(&place).ok()?;
    let file = place.join(name);
    std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25:duration=3"])
        .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=3"])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
        .arg(&file)
        .status()
        .ok()?
        .success()
        .then_some(file)
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
