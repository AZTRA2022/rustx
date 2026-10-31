//! The Rust a node tree becomes.
//!
//! The output is built as a token stream, never by pasting strings
//! together. Text written by the developer — a product name with a quote
//! in it, a price with a `<` — goes through `Literal::string`, which is
//! what makes it impossible for interface text to turn into source code.
//! Rust written by the developer is re-tokenised and passed through
//! untouched: the compiler never rewrites an expression, so what
//! `rust-analyzer` sees is what the developer typed.

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::ast::{Attribute, AttributeSource, Node, TagNode};
use crate::attributes::{dom_name, is_event, is_known, nearest};
use crate::error::CompileError;

/// What the file around a region knows: its components, and where a
/// byte offset falls in it.
///
/// A component call needs both. The parameter names decide which
/// argument a prop becomes, and the place the tag was written is the
/// call's identity across renders — the same thing a `let mut` gets
/// from the line it sits on.
pub struct Context<'a> {
    /// Every component declared in this file, parameters in order.
    pub components: &'a [(String, Vec<String>)],
    /// The whole `.rsx`, for turning an offset into a line and column.
    pub source: &'a str,
}

impl Context<'_> {
    fn parameters_of(&self, name: &str) -> Option<&[String]> {
        self.components
            .iter()
            .find(|(component, _)| component == name)
            .map(|(_, parameters)| parameters.as_slice())
    }

    fn line_and_column(&self, offset: usize) -> (u32, u32) {
        let before = &self.source[..offset.min(self.source.len())];
        let line = before.matches('\n').count() + 1;
        let start = before.rfind('\n').map_or(0, |index| index + 1);
        (
            line as u32,
            self.source[start..offset].chars().count() as u32 + 1,
        )
    }
}

/// The Rust expression that builds `node`.
pub fn emit(node: &Node, context: &Context<'_>) -> Result<String, CompileError> {
    Ok(expression(node, context)?.to_string())
}

