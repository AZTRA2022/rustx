//! Altlassian Raw — a small video editor over `ffmpeg`.
//!
//! The window is a timeline of real files: `ffprobe` reads each one's
//! duration, size and frame rate, the clip widths are those durations,
//! the preview is one frame `ffmpeg` decoded at the playhead, and
//! "render" hands the whole sequence to one `ffmpeg` process that
//! writes a playable file.
//!
//! `app/page.rsx` is the editor. `src/ffmpeg.rs` is everything that
//! talks to the two programs — and, mostly, the pure functions that
//! build the command lines, which is where the tests are.
//!
//! What this is not: a player. There is no decoding inside the window,
//! so nothing scrubs at thirty frames a second and there is no play
//! button — the preview is a still, drawn again when the playhead, the
//! trim or the colour changes. That is the real smaller thing, and it
//! is said here rather than suggested otherwise by the interface.

pub mod ffmpeg;

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Altlassian Raw")
            .size(1000, 760)
            .description("A small video editor: real files, real ffmpeg renders.")
            .publisher("rustx"),
    )?;

    app.run()
}
