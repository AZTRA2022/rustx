//! Deciding which locals become state. Only deciding.
//!
//! The rule: a local **mutated from an event handler's closure**
//! becomes a persistent cell. Everything else stays an ordinary `let`,
//! recomputed each render, costing nothing.
//!
//! The reading half used to count too — §7.4 of the design document
//! asked for a local both shown and written. It was dropped because a
//! local written by a handler and shown nowhere has no second meaning
//! to fall back on: a handler is stored as an `Rc<dyn Fn>`, so a
//! closure that assigns to a captured local does not compile at all,
//! and the developer read a borrow-check error about a file in a
//! temporary directory. Promoting it costs one cell and makes the
//! sentence in the documentation true as written — a `let mut`, and
//! its place says its scope.
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
    /// The arguments of every `task(async … )`, which run on another
    /// thread. A write in one is state, like a write in a handler, and
    /// it is rewritten to go through a handle that may cross a thread.
    pub remote_regions: Vec<Span>,
    pub promoted: Vec<Promoted>,
    /// The `keep`, `effect` and `task` calls written in the body.
    pub calls: Vec<Call>,
    /// The `let mut` declarations nothing writes to, which stay plain
    /// locals. Half-written state: the value is there, the handler that
    /// will change it is not typed yet.
    pub unwritten: Vec<Span>,
}

/// A call to one of the three names a component is given.
///
/// They are found here rather than in the rewrite for the same reason
/// promotion is: what was decided can be asked about, instead of being
/// inferred from generated Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Call {
    pub kind: CallKind,
    /// The whole call, parentheses included.
    pub span: Span,
    /// What is between the parentheses.
    pub arguments: Span,
}

/// Which of the three it is. Nothing else is touched: `keep` outside a
/// `fn … -> Element` is an ordinary name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// `keep(EXPR)` — a value built once.
    Keep,
    /// `effect(WATCHED, RUN)` — work that happens again when `WATCHED`
    /// changes.
    Effect,
    /// `task(WORK)` — a thread that stops when the component goes.
    Task,
}

