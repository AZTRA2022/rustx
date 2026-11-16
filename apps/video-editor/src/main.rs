//! Altlassian Raw — a small video editor over `ffmpeg`.
//!
//! The window is a timeline of real files: `ffprobe` reads each one's
//! duration, size and frame rate, the clip widths are those durations,
//! the monitor plays the real file with its sound, and "render" hands
//! the whole sequence to one `ffmpeg` process that writes a playable
//! file.
//!
//! `app/page.rsx` is the editor. `src/ffmpeg.rs` is everything that
//! talks to the two programs — and, mostly, the pure functions that
//! build the command lines, which is where the tests are.
//!
//! # Why there are three natives in this file
//!
//! Playing a file is a `<video>` element, and three of the things an
//! editor does to one — `play()`, `currentTime = …`, and reading the
//! keyboard before anything else does — are DOM calls that the page's
//! Rust cannot make. So the element is drawn by Rust and operated by a
//! dozen lines of JavaScript in `assets/index.html`, which talk back
//! through the natives below. Everything those natives do is write a
//! value the page already declared; the editing itself stays in Rust.

pub mod ffmpeg;

use rustx::{App, Config, NativeError, Value, register_native};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Altlassian Raw")
            .size(1180, 720)
            .description("A small video editor: real files, real playback, real ffmpeg renders.")
            .publisher("rustx")
            .on_context_created(transport),
    )?;

    app.run()
}

/// The three functions `assets/index.html` calls.
///
/// Registered here rather than from a page because the page's
/// JavaScript context lives in the renderer process, which never
/// reaches the body of `main` — this hook does.
fn transport() {
    // Where the video actually is, and whether it is moving. Written
    // on the render thread, which is where the page's own state lives,
    // so these are ordinary writes to the values the page declared.
    register_native("editor_time", |arguments| {
        let at = match arguments.first() {
            Some(Value::Number(at)) => *at,
            _ => 0.0,
        };
        let paused = !matches!(arguments.get(1), Some(Value::Bool(false)));
        rustx::store::shared("played", || 0.0_f64).set(at);
        rustx::store::shared("paused", || true).set(paused);
        Ok(Value::Null)
    });

    // A key, by name. What it means is the page's to decide: the
    // bindings need the clips, the trim and the frame rate, and all
    // three are state.
    register_native("editor_key", |arguments| {
        let Some(Value::String(name)) = arguments.first() else {
            return Err(NativeError::new("editor_key takes a key name"));
        };
        rustx::store::shared("pressed", String::new).set(name.clone());
        rustx::store::shared("presses", || 0_u64).update(|count| *count += 1);
        Ok(Value::Null)
    });

    // Whether this build of Chromium could decode the file at all. It
    // can — the CEF binaries here carry H.264 and AAC — but a file the
    // user imports may be anything, and a monitor that stays black
    // with nothing said is the failure this reports.
    register_native("editor_media", |arguments| {
        let ok = matches!(arguments.first(), Some(Value::Bool(true)));
        rustx::store::shared("decodes", || true).set(ok);
        // Chromium's own words when it refuses the file, which is the
        // difference between a black monitor and a sentence a person
        // can act on.
        if let (false, Some(Value::String(said))) = (ok, arguments.get(1)) {
            rustx::store::shared("status", String::new)
                .set(format!("This window could not decode the file ({said})."));
        }
        Ok(Value::Null)
    });
}
