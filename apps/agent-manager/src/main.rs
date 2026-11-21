//! AI Agent Manager — the showcase's "software for deploy manage your
//! AI agents".
//!
//! A real Anthropic/OpenRouter key and a real Axum backend are not
//! what this demo needs to prove the idea: deploying agents, watching
//! their request volume on a chart, and talking to one, is. The chart
//! is `recharts`, off npm, fed real `let mut` state from Rust. The
//! chat calls `answer_from`, below — a stubbed reply function, not a
//! paid API — honestly, rather than pretending it reached one.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Vantage")
            .size(1280, 820)
            .start("/agent/support-triage")
            .description("Deploy and watch a handful of agents.")
            .publisher("rustx"),
    )?;

    app.run()
}
