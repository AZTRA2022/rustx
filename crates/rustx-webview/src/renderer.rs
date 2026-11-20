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
    let hooks = HOOKS.with(|hooks| std::mem::take(&mut *hooks.borrow_mut()));
    for hook in &hooks {
        crate::panics::contain("a hook the application registered", || hook());
    }
}

/// The scripts to put in the page, in the order they were injected.
pub(crate) fn scripts() -> Vec<String> {
    SCRIPTS.with(|scripts| scripts.borrow().iter().map(|(_, text)| text.clone()).collect())
}

#[allow(dead_code)] // read by the native call path of phase 4.
pub(crate) fn lookup(name: &str) -> Option<Rc<NativeFn>> {
    NATIVES.with(|natives| natives.borrow().get(name).cloned())
}
