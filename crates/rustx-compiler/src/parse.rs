//! Reading a JSX region into a node tree.
//!
//! Everything the developer wrote is kept as it was written: text with its
//! punctuation, Rust with its braces and its closure bodies. The parser's
//! only job is to say which is which, and where each one started.

use crate::ast::{Attribute, AttributeSource, Node, TagNode};
use crate::error::CompileError;
use crate::emit::emit;
use crate::scan::{
    Span, find_jsx_range, opens_a_tag, skip_attribute_string, skip_braces,
};

/// Parse one JSX region, as found by [`crate::scan::find_jsx`].
pub fn parse_jsx(source: &str, span: Span) -> Result<Node, CompileError> {
    let mut cursor = Cursor {
        source,
        index: span.start,
        end: span.end,
    };
    let node = cursor.element()?;
    Ok(node)
}

struct Cursor<'a> {
    source: &'a str,
    index: usize,
    end: usize,
}

impl Cursor<'_> {
    fn bytes(&self) -> &[u8] {
        self.source.as_bytes()
    }

    fn peek(&self) -> Option<u8> {
        if self.index < self.end {
            Some(self.bytes()[self.index])
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while self.index < self.end && self.bytes()[self.index].is_ascii_whitespace() {
            self.index += 1;
        }
    }

    fn error(&self, offset: usize, message: impl Into<String>) -> CompileError {
        CompileError::at(self.source, offset, message)
    }

    /// One `<tag …>…</tag>`, `<tag … />` or `<>…</>`.
    fn element(&mut self) -> Result<Node, CompileError> {
        let offset = self.index;
        if self.peek() != Some(b'<') {
            return Err(self.error(offset, "expected a tag"));
        }
        self.index += 1;

        let name = self.name();
        let attributes = self.attributes()?;

        // `/>` — nothing inside.
        if self.peek() == Some(b'/') {
            self.index += 1;
            self.expect(b'>', offset)?;
            return Ok(Node::Tag(TagNode {
                name,
                attributes,
                children: Vec::new(),
                offset,
            }));
        }
        self.expect(b'>', offset)?;

        let children = self.children()?;

        // The closing tag.
        self.expect(b'<', offset)?;
        self.expect(b'/', offset)?;
        let closing_at = self.index;
        let closing = self.name();
        if closing != name {
            let opened = display_name(&name);
            let closed = display_name(&closing);
            return Err(self.error(
                closing_at,
                format!("{opened} is closed by {closed}"),
            ));
        }
        self.skip_whitespace();
        self.expect(b'>', offset)?;

        Ok(Node::Tag(TagNode {
            name,
            attributes,
            children,
            offset,
        }))
    }

    /// Children, up to this element's own closing tag.
    fn children(&mut self) -> Result<Vec<Node>, CompileError> {
        let mut children = Vec::new();
        while self.index < self.end {
            match self.peek() {
                Some(b'<') if self.bytes().get(self.index + 1) == Some(&b'/') => break,
                Some(b'<') if opens_a_tag(self.bytes(), self.index) => {
                    children.push(self.element()?);
                }
                Some(b'{') => {
                    let code = self.braced_rust(None)?;
                    children.push(Node::Expression(code));
                }
                Some(_) => {
                    if let Some(text) = self.text() {
                        children.push(Node::Text(text));
                    }
                }
                None => break,
            }
        }
        Ok(children)
    }

    /// Text up to the next tag or `{`. A `<` that does not open a tag —
    /// "10 < 20" — is text like any other character.
    fn text(&mut self) -> Option<String> {
        let start = self.index;
        while self.index < self.end {
            let byte = self.bytes()[self.index];
            if byte == b'{' {
                break;
            }
            if byte == b'<'
                && (self.bytes().get(self.index + 1) == Some(&b'/')
                    || opens_a_tag(self.bytes(), self.index))
            {
                break;
            }
            self.index += 1;
        }
        collapse(&self.source[start..self.index])
    }

    fn attributes(&mut self) -> Result<Vec<Attribute>, CompileError> {
        let mut attributes = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b'>') | Some(b'/') | None => return Ok(attributes),
                _ => {}
            }

            let offset = self.index;
            let name = self.name();
            if name.is_empty() {
                return Err(self.error(offset, "expected an attribute name"));
            }
            self.skip_whitespace();

            let value = if self.peek() == Some(b'=') {
                self.index += 1;
                self.skip_whitespace();
                match self.peek() {
                    Some(b'"') => {
                        let open = self.index;
                        self.index = skip_attribute_string(self.source, open)?;
                        AttributeSource::Literal(self.source[open + 1..self.index - 1].to_string())
                    }
                    Some(b'{') => AttributeSource::Expression(
                        self.braced_rust(crate::attributes::is_event(&name).then_some(name.as_str()))?,
                    ),
                    _ => {
                        return Err(self.error(
                            self.index,
                            format!("{name} needs a value in quotes or in braces"),
                        ));
                    }
                }
            } else {
                AttributeSource::Flag
            };

            attributes.push(Attribute {
                name,
                value,
                offset,
            });
        }
    }

    /// The Rust between a `{` and its matching `}`, with any JSX inside
    /// it already turned into Rust.
    ///
    /// `{items.iter().map(|name| <li>{name}</li>).collect::<Vec<_>>()}`
    /// is a Rust expression that happens to build elements, and a list is
    /// written that way in TSX too. The substitution happens here, where
    /// offsets are still offsets into the real file, so that an error
    /// inside a nested tag lands on the line it was written on.
    /// `event` names the event, when these braces hold a handler: a
    /// handler runs between renders, so a component written inside one
    /// has no render to belong to and is refused here rather than
    /// panicking under the developer's finger.
    fn braced_rust(&mut self, event: Option<&str>) -> Result<String, CompileError> {
        let open = self.index;
        self.index = skip_braces(self.source, open)?;
        let (from, to) = (open + 1, self.index - 1);

        let mut code = String::new();
        let mut cursor = from;
        for span in find_jsx_range(self.source, from, to)? {
            code.push_str(&self.source[cursor..span.start]);
            let node = parse_jsx(self.source, span)?;
            // A region written inside a handler sees the same
            // components as the one around it: they are the file's.
            let components = crate::promote::component_signatures(self.source);
            if let Some(event) = event
                && let Some(component) = component_in(&node)
            {
                return Err(CompileError::flagged(
                    span.start,
                    format!(
                        "`<{component} />` is written inside `{event}`, and a handler runs \
                         between two renders — there is no render for the component to \
                         belong to. Put it in what the component returns, and let the \
                         handler change the state it reads."
                    ),
                ));
            }
            let context = crate::emit::Context {
                components: &components,
                source: self.source,
            };
            code.push_str(&emit(&node, &context)?);
            cursor = span.end;
        }
        code.push_str(&self.source[cursor..to]);
        Ok(code)
    }

    /// A tag or attribute name, possibly empty — a fragment has none.
    fn name(&mut self) -> String {
        let start = self.index;
        while self.index < self.end {
            let byte = self.bytes()[self.index];
            if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b':' {
                self.index += 1;
            } else {
                break;
            }
        }
        self.source[start..self.index].to_string()
    }

    fn expect(&mut self, byte: u8, _opened_at: usize) -> Result<(), CompileError> {
        if self.peek() == Some(byte) {
            self.index += 1;
            Ok(())
        } else {
            Err(self.error(
                self.index.min(self.end),
                format!("expected `{}` in the tag opened here", byte as char),
            ))
        }
    }
}