/// A local that became state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promoted {
    pub name: String,
    /// Whether it was declared at the top of the page rather than in
    /// this component. A shared value is bound at the start of the
    /// body instead of where it was written, and everything after is
    /// the same rewrite a component's own state gets.
    pub shared: bool,
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
        if let Some(at) = async_before(source, body.start) {
            return Err(CompileError::flagged(
                at,
                format!(
                    "`{name}` is declared `async`. A component is asked for the screen as \
                     it is now, so it cannot wait — give the waiting to `task(…)`, which \
                     runs it beside the component and stops when the component goes."
                ),
            )
            .resolve(source));
        }
        let returns_jsx = find_jsx_range(source, body.start, body.end)?;
        let handlers = handler_closures(source, &returns_jsx);
        let parameters = parameter_names(source, body.start);

        let locals = locals_of(source, body);
        // Shadowing is idiomatic Rust, and matching occurrences by name
        // cannot tell two `n`s apart: both would claim the same spans
        // and the edits would overwrite each other. Refusing is the
        // honest answer until the rewrite understands scopes.
        if let Some(second) = shadowed(&locals) {
            return Err(CompileError::flagged(
                second.declaration.start,
                format!(
                    "`{}` is declared twice in this component. rustx cannot yet tell \
                     two locals of the same name apart when one of them holds state — \
                     give the second one a different name.",
                    second.name
                )
                .to_string(),
            )
            .resolve(source));
        }

        let in_code = code_spans(source, body, &returns_jsx);
        // The three calls are rewritten whole, arguments included, so a
        // use of a value inside one belongs to that rewrite. Editing it
        // twice would leave the call half replaced.
        let calls = calls_of(source, body, &in_code, &handlers)?;
        let inside_a_call = |span: &Span| calls.iter().any(|call| contains(call.span, *span));

        // `task(async move { … })`. What decides is the first word of
        // the argument, exactly as it decides which spawn is emitted.
        let remote_regions: Vec<Span> = calls
            .iter()
            .filter(|call| call.kind == CallKind::Task)
            .filter(|call| {
                let arguments = source[call.arguments.start..call.arguments.end].trim_start();
                arguments.strip_prefix("async").is_some_and(|rest| {
                    !rest.starts_with(|letter: char| letter.is_alphanumeric() || letter == '_')
                })
            })
            .map(|call| call.arguments)
            .collect();
        let writes_from = |name: &str| -> Vec<Mutation> {
            let in_a_handler = handlers
                .iter()
                .flat_map(|handler| mutations_in(source, name, *handler, &in_code, true));
            // A handler is one of the braced regions `in_code` holds, so
            // it is found there. The arguments of a call are inside one
            // of those regions rather than being one, so the region is
            // its own code — the same thing `uses_in` does for exactly
            // this reason.
            let in_a_future = remote_regions
                .iter()
                .flat_map(|region| mutations_in(source, name, *region, &[*region], true));
            in_a_handler.chain(in_a_future).collect()
        };

        let mut promoted = Vec::new();
        let mut unwritten = Vec::new();
        for local in &locals {
            if parameters.contains(&local.name) {
                continue;
            }
            let mutations: Vec<Mutation> = writes_from(&local.name);

            if mutations.is_empty() {
                // Nothing writes to it, so there is nothing to keep
                // across renders and it stays what it was written as —
                // a plain local, which `rustc` then warns is mutable
                // for no reason, pointing at a file in a temporary
                // directory the developer has never seen.
                unwritten.push(local.declaration);
                continue;
            }

            // Everywhere the name appears without writing to it — the
            // rewrite turns each into a read of the cell.
            let after_declaration: Vec<Span> = in_code
                .iter()
                .filter(|span| span.start >= local.declaration.end)
                .copied()
                .collect();
            let reads = occurrences(source, &local.name, &after_declaration)
                .into_iter()
                .filter(|span| !mutations.iter().any(|mutation| mutation.span == *span))
                .filter(|span| !inside_a_call(span))
                .collect();
            promoted.push(Promoted {
                name: local.name.clone(),
                shared: false,
                declaration: local.declaration,
                initializer: local.initializer,
                reads,
                mutations,
            });
        }

        // And the values the page declared above every component. A
        // component that declares its own `points` is talking about its
        // own: the local wins, and nothing it writes reaches the shared
        // one.
        for declaration in shared_of(source) {
            if locals.iter().any(|local| local.name == declaration.name)
                || parameters.contains(&declaration.name)
            {
                continue;
            }
            let mutations: Vec<Mutation> = writes_from(&declaration.name);
            let every_use = occurrences(source, &declaration.name, &in_code);
            let reads: Vec<Span> = every_use
                .iter()
                .copied()
                .filter(|span| !mutations.iter().any(|mutation| mutation.span == *span))
                .filter(|span| !inside_a_call(span))
                .collect();
            // A use inside a `keep`, an `effect` or a `task` counts as
            // one. That code runs later, on another thread for a task,
            // so it reaches the value by name rather than through a
            // handle — but a component credited with no use at all was
            // never asked to reach it, and the name was left bare, in a
            // scope that had nothing of that name in it.
            let in_a_call = every_use.iter().any(inside_a_call);
            if reads.is_empty() && mutations.is_empty() && !in_a_call {
                continue;
            }
            promoted.push(Promoted {
                name: declaration.name,
                shared: true,
                // Bound at the top of the body, because that is where a
                // value the component did not declare has to come into
                // scope. The span is empty: this is an insertion.
                declaration: Span {
                    start: body.start + 1,
                    end: body.start + 1,
                },
                initializer: declaration.initializer,
                reads,
                mutations,
            });
        }

        // A write to state in the body itself — typically in a loop
        // before the `return`, which is where it was found. Nothing
        // rewrites it, so what reaches rustc is an assignment to a copy
        // in a generated file; and rewriting it would be worse, because
        // a write during a render asks for another render, which would
        // write it again. Refused where it was written instead.
        for held in &promoted {
            let written = mutations_in(source, &held.name, body, &in_code, false);
            if let Some(mutation) = written.iter().find(|mutation| {
                !handlers
                    .iter()
                    .any(|handler| contains(*handler, mutation.span))
                    && !inside_a_call(&mutation.span)
                    // `let mut n = 0;` is an assignment too, and it is
                    // the one write that is not a write to state: it is
                    // what declares it.
                    && !locals
                        .iter()
                        .any(|local| contains(local.declaration, mutation.span))
            }) {
                return Err(CompileError::flagged(
                    mutation.span.start,
                    format!(
                        "`{}` is written while `{name}` is being drawn. A component is asked \
                         for the screen as it is now, so a write here would ask for another \
                         render, which would write it again — write it in a handler \
                         (`onClick={{…}}`), in an `effect(…)`, or in a `task(…)`.",
                        held.name
                    ),
                )
                .resolve(source));
            }
        }

        // A read of state inside an `async` block. The cell is an `Rc`
        // on the render thread and the block runs on another, so there
        // is nothing to read from there — and leaving it would hand the
        // developer a type error about a generated file. Refused where
        // it was written, with the way out.
        for held in &promoted {
            for region in &remote_regions {
                let (reads, _) = uses_in(source, &held.name, *region);
                if let Some(read) = reads.first() {
                    return Err(CompileError::flagged(
                        read.start,
                        format!(
                            "`{}` is read inside a `task(async ...)`. A value lives on the thread that draws, and this block runs beside it, so there is nothing there to read -- read it before the block and move the copy in, or post the work back with a write.",
                            held.name
                        ),
                    )
                    .resolve(source));
                }
            }
        }

        components.push(Component {
            handlers,
            remote_regions,
            name,
            body,
            returns_jsx,
            promoted,
            calls,
            unwritten,
        });
    }
    Ok(components)
}

