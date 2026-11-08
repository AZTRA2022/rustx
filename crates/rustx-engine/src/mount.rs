//! Putting an element tree into the document.
//!
//! A depth-first walk, and nothing more: the tree already says what the
//! interface is, so mounting makes no decisions. Names were translated by
//! the compiler, which owns that table; here a tag becomes an element, an
//! attribute becomes an attribute, and the order children were written in
//! is the order they are attached in.
//!
//! A handler is attached as a real V8 function built around the Rust
//! closure, handed straight to `addEventListener`. Nothing is evaluated
//! as JavaScript, nothing is registered under a generated global name,
//! and the closure the developer wrote is what runs — in this process, on
//! this thread, on the click's own stack. The listener reads the handler
//! out of a slot, so a re-render replaces the closure without ever
//! needing to take a listener off a node; see [`crate::tree`].

use std::cell::RefCell;
use std::rc::Rc;

use cef::{
    CefString, ImplV8Handler, V8Handler, V8Value, WrapV8Handler, rc::Rc as _, wrap_v8_handler,
};
use rustx_dom::{AttributeValue, Element, Event, TagElement};

use crate::dom::{Document, DomError, Node};
use crate::tree::{HandlerSlot, Mounted, MountedNode};
use crate::value::Value;

/// Attach `element` under `into`, forgetting what was built.
///
/// The shape plan 2 shipped, kept because a page that never re-renders
/// has no use for the bookkeeping.
pub fn mount(element: Element, into: &Node) -> Result<(), DomError> {
    let document = Document::current().ok_or(DomError::NoContext)?;
    build_into(&document, &element, into)?;
    Ok(())
}

/// Attach `element` under the element carrying this `id`.
pub fn mount_at_id(element: Element, id: &str) -> Result<(), DomError> {
    let root = root_node(id)?;
    mount(element, &root)
}

/// Attach `element` and remember everything about it, so that the next
/// render can change only what changed.
pub fn mount_tree(element: Element, into: &Node) -> Result<Mounted, DomError> {
    let document = Document::current().ok_or(DomError::NoContext)?;
    let nodes = build_into(&document, &element, into)?;
    Ok(Mounted {
        root: into.clone(),
        tree: element,
        nodes,
    })
}

/// The element a tree is mounted under.
pub fn root_node(id: &str) -> Result<Node, DomError> {
    let document = Document::current().ok_or(DomError::NoContext)?;
    document
        .get_element_by_id(id)?
        .ok_or_else(|| DomError::Failed(format!("no #{id} in the page")))
}

/// Build `element` and attach it under `parent`, returning what was made.
///
/// The subtree is built whole before it is attached, so a failure
/// partway through leaves nothing behind for the next attempt to
/// duplicate.
pub(crate) fn build_into(
    document: &Document,
    element: &Element,
    parent: &Node,
) -> Result<MountedNode, DomError> {
    match element {
        Element::Empty => Ok(MountedNode::empty()),
        Element::Text(text) => {
            let node = document.create_text_node(text)?;
            parent.append_child(&node)?;
            Ok(MountedNode {
                dom: Some(node),
                handlers: Vec::new(),
                children: Vec::new(),
                foreign: None,
            })
        }
        // A fragment has no node of its own: its children become the
        // parent's children, side by side.
        Element::Fragment(children) => {
            let mut mounted = MountedNode::empty();
            for child in children {
                mounted.children.push(build_into(document, child, parent)?);
            }
            Ok(mounted)
        }
        // The container, and nothing in it: what goes inside belongs to
        // React, and the handover is `crate::foreign`'s business.
        Element::Foreign { node: foreign, id } => {
            let node = document.create_element("div")?;
            node.set_attribute("id", id)?;
            // Attached first: React is handed a node that is in the
            // document, which is the only kind it can measure or focus.
            parent.append_child(&node)?;
            crate::foreign::hand_over(foreign, &node);
            Ok(MountedNode {
                dom: Some(node),
                handlers: Vec::new(),
                children: Vec::new(),
                foreign: Some(foreign.component.clone()),
            })
        }
        Element::Tag(tag) => {
            let (node, mounted) = build_tag(document, tag)?;
            parent.append_child(&node)?;
            Ok(mounted)
        }
    }
}

