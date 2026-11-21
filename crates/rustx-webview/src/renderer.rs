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

/// The future an asynchronous native returns. It runs on the async runtime,
/// never on the window's thread, so it must be `Send`.
pub type NativeFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, NativeError>> + Send>>;

type AsyncNativeFn = dyn Fn(&[Value]) -> NativeFuture;

/// The prefix the engine keeps for itself.
pub const RESERVED: &str = "__rustx_";

thread_local! {
    static HOOKS: RefCell<Vec<Box<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
    static NATIVES: RefCell<HashMap<String, Rc<NativeFn>>> = RefCell::new(HashMap::new());
    static ASYNC_NATIVES: RefCell<HashMap<String, Rc<AsyncNativeFn>>> = RefCell::new(HashMap::new());
    static SCRIPTS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static RAN: Cell<bool> = const { Cell::new(false) };
    /// When set, the only natives a page may call (besides the engine's own).
    static CAPABILITIES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Limit the page to these natives, like a Tauri capability file: a native
/// not named here is refused even if the application registered it. The
/// engine's own (`rustx_…`) always work, because the router needs them.
pub fn restrict_natives(allowed: &[&str]) {
    CAPABILITIES.with(|held| {
        *held.borrow_mut() = Some(allowed.iter().map(|name| name.to_string()).collect());
    });
}

fn refused(name: &str) -> Option<String> {
    CAPABILITIES.with(|held| match held.borrow().as_ref() {
        Some(allowed) if !name.starts_with("rustx_") && !allowed.iter().any(|a| a == name) => Some(
            format!("`{name}` is not in this application's capabilities"),
        ),
        _ => None,
    })
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

/// Make a Rust function callable from the page under `name`, answering with
/// a promise. The closure runs at once on the window's thread and must only
/// start the work: the future it returns runs on the async runtime, so a long
/// one leaves the page and the window free, as a Server Action does.
pub fn register_async_native<F, Fut>(name: &str, function: F)
where
    F: Fn(&[Value]) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<Value, NativeError>> + Send + 'static,
{
    if name.starts_with(RESERVED) {
        eprintln!(
            "rustx: `{name}` was not registered. Names beginning with `{RESERVED}` belong to the engine."
        );
        return;
    }
    ASYNC_NATIVES.with(|natives| {
        natives.borrow_mut().insert(
            name.to_string(),
            Rc::new(move |args| Box::pin(function(args))),
        )
    });
}

/// Start the asynchronous native `name` for call `id`: the answer is sent to
/// the page as a script once the future is done.
pub(crate) fn call_async(id: u64, name: &str, args: &[Value]) {
    if let Some(why) = refused(name) {
        return settle(id, Err(why));
    }
    let started = ASYNC_NATIVES
        .with(|natives| natives.borrow().get(name).cloned())
        .ok_or_else(|| {
            format!("`{name}` is not an asynchronous function the application registered")
        });
    let mut future = None;
    let started = started.and_then(|native| {
        crate::panics::contain(&format!("the native `{name}`"), || {
            future = Some(native(args))
        });
        future.take().ok_or_else(|| format!("`{name}` panicked"))
    });
    let work = match started {
        Ok(work) => work,
        Err(message) => return settle(id, Err(message)),
    };
    let runtime = crate::runtime::handle();
    runtime.clone().spawn(async move {
        // A panic in the future ends its task, not the worker: it is the
        // page's rejection.
        let outcome = match runtime.spawn(work).await {
            Ok(outcome) => outcome.map_err(|error| error.message),
            Err(_) => Err("the native panicked".to_string()),
        };
        settle(id, outcome);
    });
}

/// Tell the page how call `id` ended.
fn settle(id: u64, outcome: Result<Value, String>) {
    let (ok, value) = match outcome
        .and_then(|value| crate::protocol::value_to_json(&value, 0).map_err(|e| e.to_string()))
    {
        Ok(value) => (true, value),
        Err(message) => (false, serde_json::json!(message)),
    };
    crate::host::script(format!("window.__rustx.settle({id},{ok},{value})"));
}

/// Run the hooks, once. The hooks themselves are let go of before any is
/// called, so one may register another.
pub(crate) fn run_hooks() {
    if RAN.with(|ran| ran.replace(true)) {
        return;
    }
    install_engine_natives();
    // A page left to itself navigates to the file dropped on it. The paths
    // come from the host (see `dropped`); the page keeps its own `ondrop`.
    inject(
        "rustx-dropped",
        "for (const kind of ['dragover', 'drop']) window.addEventListener(kind, (event) => event.preventDefault());",
    );
    let hooks = HOOKS.with(|hooks| std::mem::take(&mut *hooks.borrow_mut()));
    for hook in &hooks {
        crate::panics::contain("a hook the application registered", hook);
    }
}

/// What the page may always ask of the framework: the scenario harness and
/// the router. `rustx_component` and `rustx_release` come with the React
/// boundary.
fn install_engine_natives() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("rustx_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

    // Written only where `--report=<path>` says.
    let report = argument("--report=");
    register_native("rustx_report", move |args| {
        let [Value::String(key), Value::String(value)] = args else {
            return Err(NativeError::new(
                "rustx_report takes a name and a value, both strings",
            ));
        };
        let Some(path) = report.as_deref() else {
            return Err(NativeError::new(
                "this application was not started with --report=<path>",
            ));
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

    // The other direction of the React boundary: the page asks for a
    // component of the application in an element React made, and lets it go.
    register_native("rustx_component", |args| {
        let [Value::String(name), Value::String(id)] = args else {
            return Err(NativeError::new(
                "rustx_component takes a component name and the id of the element it goes in",
            ));
        };
        crate::foreign::mount_registered(name, id)?;
        Ok(Value::Null)
    });
    register_native("rustx_release", |args| {
        let [Value::String(id)] = args else {
            return Err(NativeError::new(
                "rustx_release takes the id of the element to release",
            ));
        };
        Ok(Value::Bool(crate::render::unmount(id)?))
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
    SCRIPTS.with(|scripts| {
        scripts
            .borrow()
            .iter()
            .map(|(_, text)| text.clone())
            .collect()
    })
}

/// The script that gives the page a function for each registered native.
pub fn natives_script() -> String {
    let mut names: Vec<String> = NATIVES.with(|natives| natives.borrow().keys().cloned().collect());
    names.sort();
    let mut asynchronous: Vec<String> =
        ASYNC_NATIVES.with(|natives| natives.borrow().keys().cloned().collect());
    asynchronous.sort();
    format!(
        "window.__rustx && window.__rustx.natives({}, {});",
        serde_json::json!(names),
        serde_json::json!(asynchronous)
    )
}

/// Call the native `name`. A panic is an error the page can read, not a
/// dead window.
pub(crate) fn call(name: &str, args: &[Value]) -> Result<Value, String> {
    if let Some(why) = refused(name) {
        return Err(why);
    }
    let Some(native) = NATIVES.with(|natives| natives.borrow().get(name).cloned()) else {
        return Err(format!(
            "`{name}` is not a function the application registered"
        ));
    };
    let mut outcome = Err(format!("`{name}` panicked"));
    crate::panics::contain(&format!("the native `{name}`"), || {
        outcome = native(args).map_err(|error| error.message);
    });
    outcome
}

#[cfg(test)]
mod capability_tests {
    use super::*;

    #[test]
    fn a_native_outside_the_capabilities_is_refused_and_the_engines_own_still_work() {
        register_native("allowed_one", |_| Ok(Value::Null));
        register_native("forbidden_one", |_| Ok(Value::Null));
        assert!(call("forbidden_one", &[]).is_ok(), "no limit by default");
        restrict_natives(&["allowed_one"]);
        assert!(call("allowed_one", &[]).is_ok());
        let why = call("forbidden_one", &[]).unwrap_err();
        assert!(
            why.contains("not in this application's capabilities"),
            "{why}"
        );
        // The router's own door is never closed.
        assert_eq!(refused("rustx_component"), None);
    }
}
