//! The window: a tao event loop, a wry webview, and the rules of
//! [`crate::policy`] on every door into the host.
//!
//! Everything here runs on the thread that calls [`run`], which for tao on
//! Windows and macOS has to be the main thread of the process. Callbacks
//! are therefore not `Send`, and the state they close over can stay in
//! `Rc` and thread-locals as it does in the CEF engine.

use std::borrow::Cow;
use std::convert::Infallible;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::WindowBuilder;
use wry::http::{Response, header::CONTENT_TYPE};
use wry::{NewWindowResponse, WebContext, WebViewBuilder};

use crate::assets::serve;
use crate::policy::{Navigation, Policy};

#[derive(Debug, Clone)]
pub struct Config {
    pub title: String,
    pub size: (u32, u32),
    /// The folder the window's files are served from.
    pub root: PathBuf,
    /// The page loaded first, as a path: `/index.html`.
    pub entry: String,
    /// The custom protocol's name. It becomes the origin: see [`origin`].
    pub scheme: String,
    /// Origins that may load in the window. They get no native access.
    pub allow: Vec<String>,
    /// Where the webview keeps its profile. `None` lets wry choose, which
    /// on Windows is a folder next to the executable: fixed by
    /// [`default_data_dir`] for an installed application.
    pub data_dir: Option<PathBuf>,
    pub devtools: bool,
    /// What follows `#` in the first address: how a second window learns its route.
    pub fragment: Option<String>,
    /// The system's frame around the window.
    pub decorations: bool,
    /// Scripts the page runs before its own, after the interpreter.
    pub scripts: Vec<String>,
    /// A document served at `entry` and at `/` instead of a file, for an
    /// application whose assets have no `index.html`.
    pub document: Option<String>,
}

/// How long the loop waits for a frame of its own before running one: the
/// floor under work that nothing announces, such as a write a task made.
pub const FLOOR: Duration = Duration::from_millis(100);

impl Config {
    pub fn new(title: &str, root: impl Into<PathBuf>) -> Self {
        Self {
            title: title.to_string(),
            size: (900, 640),
            root: root.into(),
            entry: "/index.html".to_string(),
            scheme: "rustx".to_string(),
            allow: Vec::new(),
            data_dir: default_data_dir(title),
            devtools: cfg!(debug_assertions),
            fragment: None,
            decorations: true,
            scripts: Vec::new(),
            document: None,
        }
    }
}

/// Where a profile belongs on Windows: under the user's local app data, so
/// that an application installed in Program Files can write it. Other
/// platforms keep wry's own choice, which is already a user folder.
pub fn default_data_dir(app: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let base = std::env::var_os("LOCALAPPDATA")?;
    let safe: String = app
        .chars()
        .map(|c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            true => c,
            false => '_',
        })
        .collect();
    Some(PathBuf::from(base).join(safe).join("webview"))
}

/// The origin pages of the custom protocol have, as the webview writes
/// it: Windows serves a custom protocol as `http://<name>.localhost`, the
/// other platforms as `<name>://localhost`.
pub fn origin(scheme: &str) -> String {
    match cfg!(windows) {
        true => format!("http://{scheme}.localhost"),
        false => format!("{scheme}://localhost"),
    }
}

#[derive(Debug)]
pub struct HostError(pub String);

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HostError {}

pub(crate) enum Command {
    Exit,
    Tick,
    /// An order for the window, carried out on the loop's thread.
    Window(rustx_os::window::Order),
    /// A script for the page, from any thread.
    Script(String),
}

/// Run `code` in the page, on the loop's thread.
pub(crate) fn script(code: String) {
    let held = proxy().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proxy) = held.as_ref() {
        let _ = proxy.send_event(Command::Script(code));
    }
}

/// Give the window an order, from any thread.
pub(crate) fn order(order: rustx_os::window::Order) {
    let held = proxy().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proxy) = held.as_ref() {
        let _ = proxy.send_event(Command::Window(order));
    }
}

