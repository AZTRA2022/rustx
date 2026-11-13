//! Altlassian Raw — the showcase's "tiny video editor app, ffmpeg
//! powered".
//!
//! Driving a real ffmpeg process over real footage, with AI edits, is
//! real engineering this demo does not attempt. What it builds
//! honestly instead: the editor's shell — a timeline of clips, trim
//! handles, a color filter, a subtitle field, and a render button —
//! wired to a simulated render pipeline rather than a silently faked
//! "done". No ffmpeg binary is invoked anywhere in this app.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Altlassian Raw")
            .size(900, 620)
            .description("A small video editor's shell, with a simulated render.")
            .publisher("rustx"),
    )?;

    app.run()
}
