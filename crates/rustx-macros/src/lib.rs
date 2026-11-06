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

        // What the application wrote beside its pages. A page is in the
        // same crate as the rest of the application, so the public
        // functions and types of its modules are a page's vocabulary
        // too — without the application writing a `use` line for each
        // one in a file whose only job is a title and a size.
        let opened: Vec<TokenStream2> = modules_of(&manifest)
            .iter()
            .map(|module| {
                let module = proc_macro2::Ident::new(module, proc_macro2::Span::call_site());
                quote! {
                    #[allow(unused_imports)]
                    pub(crate) use super::#module::*;
                }
            })
            .collect();

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
                    #(#opened)*
                    pub(crate) use rustx::Element;
                    pub(crate) use rustx::state::with_key;
                    // Ce que le code engendré nomme. Importé ici, par la
                    // façade, pour qu'une application n'ait pas à déclarer
                    // `rustx-dom` ni `rustx-engine` dans son `Cargo.toml`.
                    #[allow(unused_imports)]
                    pub(crate) use rustx::{rustx_dom, rustx_engine};

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

    // And the application's JavaScript, if it declared any libraries.
    // Generated and bundled here, for the same reason the pages are:
    // this is the one moment that happens every time the crate is
    // compiled and never otherwise.
    let javascript = match rustx_compiler::bundle::build(&manifest) {
        Err(complaint) => {
            let body = body.stream();
            return quote! {
                compile_error!(#complaint);
                #signature { #body }
            }
            .into();
        }
        Ok(built) => {
            // The bundle is a file beside the document now, so what the
            // binary carries is its name and not its megabyte: a browser
            // loads a script that size from a URL, and a string that size
            // handed to `execute_java_script` sometimes does not run at
            // all.
            let bundle = built.bundle.map(|_| {
                let name = rustx_compiler::bundle::SERVED;
                quote! { rustx::install_bundle(#name); }
            });
            let stylesheet = built.stylesheet.map(|stylesheet| {
                let path = slashed(&stylesheet);
                quote! { rustx::install_stylesheet(include_str!(#path)); }
            });
            quote! { #bundle #stylesheet }
        }
    };

    let body = body.stream();
    quote! {
        #pages

        #signature {
            #prelude
            #javascript
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
///
/// The answer is the compiler.s, not this macro.s: the tool that
/// writes those files and the language server that reads them need the
/// very same one.
fn generated_directory(manifest: &Path) -> PathBuf {
    rustx_compiler::build::generated_directory(manifest)
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

/// Write a type's `ToValue` from its fields.
///
/// ```ignore
/// #[derive(rustx::ToValue)]
/// struct Point { at: f64, value: f64 }
/// ```
///
/// A chart's data is rows, and rows are a type the application already
/// has. The conversion is what it should not have to write. A field
/// whose JavaScript name is not a Rust identifier says so:
/// `#[rustx(rename = "data-key")]`.
#[proc_macro_derive(ToValue, attributes(rustx))]
pub fn to_value(item: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = TokenStream2::from(item).into_iter().collect();
    let Some(name) = type_name(&tokens) else {
        return error(
            "#[derive(ToValue)] goes on a struct with named fields: it writes the record a \
             prop crosses the boundary as."
                .to_string(),
        );
    };
    let Some(body) = tokens.iter().rev().find_map(|token| match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => Some(group.clone()),
        _ => None,
    }) else {
        return error(format!(
            "`{name}` has no named fields. #[derive(ToValue)] writes a record of a struct's \
             fields, and a tuple struct has no names to write — give its fields names, or \
             implement `ToValue` by hand."
        ));
    };

    let fields = fields_of(body.stream());
    if fields.is_empty() {
        return error(format!(
            "`{name}` declares no field. A prop with no fields is an empty object; if that is \
             what you meant, implement `ToValue` by hand and say so."
        ));
    }

    let name = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
    let entries = fields.into_iter().map(|(field, published)| {
        let field = proc_macro2::Ident::new(&field, proc_macro2::Span::call_site());
        quote! {
            (
                #published.to_string(),
                ::rustx::ToValue::to_value(&self.#field),
            )
        }
    });

    quote! {
        impl ::rustx::ToValue for #name {
            fn to_value(&self) -> ::rustx::Value {
                ::rustx::Value::Record(::std::vec![#(#entries),*])
            }
        }
    }
    .into()
}

/// The name of the struct these tokens declare.
fn type_name(tokens: &[TokenTree]) -> Option<String> {
    let mut seen_struct = false;
    for token in tokens {
        match token {
            TokenTree::Ident(ident) if ident == "struct" => seen_struct = true,
            TokenTree::Ident(ident) if seen_struct => return Some(ident.to_string()),
            _ => {}
        }
    }
    None
}

/// Each field, and the name it crosses under.
///
/// Read by hand rather than with `syn`: this crate parses tokens
/// already, and a parser is cheaper than a dependency the whole
/// framework would then carry.
fn fields_of(body: TokenStream2) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut renamed: Option<String> = None;
    let mut expecting_name = true;
    let mut depth = 0_i32;

    let tokens: Vec<TokenTree> = body.into_iter().collect();
    let mut at = 0;
    while at < tokens.len() {
        match &tokens[at] {
            // `#[rustx(rename = "data-key")]`
            TokenTree::Punct(punct) if punct.as_char() == '#' => {
                if let Some(TokenTree::Group(group)) = tokens.get(at + 1) {
                    renamed = rename_in(group.stream()).or(renamed);
                    at += 2;
                    continue;
                }
            }
            // A type is everything up to the comma that ends the field,
            // and a type may hold commas of its own.
            TokenTree::Punct(punct) if punct.as_char() == ',' && depth == 0 => {
                expecting_name = true;
            }
            TokenTree::Punct(punct) if punct.as_char() == '<' => depth += 1,
            TokenTree::Punct(punct) if punct.as_char() == '>' => depth -= 1,
            TokenTree::Ident(ident) if expecting_name => {
                let word = ident.to_string();
                // `pub`, `pub(crate)` and the rest are not the name.
                if word == "pub" {
                    at += 1;
                    if matches!(tokens.get(at), Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis)
                    {
                        at += 1;
                    }
                    continue;
                }
                let published = renamed.take().unwrap_or_else(|| word.clone());
                fields.push((word, published));
                expecting_name = false;
            }
            _ => {}
        }
        at += 1;
    }
    fields
}

/// `rename = "data-key"` inside a `#[rustx(..)]`.
fn rename_in(attribute: TokenStream2) -> Option<String> {
    // `#[rustx(..)]` arrives as the bracket's own stream, which is
    // `rustx` followed by the parentheses: the pair is nested twice.
    let tokens: Vec<TokenTree> = attribute.into_iter().collect();
    let inner = match tokens.iter().find_map(|token| match token {
        TokenTree::Group(group) => Some(group.stream()),
        _ => None,
    }) {
        Some(stream) => stream.into_iter().collect::<Vec<_>>(),
        None => tokens,
    };
    let mut seen_rename = false;
    for token in inner {
        match token {
            TokenTree::Ident(ident) if ident == "rename" => seen_rename = true,
            TokenTree::Literal(literal) if seen_rename => {
                let written = literal.to_string();
                return Some(written.trim_matches('"').to_string());
            }
            _ => {}
        }
    }
    None
}

/// The modules the application declares beside its entry point.
///
/// Read from `src/main.rs` (or `src/lib.rs`), because an attribute macro
/// is given the function it is written on and not the file around it,
/// and `Span::source_file` is not stable. A `mod` is declared on a line
/// of its own in every Rust file anyone writes, which is what makes
/// reading it this way honest rather than clever.
fn modules_of(manifest: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in ["src/main.rs", "src/lib.rs"] {
        let Ok(source) = std::fs::read_to_string(manifest.join(entry)) else {
            continue;
        };
        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            let declaration = line.strip_prefix("pub ").unwrap_or(line);
            let Some(rest) = declaration.strip_prefix("mod ") else {
                continue;
            };
            let Some(name) = rest.strip_suffix(';') else {
                continue;
            };
            let name = name.trim();
            if name
                .chars()
                .all(|letter| letter.is_alphanumeric() || letter == '_')
                && !name.is_empty()
                && !found.iter().any(|already| already == name)
            {
                found.push(name.to_string());
            }
        }
    }
    found
}
