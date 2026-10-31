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
    match value {
        AttributeValue::Text(text) => node.set_attribute(name, text),
        // A flag is present or absent; `disabled="false"` is still
        // disabled in HTML, so absence is the only way to say no.
        AttributeValue::Flag(true) => node.set_attribute(name, ""),
        AttributeValue::Flag(false) => node.remove_attribute(name),
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
            _arguments: Option<&[Option<V8Value>]>,
            _retval: Option<&mut Option<V8Value>>,
            _exception: Option<&mut CefString>,
        ) -> ::std::os::raw::c_int {
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
                    handler.call(Event { kind });
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
