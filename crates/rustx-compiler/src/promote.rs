//! Deciding which locals become state. Only deciding.
//!
//! The rule, from §7.4 of the design document: a local that is **read in
//! the JSX the component returns** and **mutated from an event handler's
//! closure** becomes a persistent cell. Everything else stays an ordinary
//! `let`, recomputed each render, costing nothing.
//!
//! Both halves matter. A local only read is recomputed anyway and has
//! nothing to remember. A local only mutated is written and thrown away,
//! and nobody would see it change. It is the combination — something the
//! screen shows and a click changes — that needs to survive the render
//! that drew it.
//!
//! The rewrite lives next door, in `rewrite.rs`. Keeping the decision
//! here means it can be tested by asking what was decided, rather than by
//! reading generated Rust and inferring.

use crate::error::CompileError;
use crate::scan::{Span, find_jsx_range, is_word_byte, skip_braces};

/// A `fn … -> Element`, and what its body does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    /// The body, braces included.
    pub body: Span,
    /// The JSX regions the body returns.
    pub returns_jsx: Vec<Span>,
    /// The `{…}` of every `on*` attribute in those regions.
    pub handlers: Vec<Span>,
    pub promoted: Vec<Promoted>,
}

/// A local that became state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promoted {
    pub name: String,
    /// `let mut n = 0;`, the whole statement.
    pub declaration: Span,
    /// `0` — what the cell starts at.
    pub initializer: Span,
    /// Every occurrence that only looks.
    pub reads: Vec<Span>,
    pub mutations: Vec<Mutation>,
}

/// One place a promoted local is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mutation {
    /// The name itself; the rewrite grows the span from here.
    pub span: Span,
    pub kind: MutationKind,
}

/// How it is written, which decides what it is rewritten into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationKind {
    /// `n = 5` → `set`
    Assign,
    /// `n += 1` → `update`
    Compound,
    /// `items.push(1);` → `update`
    Method,
}

/// Every component in a `.rsx`, with its promotion decided.
pub fn analyse(source: &str) -> Result<Vec<Component>, CompileError> {
    let mut components = Vec::new();
    for (name, body) in components_of(source) {
        let returns_jsx = find_jsx_range(source, body.start, body.end)?;
        let handlers = handler_closures(source, &returns_jsx);
        let parameters = parameter_names(source, body.start);

        let mut promoted = Vec::new();
        for local in locals_of(source, body) {
            if parameters.contains(&local.name) {
                continue;
            }
            let mutations: Vec<Mutation> = handlers
                .iter()
                .flat_map(|handler| mutations_in(source, &local.name, *handler))
                .collect();

            // "Read in the JSX" means read by what is drawn, not read by
            // a handler that happens to sit inside the JSX. A handler is
            // written between the tags, so without this an `onClick` that
            // only writes would look like a read of itself and promote a
            // variable nobody ever shows.
            let shown = occurrences(source, &local.name, &returns_jsx)
                .into_iter()
                .filter(|span| !handlers.iter().any(|handler| contains(*handler, *span)))
                .count();

            if shown == 0 || mutations.is_empty() {
                continue;
            }

            // Everywhere the name appears without writing to it — the
            // rewrite turns each into a read of the cell.
            let reads = occurrences(
                source,
                &local.name,
                &[Span {
                    start: local.declaration.end,
                    end: body.end,
                }],
            )
            .into_iter()
            .filter(|span| !mutations.iter().any(|mutation| mutation.span == *span))
            .collect();
            promoted.push(Promoted {
                name: local.name,
                declaration: local.declaration,
                initializer: local.initializer,
                reads,
                mutations,
            });
        }

        components.push(Component {
            handlers,
            name,
            body,
            returns_jsx,
            promoted,
        });
    }
    Ok(components)
}

/// A local declared `let mut` at the top level of a component's body.
struct Local {
    name: String,
    declaration: Span,
    initializer: Span,
}