fn build_tag(document: &Document, tag: &TagElement) -> Result<(Node, MountedNode), DomError> {
    let node = document.create_element(&tag.tag)?;

    for (name, value) in &tag.attributes {
        set_attribute(&node, name, value)?;
    }

    let mut handlers = Vec::new();
    for (name, handler) in &tag.handlers {
        let slot: HandlerSlot = Rc::new(RefCell::new(Some(handler.clone())));
        listen(&node, name, Rc::clone(&slot))?;
        handlers.push((name.clone(), slot));
    }

    let mut children = Vec::new();
    for child in &tag.children {
        children.push(build_into(document, child, &node)?);
    }

    Ok((
        node.clone(),
        MountedNode {
            dom: Some(node),
            handlers,
            children,
            foreign: None,
        },
    ))
}

pub(crate) fn set_attribute(
    node: &Node,
    name: &str,
    value: &AttributeValue,
) -> Result<(), DomError> {
    // `value` and `checked` are not attributes once somebody has typed
    // or clicked: the attribute says what the field *started* as, and
    // the property says what it *is*. Setting the attribute alone meant
    // a field the page cleared — `terme = String::new()` behind a
    // « vider » button — kept what was in it, and the only thing that
    // looked wrong was the application.
    if is_a_property(name) {
        set_property_too(node, name, value)?;
    }
    match value {
        AttributeValue::Text(text) => node.set_attribute(name, text),
        // A flag is present or absent; `disabled="false"` is still
        // disabled in HTML, so absence is the only way to say no.
        AttributeValue::Flag(true) => node.set_attribute(name, ""),
        AttributeValue::Flag(false) => node.remove_attribute(name),
    }
}

/// The attributes that a form element carries as a live property too.
fn is_a_property(name: &str) -> bool {
    matches!(name, "value" | "checked" | "selected")
}

/// Set the property as well as the attribute, where the element has
/// one.
///
/// The tag is asked for, because `.value` on a `<div>` would be a
/// property nobody reads and a lie in anybody's debugger. One extra
/// call through V8, and only for the three names above.
fn set_property_too(node: &Node, name: &str, value: &AttributeValue) -> Result<(), DomError> {
    let tag = node.tag_name().unwrap_or_default();
    let has_one = matches!(
        tag.as_str(),
        "INPUT" | "TEXTAREA" | "SELECT" | "OPTION" | "PROGRESS" | "METER"
    );
    if !has_one {
        return Ok(());
    }
    match value {
        AttributeValue::Text(text) => node.set_text_property(name, text),
        AttributeValue::Flag(whether) => node.set_flag_property(name, *whether),
    }
}

// The V8 side of one `onClick`.
//
// A native function validates its arguments, because JavaScript wrote
// them. An event handler does not: the argument is the DOM's own event
// object, which no `Value` could ever hold, and refusing it would mean
// refusing every click.
wrap_v8_handler! {
    struct EventHandler {
        kind: Rc<str>,
        slot: HandlerSlot,
    }

    impl V8Handler {
        fn execute(
            &self,
            _name: Option<&CefString>,
            _object: Option<&mut V8Value>,
            arguments: Option<&[Option<V8Value>]>,
            _retval: Option<&mut Option<V8Value>>,
            _exception: Option<&mut CefString>,
        ) -> ::std::os::raw::c_int {
            // The DOM event is the first argument, and what a page asks
            // of it is a short list: what was typed, which element it
            // was, which key, which modifiers. The event object itself
            // cannot cross — it holds DOM nodes — so the list is read
            // here and the rest is left behind.
            let event = arguments
                .and_then(|arguments| arguments.first())
                .and_then(Option::as_ref)
                .map(read_event)
                .unwrap_or_default();
            let kind = self.kind.to_string();
            let slot = Rc::clone(&self.slot);
            // This frame is called from C, so a panic in the
            // application's closure stops here.
            crate::panics::contain("an event handler", move || {
                // Cloned out of the slot before calling, so a handler
                // that re-renders — and so replaces itself — does not
                // find the slot borrowed when it returns.
                let handler = slot.borrow().clone();
                if let Some(handler) = handler {
                    handler.call(Event { kind, ..event });
                }
            });
            1
        }
    }
}

/// `onClick` → `addEventListener("click", <the slot's current handler>)`.
fn listen(node: &Node, name: &str, slot: HandlerSlot) -> Result<(), DomError> {
    let kind = event_kind(name);

    let mut v8_handler = EventHandler::new(Rc::from(kind.as_str()), slot);
    let callback =
        cef::v8_value_create_function(Some(&CefString::from(name)), Some(&mut v8_handler))
            .ok_or_else(|| DomError::Failed(format!("could not build a function for {name}")))?;

    let kind_value = crate::value::into_v8(Value::String(kind))
        .ok_or_else(|| DomError::Failed("could not build the event name".into()))?;

    node.call_with_v8("addEventListener", vec![Some(kind_value), Some(callback)])
}

