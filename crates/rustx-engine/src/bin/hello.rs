//! Smoke-test application for the engine.
//!
//! It opens a window on a local page whose buttons call Rust directly. Given
//! `--scenario=<name>`, the page drives itself and reports what it saw, which
//! is how the test suite watches the bridge work.

use std::cell::Cell;
use std::path::Path;

use rustx_engine::{
    App, AppConfig, Bootstrap, NativeError, Value, bootstrap, on_context_created, register_native,
};

fn main() {
    // Registered before bootstrap: the render process is this same binary and
    // never runs past that line, so this is its only chance to be told what
    // the page may call.
    on_context_created(install_natives);

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

fn install_natives() {
    // Plain Rust state, owned by the render process, with no cell of
    // framework machinery around it.
    let counter = Cell::new(0_i64);
    register_native("rustx_count", move |_args| {
        counter.set(counter.get() + 1);
        Ok(Value::Number(counter.get() as f64))
    });

    register_native("rustx_panic", |_args| -> Result<Value, NativeError> {
        panic!("this native function panics on purpose");
    });

    register_native("rustx_needs_one", |args| {
        if args.len() != 1 {
            return Err(NativeError::new(format!(
                "expected 1 argument, got {}",
                args.len()
            )));
        }
        Ok(Value::Null)
    });

    register_native("rustx_echo", |args| {
        let parts: Vec<String> = args
            .iter()
            .map(|value| match value {
                Value::Null => "null".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => format!("{n}"),
                Value::String(s) => s.clone(),
            })
            .collect();
        Ok(Value::String(parts.join("/")))
    });

    // A CEF sub-process does not reliably inherit the parent's stdout, so
    // the scenario reports through a file the test harness names.
    let report_path = argument("--report=");
    register_native("rustx_report", move |args| {
        let [Value::String(key), Value::String(value)] = args else {
            return Err(NativeError::new("expected 2 string arguments"));
        };
        let Some(path) = report_path.as_deref() else {
            return Err(NativeError::new("no --report=<path> was given"));
        };
        let line = format!("REPORT {key}={value}\n");
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
            .map_err(|e| NativeError::new(format!("could not write the report: {e}")))?;
        Ok(Value::Null)
    });

    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("rustx_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });
}

/// The value of a `--name=value` argument this process was launched with.
fn argument(prefix: &str) -> Option<String> {
    std::env::args().find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
}
