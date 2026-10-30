//! `#[rustx::main]`: everything an application should not have to write.
//!
//! A `main.rs` is a window's title and its size. Compiling the pages of
//! `app/` into the binary, telling the router about them, and leaving a
//! sub-process before it touches anything are the framework's work, and
//! a developer who has to write them is reading the framework's own
//! plumbing in their own file.
//!
//! The attribute is deliberately thin: it adds two things above `main`
//! and one line inside it. Everything it adds is ordinary code that
//! could have been written by hand — which is what makes an application
//! that outgrows it able to stop using it.

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, TokenStream as TokenStream2, TokenTree};
use quote::quote;

/// Write the ceremony of a rustx application around `main`.
///
/// `#[rustx::main]` compiles the pages under `app/` into the binary and
/// hands the generated route table to the router. `#[rustx::main(pages)]`
/// compiles them without routing, for an application that mounts its
/// components by hand.
#[proc_macro_attribute]
pub fn main(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let routed = match attribute.to_string().trim() {
        "" => true,
        "pages" => false,
        other => {
            return error(format!(
                "`#[rustx::main]` takes nothing, or `pages` for an application that \
                 mounts its own components — not `{other}`"
            ));
        }
    };

    let item = TokenStream2::from(item);
    let Some((signature, body)) = split_body(item) else {
        return error(
            "`#[rustx::main]` goes on a function. It is `fn main` that the framework \
             wraps."
                .to_string(),
        );
    };

    // Where the application's own files are, said in the application's
    // own crate: `env!` here reads the variables of whoever is being
    // compiled, which is exactly the crate that owns `assets/`.
    let prelude = if routed {
        quote! {
            rustx::install_manifest_dir(env!("CARGO_MANIFEST_DIR"));
            rustx::install_routes(route_table());
        }
    } else {
        quote! {
            rustx::install_manifest_dir(env!("CARGO_MANIFEST_DIR"));
        }
    };

    let pages = quote! {
        // The pages of `app/`, as Rust. Written by the build script into
        // `OUT_DIR`, compiled in here, and named by nobody.
        include!(concat!(env!("OUT_DIR"), "/pages.rs"));
    };

    let body = body.stream();
    quote! {
        #pages

        #signature {
            #prelude
            #body
        }
    }
    .into()
}

/// The function's tokens, split into everything before its block and
/// the block itself.
fn split_body(item: TokenStream2) -> Option<(TokenStream2, Group)> {
    let tokens: Vec<TokenTree> = item.into_iter().collect();
    let (last, rest) = tokens.split_last()?;
    match last {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
            Some((rest.iter().cloned().collect(), group.clone()))
        }
        _ => None,
    }
}

fn error(message: String) -> TokenStream {
    quote! { compile_error!(#message); }.into()
}
