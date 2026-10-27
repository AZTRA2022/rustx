//! Smoke-test application for the engine: opens a window on a local page.

use std::path::Path;

use rustx_engine::{App, AppConfig, Bootstrap, bootstrap};

fn main() {
    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/hello/index.html");

    let app = App::new(AppConfig {
        title: "rustx - hello".into(),
        width: 960,
        height: 640,
        index,
    })
    .expect("the bundled example page should be present");

    println!("WINDOW_OPENED args={:?}", std::env::args().collect::<Vec<_>>());
    app.run().expect("the engine should start");
}
