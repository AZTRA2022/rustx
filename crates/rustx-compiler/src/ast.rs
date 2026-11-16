//! What a JSX region turns into before any Rust is written.
//!
//! Every node keeps the byte offset it came from in the `.rsx`. That
//! offset is what the position map is built out of, and it is the reason
//! an error can name a line the developer recognises.

/// A node of the parsed JSX.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Tag(TagNode),
    /// Text as written, with its punctuation intact. A `<` or a `&` in
    /// interface text is text — "10 < 20", "Dupont & Fils".
    Text(String),
    /// Rust between braces, kept verbatim. The compiler never rewrites it.
    ///
    /// The offset is where the brace opened. A whole-expression call to
    /// a component of this file is an instance of its own, and an
    /// instance is identified by the place it is written — so two calls
    /// on one line have to be told apart.
    Expression {
        code: String,
        offset: usize,
    },
}

/// A tag, or a fragment when `name` is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagNode {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
    pub offset: usize,
}

impl TagNode {
    /// `<>…</>` — siblings with no wrapping element.
    pub fn is_fragment(&self) -> bool {
        self.name.is_empty()
    }
}

/// One `name`, `name="text"` or `name={rust}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: AttributeSource,
    pub offset: usize,
}

/// The three shapes an attribute's value can take in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeSource {
    Literal(String),
    Expression(String),
    /// Written bare: `disabled`.
    Flag,
}
