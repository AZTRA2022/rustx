//! R Intelligent Data — the showcase's "full GUI for Databases
//! Management".
//!
//! A real Postgres/Kubernetes client is weeks of plumbing this demo
//! does not need to prove its point. What is real here: tables held
//! in memory, browsed in a GUI, and a real CSV file — dropped on the
//! window with its real path via `rustx::dropped`, or chosen with the
//! system's own file dialog via `rustx::dialog` — turned into a new
//! table. Drag&drop onto the window is the one piece of the original
//! "Drag&Drop" tag this app can do for real; reordering columns by
//! drag is left undone.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("R Intelligent Data")
            .size(960, 680)
            .description("A GUI for the tables you hand it.")
            .publisher("rustx"),
    )?;

    app.run()
}
