//! The counter of the design document, on screen.
//!
//! `app/page.rsx` holds this and nothing else:
//!
//! ```ignore
//! fn Counter() -> Element {
//!     let mut n = 0;
//!     return <button onClick={|| n += 1}>{n}</button>;
//! }
//! ```
//!
//! No `use_state`, no `signal`, no `move`, no `Rc<RefCell<…>>`, and no
//! conversion around `{n}`. If that block ever needs one more
//! annotation to work, the framework has failed at the thing it exists
//! for, and the fix belongs in the compiler rather than in this page.

// Components are PascalCase, as they are in TSX.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote.
#![allow(clippy::needless_return, clippy::useless_vec)]



mod rows;
mod scenario;

// What the page under `app/` sees. A `.rsx` has no import line of its
// own yet, so the crate's own scope is a page's vocabulary.

use rustx::{App, Config, Element};

#[rustx::main(pages)]
fn main() -> Result<(), rustx::Error> {
    // Two counters side by side, from one `let mut n = 0;`, and a list
    // whose rows keep their own counts across a reorder.
    let app = App::new(
        Config::new("Counter")
            .size(640, 480)
            .mount("first", Counter)
            .mount("second", Counter)
            .mount("rows", rows::rows)
            .on_context_created(scenario::install),
    )?;

    app.run()
}