/// What an order does to a tao window.
fn carry_out(window: &tao::window::Window, order: rustx_os::window::Order) -> bool {
    use rustx_os::window::{Dressing, Order, Spread, Switchable};
    use tao::window::Fullscreen;
    let fullscreen = |on: bool| window.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
    let spread = |how: Spread| match how {
        Spread::Minimized => window.set_minimized(true),
        Spread::Maximized => {
            fullscreen(false);
            window.set_maximized(true)
        }
        Spread::Normal => {
            fullscreen(false);
            window.set_minimized(false);
            window.set_maximized(false)
        }
        Spread::Fullscreen => fullscreen(true),
    };
    match order {
        Order::Dress(dressing) => window.set_decorations(dressing == Dressing::System),
        Order::Spread(how) => spread(how),
        Order::Switch(Switchable::Maximized) => {
            spread(if window.is_maximized() { Spread::Normal } else { Spread::Maximized })
        }
        Order::Switch(Switchable::Fullscreen) => fullscreen(window.fullscreen().is_none()),
        Order::OnTop(above) => window.set_always_on_top(above),
        Order::Drag => {
            if let Err(error) = window.drag_window() {
                eprintln!("rustx: could not start a drag: {error}");
            }
        }
        Order::Close => return true,
    }
    false
}

/// The loop's proxy, for whoever needs a frame from any thread.
fn proxy() -> &'static Mutex<Option<EventLoopProxy<Command>>> {
    static PROXY: OnceLock<Mutex<Option<EventLoopProxy<Command>>>> = OnceLock::new();
    PROXY.get_or_init(|| Mutex::new(None))
}

/// A frame has been asked for and has not run yet.
static PENDING: AtomicBool = AtomicBool::new(false);

/// Ask the loop for a frame, from any thread. A burst of asks is one
/// frame: only the first sends anything.
pub fn wake() {
    if PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    let held = proxy().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proxy) = held.as_ref() {
        let _ = proxy.send_event(Command::Tick);
    }
}

/// A way to stop the loop from anywhere: it may be cloned and sent.
#[derive(Clone)]
pub struct Control(EventLoopProxy<Command>);

impl Control {
    pub fn exit(&self) {
        let _ = self.0.send_event(Command::Exit);
    }
}

/// What the host calls back, on the loop thread.
pub struct Hooks {
    /// Called once, before the loop starts, with the way to stop it.
    pub on_start: Box<dyn Fn(&Control)>,
    /// A message from the application's own page that is not one of the
    /// engine's, after the origin check.
    pub on_message: Box<dyn Fn(&str, &Control)>,
    /// A frame: drain what workers posted, then draw. By default
    /// [`crate::frame::tick`].
    pub on_tick: Box<dyn Fn()>,
    /// An address to give to the system browser. The default is
    /// [`rustx_os::shell::open_url`]; a test puts its own.
    pub open_external: Box<dyn Fn(&str)>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            on_start: Box::new(|_| {}),
            on_message: Box::new(|_, _| {}),
            on_tick: Box::new(crate::frame::tick),
            open_external: Box::new(|url| {
                if let Err(error) = rustx_os::shell::open_url(url) {
                    eprintln!("rustx: could not open the link: {error}");
                }
            }),
        }
    }
}

