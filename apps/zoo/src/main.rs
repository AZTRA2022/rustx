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
    let app = App::new(
        Config::new("Zoo")
            .size(900, 700)
            // The window's logo, and what this application says it is.
            // `assets/icon.ico` would have been found without the line;
            // it is written here because an application that says it is
            // easier to read than one that does not.
            .icon("assets/icon.ico")
            .version(env!("CARGO_PKG_VERSION"))
            .description("Many JavaScript libraries, one page.")
            .publisher("rustx"),
    )?;

    app.run()
}