fn expression(node: &Node, context: &Context<'_>) -> Result<TokenStream, CompileError> {
    match node {
        Node::Text(text) => {
            let text = Literal::string(text);
            Ok(quote! { rustx_dom::Element::Text(#text.to_string()) })
        }
        Node::Expression { code, offset } => {
            // `{Device("Webcam".to_string())}` is how Rust calls a
            // function, and a reader of Rust will write it that way.
            // Called plainly it would run in the middle of its parent's
            // render, so its cells would be the parent's and two calls
            // would share one state. It is an instance, like the tag.
            if whole_call_to_a_component(code, context) {
                let (line, column) = context.line_and_column(*offset);
                // `{Navbar()}` hands over the function itself: `||
                // Navbar()` is a closure clippy asks the developer to
                // simplify, in a file they never wrote.
                let trimmed = code.trim();
                let body = match trimmed.strip_suffix("()") {
                    Some(name) if !name.trim_end().is_empty() => {
                        let name = rust(name.trim_end(), *offset)?;
                        quote! { #name }
                    }
                    _ => {
                        let call = rust(code, *offset)?;
                        quote! { || #call }
                    }
                };
                return Ok(quote! {
                    rustx_dom::state::component_at(
                        rustx_dom::state::Slot { file: file!(), line: #line, column: #column },
                        #body,
                    )
                });
            }

            // Verbatim. `{clicks}` is `clicks`, and `into_element` is what
            // spares the developer a conversion they should never have to
            // write.
            let inner = rust(code, *offset)?;
            Ok(quote! { rustx_dom::into_element(#inner) })
        }
        Node::Tag(tag) if tag.is_fragment() => Ok(fragment(&tag.children, context)?),
        // A capitalised tag the page declares is the page's own
        // component. One it does not declare is a component off npm,
        // found by its name among the libraries the application
        // installed — no prefix, the way it is written in JSX. `js::`
        // and `module::Name` stay as the explicit forms, for a name two
        // libraries export or a name the page also declares.
        Node::Tag(tag) if tag.name.starts_with("js::") => foreign_expression(tag, context),
        Node::Tag(tag)
            if crate::tags::looks_like_a_component(&tag.name)
                && context.parameters_of(&tag.name).is_some() =>
        {
            component_expression(tag, context)
        }
        Node::Tag(tag) if crate::tags::looks_like_a_component(&tag.name) => {
            foreign_expression(tag, context)
        }
        Node::Tag(tag) => tag_expression(tag, context),
    }
}

/// The children of a region, as one expression.
fn fragment(children: &[Node], context: &Context<'_>) -> Result<TokenStream, CompileError> {
    let children = children
        .iter()
        .map(|child| expression(child, context))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! { rustx_dom::Element::Fragment(vec![#(#children),*]) })
}

/// `<Row count={2} label={…} />` becomes `Row(…)`, rendered as a
/// component of its own.
///
/// The props are matched to parameters **by name**: an argument list has
/// an order, and the order is not something a reader of the `.rsx` can
/// see. Everything fails closed — a component this file does not
/// declare, a prop that names no parameter, a parameter nothing fills,
/// children handed to a component that takes none.
fn component_expression(tag: &TagNode, context: &Context<'_>) -> Result<TokenStream, CompileError> {
    let Some(parameters) = context.parameters_of(&tag.name) else {
        return Err(CompileError::flagged(
            tag.offset,
            format!(
                "`{}` is not a component declared in this file. A tag can only call a \
                 component written beside it — the order of its arguments comes from its \
                 declaration, so rustx has to be able to see it.",
                tag.name
            ),
        ));
    };

    for attribute in &tag.attributes {
        if crate::attributes::is_event(&attribute.name) {
            return Err(CompileError::flagged(
                attribute.offset,
                format!(
                    "`{}` is an event, and a component is not an element — give it a \
                     parameter and call it from inside.",
                    attribute.name
                ),
            ));
        }
        if !parameters.iter().any(|name| name == &attribute.name) {
            return Err(CompileError::flagged(
                attribute.offset,
                format!(
                    "`{}` takes no parameter called `{}`. It takes: {}.",
                    tag.name,
                    attribute.name,
                    if parameters.is_empty() {
                        "nothing".to_string()
                    } else {
                        parameters.join(", ")
                    }
                ),
            ));
        }
    }

    if !tag.children.is_empty() && !parameters.iter().any(|name| name == "children") {
        return Err(CompileError::flagged(
            tag.offset,
            format!(
                "`{}` takes no `children` parameter, so nothing can be written between \
                 its tags.",
                tag.name
            ),
        ));
    }

    let mut arguments = Vec::new();
    for parameter in parameters {
        if parameter == "children" {
            arguments.push(match tag.children.len() {
                0 => quote! { rustx_dom::Element::Empty },
                1 => expression(&tag.children[0], context)?,
                _ => fragment(&tag.children, context)?,
            });
            continue;
        }
        let Some(attribute) = tag
            .attributes
            .iter()
            .find(|attribute| &attribute.name == parameter)
        else {
            return Err(CompileError::flagged(
                tag.offset,
                format!(
                    "`{}` takes `{parameter}`, and this tag does not give it one.",
                    tag.name
                ),
            ));
        };
        arguments.push(argument(attribute)?);
    }

    let name = crate::emit::rust(&tag.name, tag.offset)?;
    let (line, column) = context.line_and_column(tag.offset);
    // A component with nothing to pass is handed over as the function
    // itself. `|| Nav()` would be a closure clippy asks the developer to
    // simplify, in a file they never wrote and cannot edit.
    let body = if arguments.is_empty() {
        quote! { #name }
    } else {
        quote! { || #name(#(#arguments),*) }
    };
    Ok(quote! {
        rustx_dom::state::component_at(
            rustx_dom::state::Slot { file: file!(), line: #line, column: #column },
            #body,
        )
    })
}

/// What one prop hands to the parameter it names.
fn argument(attribute: &Attribute) -> Result<TokenStream, CompileError> {
    match &attribute.value {
        AttributeSource::Expression(code) => rust(code, attribute.offset),
        AttributeSource::Literal(text) => {
            let text = Literal::string(text);
            Ok(quote! { #text.to_string() })
        }
        AttributeSource::Flag => Ok(quote! { true }),
    }
}

fn tag_expression(tag: &TagNode, context: &Context<'_>) -> Result<TokenStream, CompileError> {
    if !crate::tags::is_known(&tag.name) {
        return Err(unknown_tag(tag));
    }
    let name = Literal::string(&tag.name);
    let mut steps: Vec<TokenStream> = Vec::new();
    let mut key = None;

    for attribute in &tag.attributes {
        // `key` is not a DOM attribute and never reaches the page. It
        // identifies the row for the reconciler and the state inside it
        // — the same meaning React gives it, and §7.4 bis asks for both
        // from one word.
        if attribute.name == "key" {
            let AttributeSource::Expression(code) = &attribute.value else {
                let text = match &attribute.value {
                    AttributeSource::Literal(text) => text.clone(),
                    _ => String::new(),
                };
                key = Some(quote! { #text });
                continue;
            };
            key = Some(rust(code, attribute.offset)?);
            continue;
        }
        if !is_known(&attribute.name) {
            return Err(unknown_attribute(attribute));
        }
        steps.push(if is_event(&attribute.name) {
            event_step(attribute)?
        } else {
            attribute_step(attribute)?
        });
    }

    let children: Vec<TokenStream> = tag
        .children
        .iter()
        .map(|child| expression(child, context))
        .collect::<Result<_, _>>()?;

    // With a key, the whole row — its children and every component
    // inside them — is built under that key, so the state found there
    // is the state of this row and follows it when the list reorders.
    // Without one, nothing is wrapped and nothing costs anything.
    let Some(key) = key else {
        let steps = steps.iter();
        return Ok(quote! {
            rustx_dom::Element::tag(#name) #(#steps)* #(.child(#children))* .build()
        });
    };
    Ok(quote! {
        {
            let __key = #key;
            rustx_dom::state::with_key(__key.clone(), || {
                rustx_dom::Element::tag(#name)
                    .key(__key.clone())
                    #(#steps)*
                    #(.child(#children))*
                    .build()
            })
        }
    })
}

/// `onClick={…}` becomes a handler, not an attribute: a closure is code,
/// and code does not belong in a DOM attribute.
fn event_step(attribute: &Attribute) -> Result<TokenStream, CompileError> {
    let AttributeSource::Expression(code) = &attribute.value else {
        return Err(CompileError::flagged(
            attribute.offset,
            format!(
                "`{name}` is an event, so it takes a closure in braces: {name}={{|event| …}}",
                name = attribute.name
            ),
        ));
    };
    let written = Literal::string(&attribute.name);
    let body = rust(code, attribute.offset)?;
    Ok(quote! { .handler(#written, rustx_dom::Handler::new(#body)) })
}

fn attribute_step(attribute: &Attribute) -> Result<TokenStream, CompileError> {
    let in_dom = Literal::string(&dom_name(&attribute.name));

    // `style={{ … }}` is the one prop of React's dialect that cannot be
    // passed through: the DOM wants a declaration string and TSX writes
    // a JavaScript object. An expression that is not an object — a
    // string the developer built — is passed through as any other.
    if attribute.name == "style"
        && let AttributeSource::Expression(code) = &attribute.value
        && code.trim().starts_with('{')
    {
        let declaration = crate::attributes::style_string(code)
            .map_err(|complaint| CompileError::flagged(attribute.offset, complaint))?;
        let declaration = Literal::string(&declaration);
        return Ok(quote! {
            .attribute(#in_dom, rustx_dom::AttributeValue::Text(#declaration.to_string()))
        });
    }

    let value = match &attribute.value {
        AttributeSource::Literal(text) => {
            let text = Literal::string(text);
            quote! { rustx_dom::AttributeValue::Text(#text.to_string()) }
        }
        AttributeSource::Flag => quote! { rustx_dom::AttributeValue::Flag(true) },
        AttributeSource::Expression(code) => {
            let inner = rust(code, attribute.offset)?;
            quote! { rustx_dom::AttributeValue::Text((#inner).to_string()) }
        }
    };
    Ok(quote! { .attribute(#in_dom, #value) })
}

/// Re-tokenise the developer's own Rust, unchanged.
fn rust(code: &str, offset: usize) -> Result<TokenStream, CompileError> {
    code.parse::<TokenStream>().map_err(|error| {
        CompileError::flagged(offset, format!("this Rust does not parse: {error}"))
    })
}

fn unknown_tag(tag: &TagNode) -> CompileError {
    let message = if crate::tags::looks_like_a_component(&tag.name) {
        format!(
            "`{}` looks like a component, and components are not supported yet — \
             a tag has to be an HTML or SVG element for now",
            tag.name
        )
    } else {
        let mut message = format!("unknown tag `{}`", tag.name);
        if let Some(suggestion) = crate::tags::nearest(&tag.name) {
            message.push_str(&format!(" — did you mean `{suggestion}`?"));
        }
        message
    };
    CompileError::flagged(tag.offset, message)
}

fn unknown_attribute(attribute: &Attribute) -> CompileError {
    let mut message = format!("unknown attribute `{}`", attribute.name);
    if let Some(suggestion) = nearest(&attribute.name) {
        message.push_str(&format!(" — did you mean `{suggestion}`?"));
    }
    CompileError::flagged(attribute.offset, message)
}

/// Is this expression nothing but a call to a component of this file?
///
/// The whole shape has to match: `Name(…)` where `Name` is declared
/// here and where the closing parenthesis ends the expression. A call
/// buried in something larger — `vec![Row()]`, an `if` with a branch
/// each — is left alone, because what surrounds it may call it zero
/// times or ten and identity by the place it is written would stop
/// meaning anything.
fn whole_call_to_a_component(code: &str, context: &Context<'_>) -> bool {
    let code = code.trim();
    let Some(open) = code.find('(') else {
        return false;
    };
    let name = &code[..open];
    if !crate::tags::looks_like_a_component(name) || context.parameters_of(name).is_none() {
        return false;
    }

    // The call's own parenthesis has to be the last thing in the
    // expression: `Row().into()` is a call and then something else.
    let mut depth = 0_i32;
    for (at, character) in code.char_indices().skip(open) {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return code[at + 1..].trim().is_empty();
                }
            }
            _ => {}
        }
    }
    false
}

/// `<js::Chart series={…} />` becomes the container React fills.
///
/// rustx says where the component goes and what its props are, and
/// stops there. So this is where the boundary's rules are enforced: no
/// children, because React owns what is inside and a child written here
/// could never be reconciled by either side; no events, because a
/// handler is a Rust closure and a closure does not cross; and no prop
/// that is not a value, because what crosses is copied in full and
/// nothing is borrowed.
fn foreign_expression(tag: &TagNode, context: &Context<'_>) -> Result<TokenStream, CompileError> {
    let node = foreign_node(tag, context)?;
    let (line, column) = context.line_and_column(tag.offset);
    let id = Literal::string(&format!("rustx-js-{line}-{column}"));
    Ok(quote! {
        rustx_dom::Element::Foreign {
            node: #node,
            id: #id.to_string(),
        }
    })
}

/// One component React owns, with its props and whatever is inside it.
fn foreign_node(tag: &TagNode, context: &Context<'_>) -> Result<TokenStream, CompileError> {
    let path = tag.name.strip_prefix("js::").unwrap_or(&tag.name);
    // The last segment is the export, whichever way the page spelled
    // the path: `recharts::LineChart`, `Dialog.Trigger`, or `LineChart`.
    let export = path
        .rsplit("::")
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .next()
        .unwrap_or(path);
    if !crate::tags::looks_like_a_component(export) {
        let capitalised = format!(
            "{}{}",
            export
                .chars()
                .next()
                .map(|first| first.to_uppercase().to_string())
                .unwrap_or_default(),
            export.get(1..).unwrap_or("")
        );
        return Err(CompileError::flagged(
            tag.offset,
            format!(
                "`{}` names no React component. A React component is capitalised — \
                 write `{capitalised}`, the name the module exports.",
                tag.name
            ),
        ));
    }

    let mut props = Vec::new();
    for attribute in &tag.attributes {
        if is_event(&attribute.name) {
            return Err(CompileError::flagged(
                attribute.offset,
                format!(
                    "`{}` is an event, and a Rust closure does not cross into React. \
                     Give the component a prop it calls itself.",
                    attribute.name
                ),
            ));
        }
        let name = Literal::string(&attribute.name);
        let value = crossing_value(attribute)?;
        props.push(quote! { (#name.to_string(), #value) });
    }

    let mut children = Vec::new();
    for child in &tag.children {
        children.push(foreign_child(tag, child, context)?);
    }

    let component = Literal::string(path);
    Ok(quote! {
        rustx_dom::ForeignNode {
            component: #component.to_string(),
            props: vec![#(#props),*],
            children: vec![#(#children),*],
        }
    })
}

/// What may be written inside a component React owns: another of its
/// own, or text.
fn foreign_child(
    parent: &TagNode,
    child: &Node,
    context: &Context<'_>,
) -> Result<TokenStream, CompileError> {
    match child {
        Node::Tag(tag) if tag.name.starts_with("js::") => {
            let node = foreign_node(tag, context)?;
            Ok(quote! { rustx_dom::ForeignChild::Node(#node) })
        }
        Node::Text(text) => {
            let text = Literal::string(text);
            Ok(quote! { rustx_dom::ForeignChild::Text(#text.to_string()) })
        }
        Node::Tag(tag) => Err(CompileError::flagged(
            tag.offset,
            format!(
                "`<{}>` is a DOM tag, and it is written inside `{}`, which React owns. \
                 A node rustx thinks it owns inside a tree React reconciles is the one \
                 thing this boundary prevents: write a component of the library there, \
                 or put the DOM outside the boundary.",
                tag.name, parent.name
            ),
        )),
        Node::Expression { offset, .. } => Err(CompileError::flagged(
            *offset,
            format!(
                "`{}` is React's, so what is inside it is a React tree, not a Rust \
                 expression. Pass the value as a prop.",
                parent.name
            ),
        )),
    }
}

/// What one prop of a React component crosses as.
///
/// The compiler does not know the types of the developer's Rust, so the
/// refusal is on what it can see: a literal, a number, a boolean, or an
/// expression that ends in `.to_string()` or `.into()`. Anything else is
/// refused by name rather than compiled into a `rustc` error about a
/// `Value` the developer never wrote.
fn crossing_value(attribute: &Attribute) -> Result<TokenStream, CompileError> {
    match &attribute.value {
        AttributeSource::Flag => Ok(quote! { rustx_dom::Value::Bool(true) }),
        AttributeSource::Literal(text) => {
            let text = Literal::string(text);
            Ok(quote! { rustx_dom::Value::String(#text.to_string()) })
        }
        // Whatever the page wrote, crossing through `ToValue`.
        //
        // The compiler used to guess from what was written — a literal,
        // something ending in `.to_string()`, something ending in
        // `as f64` — and refuse the rest. That refused `data={points}`,
        // which is the one prop a chart actually takes. `ToValue` is
        // what says a type can cross, so a type that cannot is refused
        // by `rustc`, at the line of the `.rsx` that wrote it.
        AttributeSource::Expression(code) => {
            let inner = rust(code, attribute.offset)?;
            Ok(quote! { rustx_dom::value::ToValue::to_value(&(#inner)) })
        }
    }
}
