//! Rust functions callable from the page, in the render process.
//!
//! This is where the framework's claim stops being architecture and starts
//! being code a developer touches. A function registered here is installed on
//! the JavaScript global of the render process, and calling it from an
//! `onClick` is a call, not a message: same process, same thread, same stack.
//!
//! Registration happens before [`crate::bootstrap`], because the render
//! process is this same binary and never reaches the application's `main`
//! beyond that point.

mod native;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cef::{
    CefString, Frame, ImplFrame, ImplV8Context, ImplV8Value, V8Context, V8Propertyattribute,
};

use crate::value::{NativeError, Value};

/// The shape of a function the page can call.
pub type NativeFn = dyn Fn(&[Value]) -> Result<Value, NativeError>;

thread_local! {
    /// What the application asked to run when a page's context appears.
    static HOOKS: RefCell<Vec<Box<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
    /// The functions to install on the JavaScript global.
    static NATIVES: RefCell<HashMap<String, Rc<NativeFn>>> = RefCell::new(HashMap::new());
    /// Hooks run once per process, not once per page, so that state a
    /// closure captures survives a navigation.
    static HOOKS_RUN: RefCell<bool> = const { RefCell::new(false) };
    /// JavaScript the framework puts in every page, by name.
    static SCRIPTS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// Put `source` in every page, before the page's own scripts run.
///
/// Evaluated on the context rather than scheduled through the frame: a
/// script the page's own `<script>` depends on has to be there when that
/// script runs, and `execute_java_script` only queues it. What it must
/// not do is register a `requestAnimationFrame` — one registered from
/// inside `on_context_created` is dropped, which is why the frame loop
/// goes in the other way.
pub fn inject(name: &str, source: &str) {
    SCRIPTS.with(|scripts| {
        let mut scripts = scripts.borrow_mut();
        if scripts.iter().any(|(each, _)| each == name) {
            return;
        }
        scripts.push((name.to_string(), source.to_string()));
    });
}

/// Register work to do when a page's JavaScript context is created.
///
/// Call it before [`crate::bootstrap`]; this is where an application calls
/// [`register_native`].
pub fn on_context_created(hook: impl Fn() + 'static) {
    HOOKS.with(|hooks| hooks.borrow_mut().push(Box::new(hook)));
}

/// Make a Rust function callable from JavaScript under `name`.
///
/// The closure is neither `Send` nor `Sync`: it runs on the render process's
/// own thread, which is where V8 lives and where the engine keeps all state.
pub fn register_native(
    name: &str,
    function: impl Fn(&[Value]) -> Result<Value, NativeError> + 'static,
) {
    NATIVES.with(|natives| {
        natives
            .borrow_mut()
            .insert(name.to_string(), Rc::new(function))
    });
}

fn lookup(name: &str) -> Option<Rc<NativeFn>> {
    NATIVES.with(|natives| natives.borrow().get(name).cloned())
}

/// Install every registered function on a freshly created V8 global.
pub(crate) fn install(frame: Option<&mut Frame>, context: &V8Context) {
    // Second barrier, independent of the navigation guard. A context whose
    // frame is not local gets nothing: no native functions, no frame loop.
    // Belt and braces, because what is on the other side of this `if` is
    // the application's unsandboxed Rust.
    let url = frame
        .as_ref()
        .map(|frame| CefString::from(&frame.url()).to_string())
        .unwrap_or_default();
    if !crate::guard::is_local(&url) {
        crate::guard::refuse(&url);
        return;
    }

    crate::frame::open();
    register_native("__rustx_frame", |_args| {
        crate::frame::tick();
        Ok(Value::Null)
    });

    // The other direction of the React boundary. The engine installs
    // these itself: an application chooses which components React may
    // mount, not whether the page is able to ask.
    //
    // Both take the id of the element rather than the element, because
    // a `Value` carries null, a boolean, a number or a string and
    // nothing else — widening it to carry a DOM object would widen
    // exactly the boundary this exists to keep narrow.
    install_harness();

    register_native("rustx_component", |args| {
        let [Value::String(name), Value::String(id)] = args else {
            return Err(NativeError::new(
                "rustx_component takes a component name and the id of the element it goes in",
            ));
        };
        crate::foreign::mount_registered(name, id)?;
        Ok(Value::Null)
    });

    // The page's JavaScript half navigating the application. The
    // reverse direction of the router, and the same one call the
    // developer's Rust makes.
    register_native("rustx_navigate", |args| {
        let [Value::String(path)] = args else {
            return Err(NativeError::new("rustx_navigate takes a path"));
        };
        crate::router::navigate(path.clone());
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

    HOOKS_RUN.with(|run| {
        if *run.borrow() {
            return;
        }
        *run.borrow_mut() = true;
        HOOKS.with(|hooks| {
            for hook in hooks.borrow().iter() {
                hook();
            }
        });
    });

    let Some(global) = context.global() else {
        return;
    };

    let names: Vec<String> = NATIVES.with(|natives| natives.borrow().keys().cloned().collect());
    for name in names {
        let key = CefString::from(name.as_str());
        let Some(function) = lookup(&name) else {
            continue;
        };
        let mut handler = native::handler_for(&name, function);
        if let Some(mut js_function) = cef::v8_value_create_function(Some(&key), Some(&mut handler))
        {
            global.set_value_bykey(
                Some(&key),
                Some(&mut js_function),
                V8Propertyattribute::default(),
            );
        }
    }

    // What the framework brings itself: the other half of the React
    // boundary, and the frame loop. Both go in through the frame: a
    // script evaluated on the context here does not run at all — V8 has
    // no current context to compile it in — and a
    // `requestAnimationFrame` registered from inside
    // `on_context_created` is dropped. The frame schedules them to run
    // in the page itself.
    //
    // Which means neither is there when the page's own `<script>` runs,
    // so nothing the framework injects may be something a page's first
    // line needs: what the framework injects reads what the page left
    // for it, never the other way round.
    if let Some(frame) = frame {
        for (name, source) in SCRIPTS.with(|scripts| scripts.borrow().clone()) {
            frame.execute_java_script(
                Some(&CefString::from(source.as_str())),
                Some(&CefString::from(format!("rustx://{name}").as_str())),
                1,
            );
        }
        frame.execute_java_script(
            Some(&CefString::from(crate::frame::DRIVER_SCRIPT)),
            Some(&CefString::from("rustx://frame-loop")),
            1,
        );
    }
}

/// What a scenario drives an application with.
///
/// Every application was registering the same three natives: the name
/// of the scenario it was started on, a line in a report file, and a
/// way to read a value out of the page. They are the framework's, so
/// they are installed here — an application under test writes nothing
/// at all, and an application that is not under test is given a
/// `rustx_scenario()` that answers with an empty string.
///
/// `rustx_report` is the only one that writes anything, and it writes
/// only where `--report=<path>` says.
fn install_harness() {
    let scenario = argument("--scenario=").unwrap_or_default();
    register_native("rustx_scenario", move |_args| {
        Ok(Value::String(scenario.clone()))
    });

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
}

fn argument(prefix: &str) -> Option<String> {
    std::env::args_os()
        .filter_map(|argument| argument.into_string().ok())
        .find_map(|argument| argument.strip_prefix(prefix).map(str::to_string))
}
