//! What a window is, besides a page: its icon and what it says it is.
//!
//! The controls act on the window's own tao handle, so they are the same on
//! every platform tao supports. A second window is not here yet.

use rustx_os::window::Dressing;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// What an application says it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct About {
    /// The window's title, which is also the application's name.
    pub name: String,
    /// `"0.1.0"`, as the application's own crate declares it.
    pub version: String,
    /// One line, for a window that shows one.
    pub description: String,
    /// Who publishes it.
    pub publisher: String,
}

fn declared() -> &'static Mutex<(Option<PathBuf>, About)> {
    static STATE: OnceLock<Mutex<(Option<PathBuf>, About)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((None, About::default())))
}

/// Said by the facade before the window opens.
pub fn declare(icon: Option<PathBuf>, about: About) {
    let mut held = declared()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *held = (icon, about);
}

/// What the application said it is.
pub fn about() -> About {
    declared()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .1
        .clone()
}

/// The icon the application declared, if it declared one.
pub fn icon() -> Option<PathBuf> {
    declared()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
        .clone()
}

/// What a second window is before it exists. It is a process of its own,
/// as every window was on the CEF engine, so it has its own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct What {
    pub name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// The route it opens on.
    pub at: String,
    pub dressing: Dressing,
}

impl What {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            title: name.clone(),
            name,
            width: 900,
            height: 640,
            at: "/".to_string(),
            dressing: Dressing::System,
        }
    }
    pub fn titled(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn at(mut self, route: impl Into<String>) -> Self {
        self.at = route.into();
        self
    }
    /// Without the system's frame: the page draws it, and [`drag`] moves the window.
    pub fn frameless(mut self) -> Self {
        self.dressing = Dressing::None;
        self
    }
}

/// An open window, by name. Dropping it does not close it: a handler opens
/// one and returns. [`Window::close`] does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    name: String,
}

impl Window {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Send an order to the window over the pipe that links the two processes:
    /// `{"order":"title","value":"…"}`, `size` (`[w,h]`), `position` (`[x,y]`),
    /// `center`, `focus`, `show`, `hide`, `maximize`. False when it is gone.
    pub fn order(&self, order: serde_json::Value) -> bool {
        use std::io::Write;
        children().with(|held| {
            let mut held = held.borrow_mut();
            let Some(child) = held.get_mut(&self.name) else {
                return false;
            };
            let Some(pipe) = child.stdin.as_mut() else {
                return false;
            };
            writeln!(pipe, "{order}").and_then(|_| pipe.flush()).is_ok()
        })
    }

    pub fn set_title(&self, title: &str) -> bool {
        self.order(serde_json::json!({"order": "title", "value": title}))
    }

    pub fn set_size(&self, width: u32, height: u32) -> bool {
        self.order(serde_json::json!({"order": "size", "value": [width, height]}))
    }

    pub fn set_position(&self, x: i32, y: i32) -> bool {
        self.order(serde_json::json!({"order": "position", "value": [x, y]}))
    }

    /// Close the window. The process behind it ends when its pipe does.
    pub fn close(&self) {
        children().with(|held| held.borrow_mut().remove(&self.name));
    }
}

const ARGUMENT: &str = "--rustx-window=";

