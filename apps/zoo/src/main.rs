//! Many libraries, one page.
//!
//! This application exists to find out what the boundary cannot do yet.
//! Every library here is a different shape of the same problem: icons
//! are components with props, a markdown renderer takes its content as
//! children, and tabs keep state of their own inside React while rustx
//! keeps state of its own around them.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Zoo").size(900, 700))?;

    app.run()
}
