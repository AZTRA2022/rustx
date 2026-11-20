//! Files dropped on the window.
//!
//! Not on the webview engine yet: the system webview reports the paths of
//! a drop itself, and wiring it is phase 6. The API is here so an
//! application written against it compiles; a hook registered now is kept
//! and never called.

use std::cell::RefCell;
use std::path::PathBuf;

thread_local! {
    static HOOKS: RefCell<Vec<Box<dyn Fn(Vec<PathBuf>)>>> = const { RefCell::new(Vec::new()) };
}

/// Run `hook` with the paths of the files dropped on the window.
pub fn on_files(hook: impl Fn(Vec<PathBuf>) + 'static) {
    HOOKS.with(|hooks| hooks.borrow_mut().push(Box::new(hook)));
}

/// The files of the last drop.
pub fn files() -> Vec<PathBuf> {
    Vec::new()
}