thread_local! {
    static CHILDREN: std::cell::RefCell<std::collections::HashMap<String, std::process::Child>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn children() -> &'static std::thread::LocalKey<
    std::cell::RefCell<std::collections::HashMap<String, std::process::Child>>,
> {
    &CHILDREN
}

/// A route as the router writes it, whatever was typed.
fn route(asked: &str) -> String {
    let trimmed = asked.trim();
    match trimmed {
        "" => "/".to_string(),
        _ if trimmed.starts_with('/') => trimmed.to_string(),
        _ => format!("/{trimmed}"),
    }
}

/// Open a second window: this executable again, told what to show. It
/// stays open while the pipe on its input does, so it goes with this
/// process, and with [`Window::close`].
pub fn open(what: What) -> Result<Window, crate::error::EngineError> {
    let name = what.name.clone();
    if children().with(|held| {
        held.borrow_mut()
            .get_mut(&name)
            .is_some_and(|child| child.try_wait().ok().flatten().is_none())
    }) {
        return Ok(Window { name });
    }
    let spec = serde_json::json!({
        "title": what.title, "width": what.width, "height": what.height,
        "at": route(&what.at), "frameless": what.dressing == Dressing::None,
    });
    let child = std::env::current_exe()
        .and_then(|exe| {
            std::process::Command::new(exe)
                // What the first window was started with, so a scenario or a
                // report path reaches the second.
                .args(
                    std::env::args()
                        .skip(1)
                        .filter(|arg| !arg.starts_with(ARGUMENT)),
                )
                .arg(format!("{ARGUMENT}{spec}"))
                .stdin(std::process::Stdio::piped())
                .spawn()
        })
        .map_err(|error| {
            crate::error::EngineError::EngineInit(format!("could not open a window: {error}"))
        })?;
    children().with(|held| held.borrow_mut().insert(name.clone(), child));
    Ok(Window { name })
}

/// What a second window does with one line from its parent.
pub fn obey(line: &str) {
    let Ok(order) = serde_json::from_str::<serde_json::Value>(line) else {
        return;
    };
    let pair = |v: &serde_json::Value| Some((v[0].as_i64()?, v[1].as_i64()?));
    match order["order"].as_str() {
        Some("title") => set_title(order["value"].as_str().unwrap_or_default()),
        Some("size") => {
            if let Some((w, h)) = pair(&order["value"]) {
                set_size(w.max(1) as u32, h.max(1) as u32);
            }
        }
        Some("position") => {
            if let Some((x, y)) = pair(&order["value"]) {
                set_position(x as i32, y as i32);
            }
        }
        Some("center") => center(),
        Some("focus") => focus(),
        Some("show") => show(),
        Some("hide") => hide(),
        Some("maximize") => with(|w| w.set_maximized(true)),
        _ => {}
    }
}

/// What this process was told to be, when it is a second window.
pub(crate) fn asked() -> Option<serde_json::Value> {
    static SPEC: std::sync::OnceLock<Option<serde_json::Value>> = std::sync::OnceLock::new();
    SPEC.get_or_init(|| {
        std::env::args()
            .find_map(|arg| arg.strip_prefix(ARGUMENT).map(str::to_string))
            .and_then(|text| serde_json::from_str(&text).ok())
    })
    .clone()
}

/// The route this window was opened on, when it is not the first one.
pub fn page_asked_for() -> Option<String> {
    asked()?.get("at")?.as_str().map(str::to_string)
}

// ------------------------------------------------------- absolute control

pub use tao;

/// How the first window is made, said before it opens (see the facade's
/// `Config`). Everything here can also be changed afterwards with the setters.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub min_size: Option<(u32, u32)>,
    pub max_size: Option<(u32, u32)>,
    pub resizable: bool,
    pub centered: bool,
    /// Where to put it, in logical pixels, when it is not centred.
    pub position: Option<(i32, i32)>,
    pub always_on_top: bool,
    /// Size and position come back as the user left them: see `Config::remember_window`.
    pub remember: bool,
    /// The colour behind the page, so a dark application does not flash white.
    pub background: Option<(u8, u8, u8)>,
}

impl Default for Plan {
    fn default() -> Self {
        Self {
            min_size: None,
            max_size: None,
            resizable: true,
            centered: false,
            position: None,
            always_on_top: false,
            remember: false,
            background: None,
        }
    }
}

fn planned_cell() -> &'static Mutex<Plan> {
    static PLAN: OnceLock<Mutex<Plan>> = OnceLock::new();
    PLAN.get_or_init(|| Mutex::new(Plan::default()))
}

/// Say how the window is to be made.
pub fn plan(plan: Plan) {
    *planned_cell().lock().unwrap_or_else(|p| p.into_inner()) = plan;
}

