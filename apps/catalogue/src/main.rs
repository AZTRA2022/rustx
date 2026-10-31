//! A `rustx` application, written the way anyone will write one.
//!
//! The interface is `app/page.rsx` and nowhere else.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Catalogue").size(720, 520))?;

    app.run()
}
