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
#[derive(Clone)]
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
        let initializer = opening(source, promoted);
        // A value the page declared is found by the type that names it;
        // a value this component declared is found by the place it was
        // written. Everything after this line is the same rewrite.
        let text = if promoted.shared {
            // Nothing outside a `keep`, an `effect` or a `task` uses it,
            // and what is inside one reaches it by name. A binding here
            // would be a binding nobody reads.
            if promoted.reads.is_empty()
                && promoted.mutations.is_empty()
                && promoted.captures.is_empty()
            {
                continue;
            }
            // The same, for a value only a future writes: the future
            // builds its own handle before the block, so a binding out
            // here is one `rustc` would warn about in a generated file.
            // A capture written on the screen is a read out here, even
            // though it is written in prose: without this the binding
            // was skipped and the screen read a name nothing had bound.
            let inside_a_future = |span: Span| {
                component
                    .remote_regions
                    .iter()
                    .any(|region| contains(*region, span))
            };
            let only_in_a_future = promoted.reads.is_empty()
                && promoted.captures.iter().copied().all(inside_a_future)
                && promoted
                    .mutations
                    .iter()
                    .all(|mutation| inside_a_future(mutation.span));
            if only_in_a_future {
                continue;
            }
            format!(
                "let {name} = rustx_dom::store::shared(\"{name}\", {initializer});",
                name = promoted.name,
            )
        } else {
            format!(
                "let {name} = rustx_dom::state::cell(rustx_dom::state::Slot {{ \
                 file: file!(), line: {line}u32, column: {column}u32 }}, {initializer});",
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

    // The inline captures of a format string, which are reads the
    // rewrite cannot see: they live inside a literal.
    for promoted in &component.promoted {
        for edit in capture_edits(source, &promoted.name, component.body) {
            // A handler and a `keep` / `effect` / `task` are each
            // rewritten whole, so a capture inside one is that
            // rewrite's business. Pushing it here as well put two
            // edits over the same bytes, and the second replaced a
            // range whose end had moved -- the block came out with the
            // tail of a line still in it.
            let elsewhere = handlers_of(component)
                .iter()
                .any(|handler| contains(*handler, edit.span))
                || component
                    .calls
                    .iter()
                    .any(|call| contains(call.span, edit.span));
            if !elsewhere {
                edits.push(edit);
            }
        }
    }

    // Reads outside every handler — the ones the screen shows — and the
    // writes beside them.
    //
    // A write outside every handler is a write in a plain function: the
    // `send()` an `onClick` calls, with the work in it. It is rewritten
    // exactly as a write inside a handler is, bounded by the function's
    // body instead of the handler's braces, because that is where its
    // right-hand side ends. Without this the write was left as the
    // developer wrote it, against a binding this pass had just turned
    // into a `Shared<T>` — and rustc answered "expected `Shared<bool>`,
    // found `bool`", or "no method named `push` found for struct
    // `Shared<T>`", about a generated file nobody had opened.
    //
    // The reads are gathered before a single write is rewritten, for the
    // reason a handler gathers them first: `journal.push(brouillon)` is
    // one statement with a write and a read in it, and two edits over
    // the same bytes would each overwrite the other.
    let handlers = handlers_of(component);
    let elsewhere = |span: Span| {
        handlers.iter().any(|handler| contains(*handler, span))
            // A `keep`, an `effect` and a `task` are each rewritten
            // whole, so a use inside one belongs to that rewrite.
            || component
                .calls
                .iter()
                .any(|call| contains(call.span, span))
    };

    let mut reads: Vec<(&str, Edit)> = Vec::new();
    for promoted in &component.promoted {
        for read in &promoted.reads {
            if elsewhere(*read) {
                continue;
            }
            reads.push((
                promoted.name.as_str(),
                Edit {
                    span: *read,
                    text: format!("{}.get()", promoted.name),
                },
            ));
        }
    }

    for promoted in &component.promoted {
        for mutation in &promoted.mutations {
            if elsewhere(mutation.span) {
                continue;
            }
            let inside: Vec<Edit> = reads
                .iter()
                .map(|(_, read)| Edit {
                    span: read.span,
                    text: read.text.clone(),
                })
                .collect();
            let edit = rewrite_mutation(source, &promoted.name, mutation, component.body, &inside);
            // A read the write copied into its own replacement must not
            // also be edited on its own: two edits over the same bytes
            // each overwrite the other.
            reads.retain(|(_, read)| !contains(edit.span, read.span));
            edits.push(edit);
        }
    }

    // What is left is read on its own, and a read that opens a `let` and
    // has something called on it needs the clone bound first — see
    // `hoisted`. Decided here, after the writes have taken theirs, so no
    // binding is left behind for a read that no longer exists.
    for (name, read) in reads {
        match hoisted(source, name, read.span) {
            Some((hoist, bound)) => {
                edits.push(hoist);
                edits.push(Edit {
                    span: read.span,
                    text: bound,
                });
            }
            None => edits.push(read),
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
        format!("rustx_dom::state::Slot {{ file: file!(), line: {line}u32, column: {column}u32 }}")
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
         {held}, {asked}, &rustx_engine::router::current(), \
         || {{ let __work = "
    );
    let closing =
        format!("; async move {{ __work.await{tail} }} }}) else {{ return Element::Empty; }};");
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
    // thread for a task, where a handle could not go. A future does not
    // want this rewrite -- it builds its own, through handles -- so the
    // pass is asked for, not done in advance.
    let patched = || patched_arguments(source, component, call);

    let text = match call.kind {
        CallKind::Keep => {
            let patched = patched();
            format!("rustx_dom::state::keep_at({slot}, || {})", patched.trim())
        }
        // A closure is a thread, an `async` block is a future, and the
        // first word of the argument is what says which. A future needs
        // its handles built before it, so it has a shape of its own.
        CallKind::Task
            if crate::promote::is_a_future(
                source[call.arguments.start..call.arguments.end].trim(),
            ) =>
        {
            future_task(source, component, call, &slot)
        }
        // A bare `async { … }`: already a future, by the only word that
        // could have made it one.
        CallKind::Async => future_task(source, component, call, &slot),
        CallKind::Task => {
            let patched = patched();
            format!(
                "rustx_dom::state::keep_at({slot}, || rustx_engine::task::spawn({}))",
                patched.trim()
            )
        }
        CallKind::Effect => {
            let patched = patched();
            let (watched, run) = split_first_argument(patched.trim());
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
    // Every read and every capture of every value, gathered before a
    // single write is rewritten. A write's right-hand side may name
    // another page value — `vus = lancements;` — and that read belongs
    // to the write's own text: rewritten on its own, it was applied
    // after the write instead of inside it, and what reached rustc was
    // `vus.set(lancements).get()`.
    let mut reads_and_captures: Vec<Edit> = Vec::new();
    let mut writes: Vec<(&str, crate::promote::Mutation)> = Vec::new();
    let mut edits = Vec::new();

    for promoted in &component.promoted {
        let (reads, mutations) =
            crate::promote::uses_in(source, &promoted.name, call.arguments, true);
        // The inline captures of a format string count as reads here
        // too: a loop that writes a file writes `format!("clics:
        // {clics}")`, and that is the line this whole mechanism exists
        // for.
        let captures = capture_edits(source, &promoted.name, call.arguments);
        // A value the block never names needs no handle, and a handle
        // per page value would be a line of preamble for nothing.
        if reads.is_empty() && mutations.is_empty() && captures.is_empty() {
            continue;
        }
        let name = promoted.name.as_str();
        // A read goes to where the value is; a write still posts. The
        // handle that can do both is asked for only when the block
        // actually reads, so a block that only writes keeps the handle
        // it has always had — and the stricter bound it does not need.
        let reaching = !reads.is_empty() || !captures.is_empty();
        let read_edits: Vec<Edit> = reads
            .iter()
            .map(|read| Edit {
                span: *read,
                text: format!("{name}.get()"),
            })
            .collect();
        let handle = if reaching {
            "reach_until"
        } else {
            "remote_until"
        };
        // A value the page declared is found by its name, wherever the
        // component is; a value this component declared is already bound
        // above, as a cell.
        if promoted.shared {
            let initializer = opening(source, promoted);
            preamble.push_str(&format!(
                "let {name} = rustx_dom::store::shared(\"{name}\", {initializer})\
                 .{handle}(&__cancel); "
            ));
        } else {
            preamble.push_str(&format!("let {name} = {name}.{handle}(&__cancel); "));
        }
        reads_and_captures.extend(read_edits);
        reads_and_captures.extend(captures);
        for mutation in mutations {
            writes.push((name, mutation));
        }
    }

    // The writes first, each with every read in hand; then the reads
    // and captures the writes did not swallow.
    let mut swallowed: Vec<Span> = Vec::new();
    for (name, mutation) in &writes {
        let edit = rewrite_mutation_for(
            source,
            name,
            mutation,
            call.arguments,
            &reads_and_captures,
            Owning::Moved,
        );
        swallowed.push(edit.span);
        edits.push(edit);
    }
    edits.extend(
        reads_and_captures
            .into_iter()
            .filter(|edit| !swallowed.iter().any(|write| contains(*write, edit.span))),
    );

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
/// `{ let __a0 = r.clone(); rows.update(move |__v| { __v.push(__a0); }) }`
/// rather than `rows.update(move |__v| { __v.push(r.clone()); })`. Both are
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
        "{{ {bound}{name}.update(move |__v| {{ __v{before}({}){after}; }}) }}",
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
/// Every handler that touches a page value at all -- written, read, or
/// named inside a format string.
///
/// A read counts, and that was the bug: this used to collect the
/// handlers that hold a *mutation*, so a handler that only reads was
/// never rewritten and never got the `let n = n.clone();` that makes it
/// own its handle. One such handler compiled by luck; two did not, and
/// the second was `use of moved value` in a file the developer has
/// never opened:
///
/// ```text
/// <button onClick={move |_| show(names.len())}>a</button>
/// <button onClick={move |_| show(names.len() + 1)}>b</button>
/// ```
///
/// Both of those are ordinary, and the page was accepted and then
/// refused by rustc.
fn handlers_of(component: &Component) -> Vec<Span> {
    let mut spans: Vec<Span> = component
        .promoted
        .iter()
        .flat_map(|promoted| {
            promoted
                .mutations
                .iter()
                .map(|mutation| mutation.span)
                .chain(promoted.reads.iter().copied())
                .chain(promoted.captures.iter().copied())
        })
        .filter_map(|span| component.handler_around(span))
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
    let mut reads: Vec<(&str, Edit)> = Vec::new();
    for promoted in &component.promoted {
        for read in &promoted.reads {
            if contains(handler, *read) {
                reads.push((
                    promoted.name.as_str(),
                    Edit {
                        span: *read,
                        text: format!("{}.get()", promoted.name),
                    },
                ));
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
            let inside: Vec<Edit> = reads
                .iter()
                .map(|(_, read)| Edit {
                    span: read.span,
                    text: read.text.clone(),
                })
                .collect();
            let edit = rewrite_mutation(source, promoted.name.as_str(), mutation, handler, &inside);
            reads.retain(|(_, read)| !contains(edit.span, read.span));
            inner_edits.push(edit);
        }
        if !touched && promoted.reads.iter().any(|read| contains(handler, *read)) {
            touched = true;
        }
        if touched {
            captured.push(promoted.name.as_str());
        }
    }
    // A handler body is a block like any other, so a `let` in one that
    // borrows out of a page value needs the clone bound first, exactly
    // as it does out in the function — see `hoisted`.
    for (name, read) in reads {
        match hoisted(source, name, read.span) {
            Some((hoist, bound)) => {
                inner_edits.push(hoist);
                inner_edits.push(Edit {
                    span: read.span,
                    text: bound,
                });
            }
            None => inner_edits.push(read),
        }
    }

    // And the captures of a format string written inside the handler,
    // which are reads too — just ones written in prose.
    for promoted in &component.promoted {
        let theirs = capture_edits(source, &promoted.name, handler);
        if theirs.is_empty() {
            continue;
        }
        if !captured.contains(&promoted.name.as_str()) {
            captured.push(promoted.name.as_str());
        }
        inner_edits.extend(theirs);
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

/// What a read becomes when it opens a `let`, and what has to be bound
/// before that `let` for it to compile.
///
/// `get()` hands back the value, not a borrow of it, so a read is a
/// temporary — and `let agent = agents.get(0);` becomes
/// `let agent = agents.get().get(0);`, which borrows a temporary that
/// dies at the end of that very statement. rustc's answer, "temporary
/// value dropped while borrowed", named a generated file, and the only
/// `let` binding it could suggest was one the developer could not write
/// because the line it named was not theirs.
///
/// So the compiler writes it: the clone is bound first, under a name
/// nobody can spell, and the `let` reads that binding instead. Both on
/// the one line the `let` already occupied, because plan 2's position
/// map rests on the line count.
///
/// Only when the read *opens* the initializer and something is called on
/// it. Anywhere else it is an owned value used as an owned value, which
/// has always been fine; and opening the initializer is what makes
/// hoisting it change nothing — the expression was going to be evaluated
/// first in any case.
fn hoisted(source: &str, name: &str, read: Span) -> Option<(Edit, String)> {
    let bytes = source.as_bytes();
    // Something is called on the value, or read out of it. A read with
    // nothing after it is the value itself and needs no binding.
    if bytes.get(read.end) != Some(&b'.') {
        return None;
    }

    // Back over the whitespace to the `=` this initializer hangs off.
    let mut at = read.start;
    while at > 0 && bytes[at - 1].is_ascii_whitespace() && bytes[at - 1] != b'\n' {
        at -= 1;
    }
    if at == 0 || bytes[at - 1] != b'=' {
        return None;
    }
    // `==`, `>=`, `<=`, `!=`: a comparison, not a binding.
    let equals = at - 1;
    if equals > 0 && matches!(bytes[equals - 1], b'=' | b'!' | b'<' | b'>') {
        return None;
    }

    // And back over the pattern this `=` binds, as far as the statement
    // it belongs to begins — a `;`, a brace, a newline — and then the
    // first word of what was walked over has to be `let`. Anything else
    // (`self.field = …`, a struct literal's `field: value`) is not a
    // binding and gets nothing.
    let mut at = equals;
    while at > 0 && is_pattern_byte(bytes[at - 1]) {
        at -= 1;
    }
    let head = &source[at..equals];
    if !head.trim_start().starts_with("let ") {
        return None;
    }
    let at = at + (head.len() - head.trim_start().len());
    // The `let` has to be on the line the read is on: a hoist onto
    // another line would move every line below it away from plan 2's
    // position map.
    if source[at..read.start].contains('\n') {
        return None;
    }

    let bound = format!("__r{}", read.start);
    Some((
        Edit {
            span: Span { start: at, end: at },
            text: format!("let {bound} = {name}.get(); "),
        },
        bound,
    ))
}

/// A byte that may appear in the pattern and type of a `let`.
///
/// `mut rows`, `row: Option<&Row>`, `[first, ..]`. A bracket is left
/// out on purpose — `(`, `)`, `{`, `}` and `;` all end the walk, so it
/// stops at whatever statement came before instead of running into a
/// call's arguments. A destructuring `let (a, b) = …` therefore gets no
/// binding, which is the conservative half of the answer: the read is
/// left exactly as it was before.
fn is_pattern_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'_' | b':' | b'<' | b'>' | b',' | b'&' | b'\'' | b'[' | b']' | b' ' | b'\t'
        )
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
                        Owning::Borrowed => format!("{name}.update(|__v| {{ __v{patched}; }})"),
                        Owning::Moved => moved_method(name, &patched).unwrap_or_else(|| {
                            // Not a call this can take apart — then the
                            // blunt form, which is what it was before.
                            format!("{name}.update(move |__v| {{ __v{patched}; }})")
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
            // A comma out here separates things rather than belonging to
            // one: the arms of a `match`, the arguments of a call. A
            // comma *inside* the value — `vec![1, 2]`, `(a, b)`,
            // `Foo { a: 1, b: 2 }` — is behind a bracket and so at a
            // depth of its own. Reading past this one turned
            // `0 => dit = String::new(), _ => …` into
            // `dit.set(String::new(), _ => …)`, and the developer was
            // told their correct line did not parse.
            b',' if depth == 0 => break,
            _ => {}
        }
        index += 1;
    }
    Span {
        start: from,
        end: index,
    }
}

/// What `format!("clics: {clics}")` needs for `clics` to be the value
/// and not the handle.
///
/// Rust 2021 reads an inline capture from the environment, and the
/// environment holds a cell: the capture is inside a string literal, so
/// the rewrite — which reads names in code and not in prose — never saw
/// it, and `rustc` said `Shared<u64> doesn't implement Display` about a
/// line that is the most ordinary line in Rust.
///
/// The repair is a **named argument**: `format!("… {clics} …", clics =
/// clics.get())`. A named argument goes at the end of the list, which is
/// exactly where there is room for it — no positional argument moves,
/// nothing is reordered, and the capture now names the argument instead
/// of the environment.
fn capture_edits(source: &str, name: &str, within: Span) -> Vec<Edit> {
    crate::promote::format_captures(source, name, within)
        .into_iter()
        .map(|call| {
            // Before the closing delimiter, and after a comma only if
            // there is not one already: `format!("{x}",)` is written by
            // formatters and must not become `format!("{x}",,…)`.
            let last = source[call.arguments.start..call.arguments.end]
                .trim_end()
                .chars()
                .last()
                .unwrap_or(',');
            let separator = if last == ',' { "" } else { ", " };
            Edit {
                span: Span {
                    start: call.arguments.end,
                    end: call.arguments.end,
                },
                text: format!("{separator}{name} = {name}.get()"),
            }
        })
        .collect()
}

fn operator_end(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while index < bytes.len() && bytes[index] != b'=' {
        index += 1;
    }
    index + 1
}

/// Apply edits to a slice that starts at `offset` in the original.
fn apply_relative(text: &str, offset: usize, edits: Vec<Edit>) -> String {
    let mut out = text.to_string();
    for edit in ordered(edits) {
        let start = edit.span.start - offset;
        let end = edit.span.end - offset;
        if start <= end && end <= out.len() {
            out.replace_range(start..end, &edit.text);
        }
    }
    out
}

/// The edits, applied to the source they were measured against.
///
/// This is the one place every edit of the pass goes through, so it is
/// the one place the pass's central promise can be kept: **an edit never
/// changes the line count of what it replaces.** The position map rests
/// on the two files having the same lines, and a replacement one line
/// shorter than the declaration it stands for moves every `rustc`
/// diagnostic below it further up the file, for ever and invisibly.
///
/// Padding here rather than at each of the eight places an edit is
/// built. Three of them remembered to pad and five did not, which is how
/// a `let mut` whose initializer was written over four lines came out on
/// one and took three lines off the page:
///
/// ```ignore
/// let mut agents: Vec<Agent> = vec![
///     Agent::new("scout"),
///     Agent::new("relay"),
/// ];
/// ```
///
/// The initializer is deliberately collapsed onto one line — it is also
/// copied to wherever a future reaches the value, and a newline inserted
/// there would move lines too — so the newlines it gave up are given
/// back at the end of the replacement, where they change nothing but the
/// numbering they preserve.
fn apply(source: &str, edits: Vec<Edit>) -> String {
    let mut out = source.to_string();
    for edit in ordered(edits) {
        let Some(replaced) = source.get(edit.span.start..edit.span.end) else {
            // A span that is not a span of this source, or that falls
            // inside a character rather than between two. Nothing in the
            // pass should produce one; applying it anyway would panic,
            // and guessing at a nearby boundary would corrupt the file.
            continue;
        };
        if out.get(edit.span.start..edit.span.end).is_none() {
            continue;
        }
        let text = pad_to(edit.text, replaced);
        out.replace_range(edit.span.start..edit.span.end, &text);
    }
    out
}

/// The edits in the order they can be applied in, and only the ones
/// that can be.
///
/// Back to front, so that an edit's offsets still mean what they meant
/// in the source when its turn comes. That only holds while no two
/// edits overlap: an edit *inside* another one is applied first, the
/// text around it changes length, and the outer edit then replaces a
/// range that has moved -- it cuts in the wrong place and leaves the
/// tail of what it meant to replace sitting after its own replacement.
/// The result is not Rust, and what the developer was shown was a
/// parse error on a line they had written correctly.
///
/// An enclosing edit's text is built from the source it encloses, with
/// the reads inside it already patched, so it is the one that is
/// right: the nested edit is dropped rather than applied into a range
/// that has stopped existing.
fn ordered(mut edits: Vec<Edit>) -> Vec<Edit> {
    // Widest first at a given start, so an enclosing edit is always
    // seen before anything it encloses.
    edits.sort_by(|one, two| {
        one.span
            .start
            .cmp(&two.span.start)
            .then(two.span.end.cmp(&one.span.end))
    });

    let mut kept: Vec<Edit> = Vec::with_capacity(edits.len());
    for edit in edits {
        // A zero-width insertion shares an offset with the edit it
        // belongs beside rather than overlapping it: `hoisted` puts one
        // in front of a `let` whose read it also replaces.
        let empty = edit.span.start == edit.span.end;
        let swallowed = kept.iter().any(|held| {
            held.span.start != held.span.end
                && edit.span.start >= held.span.start
                && edit.span.end <= held.span.end
                && !(empty && edit.span.start == held.span.start)
        });
        if !swallowed {
            kept.push(edit);
        }
    }

    kept.sort_by_key(|edit| std::cmp::Reverse(edit.span.start));
    kept
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

/// The closure that gives a value its first value.
///
/// `|| INIT`, and `|| -> TYPE { INIT }` when the developer wrote a type
/// on the declaration. The type has to travel: `Vec::new()`, `None` and
/// `Default::default()` all take theirs from the annotation, and
/// without it the store's `T` has nothing to be inferred from —
/// `let mut visites: Vec<String> = Vec::new();` came out as
/// `shared("visites", || Vec::new())`, and rustc answered "cannot infer
/// type of the type parameter `T`" against a generated line.
///
/// An annotation spread over several lines is left alone: this text is
/// inserted somewhere the declaration is not, and a newline inserted
/// there would move every line below it away from the position map.
///
/// And so is the initializer, for exactly the same reason — which is
/// why it is put on one line first. A declaration written over several
/// lines
///
/// ```ignore
/// let mut agents: Vec<Agent> = vec![
///     Agent { name: "scout".to_string() },
/// ];
/// ```
///
/// is relocated to wherever the value is reached, once per use, and it
/// brought its newlines with it: a page whose `keep` read that value
/// four times grew by sixteen lines, and every `rustc` diagnostic
/// below the declaration was reported sixteen lines further down than
/// the developer wrote it. Newlines between tokens mean nothing to
/// Rust, so collapsing them changes no program.
fn opening(source: &str, promoted: &crate::promote::Promoted) -> String {
    let initializer = &source[promoted.initializer.start..promoted.initializer.end];
    let flattened = one_line(initializer);
    let initializer: &str = flattened.as_deref().unwrap_or(initializer);
    match promoted.annotation {
        Some(span) => {
            let declared = &source[span.start..span.end];
            if declared.contains('\n') {
                format!("|| {initializer}")
            } else {
                format!("|| -> {declared} {{ {initializer} }}")
            }
        }
        None => format!("|| {initializer}"),
    }
}

/// The same Rust on one line, or `None` when it cannot be.
///
/// Only whitespace between tokens is touched. A newline inside a string
/// is part of the string's value, and a `//` comment swallows whatever
/// follows it on the line, so either one is a refusal rather than a
/// guess: the caller keeps the text as written and accepts the drift,
/// which is what it did for every initializer before this existed.
fn one_line(text: &str) -> Option<String> {
    if !text.contains('\n') {
        return None;
    }

    #[derive(PartialEq)]
    enum In {
        Code,
        Str,
        Char,
        Raw(usize),
        Block(usize),
    }

    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(text.len());
    let mut state = In::Code;
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        match state {
            In::Code => match byte {
                b'"' => {
                    // A raw string, whose hashes say where it ends, and
                    // which is allowed to hold a newline — so one that
                    // does is a refusal like any other literal.
                    let hashes = count_back(&out, b'#');
                    let raw = hashes > 0
                        && out
                            .get(..out.len() - hashes)
                            .is_some_and(|before| before.last() == Some(&b'r'));
                    state = if raw { In::Raw(hashes) } else { In::Str };
                    out.push(b'"');
                }
                b'\'' => {
                    // A lifetime is not a character literal: `&'a str`.
                    let lifetime = bytes
                        .get(at + 1)
                        .is_some_and(|next| next.is_ascii_alphabetic() || *next == b'_')
                        && bytes.get(at + 2) != Some(&b'\'');
                    if !lifetime {
                        state = In::Char;
                    }
                    out.push(b'\'');
                }
                b'/' if bytes.get(at + 1) == Some(&b'/') => return None,
                b'/' if bytes.get(at + 1) == Some(&b'*') => {
                    state = In::Block(1);
                    out.extend_from_slice(b"/*");
                    at += 2;
                    continue;
                }
                b'\n' | b'\r' | b'\t' => {
                    if out.last() != Some(&b' ') && !out.is_empty() {
                        out.push(b' ');
                    }
                }
                _ => out.push(byte),
            },
            In::Str | In::Char => {
                if byte == b'\n' {
                    return None;
                }
                out.push(byte);
                if byte == b'\\' {
                    // An escaped quote does not close the literal.
                    if let Some(next) = bytes.get(at + 1) {
                        out.push(*next);
                        at += 2;
                        continue;
                    }
                }
                if (state == In::Str && byte == b'"') || (state == In::Char && byte == b'\'') {
                    state = In::Code;
                }
            }
            In::Raw(hashes) => {
                if byte == b'\n' {
                    return None;
                }
                out.push(byte);
                if byte == b'"' && count_forward(bytes, at + 1, b'#') >= hashes {
                    state = In::Code;
                }
            }
            In::Block(depth) => {
                if byte == b'\n' || byte == b'\r' || byte == b'\t' {
                    out.push(b' ');
                } else {
                    out.push(byte);
                }
                if byte == b'*' && bytes.get(at + 1) == Some(&b'/') {
                    out.push(b'/');
                    state = if depth == 1 {
                        In::Code
                    } else {
                        In::Block(depth - 1)
                    };
                    at += 2;
                    continue;
                }
                if byte == b'/' && bytes.get(at + 1) == Some(&b'*') {
                    out.push(b'*');
                    state = In::Block(depth + 1);
                    at += 2;
                    continue;
                }
            }
        }
        at += 1;
    }

    // An unterminated literal is not something to put on one line.
    match state {
        In::Code => String::from_utf8(out).ok(),
        _ => None,
    }
}

/// How many `wanted` the text ends with.
fn count_back(text: &[u8], wanted: u8) -> usize {
    text.iter()
        .rev()
        .take_while(|byte| **byte == wanted)
        .count()
}

/// How many `wanted` start at `from`.
fn count_forward(bytes: &[u8], from: usize, wanted: u8) -> usize {
    bytes[from.min(bytes.len())..]
        .iter()
        .take_while(|byte| **byte == wanted)
        .count()
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
        let initializer = opening(source, promoted);
        let accessor = format!(
            "rustx_dom::store::shared(\"{name}\", {initializer})",
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
