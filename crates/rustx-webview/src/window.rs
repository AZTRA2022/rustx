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

fn state() -> &'static Mutex<(Option<PathBuf>, About)> {
    static STATE: OnceLock<Mutex<(Option<PathBuf>, About)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((None, About::default())))
}

/// Said by the facade before the window opens.
pub fn declare(icon: Option<PathBuf>, about: About) {
    let mut held = state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *held = (icon, about);
}

/// What the application said it is.
pub fn about() -> About {
    state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .1
        .clone()
}

/// The icon the application declared, if it declared one.
pub fn icon() -> Option<PathBuf> {
    state()
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
