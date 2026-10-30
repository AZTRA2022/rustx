//! What the test harness needs from the page, and nothing the
//! application needs.

use rustx::{NativeError, Value, register_native};

/// Registered when the page's JavaScript context appears.
pub fn install() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("counter_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    register_native("counter_rotate", |_args| {
        crate::rows::rotate();
        Ok(Value::Null)
    });

    let report_path = argument("--report=");
    register_native("counter_report", move |args| {
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
