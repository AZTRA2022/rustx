//! Nemo Design — the showcase's "3D Agents powered App".
//!
//! A Blender-grade modeler, and AI agents that edit a mesh for you,
//! are not within a demo's reach. What is real here: `components/
//! Scene.tsx` is a genuine `three.js` viewport — a WebGL renderer, a
//! mesh, a light, an animation loop — driven by props this page owns,
//! and a stubbed "ask an agent for a palette" button stands in
//! honestly for the "AI agents performing designing operations" the
//! showcase copy describes.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Nemo Design")
            .size(900, 700)
            .description("A small 3D viewport, with real three.js underneath.")
            .publisher("rustx"),
    )?;

    app.run()
}
