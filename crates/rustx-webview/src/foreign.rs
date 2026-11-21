//! Handing a node to React, and taking it back.
//!
//! rustx builds the container, says which React component goes in it and
//! with what props, and does not look inside. What crosses to the page is
//! a command (`fm` to mount or update, `fu` to let go) carrying the props
//! and the children as JSON. A function in them — a library's `on…` prop,
//! or the handler of a tag written inside a component — crosses as
//! `{"__rustx_cb": id}`; the page turns it back into a function that
//! reports its call, and `called` runs the closure it stands for.
//!
//! The page's half is `window.__rustx_react = { mount, unmount }`, the
//! framework's own `runtime.js`. `mount` is called again whenever the
//! props change; React's reconciler decides that it is an update.
//! `unmount` is sent **before** the container leaves the document, and
//! commands are applied in order, so React's cleanups still run.
//!
//! Not carried over from the CEF engine: a callback's return value. A call
//! is a message away, so a prop that must answer (a formatter, a render
//! prop) answers nothing; on CEF it never did either.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use rustx_dom::{
    AttributeValue, Callback, Element, Event, ForeignChild, ForeignNode, Handler, Value,
};
use serde_json::{Value as Json, json};

use crate::dom::Node;
use crate::protocol::value_to_json;

type Builder = Rc<dyn Fn() -> Element>;

thread_local! {
    static HANDED_OVER: Cell<u64> = const { Cell::new(0) };
    static GIVEN_BACK: Cell<u64> = const { Cell::new(0) };
    static COMPLAINT: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The closures the page may call, by the number it was given.
    static CALLBACKS: RefCell<HashMap<u64, Callback>> = RefCell::new(HashMap::new());
    /// The numbers a container holds, by the place of the function in its
    /// description. A render keeps the number of a function that is still
    /// there and replaces what it calls: a click the page posted with the
    /// number it was last given still reaches the latest closure, instead
    /// of a number freed by a render the page has not applied yet. A
    /// container's numbers are freed when it is given back.
    static OWNED: RefCell<HashMap<u64, HashMap<String, u64>>> = RefCell::new(HashMap::new());
    /// What the page was last told for a container: sent again only when it
    /// differs, so a render that only replaces closures costs the page nothing.
    static SENT: RefCell<HashMap<u64, String>> = RefCell::new(HashMap::new());
    static NEXT: Cell<u64> = const { Cell::new(1) };
    /// The components an application is willing to let React mount.
    static COMPONENTS: RefCell<HashMap<String, Builder>> = RefCell::new(HashMap::new());
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

/// How many closures the page may call right now.
pub fn callbacks_held() -> usize {
    CALLBACKS.with(|held| held.borrow().len())
}

fn complain(message: String) {
    eprintln!("{message}");
    COMPLAINT.with(|complaint| *complaint.borrow_mut() = Some(message));
}

/// Give `container` to the React component called `component`. Called
/// again with new props on every change.
pub(crate) fn hand_over(node: &ForeignNode, container: &Node) {
    let Some((props, children)) = describe_for(container.id(), node) else {
        return;
    };
    let sent = format!("{}|{props}|{children}", node.component);
    if SENT.with(|held| held.borrow().get(&container.id()) == Some(&sent)) {
        return;
    }
    SENT.with(|held| held.borrow_mut().insert(container.id(), sent));
    crate::dom::command(json!([
        "fm",
        container.id(),
        node.component,
        props,
        children
    ]));
    HANDED_OVER.with(|count| count.set(count.get() + 1));
}

/// Tell React to let `container` go, while it is still in the document.
pub(crate) fn give_back(container: &Node) {
    let held = OWNED
        .with(|held| held.borrow_mut().remove(&container.id()))
        .unwrap_or_default();
    CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        for id in held.values() {
            callbacks.remove(id);
        }
    });
    SENT.with(|held| held.borrow_mut().remove(&container.id()));
    crate::dom::command(json!(["fu", container.id()]));
    GIVEN_BACK.with(|count| count.set(count.get() + 1));
}

/// A boundary that was rendered before the page had a bridge: none. The
/// page keeps the ones whose component is not there yet, and tries again.
pub(crate) fn hand_over_pending() {}

/// The numbers of the functions of one hand-over: those the container
/// held before, and those this render gave.
struct Ids {
    before: HashMap<String, u64>,
    now: HashMap<String, u64>,
}