/// JSX's whitespace rule, which is not "trim".
///
/// Whitespace that crosses a newline is layout: the developer indented
/// their tree, and that indentation must not be printed. Whitespace that
/// does not is content: in `a <b>bold</b> word` the two spaces are the
/// only thing keeping the words apart, and dropping them renders
/// "aboldword".
///
/// So: lines are trimmed and joined with one space, and a run of
/// whitespace at either edge becomes a single space when it stayed on
/// one line and nothing when it crossed a newline.
fn collapse(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }

    let leading = edge_space(text.chars().take_while(|c| c.is_whitespace()));
    let trailing = edge_space(text.chars().rev().take_while(|c| c.is_whitespace()));

    let mut collapsed = String::with_capacity(text.len());
    collapsed.push_str(leading);
    let mut first = true;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !first {
            collapsed.push(' ');
        }
        collapsed.push_str(line);
        first = false;
    }
    collapsed.push_str(trailing);
    Some(collapsed)
}

/// What a run of edge whitespace is worth: a space if it stayed on one
/// line, nothing if it crossed a newline.
fn edge_space(run: impl Iterator<Item = char>) -> &'static str {
    let mut seen = false;
    for character in run {
        if character == '\n' {
            return "";
        }
        seen = true;
    }
    if seen { " " } else { "" }
}

fn display_name(name: &str) -> String {
    if name.is_empty() {
        "the fragment <>".to_string()
    } else {
        format!("<{name}>")
    }
}

/// The first component tag in this region, however deep: a component
/// written inside a plain element inside a handler is still a component
/// inside a handler.
fn component_in(node: &Node) -> Option<String> {
    match node {
        Node::Tag(tag) => {
            if crate::tags::looks_like_a_component(&tag.name) {
                return Some(tag.name.clone());
            }
            tag.children.iter().find_map(component_in)
        }
        _ => None,
    }
}
