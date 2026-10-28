//! Turning a promoted local into a cell, and its uses with it.
//!
//! What the developer writes:
//!
//! ```ignore
//! let mut n = 0;
//! return <button onClick={|| n += 1}>{n}</button>;
//! ```
//!
//! What comes out: a cell identified by where that `let mut` sits, a
//! handler that owns a clone of the handle instead of borrowing it, and
//! `{n}` reading the cell. The developer writes no conversion, no
//! `move`, no `Rc`, and no `use_state`.
//!
//! Every edit is confined to the line it starts on, because the line map
//! of plan 2 has to keep holding: a rewrite that moved lines would put
//! every `rustc` error back where it was before that map existed.

use crate::error::CompileError;
use crate::promote::{Component, MutationKind, analyse};
use crate::scan::Span;

/// One replacement, in offsets of the source being rewritten.
struct Edit {
    span: Span,
    text: String,
}

/// Rewrite every component of a `.rsx` that has state.
///
/// Source in, source out: what comes back is still `.rsx`, with its JSX
/// untouched, so the rest of the pipeline runs on it unchanged.
pub fn promote_state(source: &str) -> Result<String, CompileError> {
    let components = analyse(source)?;
    let mut edits = Vec::new();
    for component in &components {
        edits.extend(edits_for(source, component));
    }
    Ok(apply(source, edits))
}

fn edits_for(source: &str, component: &Component) -> Vec<Edit> {
    let mut edits = Vec::new();
    if component.promoted.is_empty() {
        return edits;
    }

    for promoted in &component.promoted {
        let (line, column) = line_and_column(source, promoted.declaration.start);
        let initializer = &source[promoted.initializer.start..promoted.initializer.end];
        edits.push(Edit {
            span: promoted.declaration,
            text: format!(
                "let {name} = rustx_dom::state::cell(rustx_dom::state::Slot {{ \
                 file: file!(), line: {line}u32, column: {column}u32 }}, || {initializer});",
                name = promoted.name,
            ),
        });
    }

    // A handler is rewritten whole: its mutations and its reads are
    // inside it, so editing them separately would collide with the edit
    // that wraps it.
    for handler in handlers_of(component) {
        if let Some(edit) = rewrite_handler(source, component, handler) {
            edits.push(edit);
        }
    }

    // Reads outside every handler — the ones the screen shows.
    for promoted in &component.promoted {
        for read in &promoted.reads {
            if handlers_of(component)
                .iter()
                .any(|handler| contains(*handler, *read))
            {
                continue;
            }
            edits.push(Edit {
                span: *read,
                text: format!("{}.get()", promoted.name),
            });
        }
    }

    edits
}

/// The `{…}` of every handler that touches a promoted local.
fn handlers_of(component: &Component) -> Vec<Span> {
    let mut spans: Vec<Span> = component
        .promoted
        .iter()
        .flat_map(|promoted| promoted.mutations.iter())
        .filter_map(|mutation| component.handler_around(mutation.span))
        .collect();
    spans.sort_by_key(|span| span.start);
    spans.dedup_by_key(|span| span.start);
    spans
}