/// The props and the children to send for `node` in the container
/// numbered `container`, and the bookkeeping of its functions' numbers.
fn describe_for(container: u64, node: &ForeignNode) -> Option<(Json, Json)> {
    let mut ids = Ids {
        before: OWNED
            .with(|held| held.borrow().get(&container).cloned())
            .unwrap_or_default(),
        now: HashMap::new(),
    };
    let described = props_of(&node.props, &node.callbacks, &mut ids)
        .ok_or("the props of a node React owns could not be built")
        .and_then(|props| {
            describe(&node.children, &mut ids)
                .map(|children| (props, children))
                .ok_or("what is inside a node React owns could not be built")
        });
    let (props, children) = match described {
        Ok(built) => built,
        Err(why) => {
            // The page keeps the last render that worked, with its numbers;
            // only the ones this attempt made up are let go.
            CALLBACKS.with(|held| {
                let mut held = held.borrow_mut();
                for id in ids
                    .now
                    .values()
                    .filter(|id| !ids.before.values().any(|kept| kept == *id))
                {
                    held.remove(id);
                }
            });
            complain(format!("rustx: {why}"));
            return None;
        }
    };
    // A function that is no longer there loses its number.
    CALLBACKS.with(|held| {
        let mut held = held.borrow_mut();
        for (key, id) in &ids.before {
            if !ids.now.contains_key(key) {
                held.remove(id);
            }
        }
    });
    OWNED.with(|held| held.borrow_mut().insert(container, ids.now));
    Some((props, children))
}

/// The page called a function it was given. A number nobody holds any more
/// is ignored: React may call an old prop while it unmounts.
pub(crate) fn called(id: u64, args: &[Value]) {
    let Some(callback) = CALLBACKS.with(|held| held.borrow().get(&id).cloned()) else {
        return;
    };
    crate::panics::contain("a callback a library called", move || callback.call(args));
}

/// A closure, as what stands for it on the page. Its place is the
/// ordinal of the function in the description and the name of the prop:
/// the same place keeps its number from one render to the next.
fn function(callback: Callback, name: &str, ids: &mut Ids) -> Json {
    let key = format!("{}:{name}", ids.now.len());
    let id = ids.before.get(&key).copied().unwrap_or_else(|| {
        NEXT.with(|next| {
            let id = next.get();
            next.set(id + 1);
            id
        })
    });
    CALLBACKS.with(|held| held.borrow_mut().insert(id, callback));
    ids.now.insert(key, id);
    json!({ "__rustx_cb": id })
}

fn props_of(
    props: &[(String, Value)],
    callbacks: &[(String, Callback)],
    ids: &mut Ids,
) -> Option<Json> {
    let mut object = serde_json::Map::new();
    for (name, value) in props {
        object.insert(name.clone(), value_to_json(value, 0).ok()?);
    }
    for (name, callback) in callbacks {
        object.insert(name.clone(), function(callback.clone(), name, ids));
    }
    Some(Json::Object(object))
}

/// What is inside a boundary, as the page can read it: `{ component,
/// props, children }` for a component, plain values for text.
fn describe(children: &[ForeignChild], ids: &mut Ids) -> Option<Json> {
    children
        .iter()
        .map(|child| match child {
            ForeignChild::Text(text) => Some(Json::String(text.clone())),
            ForeignChild::Value(value) => value_to_json(value, 0).ok(),
            ForeignChild::Node(node) => describe_node(node, ids),
            ForeignChild::Element(element) => describe_element(element, ids),
        })
        .collect::<Option<Vec<_>>>()
        .map(Json::Array)
}

fn describe_node(node: &ForeignNode, ids: &mut Ids) -> Option<Json> {
    Some(json!({
        "component": node.component,
        "props": props_of(&as_react_reads_them(&node.props), &node.callbacks, ids)?,
        "children": describe(&node.children, ids)?,
    }))
}

/// The props of a node, with `style` as React reads it: a mapping, where
/// the DOM writes a string. React throws on the string.
fn as_react_reads_them(props: &[(String, Value)]) -> Vec<(String, Value)> {
    props
        .iter()
        .map(|(name, value)| match (name.as_str(), value) {
            ("style", Value::String(text)) => (name.clone(), rustx_dom::value::style_object(text)),
            _ => (name.clone(), value.clone()),
        })
        .collect()
}

/// A component of the page, or a plain tag, written inside a component of
/// a library: described the way a node below a boundary is, because
/// everything below a boundary is React's.
fn describe_element(element: &Element, ids: &mut Ids) -> Option<Json> {
    match element {
        Element::Text(text) => Some(Json::String(text.clone())),
        Element::Empty => Some(Json::Null),
        Element::Fragment(children) => children
            .iter()
            .map(|child| describe_element(child, ids))
            .collect::<Option<Vec<_>>>()
            .map(Json::Array),
        Element::Foreign { node, .. } => describe_node(node, ids),
        Element::Tag(tag) => {
            let mut props = serde_json::Map::new();
            for (name, value) in &tag.attributes {
                // `class` is what the DOM calls it, `className` what React does.
                let name = match name.as_str() {
                    "class" => "className",
                    "for" => "htmlFor",
                    other => other,
                };
                let value = match value {
                    AttributeValue::Text(text) if name == "style" => {
                        value_to_json(&rustx_dom::value::style_object(text), 0).ok()?
                    }
                    AttributeValue::Text(text) => Json::String(text.clone()),
                    AttributeValue::Flag(flag) => Json::Bool(*flag),
                };
                props.insert(name.to_string(), value);
            }
            for (name, handler) in &tag.handlers {
                props.insert(
                    name.clone(),
                    function(handler_callback(name, handler), name, ids),
                );
            }
            // Where React wants a key, it is given one: it reorders the
            // state with the row, not with the position.
            if let Some(key) = tag.key.as_ref() {
                props.insert("key".to_string(), Json::String(key.as_str().to_string()));
            }
            let children = tag
                .children
                .iter()
                .map(|child| describe_element(child, ids))
                .collect::<Option<Vec<_>>>()?;
            Some(json!({ "component": tag.tag, "props": props, "children": children }))
        }
    }
}

