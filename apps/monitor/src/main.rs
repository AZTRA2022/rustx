//! A monitor fed by a background thread.
//!
//! `app/page.rsx` holds it all: `let mut ticks = 0;` at the top of the
//! page is one value two components read, and the thread that drives it
//! is a `task` the panel keeps — so leaving the page stops it, with no
//! cleanup written anywhere.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Monitor").size(640, 480))?;

    app.run()
}
