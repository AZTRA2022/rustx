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

    // Start the frame loop now that `__rustx_frame` exists to be called.
    //
    // Injected through the frame rather than evaluated on the context: a
    // `requestAnimationFrame` registered from inside `on_context_created`
    // is dropped, and the loop never starts. The frame schedules the script
    // to run in the page itself.
    if let Some(frame) = frame {
        frame.execute_java_script(
            Some(&CefString::from(crate::frame::DRIVER_SCRIPT)),
            Some(&CefString::from("rustx://frame-loop")),
            1,
        );
    }
}
