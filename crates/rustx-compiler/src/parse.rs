//! Reading a JSX region into a node tree.
//!
//! Everything the developer wrote is kept as it was written: text with its
//! punctuation, Rust with its braces and its closure bodies. The parser's
//! only job is to say which is which, and where each one started.

use crate::ast::{Attribute, AttributeSource, Node, TagNode};
use crate::emit::emit;
use crate::error::CompileError;
use crate::scan::{Span, find_jsx_range, opens_a_tag, skip_attribute_string, skip_braces};

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

    /// Les espaces, et les commentaires qui sont entre eux.
    ///
    /// Dans une balise il n'y a pas de prose : tout ce qui n'est pas un
    /// attribut est un commentaire, et un `//` y est un `//` de Rust.
    fn skip_space_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            match comment_at(self.bytes(), self.index, self.end) {
                Some(after) => self.index = after,
                None => return,
            }
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
            return Err(self.error(closing_at, format!("{opened} is closed by {closed}")));
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
                    let offset = self.index;
                    let code = self.braced_rust(None)?;
                    // `{/* … */}` est la forme de JSX, et des accolades
                    // qui ne tiennent qu'un commentaire ne tiennent
                    // aucune expression : il n'y a rien a dessiner, et
                    // `into_element({ /* … */ })` ne compilerait pas.
                    if !is_only_comments(&code) {
                        children.push(Node::Expression { code, offset });
                    }
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
    ///
    /// Un commentaire n'en fait pas partie. Il commence une ligne —
    /// `// …` au milieu d'une phrase est la phrase, et une adresse en
    /// contient un — ou il est entre `/*` et `*/`, ou qu'il soit. Ce
    /// qui l'entoure reste du texte, et ce qui est dedans n'arrive
    /// jamais a l'ecran : c'est la ce que le labo a vu, une ligne de
    /// commentaire ecrite en toutes lettres dans sa barre.
    fn text(&mut self) -> Option<String> {
        let mut kept = String::new();
        let mut from = self.index;
        // Vrai tant que cette ligne-ci n'a montre que des espaces :
        // c'est la seule condition pour qu'un `//` soit un commentaire.
        let mut starting = true;
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
            let a_line_comment = starting
                && byte == b'/'
                && self.bytes().get(self.index + 1) == Some(&b'/');
            let a_block_comment = byte == b'/' && self.bytes().get(self.index + 1) == Some(&b'*');
            if a_line_comment || a_block_comment {
                if let Some(after) = comment_at(self.bytes(), self.index, self.end) {
                    kept.push_str(&self.source[from..self.index]);
                    self.index = after;
                    from = self.index;
                    // Un commentaire de ligne s'arrete avant le saut de
                    // ligne : la ligne suivante recommence.
                    starting = !a_block_comment;
                    continue;
                }
            }
            if byte == b'\n' {
                starting = true;
            } else if !byte.is_ascii_whitespace() {
                starting = false;
            }
            self.index += 1;
        }
        kept.push_str(&self.source[from..self.index]);
        collapse(&kept)
    }

    fn attributes(&mut self) -> Result<Vec<Attribute>, CompileError> {
        let mut attributes = Vec::new();
        loop {
            self.skip_space_and_comments();
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
                    Some(b'{') => AttributeSource::Expression(self.braced_rust(
                        crate::attributes::is_event(&name).then_some(name.as_str()),
                    )?),
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
            if byte.is_ascii_alphanumeric()
                || byte == b'_'
                || byte == b'-'
                || byte == b':'
                || byte == b'.'
            {
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
/// Un pas apres le commentaire qui commence ici, s'il en commence un.
///
/// `//` va jusqu'au saut de ligne, qu'il laisse : c'est la fin de la
/// ligne et non du commentaire. `/*` va jusqu'a `*/`, et jusqu'au bout
/// de la region s'il n'est jamais referme — un commentaire ouvert est
/// une erreur que le compilateur Rust dira mieux que nous.
fn comment_at(bytes: &[u8], at: usize, end: usize) -> Option<usize> {
    if at + 1 >= end || bytes[at] != b'/' {
        return None;
    }
    match bytes[at + 1] {
        b'/' => {
            let mut index = at + 2;
            while index < end && bytes[index] != b'\n' {
                index += 1;
            }
            Some(index)
        }
        b'*' => {
            let mut index = at + 2;
            while index + 1 < end {
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    return Some(index + 2);
                }
                index += 1;
            }
            Some(end)
        }
        _ => None,
    }
}

/// Ce code n'est-il que des commentaires et des espaces ?
fn is_only_comments(code: &str) -> bool {
    let bytes = code.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        match comment_at(bytes, index, bytes.len()) {
            Some(after) if after > index => index = after,
            _ => return false,
        }
    }
    true
}

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
