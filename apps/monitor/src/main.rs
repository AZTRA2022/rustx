//! A monitor fed by a background thread.
//!
//! `app/page.rsx` holds this and nothing else:
//!
//! ```ignore
//! fn Panel() -> Element {
//!     let ticks = store::<Ticks>();
//!
//!     task(move |alive| {
//!         while alive.get() {
//!             std::thread::sleep(std::time::Duration::from_millis(50));
//!             let _ = alive.post(|| store::<Ticks>().update(|ticks| ticks.0 += 1));
//!         }
//!     });
//!
//!     return <p id="panel">{ticks.get().0}</p>;
//! }
//! ```
//!
//! That is the shape of the proof-of-concept software's capture panel,
//! without the hardware: a thread that never stops on its own, a value
//! two distant components read, and no cleanup written anywhere. There
//! is no `use_effect`, no dependency array, and no `move` written by
//! hand around the state. The task is a value the component keeps, so it
//! is dropped when the component leaves, so the thread stops.


// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]

mod scenario;

// What the page under `app/` sees. A `.rsx` has no import line of its
// own yet, so the crate's own scope is a page's vocabulary.
use rustx::store::store;
use rustx::{App, Config, Element};

/// What the thread produces. A store is found by its type, so the type
/// is the name of the application's one tick count.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ticks(pub u32);

#[rustx::main(pages)]
fn main() -> Result<(), rustx::Error> {
    // Two components that are not each other's parent, each reading the
    // same store. Nothing is handed down.
    let app = App::new(
        Config::new("Monitor")
            .size(640, 480)
            .mount("header-host", Header)
            .mount("panel-host", Panel)
            .on_context_created(scenario::install),
    )?;

    app.run()
}
