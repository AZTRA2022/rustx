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

/// The Rust expression that builds `node`.
pub fn emit(node: &Node) -> Result<String, CompileError> {
    Ok(expression(node)?.to_string())
}

fn expression(node: &Node) -> Result<TokenStream, CompileError> {
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
        Node::Tag(tag) if tag.is_fragment() => {
            let children = tag
                .children
                .iter()
                .map(expression)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(quote! { rustx_dom::Element::Fragment(vec![#(#children),*]) })
        }
        Node::Tag(tag) => tag_expression(tag),
    }
}

fn tag_expression(tag: &TagNode) -> Result<TokenStream, CompileError> {
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
        let child = expression(child)?;
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
