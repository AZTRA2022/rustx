//! Two screens, a layout that outlives both, and a stream that closes
//! itself.
//!
//! The whole application is `app/`:
//!
//! ```text
//! app/layout.rsx            the bar above every screen
//! app/page.rsx              the device list,            at /
//! app/device/[id]/page.rsx  one device's preview,       at /device/[id]
//! ```
//!
//! Nothing here mounts a screen and nothing unmounts one. Opening a
//! device is `navigate`; going back is `back`. The preview's stream is a
//! kept value, so leaving the screen closes it, and the bar's thread
//! belongs to the layout, so it keeps running while the screens under it
//! change.

// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated pages, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]

mod devices;
mod scenario;

// What the pages under `app/` see. A `.rsx` has no import line of its
// own yet, so the crate's own scope is a page's vocabulary.
use devices::{Selected, Stream, Ticks, open, started};
use rustx::state::with_key;
use rustx::store::store;
use rustx::{App, Config, Element};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Console")
            .size(800, 600)
            .on_context_created(scenario::install),
    )?;

    app.run()
}