/// `fn NAME(…) -> Element {`, and the span of its body.
fn components_of(source: &str) -> Vec<(String, Span)> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;

    while let Some(at) = find_word(source, "fn", index) {
        index = at + 2;
        let Some(name) = word_after(source, index) else {
            continue;
        };
        let after_name = name.1;
        // The parameter list, then the return type, then the body.
        let Some(open_paren) = next_byte(bytes, after_name, b'(') else {
            continue;
        };
        let Some(close_paren) = matching(bytes, open_paren, b'(', b')') else {
            continue;
        };
        let Some(brace) = next_byte(bytes, close_paren, b'{') else {
            continue;
        };
        // The marker the document already uses: a component returns an
        // Element.
        if !source[close_paren..brace].contains("Element") {
            continue;
        }
        let Ok(end) = skip_braces(source, brace) else {
            continue;
        };
        found.push((
            name.0,
            Span {
                start: brace,
                end,
            },
        ));
        index = end;
    }
    found
}

/// The names of a component's parameters. Never promoted: a prop is
/// handed in afresh at every render, and freezing it in a cell would make
/// a changing prop stop changing.
fn parameter_names(source: &str, body_start: usize) -> Vec<String> {
    let bytes = source.as_bytes();
    let Some(close) = source[..body_start].rfind(')') else {
        return Vec::new();
    };
    let Some(open) = matching_back(bytes, close) else {
        return Vec::new();
    };
    source[open + 1..close]
        .split(',')
        .filter_map(|parameter| parameter.split(':').next())
        .map(|name| name.trim().trim_start_matches("mut ").trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

/// `let mut NAME = INIT;` statements of this body.
fn locals_of(source: &str, body: Span) -> Vec<Local> {
    let bytes = source.as_bytes();
    let mut locals = Vec::new();
    let mut index = body.start;

    while let Some(at) = find_word(source, "let", index) {
        if at >= body.end {
            break;
        }
        index = at + 3;
        let Some(keyword) = word_after(source, index) else {
            continue;
        };
        if keyword.0 != "mut" {
            continue;
        }
        let Some(name) = word_after(source, keyword.1) else {
            continue;
        };
        // `let mut n: u8 = 0;` — step over an annotation if there is one.
        let mut cursor = skip_spaces(bytes, name.1);
        if bytes.get(cursor) == Some(&b':') {
            while cursor < body.end && bytes[cursor] != b'=' {
                cursor += 1;
            }
        }
        if bytes.get(cursor) != Some(&b'=') {
            continue;
        }
        let initializer_start = skip_spaces(bytes, cursor + 1);
        let Some(semicolon) = statement_end(source, initializer_start, body.end) else {
            continue;
        };

        locals.push(Local {
            name: name.0,
            declaration: Span {
                start: at,
                end: semicolon + 1,
            },
            initializer: Span {
                start: initializer_start,
                end: semicolon,
            },
        });
        index = semicolon;
    }
    locals
}

/// The body of every `on*={…}` closure inside these JSX regions.
fn handler_closures(source: &str, regions: &[Span]) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut handlers = Vec::new();

    for region in regions {
        let mut index = region.start;
        while index < region.end {
            // `on` followed by a capital is an event attribute.
            if bytes[index] == b'o'
                && bytes.get(index + 1) == Some(&b'n')
                && bytes
                    .get(index + 2)
                    .is_some_and(|byte| byte.is_ascii_uppercase())
                && (index == 0 || !is_word_byte(bytes[index - 1]))
            {
                let mut cursor = index + 2;
                while cursor < region.end && is_word_byte(bytes[cursor]) {
                    cursor += 1;
                }
                cursor = skip_spaces(bytes, cursor);
                if bytes.get(cursor) == Some(&b'=') {
                    cursor = skip_spaces(bytes, cursor + 1);
                    if bytes.get(cursor) == Some(&b'{')
                        && let Ok(end) = skip_braces(source, cursor)
                    {
                        handlers.push(Span {
                            start: cursor,
                            end,
                        });
                        index = end;
                        continue;
                    }
                }
            }
            index += 1;
        }
    }
    handlers
}

/// Every occurrence of `name` as a word inside these spans.
fn occurrences(source: &str, name: &str, spans: &[Span]) -> Vec<Span> {
    let mut found = Vec::new();
    for span in spans {
        let mut index = span.start;
        while let Some(at) = find_word(source, name, index) {
            if at >= span.end {
                break;
            }
            found.push(Span {
                start: at,
                end: at + name.len(),
            });
            index = at + name.len();
        }
    }
    found
}

/// Every occurrence of `name` inside this handler that writes to it.
fn mutations_in(source: &str, name: &str, handler: Span) -> Vec<Mutation> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();

    for span in occurrences(source, name, &[handler]) {
        let after = skip_spaces(bytes, span.end);
        let kind = match bytes.get(after) {
            // `n = 5`, but not `n == 5`.
            Some(b'=') if bytes.get(after + 1) != Some(&b'=') => Some(MutationKind::Assign),
            // `n += 1` and the rest of the compound family.
            Some(&byte)
                if matches!(byte, b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^')
                    && bytes.get(after + 1) == Some(&b'=') =>
            {
                Some(MutationKind::Compound)
            }
            Some(&byte) if matches!(byte, b'<' | b'>') && shift_assign(bytes, after) => {
                Some(MutationKind::Compound)
            }
            // `items.push(1);` as a statement of its own. In expression
            // position it is a read — `items.len()` must not become an
            // update — so the test is what surrounds it.
            Some(b'.') if is_statement(bytes, span, handler) => Some(MutationKind::Method),
            _ => None,
        };
        if let Some(kind) = kind {
            found.push(Mutation { span, kind });
        }
    }
    found
}

fn shift_assign(bytes: &[u8], at: usize) -> bool {
    bytes.get(at + 1) == bytes.get(at) && bytes.get(at + 2) == Some(&b'=')
}

/// Is this occurrence the start of a statement whose whole value is
/// thrown away? That is what tells `items.push(1);` from `items.len()`.
fn is_statement(bytes: &[u8], span: Span, handler: Span) -> bool {
    let mut before = span.start;
    while before > handler.start && bytes[before - 1].is_ascii_whitespace() {
        before -= 1;
    }
    let opens = before == handler.start
        || matches!(bytes.get(before - 1), Some(b'{') | Some(b';'));
    if !opens {
        return false;
    }
    // …and it ends at a `;`, rather than feeding something else.
    let mut depth = 0_i32;
    let mut index = span.end;
    while index < handler.end {
        match bytes[index] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b';' if depth == 0 => return true,
            b'{' | b'}' if depth == 0 => return false,
            _ => {}
        }
        index += 1;
    }
    false
}