/// The three calls, wherever they are written in a component's code.
///
/// A call inside an event handler is left alone: a handler runs long
/// after the render returned, where none of the three mean anything, and
/// the handler is rewritten whole — an edit inside it would collide with
/// that one.
fn calls_of(
    source: &str,
    body: Span,
    in_code: &[Span],
    handlers: &[Span],
) -> Result<Vec<Call>, CompileError> {
    let bytes = source.as_bytes();
    let mut calls = Vec::new();
    // A nested `fn` is not a component. Its `keep` has no instance to
    // belong to, and attaching it to whichever component happens to be
    // rendering would be worse than leaving it as the error it is.
    let nested = nested_functions(source, body);

    for (name, kind) in [
        ("keep", CallKind::Keep),
        ("effect", CallKind::Effect),
        ("task", CallKind::Task),
    ] {
        for span in occurrences(source, name, in_code) {
            if handlers.iter().any(|handler| contains(*handler, span)) {
                continue;
            }
            if nested.iter().any(|inner| contains(*inner, span)) {
                continue;
            }
            // `camera.keep(…)`, `Self::keep(…)` and `devices::keep(…)`
            // are somebody else's.
            if span.start > 0 && matches!(bytes[span.start - 1], b'.' | b':') {
                continue;
            }
            let open = skip_spaces(bytes, span.end);
            if bytes.get(open) != Some(&b'(') {
                continue;
            }
            let Some(close) = matching(bytes, open, b'(', b')') else {
                continue;
            };
            calls.push(Call {
                kind,
                span: Span {
                    start: span.start,
                    end: close + 1,
                },
                arguments: Span {
                    start: open + 1,
                    end: close,
                },
            });
        }
    }

    calls.sort_by_key(|call| call.span.start);

    // Two edits over the same bytes shred each other: the inner one is
    // applied first, and the outer one then replaces a range whose end
    // has moved. A correct nested rewrite is an expression-tree walk
    // this file is not built for, so the honest answer is a sentence.
    if let Some((outer, inner)) = nested_pair(&calls) {
        return Err(CompileError::flagged(
            outer.span.start,
            format!(
                "rustx cannot yet see a `{}` written inside a `{}`. \
                 Give the inner one a `let` of its own on the line above, \
                 and pass that.",
                name_of(inner.kind),
                name_of(outer.kind)
            ),
        )
        .resolve(source));
    }

    // The three names belong to the framework inside a component. A
    // component that declares one of them *and* calls it would have its
    // own call rewritten into the framework's, which is a bug nobody
    // could see in their own source.
    if let Some(call) = calls
        .iter()
        .find(|call| declares_local(source, body, name_of(call.kind)))
    {
        return Err(CompileError::flagged(
            call.span.start,
            format!(
                "this component declares a local named `{name}` and also calls `{name}`. \
                 Inside a component, `{name}` is the framework's — rename the local.",
                name = name_of(call.kind)
            ),
        )
        .resolve(source));
    }

    Ok(calls)
}

/// The first pair where one call is written inside another's arguments.
fn nested_pair(calls: &[Call]) -> Option<(&Call, &Call)> {
    calls.iter().find_map(|outer| {
        calls
            .iter()
            .find(|inner| inner.span != outer.span && contains(outer.arguments, inner.span))
            .map(|inner| (outer, inner))
    })
}

/// What the developer wrote, for a message that quotes them.
fn name_of(kind: CallKind) -> &'static str {
    match kind {
        CallKind::Keep => "keep",
        CallKind::Effect => "effect",
        CallKind::Task => "task",
    }
}

