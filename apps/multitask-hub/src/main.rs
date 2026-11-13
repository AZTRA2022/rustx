//! MultiTask Hub — the showcase's "Tracks & Solve orchestration
//! problems".
//!
//! Git2, SeaORM and a Supabase backend are not what makes this idea
//! real to look at: a dashboard across several concurrent projects,
//! each with its own tasks and its own blocked work, is. That is what
//! this app is — several projects, their tasks, and one roll-up that
//! shows where the whole set is stuck — with no backend behind it.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("MultiTask Hub")
            .size(900, 640)
            .description("A dashboard across several projects at once.")
            .publisher("rustx"),
    )?;

    app.run()
}
