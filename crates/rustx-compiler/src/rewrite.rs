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
use crate::promote::{CallKind, Component, MutationKind, analyse};
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

    // A value declared at the top of the page becomes the type that
    // names it. There is nothing to store here — the value itself is
    // built on the first ask, in whichever component asks first — so
    // what is left behind is the name, as a type nobody else can spell.
    for declaration in crate::promote::shared_of(source) {
        let original = &source[declaration.declaration.start..declaration.declaration.end];
        edits.push(Edit {
            span: declaration.declaration,
            text: pad_to(
                format!(
                    "#[allow(non_camel_case_types)] #[derive(Clone, Copy)] struct {};",
                    marker(&declaration.name)
                ),
                original,
            ),
        });
    }

    Ok(apply(source, edits))
}

fn edits_for(source: &str, component: &Component) -> Vec<Edit> {
    let mut edits = Vec::new();

    for call in &component.calls {
        edits.push(call_edit(source, call));
    }

    if component.promoted.is_empty() {
        return edits;
    }

    for promoted in &component.promoted {
        let (line, column) = line_and_column(source, promoted.declaration.start);
        let initializer = &source[promoted.initializer.start..promoted.initializer.end];
        // A value the page declared is found by the type that names it;
        // a value this component declared is found by the place it was
        // written. Everything after this line is the same rewrite.
        let text = if promoted.shared {
            format!(
                "let {name} = rustx_dom::store::shared::<{marker}, _>(|| {initializer});",
                name = promoted.name,
                marker = marker(&promoted.name),
            )
        } else {
            format!(
                "let {name} = rustx_dom::state::cell(rustx_dom::state::Slot {{ \
                 file: file!(), line: {line}u32, column: {column}u32 }}, || {initializer});",
                name = promoted.name,
            )
        };
        edits.push(Edit {
            span: promoted.declaration,
            text,
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

/// `keep(EXPR)`, `effect(WATCHED, RUN)` and `task(WORK)`, each given the
/// place it was written at.
///
/// The developer writes `keep(Camera::open())`. The slot is what makes
/// the value the same value on the next render, and only the compiler
/// knows where the call sits — so the compiler is what says it, exactly
/// as it does for a promoted `let mut`.
///
/// `task` is the one that carries the whole idea: it becomes a *kept*
/// token. Nothing else ties the thread to the component — the token is a
/// value the component holds, so it is dropped at unmount, so the task
/// stops.
fn call_edit(source: &str, call: &crate::promote::Call) -> Edit {
    let (line, column) = line_and_column(source, call.span.start);
    let slot = format!(
        "rustx_dom::state::Slot {{ file: file!(), line: {line}u32, column: {column}u32 }}"
    );
    let arguments = source[call.arguments.start..call.arguments.end].trim();

    let text = match call.kind {
        CallKind::Keep => {
            format!("rustx_dom::state::keep_at({slot}, || {arguments})")
        }
        CallKind::Task => {
            format!("rustx_dom::state::keep_at({slot}, || rustx_engine::task::spawn({arguments}))")
        }
        CallKind::Effect => {
            let (watched, run) = split_first_argument(arguments);
            format!("rustx_dom::state::effect_at({slot}, {watched}, {run})")
        }
    };

    // What rustfmt does to a long argument puts the call across several
    // lines, and the replacement is one line. Plan 2's position map
    // rests on the line count, so the newlines the arguments had are
    // given back at the end of the call — after the closing bracket,
    // where they change nothing but the line numbering they preserve.
    let original = &source[call.span.start..call.span.end];
    let text = pad_to(text, original);

    Edit {
        span: call.span,
        text,
    }
}

/// `text`, with as many trailing newlines as `original` has.
fn pad_to(text: String, original: &str) -> String {
    let wanted = original.matches('\n').count();
    let have = text.matches('\n').count();
    let mut text = text;
    for _ in have..wanted {
        text.push('\n');
    }
    text
}

/// `effect`'s two arguments, split at the comma between them.
///
/// The comma has to be the one at the top level: what is watched can be
/// a call or a tuple with commas of its own.
fn split_first_argument(arguments: &str) -> (&str, &str) {
    let bytes = arguments.as_bytes();
    let mut depth = 0_i32;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index = crate::scan::skip_string(arguments, index).unwrap_or(index + 1);
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                return (
                    arguments[..index].trim(),
                    arguments[index + 1..].trim(),
                );
            }
            _ => {}
        }
        index += 1;
    }

    // One argument where two were expected. Leaving it as written puts
    // the complaint where the developer can act on it — `rustc` naming
    // the line — rather than inventing a second argument here.
    (arguments.trim(), "")
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

    // Reads are gathered first, because a mutation's replacement copies
    // its own right-hand side out of the source and any read inside it
    // has to be applied to that copy. `n = n + 1` is one statement with
    // both: applying two edits over the same bytes would have each
    // overwrite the other and produce something that is not Rust.
    let mut reads: Vec<Edit> = Vec::new();
    for promoted in &component.promoted {
        for read in &promoted.reads {
            if contains(handler, *read) {
                reads.push(Edit {
                    span: *read,
                    text: format!("{}.get()", promoted.name),
                });
            }
        }
    }

    for promoted in &component.promoted {
        let mut touched = false;
        for mutation in &promoted.mutations {
            if !contains(handler, mutation.span) {
                continue;
            }
            touched = true;
            let edit = rewrite_mutation(source, promoted.name.as_str(), mutation, handler, &reads);
            reads.retain(|read| !contains(edit.span, read.span));
            inner_edits.push(edit);
        }
        if !touched && promoted.reads.iter().any(|read| contains(handler, *read)) {
            touched = true;
        }
        if touched {
            captured.push(promoted.name.as_str());
        }
    }
    inner_edits.append(&mut reads);
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
    reads: &[Edit],
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
                text: format!("{name}.set({})", read_patched(source, value, reads)),
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
                    read_patched(source, value, reads)
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
                    read_patched(source, call, reads)
                ),
            }
        }
    }
}