/// Does this body declare a local called `name`?
fn declares_local(source: &str, body: Span, name: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = body.start;
    while let Some(at) = find_word(source, "let", index) {
        if at >= body.end {
            break;
        }
        index = at + 3;
        let Some(first) = word_after(source, index) else {
            continue;
        };
        let declared = if first.0 == "mut" {
            word_after(source, first.1)
        } else {
            Some(first)
        };
        if declared.is_some_and(|(declared, _)| declared == name) {
            return true;
        }
        let _ = bytes;
    }
    false
}

/// The bodies of the `fn`s written inside this body.
fn nested_functions(source: &str, body: Span) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut index = body.start;

    while let Some(at) = find_word(source, "fn", index) {
        if at >= body.end {
            break;
        }
        index = at + 2;
        let Some(brace) = next_byte(bytes, index, b'{') else {
            break;
        };
        if brace >= body.end {
            break;
        }
        let Ok(end) = skip_braces(source, brace) else {
            break;
        };
        found.push(Span { start: at, end });
        index = end;
    }
    found
}

/// A local declared `let mut` at the top level of a component's body.
struct Local {
    name: String,
    declaration: Span,
    initializer: Span,
}

/// `fn NAME(…) -> Element {`, and the span of its body.
/// Every component this file declares, with the names of its
/// parameters in the order they were written.
///
/// What a `<Row count={2} label={…} />` needs in order to become a
/// call: the props are written by name, the arguments go by position,
/// and only the declaration knows which position a name has. A
/// component from another file is therefore not callable as a tag.
pub fn component_signatures(source: &str) -> Vec<(String, Vec<String>)> {
    components_of(source)
        .into_iter()
        .map(|(name, body)| (name, parameter_names(source, body.start)))
        .collect()
}

/// Where the `async` of an `async fn … -> Element` sits, if it has one.
///
/// `async` in front of a component is a reasonable guess — work that
/// waits is exactly what a page needs — and the answer is `task`, not a
/// future the renderer would have to await. Caught here, the developer
/// reads it on the line they wrote; left alone, rustc refuses to cast a
/// future to a function in a generated file they have never seen.
fn async_before(source: &str, body_start: usize) -> Option<usize> {
    let declaration = &source[..body_start];
    let at = declaration.rfind("fn")?;
    let before = declaration[..at].trim_end();
    before
        .ends_with("async")
        .then(|| before.len() - "async".len())
}

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
        found.push((name.0, Span { start: brace, end }));
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

/// Every comment in the file, `//` and `/* */` alike, and every string
/// literal: text that looks like code and is not.
fn comments(source: &str) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(after) = crate::scan::skip_comment(bytes, index) {
            spans.push(Span {
                start: index,
                end: after,
            });
            index = after;
            continue;
        }
        if bytes[index] == b'"' {
            match crate::scan::skip_string(source, index) {
                Ok(after) => {
                    spans.push(Span {
                        start: index,
                        end: after,
                    });
                    index = after;
                    continue;
                }
                Err(_) => break,
            }
        }
        index += 1;
    }
    spans
}

