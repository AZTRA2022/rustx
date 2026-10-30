//! What the test harness needs from the page, and nothing the
//! application needs.
//!
//! A scenario is scaffolding: it lives in its own file so that reading
//! `main.rs` tells you what the application is, not how it is tested.

use rustx::{NativeError, Value, foreign, register_native, store::store};

use crate::chart::{Points, counter};

/// Registered when the page's JavaScript context appears.
pub fn install() {
    // What JavaScript is allowed to mount, which is the application's
    // decision and nobody else's.
    foreign::register_component("Counter", counter);

    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("hybrid_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    register_native("hybrid_points", |_args| {
        Ok(Value::String(store::<Points>().get().0.to_string()))
    });

    let report_path = argument("--report=");
    register_native("hybrid_report", move |args| {
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
            .map_err(|e| NativeError::new(format!("could not report: {e}")))?;
        Ok(Value::Null)
    });
}

fn argument(prefix: &str) -> Option<String> {
    std::env::args_os()
        .filter_map(|argument| argument.into_string().ok())
        .find_map(|argument| argument.strip_prefix(prefix).map(|value| value.to_string()))
}
