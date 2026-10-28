//! The DOM, from Rust, in the render process.
//!
//! This is where a reconciler written in Rust will eventually put its
//! output. Calls go through the render process's V8 bindings — in process,
//! on the stack, with nothing serialized — and never through a patched
//! Blink, which would mean maintaining a fork of Chromium.
//!
//! # Handles and lifetimes
//!
//! A V8 value belongs to the context it was made in. Navigate, reload or
//! close the window and that context is gone; anything still pointing into
//! it is a use-after-free waiting to happen. A [`Node`] therefore carries
//! the generation of the context it came from, and every operation checks
//! that generation against the live one first. A handle that outlived its
//! context answers [`DomError::ContextDestroyed`] — it does not panic, it
//! does not quietly do nothing, and it needs no `unsafe` to be safe.

use std::cell::Cell;

use cef::{CefString, ImplV8Context, ImplV8Value, V8Value};

use crate::value::{NativeError, Value};

thread_local! {
    /// Bumped every time a V8 context goes away. Handles made before the
    /// bump are stale by definition.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
}

/// Called by the render process handler when a context is released.
pub(crate) fn context_released() {
    GENERATION.with(|generation| generation.set(generation.get() + 1));
}

fn generation() -> u64 {
    GENERATION.with(Cell::get)
}

/// What can go wrong while touching the DOM.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomError {
    /// The handle belongs to a context that no longer exists, usually
    /// because the page navigated or reloaded.
    ContextDestroyed,
    /// There is no JavaScript context on this thread right now.
    NoContext,
    /// The DOM refused the operation, or the property was not what the DOM
    /// is documented to provide.
    Failed(String),
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContextDestroyed => f.write_str("context destroyed"),
            Self::NoContext => f.write_str("no JavaScript context on this thread"),
            Self::Failed(what) => write!(f, "DOM operation failed: {what}"),
        }
    }
}

impl std::error::Error for DomError {}

impl From<DomError> for NativeError {
    fn from(error: DomError) -> Self {
        NativeError::new(error.to_string())
    }
}

/// A handle on a DOM node, valid for as long as its context is.
#[derive(Clone)]
pub struct Node {
    value: V8Value,
    generation: u64,
}

/// The document of the page currently running on this thread.
#[derive(Clone)]
pub struct Document {
    node: Node,
}

impl Document {
    /// The live document, or `None` outside a JavaScript context.
    pub fn current() -> Option<Self> {
        let context = cef::v8_context_get_current_context()?;
        let global = context.global()?;
        let document = global.value_bykey(Some(&CefString::from("document")))?;
        Some(Self {
            node: Node {
                value: document,
                generation: generation(),
            },
        })
    }

    /// The element carrying this `id`, if the page has one.
    pub fn get_element_by_id(&self, id: &str) -> Result<Option<Node>, DomError> {
        let found = self
            .node
            .call("getElementById", &[Value::String(id.to_string())])?;
        Ok(found.filter(|node| node.is_object()))
    }

    /// A new, unattached element.
    pub fn create_element(&self, tag: &str) -> Result<Node, DomError> {
        self.node
            .call("createElement", &[Value::String(tag.to_string())])?
            .ok_or_else(|| DomError::Failed(format!("createElement('{tag}') returned nothing")))
    }

    /// How many nodes match this selector.
    pub fn count_matching(&self, selector: &str) -> Result<usize, DomError> {
        let list = self
            .node
            .call("querySelectorAll", &[Value::String(selector.to_string())])?
            .ok_or_else(|| DomError::Failed("querySelectorAll returned nothing".into()))?;
        match list.get_property("length")? {
            Some(Value::Number(n)) => Ok(n as usize),
            other => Err(DomError::Failed(format!("length was {other:?}"))),
        }
    }

    /// The first node matching this selector.
    pub fn query_selector(&self, selector: &str) -> Result<Option<Node>, DomError> {
        let found = self
            .node
            .call("querySelector", &[Value::String(selector.to_string())])?;
        Ok(found.filter(|node| node.is_object()))
    }
}

impl Node {
    /// Whether this handle's context is still the live one.
    fn check(&self) -> Result<(), DomError> {
        if self.generation == generation() {
            Ok(())
        } else {
            Err(DomError::ContextDestroyed)
        }
    }

    fn is_object(&self) -> bool {
        self.value.is_object() != 0
    }

    /// Replace everything inside this node with plain text.
    pub fn set_text(&self, text: &str) -> Result<(), DomError> {
        self.set_property("textContent", Value::String(text.to_string()))
    }

    /// This node's text content.
    pub fn text(&self) -> Result<String, DomError> {
        match self.get_property("textContent")? {
            Some(Value::String(text)) => Ok(text),
            other => Err(DomError::Failed(format!("textContent was {other:?}"))),
        }
    }

    pub fn set_attribute(&self, name: &str, value: &str) -> Result<(), DomError> {
        self.call(
            "setAttribute",
            &[
                Value::String(name.to_string()),
                Value::String(value.to_string()),
            ],
        )?;
        Ok(())
    }

    pub fn append_child(&self, child: &Node) -> Result<(), DomError> {
        child.check()?;
        self.call_with_nodes("appendChild", &[child])?;
        Ok(())
    }