/// The `;` ending a statement that starts at `from`, ignoring the ones
/// inside brackets, braces and strings.
fn statement_end(source: &str, from: usize, limit: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = from;
    let mut depth = 0_i32;
    while index < limit.min(bytes.len()) {
        match bytes[index] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b'{' => index = skip_braces(source, index).ok()? - 1,
            b'"' => index = crate::scan::skip_string(source, index).ok()? - 1,
            b';' if depth == 0 => return Some(index),
            _ => {}
        }
        index += 1;
    }
    None
}

fn find_word(source: &str, word: &str, from: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = from;
    while let Some(offset) = source.get(index..)?.find(word) {
        let at = index + offset;
        let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
        let after_ok = bytes
            .get(at + word.len())
            .is_none_or(|byte| !is_word_byte(*byte));
        if before_ok && after_ok {
            return Some(at);
        }
        index = at + word.len();
    }
    None
}

/// The next word after `from`, and where it ends.
fn word_after(source: &str, from: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let start = skip_spaces(bytes, from);
    let mut end = start;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    (end > start).then(|| (source[start..end].to_string(), end))
}

fn skip_spaces(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn next_byte(bytes: &[u8], from: usize, wanted: u8) -> Option<usize> {
    (from..bytes.len()).find(|index| bytes[*index] == wanted)
}

fn matching(bytes: &[u8], open: usize, opener: u8, closer: u8) -> Option<usize> {
    let mut depth = 0_i32;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        if *byte == opener {
            depth += 1;
        } else if *byte == closer {
            depth -= 1;
            if depth == 0 {
                return Some(open + offset);
            }
        }
    }
    None
}

fn matching_back(bytes: &[u8], close: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for index in (0..=close).rev() {
        if bytes[index] == b')' {
            depth += 1;
        } else if bytes[index] == b'(' {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn contains(outer: Span, inner: Span) -> bool {
    inner.start >= outer.start && inner.end <= outer.end
}

impl Component {
    /// The handler whose braces enclose `span`, if any.
    pub fn handler_around(&self, span: Span) -> Option<Span> {
        self.handlers
            .iter()
            .copied()
            .find(|handler| contains(*handler, span))
    }
}
