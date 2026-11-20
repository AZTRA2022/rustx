//! Putting an element tree into the document.
//!
//! A depth-first walk, and nothing more: the tree already says what the
//! interface is, so mounting makes no decisions. Names were translated by
//! the compiler, which owns that table; here a tag becomes an element, an
//! attribute becomes an attribute, and the order children were written in
//! is the order they are attached in.
//!
//! A handler is one listener per node and event kind, for as long as the
//! node lives, and the listener reads the current handler out of a slot:
//! replacing a handler on a re-render is a write to a cell, not a
//! conversation with the page. The page only says *which node* and
//! *which kind*; the slot, and so the closure, never leaves this process.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use rustx_dom::{AttributeValue, Element, Event, Handler, TagElement, Value};

use crate::dom::{Document, DomError, Node};
use crate::tree::{HandlerSlot, Mounted, MountedNode};

thread_local! {
    /// The slot behind each (node, event kind). Weak: the slot belongs to
    /// the `MountedNode`, so when the bookkeeping lets a node go its
    /// events stop reaching anything, with nothing to unregister.
    static SLOTS: RefCell<HashMap<(u64, String), Weak<RefCell<Option<Handler>>>>> =
        RefCell::new(HashMap::new());
    /// The size at which the table is next swept for dead slots.
    static SWEEP_AT: std::cell::Cell<usize> = const { std::cell::Cell::new(1024) };
}

/// Attach `element` under `into`, forgetting what was built.
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

    // What a field *holds* is set last: a `<select>` has no option to choose
    // before its children exist, and a range clamps `value` to the `min` and
    // `max` it has so far. Written as `value` first, as people do, the field
    // would otherwise show the first option and the default range.
    for (name, value) in tag.attributes.iter().filter(|(name, _)| !is_a_property(name)) {
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
    for (name, value) in tag.attributes.iter().filter(|(name, _)| is_a_property(name)) {
        set_attribute(&node, name, value)?;
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
    // `value` and `checked` are not attributes once somebody has typed or
    // clicked: the attribute says what the field *started* as, and the
    // property says what it *is*.
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

/// Set the property as well as the attribute, where the element has one.
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

/// `onDoubleClick` is `dblclick` to the DOM; everything else is the name
/// with its `on` removed and the rest lowercased.
fn event_kind(name: &str) -> String {
    match name {
        "onDoubleClick" => "dblclick".to_string(),
        other => other.strip_prefix("on").unwrap_or(other).to_lowercase(),
    }
}

/// `onClick` → one `click` listener on the node, reading `slot`.
fn listen(node: &Node, name: &str, slot: HandlerSlot) -> Result<(), DomError> {
    let kind = event_kind(name);
    let first = SLOTS.with(|slots| {
        slots
            .borrow_mut()
            .insert((node.id(), kind.clone()), Rc::downgrade(&slot))
            .is_none()
    });
    if first {
        node.listen(&kind)?;
    }
    sweep_if_large();
    Ok(())
}

/// Attach a listener to a node that is already on screen, for a handler
/// it did not have when it was built.
pub(crate) fn listen_later(node: &Node, name: &str) -> Result<HandlerSlot, DomError> {
    let slot: HandlerSlot = Rc::new(RefCell::new(None));
    listen(node, name, Rc::clone(&slot))?;
    Ok(slot)
}

/// Drop the entries whose slot has gone, when the table has doubled since
/// the last sweep. Amortised: a page that never removes a node never pays.
fn sweep_if_large() {
    SLOTS.with(|slots| {
        let mut slots = slots.borrow_mut();
        if slots.len() < SWEEP_AT.with(std::cell::Cell::get) {
            return;
        }
        slots.retain(|_, slot| slot.strong_count() > 0);
        SWEEP_AT.with(|at| at.set((slots.len() * 2).max(1024)));
    });
}

/// An event the page read, delivered to the handler its node holds.
///
/// `false` when nothing is listening any more: the node was removed while
/// the event was on its way, which is ordinary and not an error. The
/// handler is cloned out of its slot before it is called, so one that
/// re-renders, and so replaces itself, does not find the slot borrowed.
pub(crate) fn dispatch(node: u64, kind: &str, event: &Value, seq: u64) -> bool {
    let slot = SLOTS.with(|slots| {
        slots
            .borrow()
            .get(&(node, kind.to_string()))
            .and_then(Weak::upgrade)
    });
    let Some(slot) = slot else {
        return false;
    };
    // Before the handler runs, so what it writes carries this number.
    if seq > 0 {
        crate::dom::acknowledge(node, seq);
    }
    let handler = slot.borrow().clone();
    let Some(handler) = handler else {
        return false;
    };
    let event = Event {
        kind: kind.to_string(),
        ..Event::from_value(event)
    };
    crate::panics::contain("an event handler", move || handler.call(event));
    true
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
