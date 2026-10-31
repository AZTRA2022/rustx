//! `#[rustx::main]`: everything an application should not have to write.
//!
//! A `main.rs` is a window's title and its size. Compiling the pages of
//! `app/` into the binary, telling the router about them, letting React
//! know what it may mount, and leaving a sub-process before it touches
//! anything are the framework's work. A developer who writes them is
//! reading the framework's own plumbing in their own file.
//!
//! There is no build script. The pages are compiled **here**, while the
//! attribute expands, because that is the one moment that happens every
//! time the crate is compiled and never otherwise. Each `.rsx` is then
//! named by an `include_bytes!`, which is what makes cargo and `rustc`
//! watch it: change a page, and the application is rebuilt.
//!
//! The generated Rust is still written to files, and included by path.
//! It would have been shorter to paste it in as tokens, but then a
//! `rustc` error about a developer's own Rust would point at this
//! attribute instead of at the line they wrote. The file keeps the line
//! numbers of the `.rsx` (§7.3), so the error lands where they are
//! looking.

use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, TokenStream as TokenStream2, TokenTree};
use quote::quote;

/// Write the ceremony of a rustx application around `main`.
///
/// `#[rustx::main]` compiles the pages under `app/` and hands the
/// generated route table to the router. `#[rustx::main(pages)]` compiles
/// them without routing, for an application that mounts its own
/// components.
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

    let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") else {
        return error(
            "`#[rustx::main]` could not tell which crate it is in: cargo sets \
             CARGO_MANIFEST_DIR, so this is a build run by something else."
                .to_string(),
        );
    };
    let manifest = PathBuf::from(manifest);
    let app = manifest.join("app");

    let (pages, prelude) = if app.is_dir() {
        let out = generated_directory(&manifest);
        if let Err(complaint) = compile(&app, &out, routed) {
            // The function is still emitted, untouched. A mistake in a
            // page is one error; without this, the developer also gets
            // "`main` function not found", which is about nothing they
            // did and sends them looking in the wrong file.
            let body = body.stream();
            return quote! {
                compile_error!(#complaint);
                #signature { #body }
            }
            .into();
        }

        // Every source named, so that changing one rebuilds the
        // application. `include_bytes!` is the only way to say "this
        // file is part of me" that both cargo and `rustc` follow.
        let watched: Vec<TokenStream2> = sources(&app)
            .iter()
            .map(|source| {
                let path = slashed(source);
                quote! {
                    const _: &[u8] = include_bytes!(#path);
                }
            })
            .collect();

        let generated = slashed(&out.join(rustx_compiler::build::PAGES));

        let prelude = if routed {
            quote! {
                rustx::install_manifest_dir(env!("CARGO_MANIFEST_DIR"));
                rustx::install_routes(__rustx_pages::route_table());
                rustx::install_components(__rustx_pages::component_table());
                rustx::install_shared_values(__rustx_pages::shared_values());
            }
        } else {
            quote! {
                rustx::install_manifest_dir(env!("CARGO_MANIFEST_DIR"));
            }
        };

        (
            // The pages live in a module of their own, which is what
            // lets them have the vocabulary of a page — `Element` in
            // scope — without an application importing it, and keeps
            // the lints the generated shape always trips off the
            // developer's own file.
            quote! {
                #[doc(hidden)]
                mod __rustx_pages {
                    #![allow(non_snake_case, clippy::needless_return, clippy::useless_vec, clippy::type_complexity, clippy::redundant_closure)]
                    pub(crate) use super::*;
                    pub(crate) use rustx::Element;
                    pub(crate) use rustx::state::with_key;

                    include!(#generated);
                    #(#watched)*
                }
            },
            prelude,
        )
    } else {
        // An application with no `app/` directory is a window and
        // nothing else — which is a legitimate place to start.
        (
            TokenStream2::new(),
            quote! {
                rustx::install_manifest_dir(env!("CARGO_MANIFEST_DIR"));
            },
        )
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

/// Compile the pages, turning a mistake in a `.rsx` into a sentence in
/// the coordinates of the `.rsx`.
fn compile(app: &Path, out: &Path, routed: bool) -> Result<(), String> {
    if let Err(error) = std::fs::create_dir_all(out) {
        return Err(format!(
            "rustx could not make a place for the compiled pages at {}: {error}",
            out.display()
        ));
    }
    let outcome = if routed {
        rustx_compiler::build::compile_app(app, out)
    } else {
        rustx_compiler::build::compile_pages(app, out)
    };
    outcome
        .map(|_| ())
        .map_err(|complaint| complaint.to_string())
}

/// Where the compiled pages of this crate live.
///
/// Not in the source tree, which is the developer's; not in `OUT_DIR`,
/// which only a build script has. A directory of this crate's own, named
/// after where it is, so that two applications compiling at once never
/// write the same file.
fn generated_directory(manifest: &Path) -> PathBuf {
    let name = manifest
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".to_string());
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in manifest.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    std::env::temp_dir()
        .join("rustx-pages")
        .join(format!("{name}-{hash:016x}"))
}

/// Every `.rsx` under `app/`, in a stable order.
fn sources(app: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(app, &mut found);
    found.sort();
    found
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rsx") {
            into.push(path);
        }
    }
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

/// A path an `include!` will take, on a system whose separator it will
/// not.
fn slashed(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}
