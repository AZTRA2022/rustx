//! What a window is, besides a page: its icon and what it says it is.
//!
//! The controls act on the window's own tao handle, so they are the same on
//! every platform tao supports. A second window is not here yet.

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

/// The route this window was opened on, when it is not the first one.
/// The first window has none.
pub fn page_asked_for() -> Option<String> {
    None
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

use rustx_os::window::{Dressing, Order, Spread, Switchable};

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