/// `let mut NAME = INIT;` statements of this body.
fn locals_of(source: &str, body: Span) -> Vec<Local> {
    let bytes = source.as_bytes();
    let mut locals = Vec::new();
    let mut index = body.start;
    let commented = comments(source);

    while let Some(at) = find_word(source, "let", index) {
        if at >= body.end {
            break;
        }
        index = at + 3;
        // A page documents itself, and a page about state documents
        // `let mut n = 0;`. Read as a declaration, it was blanked out of
        // the comment it was written in — leaving `/// One ``,` — and
        // published as a value of the application under the name `n`.
        if commented
            .iter()
            .any(|span| span.start <= at && at < span.end)
        {
            continue;
        }
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
                        handlers.push(Span { start: cursor, end });
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

/// Every occurrence of `name` as a word inside these spans, counting
/// only the ones that are code.
///
/// A name is a name in Rust and a word everywhere else. `count` can be
/// the state, the CSS class in `className="count"`, and a word in the
/// sentence `{count} count` — all three on one line. Rewriting the last
/// two ships a corrupted interface, so string literals, comments and
/// JSX text are not searched at all.
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

/// The parts of a component's body that are Rust, as opposed to the
/// text and tag syntax of its JSX.
///
/// Outside a JSX region everything is code, minus strings and comments.
/// Inside one, only what is between braces is: an expression child or an
/// attribute's value. `<span className="count">{count} count</span>`
/// contributes exactly one span, the one holding `count`.
fn code_spans(source: &str, body: Span, regions: &[Span]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut cursor = body.start;

    for region in regions {
        push_code(
            source,
            Span {
                start: cursor,
                end: region.start,
            },
            &mut spans,
        );
        for braced in braced_spans(source, *region) {
            push_code(source, braced, &mut spans);
        }
        cursor = region.end;
    }
    push_code(
        source,
        Span {
            start: cursor,
            end: body.end,
        },
        &mut spans,
    );
    spans
}

/// The `{…}` of a region, at its top level — nested braces belong to the
/// expression that opened them.
fn braced_spans(source: &str, region: Span) -> Vec<Span> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut index = region.start;
    while index < region.end {
        if bytes[index] == b'"' {
            // An attribute's quotes are HTML's: no escapes, ends at the
            // next quote.
            index = source[index + 1..region.end]
                .find('"')
                .map_or(region.end, |offset| index + offset + 2);
            continue;
        }
        if bytes[index] == b'{' {
            match skip_braces(source, index) {
                Ok(end) => {
                    spans.push(Span {
                        start: index + 1,
                        end: end - 1,
                    });
                    index = end;
                    continue;
                }
                Err(_) => break,
            }
        }
        index += 1;
    }
    spans
}

/// Split a run of Rust around its string literals and comments, so that
/// nothing inside either is ever searched.
fn push_code(source: &str, span: Span, into: &mut Vec<Span>) {
    if span.start >= span.end {
        return;
    }
    let bytes = source.as_bytes();
    let mut start = span.start;
    let mut index = span.start;

    while index < span.end {
        let skipped = if let Ok(Some(after)) = crate::scan::skip_raw_string(source, index) {
            Some(after)
        } else if bytes[index] == b'"' {
            crate::scan::skip_string(source, index).ok()
        } else if bytes[index] == b'\'' {
            Some(crate::scan::skip_char_or_lifetime(bytes, index))
        } else {
            crate::scan::skip_comment(bytes, index)
        };

        match skipped {
            Some(after) if after > index => {
                if index > start {
                    into.push(Span { start, end: index });
                }
                index = after.min(span.end);
                start = index;
            }
            _ => index += 1,
        }
    }
    if span.end > start {
        into.push(Span {
            start,
            end: span.end,
        });
    }
}

/// Every occurrence of `name` inside this handler that writes to it.
fn mutations_in(
    source: &str,
    name: &str,
    handler: Span,
    in_code: &[Span],
    tail_discarded: bool,
) -> Vec<Mutation> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();

    let inside: Vec<Span> = in_code
        .iter()
        .filter(|span| contains(handler, **span))
        .copied()
        .collect();
    for span in occurrences(source, name, &inside) {
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
            // update — so the test is what surrounds it. `p.x = 1` and
            // `v[0] = 1` land here too: they are writes through the
            // local, and `update` is exactly the right shape for them.
            // `p.x = 1`, `v[0] = 1`: a write through the local. There is
            // no ambiguity in an assignment, so it does not need the
            // statement-position test that a method call needs.
            Some(b'.') | Some(b'[') if assigns_through_path(bytes, span.end) => {
                Some(MutationKind::Method)
            }
            Some(b'.') if is_statement(bytes, span, handler, tail_discarded) => {
                Some(MutationKind::Method)
            }
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
///
/// `tail_discarded` says whether the closure's last expression counts as
/// one too. In a handler it does: a handler returns nothing, so
/// `move |_| items.push(1)` — no braces, no semicolon — throws its value
/// away exactly like a statement would. Read as an expression instead,
/// it became a call on a copy: it compiled, did nothing, and the list on
/// screen never reordered. In the argument of a `keep` or a `task` it
/// does not: there the last expression is the value being computed.
fn is_statement(bytes: &[u8], span: Span, handler: Span, tail_discarded: bool) -> bool {
    let mut before = span.start;
    while before > handler.start && bytes[before - 1].is_ascii_whitespace() {
        before -= 1;
    }
    let opens = before == handler.start
        || matches!(bytes.get(before - 1), Some(b'{') | Some(b';'))
        || opens_a_closure_body(bytes, handler, before);
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
            // The brace that closes the body it is the last expression
            // of. Whether that counts is the caller's business.
            b'}' if depth == 0 => return tail_discarded,
            b'{' if depth == 0 => return false,
            _ => {}
        }
        index += 1;
    }
    tail_discarded
}