    /// Detach this node from its parent.
    pub fn remove(&self) -> Result<(), DomError> {
        self.call("remove", &[])?;
        Ok(())
    }

    fn get_property(&self, name: &str) -> Result<Option<Value>, DomError> {
        self.check()?;
        let Some(value) = self.value.value_bykey(Some(&CefString::from(name))) else {
            return Ok(None);
        };
        Value::from_v8(&value)
            .map(Some)
            .map_err(|error| DomError::Failed(error.message))
    }

    fn set_property(&self, name: &str, value: Value) -> Result<(), DomError> {
        self.check()?;
        let mut value = value
            .into_v8()
            .ok_or_else(|| DomError::Failed(format!("could not build a value for '{name}'")))?;
        let written = self.value.set_value_bykey(
            Some(&CefString::from(name)),
            Some(&mut value),
            cef::V8Propertyattribute::default(),
        );
        if written == 0 {
            return Err(DomError::Failed(format!("'{name}' was not writable")));
        }
        Ok(())
    }

    /// Call one of this node's own methods with plain values.
    fn call(&self, method: &str, arguments: &[Value]) -> Result<Option<Node>, DomError> {
        let arguments: Vec<Option<V8Value>> =
            arguments.iter().cloned().map(Value::into_v8).collect();
        self.invoke(method, arguments)
    }

    /// Call one of this node's own methods with other nodes as arguments.
    fn call_with_nodes(&self, method: &str, arguments: &[&Node]) -> Result<Option<Node>, DomError> {
        let arguments: Vec<Option<V8Value>> = arguments
            .iter()
            .map(|node| Some(node.value.clone()))
            .collect();
        self.invoke(method, arguments)
    }

    fn invoke(
        &self,
        method: &str,
        arguments: Vec<Option<V8Value>>,
    ) -> Result<Option<Node>, DomError> {
        self.check()?;

        let function = self
            .value
            .value_bykey(Some(&CefString::from(method)))
            .ok_or_else(|| DomError::Failed(format!("no '{method}' on this node")))?;
        if function.is_function() == 0 {
            return Err(DomError::Failed(format!("'{method}' is not a function")));
        }

        let mut receiver = self.value.clone();
        let returned = function.execute_function(Some(&mut receiver), Some(&arguments));

        if function.has_exception() != 0 {
            function.clear_exception();
            return Err(DomError::Failed(format!("'{method}' threw")));
        }

        Ok(returned.map(|value| Node {
            value,
            generation: self.generation,
        }))
    }
}

impl Node {
    /// The value of an attribute, if the node carries it.
    pub fn attribute(&self, name: &str) -> Result<Option<String>, DomError> {
        match self.call("getAttribute", &[Value::String(name.to_string())])? {
            Some(node) => match Value::from_v8(&node.value) {
                Ok(Value::String(text)) => Ok(Some(text)),
                Ok(Value::Null) => Ok(None),
                Ok(other) => Err(DomError::Failed(format!("attribute was {other:?}"))),
                Err(error) => Err(DomError::Failed(error.message)),
            },
            None => Ok(None),
        }
    }
}

impl Document {
    /// A new, unattached text node.
    ///
    /// `set_text` replaces everything a node contains, so it cannot place
    /// a piece of text beside a sibling element. Mounting
    /// `<li><b>c</b> and more</li>` needs a node of its own for the text.
    pub fn create_text_node(&self, text: &str) -> Result<Node, DomError> {
        self.node
            .call("createTextNode", &[Value::String(text.to_string())])?
            .ok_or_else(|| DomError::Failed("createTextNode returned nothing".into()))
    }
}

impl Node {
    /// Call a method with V8 values built elsewhere in the engine — a
    /// function object, for instance, which `Value` deliberately cannot
    /// represent.
    pub(crate) fn call_with_v8_returning(
        &self,
        method: &str,
        arguments: Vec<Option<V8Value>>,
    ) -> Result<Option<Node>, DomError> {
        self.invoke(method, arguments)
    }

    pub(crate) fn call_with_v8(
        &self,
        method: &str,
        arguments: Vec<Option<V8Value>>,
    ) -> Result<(), DomError> {
        self.invoke(method, arguments)?;
        Ok(())
    }
}

impl Node {
    /// Take an attribute off this node.
    pub fn remove_attribute(&self, name: &str) -> Result<(), DomError> {
        self.call("removeAttribute", &[Value::String(name.to_string())])?;
        Ok(())
    }
}

impl Node {
    /// Put `child` before `before`, or at the end when `before` is
    /// `None` — which is what `insertBefore(node, null)` already means.
    pub fn insert_before(&self, child: &Node, before: Option<&Node>) -> Result<(), DomError> {
        child.check()?;
        match before {
            Some(before) => {
                before.check()?;
                self.call_with_nodes("insertBefore", &[child, before])?;
            }
            None => {
                let null = Value::Null
                    .into_v8()
                    .ok_or_else(|| DomError::Failed("could not build null".into()))?;
                self.call_with_v8_returning("insertBefore", vec![Some(child.value.clone()), Some(null)])?;
            }
        }
        Ok(())
    }
}
