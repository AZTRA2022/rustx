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
            // Nothing is left behind: the value is built on the first ask,
            // by whichever component asks first, and found by the name
            // it was written with. What was a declaration is now the
            // blank line it occupied, so every line after it keeps its
            // number.
            text: pad_to(String::new(), original),
        });
    }

    Ok(apply(source, edits))
}

fn edits_for(source: &str, component: &Component) -> Vec<Edit> {
    let mut edits = Vec::new();

    for call in &component.calls {
        edits.push(call_edit(source, component, call));
    }

    for waited in &component.awaited {
        edits.extend(awaited_edits(source, waited));
    }

    // A `let mut` nothing writes to is left exactly as it was written —
    // there is no state to keep — but the warning it earns names a
    // generated file, so it is silenced where it is generated. On the
    // same line, like every other edit here, because the line map has to
    // keep holding.
    for declaration in &component.unwritten {
        edits.push(Edit {
            span: Span {
                start: declaration.start,
                end: declaration.start,
            },
            text: "#[allow(unused_mut)] ".to_string(),
        });
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
            // Nothing outside a `keep`, an `effect` or a `task` uses it,
            // and what is inside one reaches it by name. A binding here
            // would be a binding nobody reads.
            if promoted.reads.is_empty() && promoted.mutations.is_empty() {
                continue;
            }
            // The same, for a value only a future writes: the future
            // builds its own handle before the block, so a binding out
            // here is one `rustc` would warn about in a generated file.
            let only_in_a_future = promoted.reads.is_empty()
                && promoted.mutations.iter().all(|mutation| {
                    component
                        .remote_regions
                        .iter()
                        .any(|region| contains(*region, mutation.span))
                });
            if only_in_a_future {
                continue;
            }
            format!(
                "let {name} = rustx_dom::store::shared(\"{name}\", || {initializer});",
                name = promoted.name,
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
/// `let answer = work().await;` without the waiting.
///
/// Two edits, one at each end of the statement, and nothing in between:
/// the expression stays exactly where it was written, so a read of state
/// inside it is rewritten by the same pass that rewrites every other
/// read. What the ends become is an ask that is remembered and a render
/// that draws nothing until the answer is there.
fn awaited_edits(source: &str, waited: &crate::promote::Awaited) -> Vec<Edit> {
    let slot_at = |at: usize| {
        let (line, column) = line_and_column(source, at);
        format!(
            "rustx_dom::state::Slot {{ file: file!(), line: {line}u32, column: {column}u32 }}"
        )
    };
    // Two places, because one slot holds one type: the cell the answer
    // lands in, and the memory of having asked.
    let held = slot_at(waited.prefix.start);
    let asked = slot_at(waited.suffix.start);
    let bound = if waited.mutable {
        format!("mut {}", waited.name)
    } else {
        waited.name.clone()
    };
    // The type the developer wrote is kept, around the `Option` the cell
    // holds, so inference has no more to do than it had before.
    let annotation = match &waited.annotation {
        Some(written) => format!(": Option<{written}>"),
        None => String::new(),
    };
    // The future is built on the render thread -- which is what lets the
    // expression read a cell -- and awaited on the runtime's. What the
    // developer wrote after the `.await` goes with it, because the
    // answer it works on only exists there.
    let tail = source[waited.suffix.start + ".await".len()..waited.suffix.end]
        .trim_end()
        .trim_end_matches(';');
    let opening = format!(
        "let Some({bound}){annotation} = rustx_engine::task::awaited( \
         rustx_dom::state::cell({held}, || None), {asked}, \
         || {{ let __work = "
    );
    let closing = format!(
        "; async move {{ __work.await{tail} }} }}) else {{ return Element::Empty; }};"
    );
    vec![
        Edit {
            span: waited.prefix,
            text: pad_to(opening, &source[waited.prefix.start..waited.prefix.end]),
        },
        Edit {
            span: waited.suffix,
            text: pad_to(closing, &source[waited.suffix.start..waited.suffix.end]),
        },
    ]
}

fn call_edit(source: &str, component: &Component, call: &crate::promote::Call) -> Edit {
    let (line, column) = line_and_column(source, call.span.start);
    let slot =
        format!("rustx_dom::state::Slot {{ file: file!(), line: {line}u32, column: {column}u32 }}");
    // A shared value written inside these arguments is reached by its
    // name, not by a handle: this code runs later, and on another
    // thread for a task, where a handle could not go.
    let patched = patched_arguments(source, component, call);
    let arguments = patched.trim();

    let text = match call.kind {
        CallKind::Keep => {
            format!("rustx_dom::state::keep_at({slot}, || {arguments})")
        }
        // A closure is a thread, an `async` block is a future, and the
        // first word of the argument is what says which. A future needs
        // its handles built before it, so it has a shape of its own.
        CallKind::Task if crate::promote::is_a_future(arguments) => {
            future_task(source, component, call, &slot)
        }
        // A bare `async { … }`: already a future, by the only word that
        // could have made it one.
        CallKind::Async => future_task(source, component, call, &slot),
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

/// `task(async { … })`, with its writes turned into remote writes.
///
/// Two halves. Before the block, on the render thread, a handle per
/// value the block writes — that is the only place a handle can be
/// built, because the cell and the store live here. Inside the block,
/// `n += 1` becomes `n.update(|__v| *__v += 1)`, which is the same shape
/// a handler gets; `Remote` answers to the same two names as `State` on
/// purpose, so the rewrite of a write does not have to know where it is.
fn future_task(
    source: &str,
    component: &Component,
    call: &crate::promote::Call,
    slot: &str,
) -> String {
    let text = &source[call.arguments.start..call.arguments.end];
    let mut preamble = String::new();
    let mut edits = Vec::new();

    for promoted in &component.promoted {
        let (_, mutations) =
            crate::promote::uses_in(source, &promoted.name, call.arguments, true);
        if mutations.is_empty() {
            continue;
        }
        let name = promoted.name.as_str();
        // A value the page declared is found by its name, wherever the
        // component is; a value this component declared is already bound
        // above, as a cell.
        if promoted.shared {
            let initializer = &source[promoted.initializer.start..promoted.initializer.end];
            preamble.push_str(&format!(
                "let {name} = rustx_dom::store::shared(\"{name}\", || {initializer})\
                 .remote_until(&__cancel); "
            ));
        } else {
            preamble.push_str(&format!("let {name} = {name}.remote_until(&__cancel); "));
        }
        for mutation in &mutations {
            edits.push(rewrite_mutation_for(
                source,
                name,
                mutation,
                call.arguments,
                &[],
                Owning::Moved,
            ));
        }
    }

    let arguments = apply_relative(text, call.arguments.start, edits);
    let arguments = arguments.trim();
    // The token every handle in the preamble shares with the `Task`.
    // Dropping the token cancels it, so a write this future had already
    // posted is refused rather than applied to the page that came next
    // -- the only thing that stops it for a value found by a name,
    // because a name outlives every page.
    format!(
        "rustx_dom::state::keep_at({slot}, || {{ \
         let __cancel = rustx_dom::state::Cancel::new(); {preamble}\
         rustx_engine::task::spawn_future_until(__cancel, {arguments}) }})"
    )
}


/// The arguments of a method call, as the text between its parentheses
/// split at the commas that belong to it.
///
/// Depth-aware, so `insert(k, f(a, b))` is two arguments and not three,
/// and quote-aware, so a comma inside a string is not a separator.
fn arguments_of(call: &str) -> Option<(&str, Vec<&str>, &str)> {
    let open = call.find('(')?;
    let bytes = call.as_bytes();
    let mut depth = 0usize;
    let mut inside: Option<u8> = None;
    let mut escaped = false;
    let mut close = None;
    let mut commas = Vec::new();
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        if escaped {
            escaped = false;
            continue;
        }
        match (inside, byte) {
            (Some(_), b'\\') => escaped = true,
            (Some(quote), byte) if *byte == quote => inside = None,
            (Some(_), _) => {}
            (None, b'"') => inside = Some(b'"'),
            (None, b'\'') => inside = Some(b'\''),
            (None, b'(' | b'[' | b'{') => depth += 1,
            (None, b')' | b']' | b'}') => {
                depth -= 1;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            (None, b',') if depth == 1 => commas.push(index),
            (None, _) => {}
        }
    }
    let close = close?;
    let mut arguments = Vec::new();
    let mut from = open + 1;
    for comma in commas {
        arguments.push(call[from..comma].trim());
        from = comma + 1;
    }
    let last = call[from..close].trim();
    if !last.is_empty() || !arguments.is_empty() {
        arguments.push(last);
    }
    Some((&call[..open], arguments, &call[close + 1..]))
}

/// A method write whose arguments are evaluated before the closure.
///
/// `rows.push(r.clone())` becomes
/// `{ let __a0 = r.clone(); rows.update(move |__v| __v.push(__a0)) }`
/// rather than `rows.update(move |__v| __v.push(r.clone()))`. Both are
/// `Send + 'static`, and only the first leaves `r` usable on the line
/// after: a `move` closure takes ownership of everything the expression
/// mentions, and the error it produced named a generated file for code
/// that would have compiled against a plain local.
///
/// One line, because plan 2's position map rests on the line count — a
/// braced block keeps it.
fn moved_method(name: &str, call: &str) -> Option<String> {
    let (before, arguments, after) = arguments_of(call)?;
    let mut bound = String::new();
    let mut passed = Vec::new();
    for (at, argument) in arguments.iter().enumerate() {
        if argument.is_empty() {
            continue;
        }
        bound.push_str(&format!("let __a{at} = {argument}; "));
        passed.push(format!("__a{at}"));
    }
    Some(format!(
        "{{ {bound}{name}.update(move |__v| __v{before}({}){after}) }}",
        passed.join(", ")
    ))
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
                return (arguments[..index].trim(), arguments[index + 1..].trim());
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
    rewrite_mutation_for(source, name, mutation, handler, reads, Owning::Borrowed)
}

/// Does the closure a write becomes own what it captures?
///
/// A handler runs on the thread that made it and may borrow what is
/// around it. A write posted from a future is a `FnOnce + Send +
/// 'static` that the frame runs later, so it owns everything it was
/// given — without this, `rows.push(step)` in an `async` block was a
/// closure borrowing a loop variable, and the error named a file the
/// developer had never opened.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owning {
    Borrowed,
    Moved,
}

fn rewrite_mutation_for(
    source: &str,
    name: &str,
    mutation: &crate::promote::Mutation,
    handler: Span,
    reads: &[Edit],
    owning: Owning,
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
                text: match owning {
                    Owning::Borrowed => format!(
                        "{name}.update(|__v| *__v {operator} {})",
                        read_patched(source, value, reads)
                    ),
                    // The right-hand side first, so the closure owns a
                    // binding and not everything the expression named.
                    Owning::Moved => format!(
                        "{{ let __a0 = {}; {name}.update(move |__v| *__v {operator} __a0) }}",
                        read_patched(source, value, reads)
                    ),
                },
            }
        }
        MutationKind::Method => {
            let call = expression_after(source, mutation.span.end, handler);
            Edit {
                span: Span {
                    start: mutation.span.start,
                    end: call.end,
                },
                text: {
                    let patched = read_patched(source, call, reads);
                    match owning {
                        Owning::Borrowed => format!("{name}.update(|__v| __v{patched})"),
                        Owning::Moved => moved_method(name, &patched).unwrap_or_else(|| {
                            // Not a call this can take apart — then the
                            // blunt form, which is what it was before.
                            format!("{name}.update(move |__v| __v{patched})")
                        }),
                    }
                },
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

/// The arguments of a `keep`, `effect` or `task`, with every use of a
/// shared value rewritten to reach it by name.
///
/// A handler captures a handle, because a handler runs on this thread
/// with the render still in memory. These arguments do not: `task`'s
/// body is another thread's, and what it posts back has to be able to
/// cross on its own. A name crosses; an `Rc` does not.
fn patched_arguments(source: &str, component: &Component, call: &crate::promote::Call) -> String {
    let text = &source[call.arguments.start..call.arguments.end];
    let mut edits = Vec::new();

    for promoted in component.promoted.iter().filter(|promoted| promoted.shared) {
        let initializer = &source[promoted.initializer.start..promoted.initializer.end];
        let accessor = format!(
            "rustx_dom::store::shared(\"{name}\", || {initializer})",
            name = promoted.name
        );
        let (reads, mutations) =
            crate::promote::uses_in(source, &promoted.name, call.arguments, false);

        let read_edits: Vec<Edit> = reads
            .iter()
            .map(|read| Edit {
                span: *read,
                text: format!("{accessor}.get()"),
            })
            .collect();

        for mutation in &mutations {
            edits.push(rewrite_mutation(
                source,
                &accessor,
                mutation,
                call.arguments,
                &read_edits,
            ));
        }
        edits.extend(read_edits);
    }

    apply_relative(text, call.arguments.start, edits)
}
