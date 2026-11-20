//! The boundary where React owns a subtree.
//!
//! Not on the webview engine yet: phase 5 of the migration moves it. Until
//! then a boundary is a container with nothing in it, and a page that
//! asks for one is told, once, rather than left with an empty box.

use std::cell::Cell;

use rustx_dom::ForeignNode;

use crate::dom::Node;

thread_local! {
    static SAID: Cell<bool> = const { Cell::new(false) };
}

/// Let React mount a component of the application by `name`. Kept for the
/// facade; React is not hosted yet, so nothing asks for it.
pub fn register_component(_name: &str, _component: impl Fn() -> rustx_dom::Element + 'static) {}

/// Hand a component to React. Says that it cannot, the first time.
pub(crate) fn hand_over(node: &ForeignNode, _container: &Node) {
    if SAID.with(|said| said.replace(true)) {
        return;
    }
    eprintln!(
        "rustx: `{}` is a React component, and the webview engine does not host React \
         boundaries yet; its container is empty",
        node.component
    );
}

/// A boundary that was rendered before the page had a bridge. None here.
pub(crate) fn hand_over_pending() {}

/// Tell React its container is leaving the document. Nothing to tell.
pub(crate) fn give_back(_container: &Node) {}