/// Open the window and run until it closes. Returns only on an error
/// before the loop starts: once it runs, closing the window ends the
/// process.
pub fn run(config: Config, hooks: Hooks) -> Result<Infallible, HostError> {
    let policy = Policy::new(
        &origin(&config.scheme),
        &config.allow.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let event_loop = EventLoopBuilder::<Command>::with_user_event().build();
    let control = Control(event_loop.create_proxy());
    *proxy()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(event_loop.create_proxy());
    let window = WindowBuilder::new()
        .with_title(&config.title)
        .with_decorations(config.decorations)
        .with_inner_size(LogicalSize::new(config.size.0, config.size.1))
        .build(&event_loop)
        .map_err(|error| HostError(format!("could not open the window: {error}")))?;

    let hooks = Rc::new(hooks);
    let policy = Rc::new(policy);

    let root = config.root.clone();
    let (document, entry) = (config.document.clone(), config.entry.clone());
    let mut context = WebContext::new(config.data_dir.clone());
    let mut builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_devtools(config.devtools)
        .with_initialization_script(format!(
            "window.__rustx_origin = {};{INTERPRETER}",
            serde_json::json!(origin(&config.scheme))
        ))
        .with_custom_protocol(config.scheme.clone(), {
            // Its own copy: wry wants this closure to be `Send + Sync`,
            // though it only ever runs on the thread of the window.
            let policy = std::sync::Arc::new(Policy::new(
                &origin(&config.scheme),
                &config.allow.iter().map(String::as_str).collect::<Vec<_>>(),
            ));
            move |_id, request| {
            let path = request.uri().path();
            if let Some(name) = path.strip_prefix(NATIVE_PATH) {
                return native(&policy, name, &request);
            }
            if let Some(rest) = path.strip_prefix(crate::files::PATH) {
                let header = |name: &str| request.headers().get(name).and_then(|v| v.to_str().ok());
                let answer = crate::files::serve(
                    rest,
                    header("range"),
                    header("sec-fetch-site") == Some("cross-site"),
                );
                let mut response = Response::builder()
                    .status(answer.status)
                    .header(CONTENT_TYPE, answer.mime)
                    .header("Accept-Ranges", "bytes")
                    .header("X-Content-Type-Options", "nosniff");
                if let Some(range) = answer.content_range {
                    response = response.header("Content-Range", range);
                }
                return response
                    .body(Cow::Owned(answer.body))
                    .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])));
            }
            let served = match &document {
                Some(page) if path == "/" || path == entry => crate::assets::Served {
                    status: 200,
                    mime: "text/html; charset=utf-8",
                    body: page.clone().into_bytes(),
                },
                _ => serve(&root, path),
            };
            Response::builder()
                .status(served.status)
                .header(CONTENT_TYPE, served.mime)
                .header("X-Content-Type-Options", "nosniff")
                .body(Cow::Owned(served.body))
                .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
            }
        })
        .with_ipc_handler({
            let (policy, hooks, control) = (Rc::clone(&policy), Rc::clone(&hooks), control.clone());
            move |request| {
                let from = request.uri().to_string();
                match policy.ipc_allowed(&from) {
                    true => handle(request.body(), &hooks, &control),
                    false => eprintln!(
                        "rustx: ignored a message from {}: only the application's own \
                         pages may call the host",
                        from.chars().take(40).collect::<String>()
                    ),
                }
            }
        })
        .with_drag_drop_handler(|event| {
            if let wry::DragDropEvent::Drop { paths, .. } = event {
                crate::dropped::landed(paths);
                wake();
            }
            // The page's own `ondrop` runs as well.
            false
        })
        .with_navigation_handler({
            let (policy, hooks) = (Rc::clone(&policy), Rc::clone(&hooks));
            move |url| {
                let stays = decide(&policy, &hooks, &url);
                // A page of the application is about to replace this one:
                // every handle made for the old one is stale, and what was
                // mounted on it is released, so a component's `Drop` runs.
                if stays && policy.ipc_allowed(&url) {
                    crate::dom::page_gone();
                    crate::render::unmount_all();
                }
                stays
            }
        })
        .with_new_window_req_handler({
            let (policy, hooks) = (Rc::clone(&policy), Rc::clone(&hooks));
            move |url, _features| {
                // A window of its own is never opened: a page asking for
                // one gets the system browser, or nothing.
                decide(&policy, &hooks, &url);
                NewWindowResponse::Deny
            }
        })
        .with_url(format!(
            "{}{}{}",
            origin(&config.scheme),
            config.entry,
            config.fragment.as_deref().map(|route| format!("#{route}")).unwrap_or_default()
        ));
    for script in &config.scripts {
        builder = builder.with_initialization_script(script.clone());
    }
    // The browser's own chrome is for development: reload, print, zoom, view
    // source and the context menu would each break an application that is not
    // a web page. Left on only where the devtools are.
    builder = builder.with_hotkeys_zoom(config.devtools).with_back_forward_navigation_gestures(false);
    #[cfg(windows)]
    {
        use wry::WebViewBuilderExtWindows;
        builder = builder
            .with_browser_accelerator_keys(config.devtools)
            .with_default_context_menus(config.devtools);
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let webview = builder.build(&window).map_err(|error| {
        // On Windows the usual cause is the missing runtime, and the
        // system's answer is an HRESULT that does not say so.
        let hint = match cfg!(windows) {
            true => {
                " The Microsoft Edge WebView2 Runtime may be missing or damaged: install \
                 it from developer.microsoft.com/microsoft-edge/webview2."
            }
            false => "",
        };
        HostError(format!("could not start the webview: {error}.{hint}"))
    })?;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window
            .default_vbox()
            .ok_or_else(|| HostError("the window has no container for the webview".to_string()))?;
        builder
            .build_gtk(vbox)
            .map_err(|error| HostError(format!("could not start the webview: {error}")))?
    };

    // This thread draws now: a call that waits, made here, says so.
    rustx_os::blocking::draws_here();
    (hooks.on_start)(&control);

    event_loop.run(move |event, _, flow| {
        // A frame at least every FLOOR, and at once when one is asked for.
        *flow = ControlFlow::WaitUntil(Instant::now() + FLOOR);
        let _ = (&window, &context);
        let frame = || {
            (hooks.on_tick)();
            // One message for the whole frame, commands in the order they
            // were written.
            if let Some(commands) = crate::dom::take_ops()
                && let Err(error) = webview.evaluate_script(&format!("window.__rustx.apply({commands})"))
            {
                eprintln!("rustx: could not send a frame to the page: {error}");
            }
        };
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::UserEvent(Command::Exit) => *flow = ControlFlow::Exit,
            Event::UserEvent(Command::Script(code)) => {
                if let Err(error) = webview.evaluate_script(&code) {
                    eprintln!("rustx: could not answer the page: {error}");
                }
            }
            Event::UserEvent(Command::Window(asked)) => {
                if carry_out(&window, asked) {
                    *flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(Command::Tick) => {
                // Cleared first: a write the frame itself makes asks for the next one.
                PENDING.store(false, Ordering::Release);
                frame();
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => frame(),
            _ => {}
        }
    })
}

/// Where a page calls a native: a synchronous request, so the function it
/// gets back answers with a value, as it always has.
const NATIVE_PATH: &str = "/__rustx/native/";

/// Run a native for the page and answer `{ok}` or `{err}`. Only a page of
/// the application may: the request says where it comes from.
fn native(policy: &Policy, name: &str, request: &wry::http::Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let from = request
        .headers()
        .get("origin")
        .and_then(|origin| origin.to_str().ok())
        .unwrap_or_default();
    let outcome = if !policy.ipc_allowed(from) {
        Err("only the application's own pages may call a native".to_string())
    } else {
        let name = name.to_string();
        std::str::from_utf8(request.body())
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str::<serde_json::Value>(text).map_err(|error| error.to_string()))
            .and_then(|json| crate::protocol::json_to_value(&json, 0).map_err(|error| error.to_string()))
            .and_then(|args| match args {
                crate::value::Value::List(args) => crate::renderer::call(&name, &args),
                _ => Err("the arguments are not a list".to_string()),
            })
    };
    // What the native wrote to the DOM goes out with the next frame.
    wake();
    let body = match outcome.and_then(|value| crate::protocol::value_to_json(&value, 0).map_err(|e| e.to_string())) {
        Ok(value) => serde_json::json!({ "ok": value }),
        Err(message) => serde_json::json!({ "err": message }),
    };
    Response::builder()
        .header(CONTENT_TYPE, "application/json")
        .body(Cow::Owned(body.to_string().into_bytes()))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&b""[..])))
}

