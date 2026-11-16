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

/// The prefix the engine keeps for itself on the JavaScript global.
///
/// `__rustx_frame` is how the page drives the frame loop, and it used to
/// be registered through the public [`register_native`] — so it landed in
/// the application's own namespace, where an application could replace
/// it, and on the global as an ordinary writable property, where a page's
/// script could call it (re-entering `tick`) or shadow it (stopping the
/// loop). Neither corrupts anything: `tick` takes its callbacks out and
/// puts them back, so re-entrance loses nothing. It is robustness, and it
/// costs one reserved prefix.
pub const RESERVED: &str = "__rustx_";

thread_local! {
    /// What the application asked to run when a page's context appears.
    static HOOKS: RefCell<Vec<Box<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
    /// The functions to install on the JavaScript global.
    static NATIVES: RefCell<HashMap<String, Rc<NativeFn>>> = RefCell::new(HashMap::new());
    /// The engine's own, kept apart from the application's: an
    /// application cannot replace one by registering its name, and the
    /// page cannot write over one or delete it.
    static INTERNAL: RefCell<HashMap<String, Rc<NativeFn>>> = RefCell::new(HashMap::new());
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
///
/// # Once per process, not once per page
///
/// The hooks run on the **first** context of the render process and
/// never again. That is deliberate: a hook is where an application
/// registers its natives and its components, and running them again per
/// page would rebuild whatever their closures captured — so a counter
/// held by a hook would reset on every navigation.
///
/// What follows from it, and is the part worth stating: a hook
/// registered *after* that first context has appeared never runs at
/// all. In practice every caller registers from `main`, before
/// [`crate::bootstrap`], which is the only place a render process still
/// reaches. An application that registers one later is registering it
/// for a context that has already been and gone.
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
    if name.starts_with(RESERVED) {
        eprintln!(
            "rustx: `{name}` was not registered. Names beginning with \
             `{RESERVED}` belong to the engine — one of them is the frame \
             loop itself, and an application that replaced it would stop \
             its own window drawing."
        );
        return;
    }
    NATIVES.with(|natives| {
        natives
            .borrow_mut()
            .insert(name.to_string(), Rc::new(function))
    });
}

/// The same, for the engine's own functions.
///
/// They go on the global read-only, not enumerable and not deletable,
/// after the application's: a page that writes `__rustx_frame = …` or
/// `delete __rustx_frame` changes nothing, and `Object.keys(window)`
/// does not list the framework's plumbing.
pub(crate) fn register_internal(
    name: &str,
    function: impl Fn(&[Value]) -> Result<Value, NativeError> + 'static,
) {
    debug_assert!(name.starts_with(RESERVED), "an engine native is reserved");
    INTERNAL.with(|natives| {
        natives
            .borrow_mut()
            .insert(name.to_string(), Rc::new(function))
    });
}

/// What the engine's own functions are put on the global with.
fn kept_for_the_engine() -> V8Propertyattribute {
    use cef::sys::cef_v8_propertyattribute_t as Attribute;
    Attribute(
        [
            Attribute::V8_PROPERTY_ATTRIBUTE_READONLY,
            Attribute::V8_PROPERTY_ATTRIBUTE_DONTENUM,
            Attribute::V8_PROPERTY_ATTRIBUTE_DONTDELETE,
        ]
        .into_iter()
        .fold(0, |all, one| all | one.0),
    )
    .into()
}

fn lookup(name: &str) -> Option<Rc<NativeFn>> {
    NATIVES.with(|natives| natives.borrow().get(name).cloned())
}

fn lookup_internal(name: &str) -> Option<Rc<NativeFn>> {
    INTERNAL.with(|natives| natives.borrow().get(name).cloned())
}

