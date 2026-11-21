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
        .map(
            |c| match c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                true => c,
                false => '_',
            },
        )
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
    /// The system's print dialog for the page.
    Print,
    /// Anything at all with the window, on the loop's thread.
    Do(Box<dyn FnOnce(&tao::window::Window) + Send>),
}

/// Run `f` on the window, on the loop's thread, from any thread.
pub(crate) fn with_window(f: Box<dyn FnOnce(&tao::window::Window) + Send>) {
    let held = proxy()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proxy) = held.as_ref() {
        let _ = proxy.send_event(Command::Do(f));
    }
}

/// The window has something to show: the first frame of the page is in, or
/// the wait for it has run out.
static SHOWN: AtomicBool = AtomicBool::new(false);

fn show_once() {
    if !SHOWN.swap(true, Ordering::AcqRel) {
        with_window(Box::new(|w| w.set_visible(true)));
    }
}

/// What the window is, for [`crate::window::state`].
fn snapshot(window: &tao::window::Window) {
    let (position, size) = (
        window.outer_position().unwrap_or_default(),
        window.inner_size(),
    );
    crate::window::publish(crate::window::State {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        scale: window.scale_factor(),
        maximized: window.is_maximized(),
        minimized: window.is_minimized(),
        fullscreen: window.fullscreen().is_some(),
        focused: window.is_focused(),
        visible: window.is_visible(),
    });
}

/// Where the size and position the user left are kept.
const STATE_FILE: &str = "window-state.json";

/// The last place and size the user left (logical pixels), when it still lands
/// on a screen that exists.
fn remembered(
    title: &str,
    event_loop: &tao::event_loop::EventLoop<Command>,
) -> Option<(i32, i32, u32, u32, bool)> {
    let text = rustx_os::data::read_text(title, STATE_FILE).ok()??;
    let saved: serde_json::Value = serde_json::from_str(&text).ok()?;
    let (x, y) = (saved["x"].as_i64()? as i32, saved["y"].as_i64()? as i32);
    let (w, h) = (saved["w"].as_u64()? as u32, saved["h"].as_u64()? as u32);
    let on_a_screen = event_loop.available_monitors().any(|m| {
        let (p, s, k) = (m.position(), m.size(), m.scale_factor());
        let (px, py) = (x as f64 * k, y as f64 * k);
        px >= p.x as f64
            && py >= p.y as f64
            && px < (p.x + s.width as i32) as f64
            && py < (p.y + s.height as i32) as f64
    });
    on_a_screen.then_some((
        x,
        y,
        w.max(200),
        h.max(150),
        saved["maximized"].as_bool().unwrap_or(false),
    ))
}

fn remember_now(title: &str, window: &tao::window::Window) {
    if window.is_minimized() || window.fullscreen().is_some() {
        return;
    }
    let scale = window.scale_factor();
    let (Ok(position), size) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    let saved = serde_json::json!({
        "x": (position.x as f64 / scale).round() as i64, "y": (position.y as f64 / scale).round() as i64,
        "w": (size.width as f64 / scale).round() as u64, "h": (size.height as f64 / scale).round() as u64,
        "maximized": window.is_maximized(),
    });
    let _ = rustx_os::data::write_text(title, STATE_FILE, &saved.to_string());
}

/// Open the system's print dialog for the window's page.
pub(crate) fn print() {
    let held = proxy()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proxy) = held.as_ref() {
        let _ = proxy.send_event(Command::Print);
    }
}

/// Run `code` in the page, on the loop's thread.
pub(crate) fn script(code: String) {
    let held = proxy()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(proxy) = held.as_ref() {
        let _ = proxy.send_event(Command::Script(code));
    }
}

/// What a window owes the application when it closes: what is mounted is
/// taken off, so a component's `Drop` runs (a file saved, a device let go),
/// and the async runtime stops. tao's loop never returns, so this is the
/// only moment there is. Once only, whichever way the window closes.
fn shut_down() {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::AcqRel) {
        return;
    }
    crate::render::unmount_all();
    crate::runtime::stop();
}

