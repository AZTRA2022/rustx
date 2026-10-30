//! What the test harness needs from the page, and nothing the
//! application needs.

use rustx::store::store;
use rustx::{NativeError, Value, register_native, render};

use crate::Ticks;

/// Registered when the page's JavaScript context appears.
pub fn install() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("monitor_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    // Taking the panel off the screen is all the scenario does to stop
    // the thread. There is no `stop` to call.
    register_native("monitor_unmount_panel", |_args| {
        Ok(Value::String(render::unmount("panel-host")?.to_string()))
    });

    // The count read from the store rather than from the page, so that
    // "is it still ticking?" can be asked after the panel is gone.
    register_native("monitor_ticks", |_args| {
        Ok(Value::String(store::<Ticks>().get().0.to_string()))
    });

    let report_path = argument("--report=");
    register_native("monitor_report", move |args| {
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
