//! The element tree, and what may be written inside `{}`.

use std::fmt::Display;

/// A node of the tree a component returns.
#[derive(Debug)]
pub enum Element {
    Tag(TagElement),
    Text(String),
    /// Siblings with no wrapping node — what `<>…</>` produces.
    Fragment(Vec<Element>),
    /// Nothing at all. A component may legitimately render nothing.
    Empty,
}

/// A tag, its attributes, its event handlers and its children, in the
/// order they were written.
#[derive(Debug)]
pub struct TagElement {
    pub tag: String,
    pub attributes: Vec<(String, AttributeValue)>,
    pub handlers: Vec<(String, Handler)>,
    pub children: Vec<Element>,
}

/// What an attribute is set to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeValue {
    Text(String),
    /// A bare attribute: `disabled` rather than `disabled="true"`.
    Flag(bool),
}

/// What an event handler is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: String,
}

/// A handler written by the application, as a plain Rust closure.
///
/// Neither `Send` nor `Sync`, and that is the point: it runs on the render
/// process's UI thread and nowhere else, so it may capture an `Rc`, a
/// `Cell`, or anything else the developer already has in hand. Requiring a
/// thread bound here would make every `onClick` an argument with the
/// borrow checker over a thread the closure never reaches.
pub struct Handler(Box<dyn Fn(Event) + 'static>);

impl Handler {
    pub fn new(handler: impl Fn(Event) + 'static) -> Self {
        Self(Box::new(handler))
    }

    pub fn call(&self, event: Event) {
        (self.0)(event)
    }
}

impl std::fmt::Debug for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handler(..)")
    }
}

impl Element {
    /// Start building a tag. This is the call the compiler emits.
    pub fn tag(name: &str) -> TagElement {
        TagElement {
            tag: name.to_string(),
            attributes: Vec::new(),
            handlers: Vec::new(),
            children: Vec::new(),
        }
    }
}

impl TagElement {
    pub fn attribute(mut self, name: &str, value: AttributeValue) -> Self {
        self.attributes.push((name.to_string(), value));
        self
    }

    pub fn handler(mut self, name: &str, handler: Handler) -> Self {
        self.handlers.push((name.to_string(), handler));
        self
    }

    pub fn child(mut self, child: Element) -> Self {
        self.children.push(child);
        self
    }

    pub fn build(self) -> Element {
        Element::Tag(self)
    }
}

/// Anything that may be written between braces in a `.rsx` file.
///
/// `{clicks}`, `{user.name}`, `{items.len()}` — the developer writes the
/// value, never a conversion. The `Marker` parameter is what lets a single
/// function accept both an `Element` and anything merely displayable:
/// without it the blanket `T: Display` implementation would be judged to
/// overlap the specific ones, since Rust will not reason from the absence
/// of an implementation.
pub trait IntoElement<Marker> {
    fn into_element(self) -> Element;
}

/// Markers. They exist only to keep the implementations below disjoint and
/// are never named by anyone outside this file.
pub struct IsElement;
pub struct IsList;
pub struct IsOption;
pub struct IsDisplay;

impl IntoElement<IsElement> for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl IntoElement<IsList> for Vec<Element> {
    fn into_element(self) -> Element {
        Element::Fragment(self)
    }
}

impl IntoElement<IsOption> for Option<Element> {
    fn into_element(self) -> Element {
        self.unwrap_or(Element::Empty)
    }
}

impl<T: Display> IntoElement<IsDisplay> for T {
    fn into_element(self) -> Element {
        Element::Text(self.to_string())
    }
}

/// Turn whatever was written between braces into a node.
pub fn into_element<Marker, T: IntoElement<Marker>>(value: T) -> Element {
    value.into_element()
}
