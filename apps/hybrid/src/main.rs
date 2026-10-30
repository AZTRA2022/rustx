//! A page where Rust and JavaScript own one DOM between them.

#![allow(non_snake_case)]
#![allow(clippy::needless_return, clippy::useless_vec)]

mod chart;
mod scenario;

// What the pages under `app/` see. A `.rsx` has no import line of its
// own yet, so the crate's own scope is a page's vocabulary.
use chart::{Points, add_point, leave};
use rustx::store::store;
use rustx::{App, Config, Element};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Hybrid")
            .size(900, 640)
            .on_context_created(scenario::install),
    )?;

    app.run()
}
