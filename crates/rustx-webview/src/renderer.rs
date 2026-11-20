//! What the application registers before its window opens.
//!
//! On the CEF engine this ran when a page's JavaScript context was
//! created, in the render process. Here the application's code and the
//! window are in the same process, so a hook runs once, on the thread that
//! will run the loop, just before the webview is built: that is early
//! enough for `inject`, whose scripts must be in the page before its own.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::{NativeError, Value};

/// The shape of a function the page can call.
pub type NativeFn = dyn Fn(&[Value]) -> Result<Value, NativeError>;

/// The prefix the engine keeps for itself.
pub const RESERVED: &str = "__rustx_";

thread_local! {
    static HOOKS: RefCell<Vec<Box<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
    static NATIVES: RefCell<HashMap<String, Rc<NativeFn>>> = RefCell::new(HashMap::new());
    static SCRIPTS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static RAN: Cell<bool> = const { Cell::new(false) };
}

/// Put `source` in every page, before the page's own scripts run. A name
/// already used is not put twice.
pub fn inject(name: &str, source: &str) {
    SCRIPTS.with(|scripts| {
        let mut scripts = scripts.borrow_mut();
        if scripts.iter().any(|(each, _)| each == name) {
            return;
        }
        scripts.push((name.to_string(), source.to_string()));
    });
}

/// Register work to do when the window is about to open: this is where an
/// application calls [`register_native`] and [`inject`].
///
/// Hooks run once per process, in the order they were registered. One
/// registered after the window has opened never runs.
pub fn on_context_created(hook: impl Fn() + 'static) {
    HOOKS.with(|hooks| hooks.borrow_mut().push(Box::new(hook)));
}

/// Make a Rust function callable from the page under `name`.
pub fn register_native(
    name: &str,
    function: impl Fn(&[Value]) -> Result<Value, NativeError> + 'static,
) {
    if name.starts_with(RESERVED) {
        eprintln!(
            "rustx: `{name}` was not registered. Names beginning with `{RESERVED}` belong to \
             the engine."
        );
        return;
    }
    NATIVES.with(|natives| {
        natives
            .borrow_mut()
            .insert(name.to_string(), Rc::new(function))
    });
}

/// Run the hooks, once. The hooks themselves are let go of before any is
/// called, so one may register another.
pub(crate) fn run_hooks() {
    if RAN.with(|ran| ran.replace(true)) {
        return;
    }
    install_engine_natives();
    let hooks = HOOKS.with(|hooks| std::mem::take(&mut *hooks.borrow_mut()));
    for hook in &hooks {
        crate::panics::contain("a hook the application registered", || hook());
    }
}

/// What the page may always ask of the framework: the scenario harness and
/// the router. `rustx_component` and `rustx_release` come with the React
/// boundary.
fn install_engine_natives() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("rustx_scenario", move |_args| Ok(Value::String(scenario.clone())));

    // Written only where `--report=<path>` says.
    let report = argument("--report=");
    register_native("rustx_report", move |args| {
        let [Value::String(key), Value::String(value)] = args else {
            return Err(NativeError::new("rustx_report takes a name and a value, both strings"));
        };
        let Some(path) = report.as_deref() else {
            return Err(NativeError::new("this application was not started with --report=<path>"));
        };
        let line = format!("REPORT {key}={value}\n");
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
            .map_err(|error| NativeError::new(format!("could not report: {error}")))?;
        Ok(Value::Null)
    });

    register_native("rustx_navigate", |args| {
        let [Value::String(path)] = args else {
            return Err(NativeError::new("rustx_navigate takes a path"));
        };
        crate::router::navigate(path.clone());
        Ok(Value::Null)
    });
}

fn argument(prefix: &str) -> Option<String> {
    std::env::args_os()
        .filter_map(|argument| argument.into_string().ok())
        .find_map(|argument| argument.strip_prefix(prefix).map(str::to_string))
}

/// The scripts to put in the page, in the order they were injected.
pub(crate) fn scripts() -> Vec<String> {
    SCRIPTS.with(|scripts| scripts.borrow().iter().map(|(_, text)| text.clone()).collect())
}

/// The script that gives the page a function for each registered native.
pub fn natives_script() -> String {
    let mut names: Vec<String> = NATIVES.with(|natives| natives.borrow().keys().cloned().collect());
    names.sort();
    format!("window.__rustx.natives({});", serde_json::json!(names))
}

pub(crate) fn lookup(name: &str) -> Option<Rc<NativeFn>> {
    NATIVES.with(|natives| natives.borrow().get(name).cloned())
}
