//! Handing a node to React, and taking it back.
//!
//! rustx builds the container, says which React component goes in it and
//! with what props, and does not look inside. The page supplies the
//! other half:
//!
//! ```js
//! window.__rustx_react = { mount(name, node, props), unmount(node) };
//! ```
//!
//! `mount` is called just after the container is attached, and again
//! whenever the props change — React's own reconciler is what decides
//! that a second call with new props is an update rather than a new
//! mount, which is exactly the division of labour this boundary is for.
//!
//! `unmount` is called **before** the container leaves the document, and
//! that order is half of the work: a React tree told afterwards never
//! runs its cleanups, and its listeners stay attached to a node nobody
//! can reach any more.

use std::cell::{Cell, RefCell};

use rustx_dom::Value;

use crate::dom::Node;

/// A boundary that has been rendered and has nobody to be handed to
/// yet: the component it names, its container, and the props of the
/// render that built it.
type Waiting = (String, Node, Vec<(String, Value)>);

/// What an application sees if it renders a boundary before the bridge
/// to React is installed.
pub const NOTICE: &str = "rustx: a node React owns was rendered, and the page has no `window.__rustx_react` — \
     the container is on the page and empty";

thread_local! {
    static SAID: Cell<bool> = const { Cell::new(false) };
    static HANDED_OVER: Cell<u64> = const { Cell::new(0) };
    static GIVEN_BACK: Cell<u64> = const { Cell::new(0) };
    /// The last thing the boundary had to complain about, for a test or
    /// a diagnostic to read.
    static COMPLAINT: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Boundaries rendered while the page had no bridge, waiting for
    /// one to appear.
    static PENDING: RefCell<Vec<Waiting>> = const { RefCell::new(Vec::new()) };
}

/// How many times a node has been handed to React.
pub fn handed_over() -> u64 {
    HANDED_OVER.with(Cell::get)
}

/// How many times React has been told to let one go.
pub fn given_back() -> u64 {
    GIVEN_BACK.with(Cell::get)
}

/// The last complaint about the boundary, if there has been one.
pub fn last_complaint() -> Option<String> {
    COMPLAINT.with(|complaint| complaint.borrow().clone())
}

/// Give `container` to the React component called `component`.
///
/// Called again with new props on every change: the page's bridge is
/// what tells an update from a mount.
pub(crate) fn hand_over(component: &str, container: &Node, props: &[(String, Value)]) {
    let Some(bridge) = Node::on_window("__rustx_react") else {
        // Held, not dropped. A page installs its half of the boundary in
        // a script, and rustx renders as soon as the host element
        // exists: whichever came first, the container is handed over on
        // the frame the bridge appears.
        PENDING.with(|pending| {
            pending
                .borrow_mut()
                .push((component.to_string(), container.clone(), props.to_vec()))
        });
        report_missing_bridge();
        return;
    };
    let Some(props) = crate::value::object_of(props) else {
        complain("rustx: the props of a node React owns could not be built".to_string());
        return;
    };

    let name = crate::value::into_v8(Value::String(component.to_string()));
    let arguments = vec![name, Some(container.v8()), Some(props)];
    match bridge.call_with_v8_returning("mount", arguments) {
        // `false` is the page saying it has no component by that name. A
        // typo leaves an empty container; it does not take the window.
        Ok(Some(returned)) if returned.as_value() == Some(Value::Bool(true)) => {
            HANDED_OVER.with(|count| count.set(count.get() + 1));
        }
        Ok(_) => complain(format!(
            "rustx: the page has no React component named '{component}'"
        )),
        Err(error) => complain(format!("rustx: handing a node to React failed: {error}")),
    }
}

/// Tell React to let `container` go, while it is still in the document.
pub(crate) fn give_back(container: &Node) {
    let Some(bridge) = Node::on_window("__rustx_react") else {
        return;
    };
    match bridge.call_with_v8("unmount", vec![Some(container.v8())]) {
        Ok(()) => GIVEN_BACK.with(|count| count.set(count.get() + 1)),
        Err(error) => complain(format!(
            "rustx: taking a node back from React failed: {error}"
        )),
    }
}

/// Say it once per window, not once per frame.
pub(crate) fn report_missing_bridge() {
    if !SAID.with(Cell::get) {
        SAID.with(|said| said.set(true));
        complain(NOTICE.to_string());
    }
}

fn complain(message: String) {
    eprintln!("{message}");
    COMPLAINT.with(|complaint| *complaint.borrow_mut() = Some(message));
}

// ---------------------------------------------------------------------
// The other direction: a rustx component inside a node React owns.
// ---------------------------------------------------------------------

use std::collections::HashMap;
use std::rc::Rc;

use rustx_dom::Element;

type Builder = Rc<dyn Fn() -> Element>;

thread_local! {
    /// The components an application is willing to let React mount.
    static COMPONENTS: RefCell<HashMap<String, Builder>> = RefCell::new(HashMap::new());
}

/// Let the page mount `component` by that name.
///
/// What React gets is not a snapshot: the component becomes a root of
/// its own, re-rendered by the same frame as everything else on the
/// page, with its own cells, effects and tasks. Releasing the node
/// releases all of it, the way unmounting any root does.
pub fn register_component(name: &str, component: impl Fn() -> Element + 'static) {
    COMPONENTS.with(|components| {
        components
            .borrow_mut()
            .insert(name.to_string(), Rc::new(component))
    });
}

/// Mount the registered component `name` under the element `id`.
pub(crate) fn mount_registered(name: &str, id: &str) -> Result<(), crate::NativeError> {
    let Some(component) = COMPONENTS.with(|components| components.borrow().get(name).cloned())
    else {
        return Err(crate::NativeError::new(format!(
            "rustx: no component named '{name}' has been registered — \
             an application says what React may mount by calling \
             `rustx_engine::foreign::register_component`"
        )));
    };
    crate::render::mount_app(move || component(), id)?;
    Ok(())
}

/// Hand over anything that was rendered before the bridge existed.
///
/// Called on every frame, and costs a look at an empty list when there
/// is nothing waiting — which is every frame of an application that
/// loads its script before its host element, and every frame of one
/// that has no boundary at all.
pub(crate) fn hand_over_pending() {
    if PENDING.with(|pending| pending.borrow().is_empty()) {
        return;
    }
    if Node::on_window("__rustx_react").is_none() {
        return;
    }
    let waiting = PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()));
    for (component, container, props) in waiting {
        hand_over(&component, &container, &props);
    }
}
