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
//! this thread, on the click's own stack.

use std::rc::Rc;

use cef::{CefString, ImplV8Handler, V8Handler, V8Value, WrapV8Handler, rc::Rc as _, wrap_v8_handler};
use rustx_dom::{AttributeValue, Element, Event, Handler, TagElement};

use crate::dom::{Document, DomError, Node};
use crate::value::Value;

/// Attach `element` under `into`.
pub fn mount(element: Element, into: &Node) -> Result<(), DomError> {
    let document = Document::current().ok_or(DomError::NoContext)?;
    mount_in(&document, element, into)
}

/// Attach `element` under the element carrying this `id`.
pub fn mount_at_id(element: Element, id: &str) -> Result<(), DomError> {
    let document = Document::current().ok_or(DomError::NoContext)?;
    let root = document
        .get_element_by_id(id)?
        .ok_or_else(|| DomError::Failed(format!("no #{id} in the page")))?;
    mount_in(&document, element, &root)
}

fn mount_in(document: &Document, element: Element, into: &Node) -> Result<(), DomError> {
    match element {
        Element::Empty => Ok(()),
        Element::Text(text) => {
            let node = document.create_text_node(&text)?;
            into.append_child(&node)
        }
        // A fragment has no node of its own: its children become the
        // parent's children, side by side.
        Element::Fragment(children) => {
            for child in children {
                mount_in(document, child, into)?;
            }
            Ok(())
        }
        Element::Tag(tag) => {
            let node = build_tag(document, tag)?;
            into.append_child(&node)
        }
    }
}

fn build_tag(document: &Document, tag: TagElement) -> Result<Node, DomError> {
    let node = document.create_element(&tag.tag)?;

    for (name, value) in tag.attributes {
        match value {
            AttributeValue::Text(text) => node.set_attribute(&name, &text)?,
            // A flag is present or absent; `disabled="false"` is still
            // disabled in HTML, so absence is the only way to say no.
            AttributeValue::Flag(true) => node.set_attribute(&name, "")?,
            AttributeValue::Flag(false) => {}
        }
    }

    for (name, handler) in tag.handlers {
        listen(&node, &name, handler)?;
    }

    for child in tag.children {
        mount_in(document, child, &node)?;
    }

    Ok(node)
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
        handler: Rc<Handler>,
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
            let handler = Rc::clone(&self.handler);
            // This frame is called from C, so a panic in the
            // application's closure stops here.
            crate::panics::contain("an event handler", move || {
                handler.call(Event { kind })
            });
            1
        }
    }
}

/// `onClick` → `addEventListener("click", <the Rust closure>)`.
fn listen(node: &Node, name: &str, handler: Handler) -> Result<(), DomError> {
    let kind = event_kind(name);

    let mut v8_handler = EventHandler::new(Rc::from(kind.as_str()), Rc::new(handler));
    let callback =
        cef::v8_value_create_function(Some(&CefString::from(name)), Some(&mut v8_handler))
            .ok_or_else(|| DomError::Failed(format!("could not build a function for {name}")))?;

    let kind_value = Value::String(kind)
        .into_v8()
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
