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

mod devices;

// What the pages under `app/` see. A `.rsx` has no import line of its
// own yet, so the crate's own scope is a page's vocabulary.
use devices::Stream;
use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Console").size(800, 600))?;

    app.run()
}