/// Is `at` the first thing in the body of the closure the handler is?
///
/// `onClick={move |_| items.push(1)}`: what comes before `items` is the
/// closure's parameter list, not a `{` or a `;`, and that is as much the
/// start of a statement as either of them.
fn opens_a_closure_body(bytes: &[u8], handler: Span, at: usize) -> bool {
    if at == 0 || bytes.get(at - 1) != Some(&b'|') {
        return false;
    }
    // Back over the parameter list to the `|` that opened it.
    let mut index = at - 1;
    while index > handler.start && bytes[index - 1] != b'|' {
        index -= 1;
    }
    if index == handler.start {
        return false;
    }
    index -= 1;
    // …and over an optional `move`, to the brace the attribute opened.
    index = skip_spaces_back(bytes, index);
    if index >= 4 && &bytes[index - 4..index] == b"move" {
        index = skip_spaces_back(bytes, index - 4);
    }
    index == handler.start + 1
}

fn skip_spaces_back(bytes: &[u8], from: usize) -> usize {
    let mut index = from;
    while index > 0 && bytes[index - 1].is_ascii_whitespace() {
        index -= 1;
    }
    index
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

/// The second declaration of a name already declared in this component.
fn shadowed(locals: &[Local]) -> Option<&Local> {
    locals
        .iter()
        .enumerate()
        .find(|(at, local)| {
            locals[..*at]
                .iter()
                .any(|earlier| earlier.name == local.name)
        })
        .map(|(_, local)| local)
}

/// Does a path starting here end in an assignment?
///
/// `p.x = 1` and `v[0].y = 2` are writes through the local. A method
/// call in the middle — `v.get(0).x = 1` — is not something that can be
/// assigned to in Rust, so a `(` ends the walk and the answer is no.
fn assigns_through_path(bytes: &[u8], from: usize) -> bool {
    let mut index = from;
    while index < bytes.len() {
        match bytes[index] {
            b'.' => index += 1,
            b'[' => {
                let mut depth = 0_i32;
                while index < bytes.len() {
                    match bytes[index] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                index += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    index += 1;
                }
            }
            byte if is_word_byte(byte) => index += 1,
            b' ' | b'\t' => index += 1,
            // A plain `=`, and not `==`.
            b'=' => return bytes.get(index + 1) != Some(&b'='),
            _ => return false,
        }
    }
    false
}

/// A value declared at the top of a page: shared by every component of
/// that page, owned by none of them.
///
/// The same sentence as a component's own state, written one level out.
/// `let mut n = 0;` inside a component is that component's; the same
/// line outside one is the page's, and two components that read it see
/// one value without either having been handed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedDeclaration {
    pub name: String,
    /// `let mut points = 0;`, the whole statement.
    pub declaration: Span,
    /// `0` — what it starts at, the first time anything asks.
    pub initializer: Span,
}

/// Every `let mut` written outside a component.
///
/// A page is the unit: two files that each declare `points` declare two
/// values, the way two modules declaring the same `static` do. Sharing
/// across files is not this; it is what a parameter or a route is for.
pub fn shared_of(source: &str) -> Vec<SharedDeclaration> {
    let mut bodies: Vec<Span> = components_of(source)
        .into_iter()
        .map(|(_, body)| body)
        .collect();

    // And everything inside a tag. A page that shows code to its reader
    // writes `let mut` in its own interface: that is text, like any
    // other text, and it declares nothing.
    if let Ok(regions) = crate::scan::find_jsx(source) {
        bodies.extend(regions);
    }

    // `locals_of` over the whole file, minus everything inside a
    // component — which is exactly "written at the top level".
    locals_of(
        source,
        Span {
            start: 0,
            end: source.len(),
        },
    )
    .into_iter()
    .filter(|local| !bodies.iter().any(|body| contains(*body, local.declaration)))
    .map(|local| SharedDeclaration {
        name: local.name,
        declaration: local.declaration,
        initializer: local.initializer,
    })
    .collect()
}

/// What a region of code does to `name`: every write, and every read.
///
/// For the arguments of `keep`, `effect` and `task`, which are code
/// that runs later — on the next render, or on another thread — and so
/// cannot hold a handle the way a handler can.
pub fn uses_in(source: &str, name: &str, region: Span) -> (Vec<Span>, Vec<Mutation>) {
    let mutations = mutations_in(source, name, region, &[region], false);
    let reads = occurrences(source, name, &[region])
        .into_iter()
        .filter(|span| !mutations.iter().any(|mutation| mutation.span == *span))
        .collect();
    (reads, mutations)
}
