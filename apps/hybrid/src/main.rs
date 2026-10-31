//! A page where Rust and JavaScript own one DOM between them.
//!
//! Both rules of ownership are in `app/page.rsx`: `<js::Chart />` is a
//! node JavaScript owns, and `#panel-host` is a node JavaScript fills by
//! asking rustx for `Counter`. One `let mut points = 0;` at the top of
//! the page feeds both, so a frame that draws one draws the other.

#![allow(non_snake_case)]
#![allow(clippy::needless_return, clippy::useless_vec)]

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Hybrid").size(900, 640))?;

    app.run()
}