pub(crate) fn planned() -> Plan {
    planned_cell()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

/// What the window is now. Kept up to date by the loop, so it can be read from
/// any thread without asking the window.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct State {
    /// The window's outer position, in physical pixels.
    pub x: i32,
    pub y: i32,
    /// The page area, in physical pixels.
    pub width: u32,
    pub height: u32,
    /// The whole window, frame included, in physical pixels.
    pub outer_width: u32,
    pub outer_height: u32,
    /// Physical pixels per logical one.
    pub scale: f64,
    pub maximized: bool,
    pub minimized: bool,
    pub fullscreen: bool,
    pub focused: bool,
    pub visible: bool,
}

fn state_cell() -> &'static Mutex<State> {
    static STATE_NOW: OnceLock<Mutex<State>> = OnceLock::new();
    STATE_NOW.get_or_init(|| Mutex::new(State::default()))
}

pub(crate) fn publish(state: State) {
    *state_cell().lock().unwrap_or_else(|p| p.into_inner()) = state;
}

/// The window as the loop last saw it.
pub fn state() -> State {
    *state_cell().lock().unwrap_or_else(|p| p.into_inner())
}

/// Do anything tao allows with the window: `f` runs on the loop's thread,
/// from whichever thread asked. The other functions here are this, named.
pub fn with(f: impl FnOnce(&tao::window::Window) + Send + 'static) {
    crate::host::with_window(Box::new(f));
}

pub fn set_title(title: &str) {
    let title = title.to_string();
    with(move |w| w.set_title(&title));
}
/// Resize the page area, in logical pixels.
pub fn set_size(width: u32, height: u32) {
    // Windows applies the limits to the user's dragging, not to a size the
    // program sets: clamp here so a limit means the same everywhere.
    let (min, max) = {
        let held = planned_cell().lock().unwrap_or_else(|p| p.into_inner());
        (held.min_size, held.max_size)
    };
    let (mut width, mut height) = (width, height);
    if let Some((a, b)) = min {
        (width, height) = (width.max(a), height.max(b));
    }
    if let Some((a, b)) = max {
        (width, height) = (width.min(a), height.min(b));
    }
    with(move |w| w.set_inner_size(tao::dpi::LogicalSize::new(width, height)));
}
/// Move the window, in logical pixels.
pub fn set_position(x: i32, y: i32) {
    with(move |w| w.set_outer_position(tao::dpi::LogicalPosition::new(x, y)));
}
pub fn set_min_size(size: Option<(u32, u32)>) {
    planned_cell()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .min_size = size;
    with(move |w| w.set_min_inner_size(size.map(|(a, b)| tao::dpi::LogicalSize::new(a, b))));
}
pub fn set_max_size(size: Option<(u32, u32)>) {
    planned_cell()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .max_size = size;
    with(move |w| w.set_max_inner_size(size.map(|(a, b)| tao::dpi::LogicalSize::new(a, b))));
}
pub fn set_resizable(resizable: bool) {
    with(move |w| w.set_resizable(resizable));
}
/// Put the window in the middle of the screen it is on.
pub fn center() {
    with(|w| {
        if let Some(monitor) = w.current_monitor() {
            let (m, s) = (monitor.size(), w.outer_size());
            let origin = monitor.position();
            w.set_outer_position(tao::dpi::PhysicalPosition::new(
                origin.x + (m.width as i32 - s.width as i32) / 2,
                origin.y + (m.height as i32 - s.height as i32) / 2,
            ));
        }
    });
}
/// Bring it to the front and give it the keyboard.
pub fn focus() {
    with(|w| w.set_focus());
}
pub fn show() {
    with(|w| w.set_visible(true));
}
pub fn hide() {
    with(|w| w.set_visible(false));
}
/// Keep it off the taskbar (Windows and Linux).
pub fn skip_taskbar(skip: bool) {
    with(move |w| {
        #[cfg(windows)]
        {
            use tao::platform::windows::WindowExtWindows;
            let _ = w.set_skip_taskbar(skip);
        }
        #[cfg(target_os = "linux")]
        {
            use tao::platform::unix::WindowExtUnix;
            let _ = w.set_skip_taskbar(skip);
        }
        let _ = (w, skip);
    });
}
/// Let mouse events through the window to what is behind it.
pub fn click_through(through: bool) {
    with(move |w| {
        let _ = w.set_ignore_cursor_events(through);
    });
}

