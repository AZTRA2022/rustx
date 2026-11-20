//! Files dropped on the window, with their real paths.
//!
//! A page sees a drop as `File` objects and never learns where they are.
//! The webview host knows: wry reports the paths at the moment of the drop,
//! and they arrive here, on the thread that owns the window. The page's own
//! `ondrop` still fires, so [`files`] read inside it finds the paths already.

use std::cell::RefCell;
use std::path::PathBuf;

type Hooks = Vec<Box<dyn Fn(Vec<PathBuf>)>>;

thread_local! {
    static WAITING: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    static HOOKS: RefCell<Hooks> = RefCell::new(Vec::new());
}

/// Run `hook` with the paths of the files dropped on the window, in the
/// order the hooks were registered.
pub fn on_files(hook: impl Fn(Vec<PathBuf>) + 'static) {
    HOOKS.with(|hooks| hooks.borrow_mut().push(Box::new(hook)));
}

/// The files of the last drop nobody has read yet.
pub fn files() -> Vec<PathBuf> {
    WAITING.with(|waiting| std::mem::take(&mut *waiting.borrow_mut()))
}

/// Said by the host when a drop lands.
pub(crate) fn landed(paths: Vec<PathBuf>) {
    WAITING.with(|waiting| *waiting.borrow_mut() = paths.clone());
    // Out of the table first: a hook may register another.
    let hooks = HOOKS.with(|hooks| std::mem::take(&mut *hooks.borrow_mut()));
    for hook in &hooks {
        crate::panics::contain("a dropped-files hook", || hook(paths.clone()));
    }
    HOOKS.with(|held| {
        let mut held = held.borrow_mut();
        let added = std::mem::take(&mut *held);
        *held = hooks;
        held.extend(added);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn a_drop_reaches_the_hooks_and_waits_to_be_read_once() {
        let seen = Rc::new(Cell::new(0));
        let counted = Rc::clone(&seen);
        on_files(move |paths| counted.set(counted.get() + paths.len()));
        landed(vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]);
        landed(vec![PathBuf::from("c.txt")]);
        assert_eq!(seen.get(), 3);
        assert_eq!(files(), vec![PathBuf::from("c.txt")]);
        assert!(files().is_empty());
    }
}