/// How far an assignment's right-hand side runs.
///
/// To the `;` that ends the statement, or to the end of the closure's
/// body when there is none — `|| n += 1` has no semicolon at all. Two
/// things make this less obvious than it looks: a value can be a
/// `match` with braces and arms in it, so braces have to nest rather
/// than terminate; and the expression can sit inside a call the closure
/// did not open, as in `for_each(|x| n += x)`, so a closing bracket
/// that would take the depth below zero is the end too.
fn expression_after(source: &str, from: usize, handler: Span) -> Span {
    let bytes = source.as_bytes();
    let mut depth = 0_i32;
    let mut index = from;
    let limit = handler.end - 1;

    while index < limit {
        match bytes[index] {
            b'"' => {
                index = crate::scan::skip_string(source, index).unwrap_or(index + 1);
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            // Past our own nesting: this bracket belongs to whoever
            // called us, and the expression stopped before it.
            b')' | b']' | b'}' if depth == 0 => break,
            b')' | b']' | b'}' => depth -= 1,
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

/// The source of `span`, with every read of a promoted local inside it
/// already turned into a read of the cell.
///
/// This is where `n = n + 1` is resolved: the right-hand side is copied
/// once, patched once, and placed once. Two edits over the same bytes
/// would each overwrite the other.
fn read_patched(source: &str, span: Span, reads: &[Edit]) -> String {
    let inside: Vec<Edit> = reads
        .iter()
        .filter(|read| contains(span, read.span))
        .map(|read| Edit {
            span: Span {
                start: read.span.start - span.start,
                end: read.span.end - span.start,
            },
            text: read.text.clone(),
        })
        .collect();
    apply_relative(&source[span.start..span.end], 0, inside)
        .trim()
        .to_string()
}

/// The type that names a shared value.
///
/// It is a type rather than a key string because the registry finds a
/// value by `TypeId`, and because a name nobody can spell twice is what
/// keeps two pages' values apart.
fn marker(name: &str) -> String {
    format!("__shared_{name}")
}
