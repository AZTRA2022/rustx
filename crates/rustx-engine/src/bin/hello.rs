//! Smoke-test application for the engine: opens a window on a local page.

use std::path::Path;

use rustx_engine::{App, AppConfig};

fn main() {
    // Temporary guard: CEF relaunches this executable for each of its
    // sub-processes, and without this they would each open a window of their
    // own. Task 2 replaces it with the tested `rustx_engine::bootstrap()`.
    rustx_engine::declare_api_version();
    let child = cef::execute_process(Some(cef::args::Args::new().as_main_args()), None, std::ptr::null_mut());
    if child >= 0 {
        std::process::exit(child);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/hello/index.html");

    let app = App::new(AppConfig {
        title: "rustx - hello".into(),
        width: 960,
        height: 640,
        index,
    })
    .expect("the bundled example page should be present");

    println!("WINDOW_OPENED");
    app.run().expect("the engine should start");
}