/// `{|| n += 1}` → `{{ let n = n.clone(); move |_| n.update(|v| *v += 1) }}`
fn rewrite_handler(source: &str, component: &Component, handler: Span) -> Option<Edit> {
    let inner = Span {
        start: handler.start + 1,
        end: handler.end - 1,
    };
    let text = &source[inner.start..inner.end];

    // The closure's parameter list. `onClick={|| …}` is the shape the
    // design document asks for, and a handler is handed an event, so a
    // closure that takes none is given one it ignores. That is the one
    // place the compiler touches a closure's own syntax, and it is the
    // difference between the document's example compiling and not.
    let bars = text.find('|')?;
    let after_params = text[bars + 1..].find('|')? + bars + 2;
    let params = text[bars..after_params].trim();
    let params = if params == "||" { "|_|" } else { params };

    let mut body = String::from(&text[after_params..]);
    let mut inner_edits = Vec::new();
    let mut captured: Vec<&str> = Vec::new();

    for promoted in &component.promoted {
        let mut touched = false;
        for mutation in &promoted.mutations {
            if !contains(handler, mutation.span) {
                continue;
            }
            touched = true;
            inner_edits.push(rewrite_mutation(source, promoted.name.as_str(), mutation, handler));
        }
        for read in &promoted.reads {
            if !contains(handler, *read) {
                continue;
            }
            touched = true;
            inner_edits.push(Edit {
                span: *read,
                text: format!("{}.get()", promoted.name),
            });
        }
        if touched {
            captured.push(promoted.name.as_str());
        }
    }
    if captured.is_empty() {
        return None;
    }

    // The edits are in source offsets; the body starts partway in.
    let body_start = inner.start + after_params;
    inner_edits.retain(|edit| edit.span.start >= body_start);
    body = apply_relative(&body, body_start, inner_edits);

    // A handler outlives the render that built it, so it owns clones of
    // the handles rather than borrowing the locals.
    let clones: String = captured
        .iter()
        .map(|name| format!("let {name} = {name}.clone(); "))
        .collect();

    Some(Edit {
        span: handler,
        text: format!("{{{{ {clones}move {params}{body} }}}}"),
    })
}

fn rewrite_mutation(
    source: &str,
    name: &str,
    mutation: &crate::promote::Mutation,
    handler: Span,
) -> Edit {
    let bytes = source.as_bytes();
    let after = skip_spaces(bytes, mutation.span.end);

    match mutation.kind {
        MutationKind::Assign => {
            let value = expression_after(source, after + 1, handler);
            Edit {
                span: Span {
                    start: mutation.span.start,
                    end: value.end,
                },
                text: format!("{name}.set({})", source[value.start..value.end].trim()),
            }
        }
        MutationKind::Compound => {
            let operator_end = operator_end(bytes, after);
            let operator = &source[after..operator_end];
            let value = expression_after(source, operator_end, handler);
            Edit {
                span: Span {
                    start: mutation.span.start,
                    end: value.end,
                },
                text: format!(
                    "{name}.update(|__v| *__v {operator} {})",
                    source[value.start..value.end].trim()
                ),
            }
        }
        MutationKind::Method => {
            let call = expression_after(source, mutation.span.end, handler);
            Edit {
                span: Span {
                    start: mutation.span.start,
                    end: call.end,
                },
                text: format!(
                    "{name}.update(|__v| __v{})",
                    source[call.start..call.end].trim()
                ),
            }
        }
    }
}

/// How far an assignment's right-hand side runs: to the `;` that ends the
/// statement, or to the end of the closure's body when there is none —
/// `|| n += 1` has no semicolon at all.
fn expression_after(source: &str, from: usize, handler: Span) -> Span {
    let bytes = source.as_bytes();
    let mut depth = 0_i32;
    let mut index = from;
    let limit = handler.end - 1;
    while index < limit {
        match bytes[index] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' => depth -= 1,
            b'}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            b';' if depth == 0 => break,
            _ => {}
        }
        index += 1;
    }
    Span {
        start: from,
        end: index,
    }
}

fn operator_end(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while index < bytes.len() && bytes[index] != b'=' {
        index += 1;
    }
    index + 1
}

/// Apply edits to a slice that starts at `offset` in the original.
fn apply_relative(text: &str, offset: usize, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.span.start));
    let mut out = text.to_string();
    for edit in edits {
        let start = edit.span.start - offset;
        let end = edit.span.end - offset;
        if start <= end && end <= out.len() {
            out.replace_range(start..end, &edit.text);
        }
    }
    out
}

fn apply(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.span.start));
    let mut out = source.to_string();
    for edit in edits {
        if edit.span.start <= edit.span.end && edit.span.end <= out.len() {
            out.replace_range(edit.span.start..edit.span.end, &edit.text);
        }
    }
    out
}

fn contains(outer: Span, inner: Span) -> bool {
    inner.start >= outer.start && inner.end <= outer.end
}

fn skip_spaces(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    (line, source[line_start..offset].chars().count() + 1)
}