/// What the page may know about a DOM event.
///
/// Read field by field rather than converted whole: an event holds DOM
/// nodes, and a node does not cross. `target` is read once and asked
/// three questions, because `event.target` on a DOM event is a getter
/// and calling it four times is four trips through V8 for nothing.
///
/// `pub(crate)`, parce que l'autre chemin en a besoin : une balise
/// ordinaire ecrite dans un sous-arbre React est montee par React, et
/// son `onInput` revient par les props. Ce qu'une closure recoit doit
/// etre le meme evenement -- deux lectures separees finiraient par ne
/// plus dire la meme chose.
pub(crate) fn read_event(event: &V8Value) -> Event {
    let target = field(event, "target");
    Event {
        kind: String::new(),
        value: target.as_ref().map(|target| text(target, "value")).unwrap_or_default(),
        id: target.as_ref().map(|target| text(target, "id")).unwrap_or_default(),
        key: text(event, "key"),
        checked: target
            .as_ref()
            .map(|target| flag(target, "checked"))
            .unwrap_or_default(),
        shift: flag(event, "shiftKey"),
        control: flag(event, "ctrlKey"),
        alt: flag(event, "altKey"),
        meta: flag(event, "metaKey"),
    }
}

/// One field of a JavaScript object, if it has one.
fn field(object: &V8Value, name: &str) -> Option<V8Value> {
    use cef::ImplV8Value;
    object.value_bykey(Some(&CefString::from(name)))
}

/// A field as text. A number or a boolean is written out, because an
/// `onInput` on `<input type="number">` is the ordinary case and the
/// page asked for what is in the field.
fn text(object: &V8Value, name: &str) -> String {
    use cef::ImplV8Value;
    let Some(held) = field(object, name) else {
        return String::new();
    };
    if held.is_string() != 0 {
        return CefString::from(&held.string_value()).to_string();
    }
    if held.is_int() != 0 {
        return held.int_value().to_string();
    }
    if held.is_double() != 0 {
        return held.double_value().to_string();
    }
    if held.is_bool() != 0 {
        return held.bool_value().to_string();
    }
    String::new()
}

/// A field as a boolean, false when it is not there.
fn flag(object: &V8Value, name: &str) -> bool {
    use cef::ImplV8Value;
    field(object, name).is_some_and(|held| held.is_bool() != 0 && held.bool_value() != 0)
}

/// Cet objet est-il un evenement ?
///
/// La question se pose parce qu'une bibliothèque appelle ses props avec
/// ce qu'elle veut : `onValueChange("clair")` passe une chaine, et
/// `onClick` passe l'evenement de React. Un evenement a un `type` et une
/// cible ; c'est ce qu'on regarde, et pas le nom de la prop, que
/// personne ne garantit.
pub(crate) fn is_an_event(object: &V8Value) -> bool {
    use cef::ImplV8Value;
    if object.is_object() == 0 {
        return false;
    }
    let has = |name: &str| field(object, name).is_some_and(|held| held.is_undefined() == 0);
    has("type") && (has("target") || has("currentTarget") || has("nativeEvent"))
}

/// `onDoubleClick` is `dblclick` to the DOM; everything else is the name
/// with its `on` removed and the rest lowercased.
fn event_kind(name: &str) -> String {
    match name {
        "onDoubleClick" => "dblclick".to_string(),
        other => other.strip_prefix("on").unwrap_or(other).to_lowercase(),
    }
}

/// Attach a listener to a node that is already on screen, for a handler
/// it did not have when it was built.
pub(crate) fn listen_later(node: &Node, name: &str) -> Result<HandlerSlot, DomError> {
    let slot: HandlerSlot = Rc::new(RefCell::new(None));
    listen(node, name, Rc::clone(&slot))?;
    Ok(slot)
}

#[cfg(test)]
mod tests {
    use super::event_kind;

    #[test]
    fn a_react_event_name_becomes_a_dom_event_name() {
        assert_eq!(event_kind("onClick"), "click");
        assert_eq!(event_kind("onMouseEnter"), "mouseenter");
        assert_eq!(event_kind("onDoubleClick"), "dblclick");
    }
}