/// What happened to the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Physical pixels.
    Moved {
        x: i32,
        y: i32,
    },
    Resized {
        width: u32,
        height: u32,
    },
    Focused(bool),
    CloseRequested,
}

thread_local! {
    static LISTENERS: std::cell::RefCell<Vec<Box<dyn Fn(&Event)>>> = const { std::cell::RefCell::new(Vec::new()) };
    static GUARD: std::cell::RefCell<Option<Box<dyn Fn() -> bool>>> = const { std::cell::RefCell::new(None) };
}

/// Hear about the window moving, resizing, gaining or losing focus, and being
/// asked to close. Called on the loop's thread (the application's own).
pub fn on_event(listener: impl Fn(&Event) + 'static) {
    LISTENERS.with(|held| held.borrow_mut().push(Box::new(listener)));
}

/// Decide whether a request to close (the window's X, Alt+F4) is honoured:
/// return `false` to keep the window, to ask about unsaved work first.
/// [`shut`] and the system shutting down are not asked.
pub fn confirm_close(guard: impl Fn() -> bool + 'static) {
    GUARD.with(|held| *held.borrow_mut() = Some(Box::new(guard)));
}

pub(crate) fn announce(event: Event) {
    LISTENERS.with(|held| held.borrow().iter().for_each(|f| f(&event)));
}

pub(crate) fn may_close() -> bool {
    GUARD.with(|held| held.borrow().as_ref().is_none_or(|guard| guard()))
}

/// Print the page this window shows, with the system's own dialog. Writing
/// straight to a PDF is not here: the system webviews give no way to do it.
pub fn print() {
    crate::host::print();
}

/// Give the window an order.
pub fn command(order: rustx_os::window::Order) {
    crate::host::order(order);
}

macro_rules! controls {
    ($($(#[$doc:meta])* $name:ident($($arg:ident: $ty:ty),*) => $order:expr;)*) => {
        $($(#[$doc])* pub fn $name($($arg: $ty),*) { command($order); })*
    };
}

use rustx_os::window::{Order, Spread, Switchable};

controls! {
    /// Remove the system's frame: the page draws everything, and [`drag`] moves the window.
    frameless() => Order::Dress(Dressing::None);
    /// Put the system's frame back.
    framed() => Order::Dress(Dressing::System);
    minimize() => Order::Spread(Spread::Minimized);
    maximize() => Order::Spread(Spread::Maximized);
    restore() => Order::Spread(Spread::Normal);
    fullscreen() => Order::Spread(Spread::Fullscreen);
    toggle_maximize() => Order::Switch(Switchable::Maximized);
    toggle_fullscreen() => Order::Switch(Switchable::Fullscreen);
    /// Keep the window in front of the others, or give it back its rank.
    on_top(above: bool) => Order::OnTop(above);
    /// Start moving the window with the mouse: call it from the press on the page's own title bar.
    drag() => Order::Drag;
    /// Close the window.
    shut() => Order::Close;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_route_is_written_the_way_the_router_writes_it() {
        assert_eq!(route(""), "/");
        assert_eq!(route("pause"), "/pause");
        assert_eq!(route(" /a/b "), "/a/b");
    }

    #[test]
    fn a_window_is_framed_until_it_says_otherwise() {
        assert_eq!(What::new("x").dressing, Dressing::System);
        assert_eq!(What::new("x").frameless().dressing, Dressing::None);
    }
}
