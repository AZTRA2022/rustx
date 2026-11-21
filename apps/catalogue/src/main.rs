//! A `rustx` application, written the way anyone will write one.
//!
//! The interface is `app/page.rsx` and nowhere else.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Catalogue").size(720, 520))?;

    app.run()
}