/// The closure of a handler written on a tag inside a component. The page
/// sends the event's fields as it does for a tag rustx owns, so the
/// handler is given an `Event` that holds what was typed.
fn handler_callback(name: &str, handler: &Handler) -> Callback {
    let kind = match name {
        "onDoubleClick" => "dblclick".to_string(),
        other => other.strip_prefix("on").unwrap_or(other).to_lowercase(),
    };
    let handler = handler.clone();
    Callback::new(move |args| {
        let event = match args.first() {
            Some(record @ Value::Record(_)) => Event::from_value(record),
            _ => Event::default(),
        };
        handler.call(Event {
            kind: kind.clone(),
            ..event
        });
    })
}

// ---------------------------------------------------------------------
// The other direction: a rustx component inside a node React owns.
// ---------------------------------------------------------------------

/// Let the page mount `component` by that name.
///
/// What React gets is not a snapshot: the component becomes a root of its
/// own, drawn by the same frame as everything else, with its own cells,
/// effects and tasks. Releasing the node releases all of it.
pub fn register_component(name: &str, component: impl Fn() -> Element + 'static) {
    COMPONENTS.with(|components| {
        components
            .borrow_mut()
            .insert(name.to_string(), Rc::new(component))
    });
}

/// Mount the registered component `name` under the element `id`, which the
/// page created after it said it was ready.
pub(crate) fn mount_registered(name: &str, id: &str) -> Result<(), crate::NativeError> {
    let Some(component) = COMPONENTS.with(|components| components.borrow().get(name).cloned())
    else {
        return Err(crate::NativeError::new(format!(
            "rustx: no component named '{name}' has been registered — an application says what \
             React may mount by calling `register_component`"
        )));
    };
    // The page is asking for the element, so it is there.
    crate::dom::vouch(id);
    crate::render::mount_forgotten(move || component(), id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundary(clicks: Rc<Cell<u32>>) -> ForeignNode {
        let counted = Callback::new(move |_| clicks.set(clicks.get() + 1));
        ForeignNode {
            component: "ui::Button".to_string(),
            props: vec![("id".to_string(), Value::String("add".to_string()))],
            callbacks: vec![("onClick".to_string(), counted)],
            children: Vec::new(),
        }
    }

    fn number_in(props: &Json) -> u64 {
        props["onClick"]["__rustx_cb"]
            .as_u64()
            .expect("onClick crosses as a function")
    }

    #[test]
    fn a_hundred_renders_leave_the_table_as_it_was_and_the_number_the_same() {
        let clicks = Rc::new(Cell::new(0));
        let (first, _) = describe_for(900, &boundary(clicks.clone())).expect("described");
        let number = number_in(&first);
        let held = callbacks_held();
        for _ in 0..100 {
            let (props, _) = describe_for(900, &boundary(clicks.clone())).expect("described");
            assert_eq!(
                number_in(&props),
                number,
                "a click posted with the old number is lost"
            );
        }
        assert_eq!(callbacks_held(), held, "the table grows with every render");
        called(number, &[]);
        assert_eq!(clicks.get(), 1, "the number did not reach the closure");
    }

    #[test]
    fn giving_a_container_back_frees_its_numbers_and_a_stale_call_is_ignored() {
        let clicks = Rc::new(Cell::new(0));
        let before = callbacks_held();
        let (props, _) = describe_for(901, &boundary(clicks.clone())).expect("described");
        let number = number_in(&props);
        assert_eq!(callbacks_held(), before + 1);
        // What `give_back` does to the table, without a page to tell.
        let held = OWNED
            .with(|held| held.borrow_mut().remove(&901))
            .unwrap_or_default();
        CALLBACKS.with(|callbacks| {
            held.values()
                .for_each(|id| drop(callbacks.borrow_mut().remove(id)))
        });
        assert_eq!(callbacks_held(), before);
        called(number, &[]);
        assert_eq!(clicks.get(), 0, "a freed number still ran a closure");
    }
}