/// Give the window an order, from any thread.
pub(crate) fn order(order: rustx_os::window::Order) {
    let held = proxy()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        Order::Switch(Switchable::Maximized) => spread(if window.is_maximized() {
            Spread::Normal
        } else {
            Spread::Maximized
        }),
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
    let held = proxy()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    // `rustx dev` touches this file when only the assets changed: the page is
    // reloaded in place, and what Rust holds is mounted again on its own. Nothing
    // else in the world sets it, so a shipped application never looks.
    if let Some(file) = std::env::var_os("RUSTX_RELOAD") {
        std::thread::spawn(move || {
            let stamp = || {
                std::fs::metadata(&file)
                    .and_then(|about| about.modified())
                    .ok()
            };
            let mut seen = stamp();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(150));
                let now = stamp();
                if now != seen {
                    // A write is often two (truncate, then data): settle first.
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    seen = stamp();
                    script("location.reload()".to_string());
                }
            }
        });
    }
    let plan = crate::window::planned();
    // Built hidden and shown when the page has its first frame: no white
    // window first.
    let mut builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_decorations(config.decorations)
        .with_visible(false)
        .with_resizable(plan.resizable)
        .with_always_on_top(plan.always_on_top)
        .with_inner_size(LogicalSize::new(config.size.0, config.size.1));
    if let Some((w, h)) = plan.min_size {
        builder = builder.with_min_inner_size(LogicalSize::new(w, h));
    }
    if let Some((w, h)) = plan.max_size {
        builder = builder.with_max_inner_size(LogicalSize::new(w, h));
    }
    let mut maximized = false;
    match (
        plan.remember
            .then(|| remembered(&config.title, &event_loop))
            .flatten(),
        plan.position,
    ) {
        (Some((x, y, w, h, was_maximized)), _) => {
            builder = builder
                .with_inner_size(LogicalSize::new(w, h))
                .with_position(tao::dpi::LogicalPosition::new(x, y));
            maximized = was_maximized;
        }
        (None, Some((x, y))) => {
            builder = builder.with_position(tao::dpi::LogicalPosition::new(x, y))
        }
        (None, None) if plan.centered => {
            if let Some(monitor) = event_loop.primary_monitor() {
                let k = monitor.scale_factor();
                let (m, p) = (monitor.size(), monitor.position());
                builder = builder.with_position(tao::dpi::LogicalPosition::new(
                    p.x as f64 / k + (m.width as f64 / k - config.size.0 as f64) / 2.0,
                    p.y as f64 / k + (m.height as f64 / k - config.size.1 as f64) / 2.0,
                ));
            }
        }
        _ => {}
    }
    let window = builder
        .build(&event_loop)
        .map_err(|error| HostError(format!("could not open the window: {error}")))?;
    if maximized {
        window.set_maximized(true);
    }
    snapshot(&window);
    let started = Instant::now();
    let title_for_state = config.title.clone();
    let mut unsaved: Option<Instant> = None;

    let hooks = Rc::new(hooks);
    let policy = Rc::new(policy);

    let root = config.root.clone();
    let (document, entry) = (config.document.clone(), config.entry.clone());
    // The builder is made by a macro and not once: a start that fails because
    // the last process of this application still holds the profile is tried
    // again, and a builder is spent by `build`.
    macro_rules! make_builder {
        ($context:expr) => {{
            let (root, document, entry) = (root.clone(), document.clone(), entry.clone());
            let mut builder = WebViewBuilder::new_with_web_context($context)
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
                            let header = |name: &str| {
                                request.headers().get(name).and_then(|v| v.to_str().ok())
                            };
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
                    let (policy, hooks, control) =
                        (Rc::clone(&policy), Rc::clone(&hooks), control.clone());
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
                    config
                        .fragment
                        .as_deref()
                        .map(|route| format!("#{route}"))
                        .unwrap_or_default()
                ));
            if let Some((r, g, b)) = plan.background {
                builder = builder.with_background_color((r, g, b, 255));
            }
            if config.devtools {
                builder = builder
                    .with_initialization_script("window.__rustx_devtools = true;".to_string());
            }
            for script in &config.scripts {
                builder = builder.with_initialization_script(script.clone());
            }
            // The browser's own chrome is for development: reload, print, zoom, view
            // source would each break an application that is not a web page. Left on
            // only where the devtools are. The context menu stays (cut, copy, paste
            // belong to the system) and the page cancels it off text, see interpreter.
            builder = builder
                .with_hotkeys_zoom(config.devtools)
                .with_back_forward_navigation_gestures(false);
            #[cfg(windows)]
            {
                use wry::WebViewBuilderExtWindows;
                builder = builder
                    .with_browser_accelerator_keys(config.devtools)
                    .with_default_context_menus(true);
            }
            builder
        }};
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let (webview, context) = {
        let mut attempt = 0;
        loop {
            let mut context = WebContext::new(config.data_dir.clone());
            match make_builder!(&mut context).build(&window) {
                Ok(webview) => break (webview, context),
                // 0x800700AA: the previous run's browser process is still
                // letting go of the profile folder. It is a matter of a
                // moment, and a user who closes the application and opens it
                // again is the one who meets it.
                Err(error) if attempt < 40 && error.to_string().contains("800700AA") => {
                    attempt += 1;
                    std::thread::sleep(Duration::from_millis(250));
                }
                Err(error) => {
                    // On Windows the usual cause is the missing runtime, and
                    // the system's answer is an HRESULT that does not say so.
                    let hint = match cfg!(windows) {
                        true => {
                            " The Microsoft Edge WebView2 Runtime may be missing or damaged: \
                             install it from developer.microsoft.com/microsoft-edge/webview2."
                        }
                        false => "",
                    };
                    return Err(HostError(format!(
                        "could not start the webview: {error}.{hint}"
                    )));
                }
            }
        }
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let (webview, context) = {
        let mut context = WebContext::new(config.data_dir.clone());
        let builder = make_builder!(&mut context);
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window
            .default_vbox()
            .ok_or_else(|| HostError("the window has no container for the webview".to_string()))?;
        let webview = builder
            .build_gtk(vbox)
            .map_err(|error| HostError(format!("could not start the webview: {error}")))?;
        (webview, context)
    };

    // This thread draws now: a call that waits, made here, says so.
    rustx_os::blocking::draws_here();
    (hooks.on_start)(&control);

    event_loop.run(move |event, _, flow| {
        // A frame at least every FLOOR, and at once when one is asked for.
        *flow = ControlFlow::WaitUntil(Instant::now() + FLOOR);
        let _ = (&window, &context);
        let frame = || {
            // What the window is, as the system settles it: a hide or a
            // maximise may not have taken effect when the order was carried out.
            snapshot(&window);
            (hooks.on_tick)();
            // One message for the whole frame, commands in the order they
            // were written.
            if let Some(commands) = crate::dom::take_ops()
                && let Err(error) =
                    webview.evaluate_script(&format!("window.__rustx.apply({commands})"))
            {
                eprintln!("rustx: could not send a frame to the page: {error}");
            }
        };
        // The first frame of the page is in, or the wait has run out.
        if !SHOWN.load(Ordering::Acquire) && started.elapsed() > Duration::from_millis(1200) {
            show_once();
        }
        // What the user left is written when they have stopped moving it.
        if plan.remember && unsaved.is_some_and(|at| at.elapsed() > Duration::from_millis(500)) {
            unsaved = None;
            remember_now(&title_for_state, &window);
        }
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                crate::window::announce(crate::window::Event::CloseRequested);
                if crate::window::may_close() {
                    if plan.remember {
                        remember_now(&title_for_state, &window);
                    }
                    shut_down();
                    *flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(Command::Exit) => {
                shut_down();
                *flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                snapshot(&window);
                unsaved = Some(Instant::now());
                crate::window::announce(crate::window::Event::Resized {
                    width: size.width,
                    height: size.height,
                });
            }
            Event::WindowEvent {
                event: WindowEvent::Moved(at),
                ..
            } => {
                snapshot(&window);
                unsaved = Some(Instant::now());
                crate::window::announce(crate::window::Event::Moved { x: at.x, y: at.y });
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(on),
                ..
            } => {
                snapshot(&window);
                crate::window::announce(crate::window::Event::Focused(on));
            }
            Event::UserEvent(Command::Do(job)) => {
                job(&window);
                snapshot(&window);
            }
            Event::LoopDestroyed => shut_down(),
            Event::UserEvent(Command::Print) => {
                if let Err(error) = webview.print() {
                    eprintln!("rustx: could not print: {error}");
                }
            }
            Event::UserEvent(Command::Script(code)) => {
                if let Err(error) = webview.evaluate_script(&code) {
                    eprintln!("rustx: could not answer the page: {error}");
                }
            }
            Event::UserEvent(Command::Window(asked)) => {
                if carry_out(&window, asked) {
                    shut_down();
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
fn native(
    policy: &Policy,
    name: &str,
    request: &wry::http::Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let from = request
        .headers()
        .get("origin")
        .and_then(|origin| origin.to_str().ok())
        .unwrap_or_default();
    let outcome = if !policy.ipc_allowed(from) {
        Err("only the application's own pages may call a native".to_string())
    } else {
        let name = name.to_string();
        // Small arguments travel in the address (`?a=`), large ones in the body.
        let in_address = request
            .uri()
            .query()
            .and_then(|query| query.split('&').find_map(|pair| pair.strip_prefix("a=")))
            .and_then(crate::assets::percent_decoded);
        in_address
            .map(Ok)
            .unwrap_or_else(|| {
                std::str::from_utf8(request.body())
                    .map(str::to_string)
                    .map_err(|error| error.to_string())
            })
            .and_then(|text| {
                serde_json::from_str::<serde_json::Value>(&text).map_err(|error| error.to_string())
            })
            .and_then(|json| {
                crate::protocol::json_to_value(&json, 0).map_err(|error| error.to_string())
            })
            .and_then(|args| match args {
                crate::value::Value::List(args) => crate::renderer::call(&name, &args),
                _ => Err("the arguments are not a list".to_string()),
            })
    };
    // What the native wrote to the DOM goes out with the next frame.
    wake();
    let body = match outcome
        .and_then(|value| crate::protocol::value_to_json(&value, 0).map_err(|e| e.to_string()))
    {
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
            show_once();
            crate::dom::page_ready(ids);
            // A document that follows one the application had mounted on:
            // a reload. What was mounted goes back.
            crate::render::remount();
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
        Ok(FromPage::Error { message }) => {
            eprintln!("rustx: the page could not run a command: {message}")
        }
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