/// What the interpreter on the page is made of.
const INTERPRETER: &str = include_str!("../assets/interpreter.js");

/// One message from the application's own page: the engine's are acted on,
/// anything else goes to the application's hook.
fn handle(body: &str, hooks: &Hooks, control: &Control) {
    use crate::protocol::{FromPage, decode};
    match decode(body) {
        Ok(FromPage::Ready { ids }) => {
            crate::dom::page_ready(ids);
            wake();
        }
        Ok(FromPage::Event {
            node,
            name,
            seq,
            event,
        }) => {
            crate::mount::dispatch(node, &name, &event, seq);
            wake();
        }
        Ok(FromPage::Error { message }) => eprintln!("rustx: the page could not run a command: {message}"),
        // An asynchronous native: the answer comes back as a script.
        Ok(FromPage::Call { id, name, args }) => crate::renderer::call_async(id, &name, &args),
        Ok(FromPage::Callback { id, args }) => {
            crate::foreign::called(id, &args);
            wake();
        }
        // Not one of ours: the application's own message.
        Err(_) => (hooks.on_message)(body, control),
    }
}

/// The policy's answer to an address, acted on. `true` means load it here.
fn decide(policy: &Policy, hooks: &Hooks, url: &str) -> bool {
    match policy.navigation(url) {
        Navigation::Stay => true,
        Navigation::OpenExternally => {
            (hooks.open_external)(url);
            false
        }
        Navigation::Refuse(why) => {
            eprintln!("rustx: refused to navigate: {why}");
            false
        }
    }
}
