//! The counter of the design document, written in `.rsx`.
//!
//! `app/page.rsx` is the landing page. `app/counter/page.rsx` is the
//! counter itself, at `/counter`: one `let mut n = 0;`, two counters
//! side by side from that one line, and a keyed list whose rows keep
//! their own counts across a reorder.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Counter")
            .size(900, 640)
            .frameless()
            .updates("owner/counter"),
    )?;

    app.run()
}
