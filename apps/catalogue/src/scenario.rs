//! What the test harness drives the page with. An application that is
//! not under test registers none of this.

use std::cell::Cell;

use rustx::{NativeError, Value, register_native};

thread_local! {
    /// Set by the closure written in `page.rsx`.
    static CLICKED: Cell<bool> = const { Cell::new(false) };
}

/// Called from `app/page.rsx`. Plain Rust, in the same process as the
/// click that ran it.
pub fn clicked() {
    CLICKED.with(|clicked| clicked.set(true));
}

/// Registered when the page's JavaScript context appears.
pub fn install() {
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
        .filter_map(|arg| arg.into_string().ok())
        .find_map(|arg| arg.strip_prefix(prefix).map(str::to_string))
}