/// Put one function on the global under `name`.
fn install_one(
    global: &cef::V8Value,
    name: &str,
    function: Rc<NativeFn>,
    attributes: V8Propertyattribute,
) {
    let key = CefString::from(name);
    let mut handler = native::handler_for(name, function);
    if let Some(mut built) = cef::v8_value_create_function(Some(&key), Some(&mut handler)) {
        global.set_value_bykey(Some(&key), Some(&mut built), attributes);
    }
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
    if std::env::var("RUSTX_TRACE").is_ok() {
        eprintln!(
            "rustx: contexte créé — processus {}, fil {:?}, url {url}",
            std::process::id(),
            std::thread::current().id()
        );
    }
    // L'URL de cette page, pour le routeur : une deuxième fenêtre reçoit
    // le même document que la première, et c'est son fragment qui dit la
    // route sur laquelle elle s'ouvre.
    crate::window::loaded_at(&url);

    remember(context);
    // This thread owns the screen from now on. The platform layer has
    // no way of knowing that by itself, and it is the layer with the
    // calls that wait: a file dialog, a script, the clipboard. Told
    // here, a call made from an `onClick` says so instead of silently
    // stopping the window — see `rustx_os::blocking`.
    rustx_os::blocking::draws_here();
    crate::frame::open();
    // Ce que la fenêtre a à dire à la page est écouté dès maintenant, et
    // non à la première lecture : des fichiers lâchés sur la fenêtre
    // avant le premier battement d'une page seraient sinon perdus.
    crate::dropped::ready();
    register_internal("__rustx_dropped", |_args| {
        crate::dropped::landed();
        Ok(Value::Null)
    });
    // Et le lâcher appartient à l'application : sans cela, Chromium
    // navigue vers le fichier lâché et la fenêtre n'est plus celle du
    // logiciel.
    inject("rustx-dropped", crate::dropped::LISTENER);
    // And the floor under the loop, which is the engine's own timer: a
    // window that is never asked to draw still has to run.
    crate::frame::watch();
    register_internal("__rustx_frame", |_args| {
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
        let Some(function) = lookup(&name) else {
            continue;
        };
        install_one(&global, &name, function, V8Propertyattribute::default());
    }

    // The engine's own, last and protected. Last so that nothing an
    // application registered can be sitting on one of these names;
    // protected so that the page's own script cannot write over one or
    // delete it.
    let ours: Vec<String> = INTERNAL.with(|natives| natives.borrow().keys().cloned().collect());
    for name in ours {
        let Some(function) = lookup_internal(&name) else {
            continue;
        };
        install_one(&global, &name, function, kept_for_the_engine());
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

/// Run `work` on this thread, `delay` milliseconds from now.
///
/// The engine's own timer. A page's `setTimeout` is throttled to one a
/// second in a window Chromium believes is hidden, and a window that is
/// not being composited is exactly that — so what has to keep running
/// whatever the compositor is doing runs here instead.
pub(crate) fn after(delay: u64, work: impl FnOnce() + 'static) {
    use std::cell::RefCell;
    use std::rc::Rc;

    use cef::{ImplTask, Task, WrapTask, rc::Rc as _};

    cef::wrap_task! {
        struct Once {
            work: Pending,
        }

        impl Task {
            fn execute(&self) {
                let taken = self.work.borrow_mut().take();
                if let Some(work) = taken {
                    crate::panics::contain("a delayed task", work);
                }
            }
        }
    }

    let work: Box<dyn FnOnce()> = Box::new(work);
    let mut task = Once::new(Rc::new(RefCell::new(Some(work))));
    cef::post_delayed_task(cef::ThreadId::RENDERER, Some(&mut task), delay as i64);
}

/// What a delayed task carries, once.
type Pending = std::rc::Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

thread_local! {
    /// The page's own JavaScript context, kept for the work the engine
    /// does outside a call from the page.
    static CONTEXT: RefCell<Option<V8Context>> = const { RefCell::new(None) };
}

/// Remember the context a page was given, and forget it when it goes.
pub(crate) fn remember(context: &V8Context) {
    CONTEXT.with(|held| *held.borrow_mut() = Some(context.clone()));
}

/// The frame this page lives in, for the engine's own work.
///
/// A page asking for a second window is the one caller: the request has
/// to travel to the process that owns the windows, and a frame is what
/// carries it. `None` outside the render process, or before a page.
pub(crate) fn page_frame() -> Option<Frame> {
    let context = CONTEXT.with(|held| held.borrow().clone())?;
    if context.is_valid() == 0 {
        return None;
    }
    context.frame()
}

pub(crate) fn forget_context() {
    CONTEXT.with(|held| *held.borrow_mut() = None);
}

/// Run `work` inside the page's context.
///
/// Anything that touches the DOM has to: V8 answers `get_current_context`
/// with nothing when no context is entered, and every handle the work
/// makes would be stale before it was used. A call that came *from* the
/// page is already inside one; a delayed task of the engine's own is not.
pub(crate) fn in_context(work: impl FnOnce()) {
    let context = CONTEXT.with(|held| held.borrow().clone());
    let Some(context) = context else {
        return;
    };
    if context.is_valid() == 0 {
        return;
    }
    if context.enter() == 0 {
        // Entering can fail, and work done outside a context makes
        // handles that are stale the moment they are used.
        return;
    }
    // Left on the way out whichever way `work` goes. `work` is a frame,
    // and a frame runs the application's own code: a panic that escaped
    // between `enter` and `exit` left V8 with a context entered for
    // ever, and every frame after it entered another one on top. Only a
    // `Drop` can promise the exit, which is why this is a guard and not
    // a second call.
    let _left = Left(context);
    work();
}

/// Leaves the page's context, panic or not.
struct Left(V8Context);

impl Drop for Left {
    fn drop(&mut self) {
        self.0.exit();
    }
}
