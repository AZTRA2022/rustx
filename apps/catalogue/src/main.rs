//! A `rustx` application, written the way anyone will write one.
//!
//! The interface is in `app/page.rsx` and nowhere else. What is left
//! here is a window's name and its size.

// Components are PascalCase, as they are in TSX: `Page`, not `page`.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]

mod scenario;

// What the page under `app/` sees. A `.rsx` has no import line of its
// own yet, so the crate's own scope is a page's vocabulary.
use rustx::{App, Config, Element};
use scenario::clicked;

#[rustx::main(pages)]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Catalogue")
            .size(720, 520)
            .mount("root", Page)
            .on_context_created(scenario::install),
    )?;

    app.run()
}
