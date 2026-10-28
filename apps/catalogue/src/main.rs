//! A `rustx` application, written the way anyone will write one.
//!
//! The interface is in `app/page.rsx` and nowhere else. This file is the
//! wiring a later plan's CLI will generate: open a window, and mount the
//! page into it.

// Components are PascalCase, as they are in TSX: `Page`, not `page`. That
// is the naming the framework asks for, so the lint that argues with it is
// off for the whole application.
#![allow(non_snake_case)]
// And clippy reads the generated page, not the `.rsx`: its advice names
// lines in OUT_DIR that nobody wrote. Until the compiler can carry
// attributes through, the two lints the generated shape always trips are
// off here.
#![allow(clippy::needless_return, clippy::useless_vec)]

use std::cell::Cell;
use std::path::Path;

use rustx_dom::Element;
use rustx_engine::{
    App, AppConfig, Bootstrap, Value, bootstrap, mount_at_id, on_context_created, on_frame,
    register_native,
};

// The page, compiled from `app/page.rsx` by `build.rs`.
include!(concat!(env!("OUT_DIR"), "/page.rs"));

fn main() {
    on_context_created(install);

    if let Bootstrap::Child { exit_code } = bootstrap() {
        std::process::exit(exit_code);
    }

    let index = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/index.html");
    let app = App::new(AppConfig {
        title: "Catalogue".into(),
        width: 720,
        height: 520,
        index,
    })
    .expect("the application's index page should be present");

    app.run().expect("the engine should start");
}

thread_local! {
    /// Set by the closure written in `page.rsx`.
    static CLICKED: Cell<bool> = const { Cell::new(false) };
}

/// Called from `app/page.rsx`. Plain Rust, in the same process as the
/// click that ran it.
fn clicked() {
    CLICKED.with(|clicked| clicked.set(true));
}

fn install() {
    // Mount once, on the first frame where the document has been parsed.
    // A V8 context exists before the body does, so mounting from
    // `on_context_created` would find no `#root`.
    let mounted = Cell::new(false);
    on_frame(move || {
        if mounted.get() {
            return;
        }
        if mount_at_id(Page(), "root").is_ok() {
            mounted.set(true);
        }
    });

    install_test_natives();
}

/// What the test harness drives the page with. An application that is not
/// under test registers none of this.
fn install_test_natives() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("catalogue_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    register_native("catalogue_clicked", |_args| {
        Ok(Value::String(
            if CLICKED.with(Cell::get) { "yes" } else { "no" }.to_string(),
        ))
    });

    let report_path = argument("--report=");
    register_native("catalogue_report", move |args| {
        let [Value::String(key), Value::String(value)] = args else {
            return Err(rustx_engine::NativeError::new("expected 2 string arguments"));
        };
        let Some(path) = report_path.as_deref() else {
            return Err(rustx_engine::NativeError::new("no --report=<path> was given"));
        };
        let line = format!("REPORT {key}={value}\n");
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
            .map_err(|e| rustx_engine::NativeError::new(format!("could not report: {e}")))?;
        Ok(Value::Null)
    });
}

fn argument(prefix: &str) -> Option<String> {
    std::env::args_os()
        .filter_map(|arg| arg.into_string().ok())
        .find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
}
