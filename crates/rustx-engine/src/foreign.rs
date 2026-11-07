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

use rustx_dom::{ForeignChild, ForeignNode, Value};

use crate::dom::Node;

/// A boundary that has been rendered and has nobody to be handed to
/// yet: its container, and the tree of the render that built it.
type Waiting = (ForeignNode, Node);

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
pub(crate) fn hand_over(node: &ForeignNode, container: &Node) {
    let Some(bridge) = Node::on_window("__rustx_react") else {
        // Held, not dropped. A page installs its half of the boundary in
        // a script, and rustx renders as soon as the host element
        // exists: whichever came first, the container is handed over on
        // the frame the bridge appears.
        PENDING.with(|pending| pending.borrow_mut().push((node.clone(), container.clone())));
        report_missing_bridge();
        return;
    };
    let Some(props) = crate::value::object_of(&node.props) else {
        complain("rustx: the props of a node React owns could not be built".to_string());
        return;
    };
    // And the props the library calls. A closure stays in Rust; what
    // crosses is a function that calls it.
    for (name, callback) in &node.callbacks {
        let Some(mut function) = function_for(name, callback) else {
            complain(format!(
                "rustx: `{name}` could not be given to React as a function"
            ));
            return;
        };
        props.set_value_bykey(
            Some(&cef::CefString::from(name.as_str())),
            Some(&mut function),
            cef::V8Propertyattribute::default(),
        );
    }
    // The children as a list of descriptions. React builds the tree;
    // what crosses is what to build.
    let Some(children) = describe(&node.children) else {
        complain("rustx: what is inside a node React owns could not be built".to_string());
        return;
    };

    let component = &node.component;
    let name = crate::value::into_v8(Value::String(component.to_string()));
    let arguments = vec![name, Some(container.v8()), Some(props), Some(children)];
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

/// What is inside a boundary, as JavaScript React's half can read.
///
/// `{ component, props, children }` for a component, a plain string for
/// text — which is what React itself accepts as a child — and, for a
/// handler, a function on the props.
///
/// Built in V8 rather than as a [`Value`] because of that last part: a
/// `Value` carries null, booleans, numbers, strings, lists and objects
/// of them, and a function is none of those. A description that went
/// through `Value` therefore lost every handler below the boundary's own
/// node — which is every button in a page made of installed components,
/// since those are components inside components. The page still drew,
/// and nothing a developer clicked did anything.
fn describe(children: &[ForeignChild]) -> Option<V8Value> {
    let array = cef::v8_value_create_array(children.len() as i32)?;
    for (at, child) in children.iter().enumerate() {
        let mut item = match child {
            ForeignChild::Text(text) => crate::value::into_v8(Value::String(text.clone()))?,
            ForeignChild::Value(value) => crate::value::into_v8(value.clone())?,
            ForeignChild::Node(node) => describe_node(node)?,
            ForeignChild::Element(element) => describe_element(element)?,
        };
        array.set_value_byindex(at as i32, Some(&mut item));
    }
    Some(array)
}

/// One node inside a boundary: what to build, with what, and what it
/// holds — handlers included.
fn describe_node(node: &ForeignNode) -> Option<V8Value> {
    let object = cef::v8_value_create_object(None, None)?;
    let set = |name: &str, value: &mut V8Value| {
        object.set_value_bykey(
            Some(&cef::CefString::from(name)),
            Some(value),
            cef::V8Propertyattribute::default(),
        );
    };

    let mut component = crate::value::into_v8(Value::String(node.component.clone()))?;
    set("component", &mut component);

    let props = crate::value::object_of(&node.props)?;
    for (name, callback) in &node.callbacks {
        let mut function = function_for(name, callback)?;
        props.set_value_bykey(
            Some(&cef::CefString::from(name.as_str())),
            Some(&mut function),
            cef::V8Propertyattribute::default(),
        );
    }
    let mut props = props;
    set("props", &mut props);

    let mut children = describe(&node.children)?;
    set("children", &mut children);
    Some(object)
}

/// A component of the page, written inside a component of a library.
///
/// `<Card>{Resume()}</Card>`. What crosses is a description of the same
/// shape a native tag below a boundary already crosses as — React
/// builds the tree and reconciles it — because everything below a
/// boundary is React's, and that is the rule the boundary exists for.
///
/// A handler crosses as a function, like a library's callback: a
/// `<button onClick={…}>` written inside a `<Card>` has to work, and it
/// is the same closure either way.
fn describe_element(element: &rustx_dom::Element) -> Option<V8Value> {
    use rustx_dom::{AttributeValue, Element};

    match element {
        Element::Text(text) => crate::value::into_v8(Value::String(text.clone())),
        // React renders `null` as nothing, and an array as its items.
        Element::Empty => crate::value::into_v8(Value::Null),
        Element::Fragment(children) => {
            let array = cef::v8_value_create_array(children.len() as i32)?;
            for (at, child) in children.iter().enumerate() {
                let mut item = describe_element(child)?;
                array.set_value_byindex(at as i32, Some(&mut item));
            }
            Some(array)
        }
        Element::Foreign { node, .. } => describe_node(node),
        Element::Tag(tag) => {
            let object = cef::v8_value_create_object(None, None)?;
            let set = |name: &str, value: &mut V8Value| {
                object.set_value_bykey(
                    Some(&cef::CefString::from(name)),
                    Some(value),
                    cef::V8Propertyattribute::default(),
                );
            };

            let mut component = crate::value::into_v8(Value::String(tag.tag.clone()))?;
            set("component", &mut component);

            // `class` is what the DOM calls it and `className` is what
            // React calls it. The page wrote one of the two; React has
            // to be given its own name or the styles do not arrive.
            let named: Vec<(String, Value)> = tag
                .attributes
                .iter()
                .map(|(name, value)| {
                    let name = match name.as_str() {
                        "class" => "className".to_string(),
                        "for" => "htmlFor".to_string(),
                        other => other.to_string(),
                    };
                    let value = match value {
                        AttributeValue::Text(text) => Value::String(text.clone()),
                        AttributeValue::Flag(flag) => Value::Bool(*flag),
                    };
                    (name, value)
                })
                .collect();
            let props = crate::value::object_of(&named)?;
            for (name, handler) in &tag.handlers {
                let mut function = function_for_handler(name, handler)?;
                props.set_value_bykey(
                    Some(&cef::CefString::from(name.as_str())),
                    Some(&mut function),
                    cef::V8Propertyattribute::default(),
                );
            }
            // A key, where React wants one: a list of rows written
            // inside a library's component is reordered by React, and
            // without the key it reorders the state with the position.
            if let Some(key) = tag.key.as_ref() {
                let mut key = crate::value::into_v8(Value::String(key.as_str().to_string()))?;
                props.set_value_bykey(
                    Some(&cef::CefString::from("key")),
                    Some(&mut key),
                    cef::V8Propertyattribute::default(),
                );
            }
            let mut props = props;
            set("props", &mut props);

            let array = cef::v8_value_create_array(tag.children.len() as i32)?;
            for (at, child) in tag.children.iter().enumerate() {
                let mut item = describe_element(child)?;
                array.set_value_byindex(at as i32, Some(&mut item));
            }
            let mut children = array;
            set("children", &mut children);
            Some(object)
        }
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
    for (node, container) in waiting {
        hand_over(&node, &container);
    }
}

// ---------------------------------------------------------------------
// A prop a library calls.
// ---------------------------------------------------------------------

use cef::{
    CefString, ImplV8Handler, ImplV8Value, V8Handler, V8Value, WrapV8Handler, rc::Rc as _,
    wrap_v8_handler,
};
use rustx_dom::Callback;

wrap_v8_handler! {
    struct Called {
        callback: Callback,
    }

    impl V8Handler {
        /// The library called the prop.
        ///
        /// Whatever it passed arrives as values; what cannot cross — an
        /// event object, a DOM node, another function — arrives as null,
        /// because a callback that refused the call would be a library
        /// that stops working halfway through.
        fn execute(
            &self,
            _name: Option<&CefString>,
            _object: Option<&mut V8Value>,
            arguments: Option<&[Option<V8Value>]>,
            _retval: Option<&mut Option<V8Value>>,
            _exception: Option<&mut CefString>,
        ) -> ::std::os::raw::c_int {
            let given: Vec<rustx_dom::Value> = arguments
                .unwrap_or(&[])
                .iter()
                .map(|argument| {
                    argument
                        .as_ref()
                        .and_then(crate::value::read_or_null)
                        .unwrap_or(rustx_dom::Value::Null)
                })
                .collect();

            let callback = self.callback.clone();
            // Called from C: a panic in the application's closure stops
            // here, the way it does in an event handler.
            crate::panics::contain("a callback a library called", move || {
                callback.call(&given);
            });
            1
        }
    }
}

/// The function a library is given for one of its `on…` props.
pub(crate) fn function_for(name: &str, callback: &Callback) -> Option<V8Value> {
    let mut handler = Called::new(callback.clone());
    cef::v8_value_create_function(Some(&CefString::from(name)), Some(&mut handler))
}

/// The same, for the handler of a tag of the page described to React.
///
/// A `<button onClick={…}>` written inside a `<Card>` is React's to
/// build, so its handler has to cross as a React prop rather than be
/// attached to a node — but it is the same closure, and it is given the
/// same `Event` a handler on a tag rustx owns is given. What React
/// passes is ignored: an event object does not cross, and the kind is
/// known from the name.
fn function_for_handler(name: &str, handler: &rustx_dom::Handler) -> Option<V8Value> {
    let kind = match name {
        "onDoubleClick" => "dblclick".to_string(),
        other => other.strip_prefix("on").unwrap_or(other).to_lowercase(),
    };
    let handler = handler.clone();
    let callback = Callback::new(move |_| {
        handler.call(rustx_dom::Event { kind: kind.clone() });
    });
    function_for(name, &callback)
}
