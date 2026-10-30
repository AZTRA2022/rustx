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
        (line as u32, self.source[start..offset].chars().count() as u32 + 1)
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
        Node::Expression(code) => {
            // Verbatim. `{clicks}` is `clicks`, and `into_element` is what
            // spares the developer a conversion they should never have to
            // write.
            let inner = rust(code, 0)?;
            Ok(quote! { rustx_dom::into_element(#inner) })
        }
        Node::Tag(tag) if tag.is_fragment() => Ok(fragment(&tag.children, context)?),
        Node::Tag(tag) if crate::tags::looks_like_a_component(&tag.name) => {
            component_expression(tag, context)
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
    Ok(quote! {
        rustx_dom::state::component_at(
            rustx_dom::state::Slot { file: file!(), line: #line, column: #column },
            || #name(#(#arguments),*),
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

    for attribute in &tag.attributes {
        if !is_known(&attribute.name) {
            return Err(unknown_attribute(attribute));
        }
        steps.push(if is_event(&attribute.name) {
            event_step(attribute)?
        } else {
            attribute_step(attribute)?
        });
    }

    for child in &tag.children {
        let child = expression(child, context)?;
        steps.push(quote! { .child(#child) });
    }

    Ok(quote! { rustx_dom::Element::tag(#name) #(#steps)* .build() })
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
