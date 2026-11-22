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

use rustx_compiler::build::modules_of;
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

    // `main` cannot be `async` in Rust, so the `async` comes off the
    // signature here -- before anything can go wrong, and not after.
    // A page the compiler refuses used to re-emit the signature exactly
    // as it was written, so the developer got their own error *and*
    // `error[E0752]: main function is not allowed to be async`, which
    // is about nothing they did and points at `src/main.rs`.
    let (signature, waits) = without_async(signature);
    // What a refused build emits: the function, with its body still on
    // the runtime if it had one, so the only error is the real one.
    let still_a_main = |body: TokenStream2| {
        let body = if waits {
            quote! { rustx::block_on(async move { #body }) }
        } else {
            body
        };
        quote! { #signature { #body } }
    };

    let (pages, prelude) = if app.is_dir() {
        let out = generated_directory(&manifest);
        // With routing, the OS-facing pages of `system/` (or the framework's
        // defaults) are compiled as pages under a reserved route.
        let (tree, staged) = if routed {
            match stage(&manifest, &app) {
                Ok(tree) => (tree, true),
                Err(complaint) => {
                    let emitted = still_a_main(body.stream());
                    return quote! { compile_error!(#complaint); #emitted }.into();
                }
            }
        } else {
            (app.clone(), false)
        };
        let compiled = compile(&tree, &out, routed);
        if staged {
            let _ = std::fs::remove_dir_all(&tree);
        }
        if let Err(complaint) = compiled.map_err(|c| {
            if staged {
                unstage(&c, &tree, &manifest)
            } else {
                c
            }
        }) {
            // The function is still emitted, untouched. A mistake in a
            // page is one error; without this, the developer also gets
            // "`main` function not found", which is about nothing they
            // did and sends them looking in the wrong file.
            let emitted = still_a_main(body.stream());
            return quote! {
                compile_error!(#complaint);
                #emitted
            }
            .into();
        }

        // Every source named, so that changing one rebuilds the
        // application. `include_bytes!` is the only way to say "this
        // file is part of me" that both cargo and `rustc` follow.
        let mut named = sources(&app);
        named.extend(sources(&manifest.join("system")));
        // The JavaScript the bundler reads is part of the application too: an edited
        // `components/Foo.tsx` used to leave the old bundle in the binary, silently,
        // because nothing named it, so nothing re-ran this expansion.
        named.extend(bundle_inputs(&manifest));
        let watched: Vec<TokenStream2> = named
            .iter()
            .map(|source| {
                let path = slashed(source);
                quote! {
                    const _: &[u8] = include_bytes!(#path);
                }
            })
            .collect();

        let file = out.join(rustx_compiler::build::PAGES);
        let generated = slashed(&file);
        let pages = pages_brought_in(&file, &generated);

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
                rustx::install_package(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"), env!("CARGO_PKG_DESCRIPTION"));
                rustx::install_routes(__rustx_pages::route_table());
                rustx::install_components(__rustx_pages::component_table());
                rustx::install_shared_values(__rustx_pages::shared_values());
            }
        } else {
            quote! {
                rustx::install_manifest_dir(env!("CARGO_MANIFEST_DIR"));
                rustx::install_package(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_AUTHORS"), env!("CARGO_PKG_DESCRIPTION"));
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
                    // `unused_braces`: a `{ … }` in markup is an
                    // expression the developer wrote, and it is emitted
                    // as it was read. Forty warnings about braces
                    // nobody can remove are forty warnings in front of
                    // the one that matters.
                    #![allow(non_snake_case, unused_braces, clippy::needless_return, clippy::useless_vec, clippy::type_complexity, clippy::redundant_closure)]
                    pub(crate) use super::*;
                    #(#opened)*
                    pub(crate) use rustx::Element;
                    pub(crate) use rustx::state::with_key;
                    // Ce que le code engendré nomme. Importé ici, par la
                    // façade, pour qu'une application n'ait pas à déclarer
                    // `rustx-dom` ni `rustx-engine` dans son `Cargo.toml`.
                    #[allow(unused_imports)]
                    pub(crate) use rustx::{rustx_dom, rustx_engine};
                    // Les crates que la facade porte, sous le nom que
                    // tout le monde leur donne. Une page ecrit
                    // `tokio::fs::write(...)` comme n'importe quel code
                    // Rust : `rustx::tokio::` etait un chemin que
                    // personne n'ecrit ailleurs, et cargo n'unifie
                    // qu'un seul tokio 1.x de toute facon, donc une
                    // application qui le declare aussi obtient le meme.
                    #[allow(unused_imports)]
                    pub(crate) use rustx::tokio;

                    #pages
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
    // The bundle and the style sheet, unless an editor is asking --
    // see `inside_an_editor`. Nothing the bundler produces changes
    // whether this crate compiles, so leaving it out of an editor's
    // expansion costs the editor nothing and saves it a child process
    // per keystroke.
    let javascript = if inside_an_editor() {
        TokenStream2::new()
    } else {
        match rustx_compiler::bundle::build(&manifest) {
            Err(complaint) => {
                let emitted = still_a_main(body.stream());
                return quote! {
                    compile_error!(#complaint);
                    #emitted
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
        }
    };

    // And everything under `assets/`, carried inside the executable.
    //
    // This is the one that made a built application only work on the
    // machine that built it: the document, the bundle, the window's
    // icon and every image a page named were read at run time from the
    // directory cargo compiled in. Copying the `.exe` anywhere else --
    // which is what shipping is -- gave a blank window.
    //
    // `include_bytes!` is also what tells cargo the file is part of the
    // crate, so editing an asset rebuilds the application.
    let assets = if inside_an_editor() {
        Vec::new()
    } else {
        assets_of(&manifest)
    };
    let assets = if assets.is_empty() {
        TokenStream2::new()
    } else {
        let entries: Vec<TokenStream2> = assets
            .iter()
            .map(|(name, path)| {
                let path = slashed(path);
                quote! { (#name, include_bytes!(#path) as &[u8]) }
            })
            .collect();
        quote! {
            rustx::install_assets(&[#(#entries),*]);
        }
    };

    let body = body.stream();

    // The body on the framework's own runtime, since the `async` came
    // off the signature above. A sub-process of Chromium runs this same
    // `main`, so the runtime it builds is the one its futures would
    // have used.
    let body = if waits {
        quote! { rustx::block_on(async move { #body }) }
    } else {
        body
    };

    quote! {
        #pages

        // Le meme nom que dans une page, et a cote de `main` plutot que
        // dedans : le code de l'application qui attend n'est pas tout
        // dans `main`, et une `async fn` ecrite a cote en a autant
        // besoin. Un seul tokio 1.x est resolu de toute facon, donc une
        // application qui le declare aussi obtient le meme.
        #[allow(unused_imports)]
        use rustx::tokio;

        #signature {
            #prelude
            #assets
            #javascript
            #body
        }
    }
    .into()
}

/// Compile the pages, turning a mistake in a `.rsx` into a sentence in
/// the coordinates of the `.rsx`.
/// The pages the OS asks an application for, by their locked file names in
/// `system/`. Each is optional; the framework brings a default for the ones it
/// writes.
const SYSTEM_PAGES: &[(&str, Option<&str>)] = &[
    ("install", Some(include_str!("../system/install.rsx"))),
    ("uninstall", Some(include_str!("../system/uninstall.rsx"))),
    ("update", Some(include_str!("../system/update.rsx"))),
    ("license", Some(include_str!("../system/license.rsx"))),
    (
        "permissions",
        Some(include_str!("../system/permissions.rsx")),
    ),
    ("about", Some(include_str!("../system/about.rsx"))),
    ("settings", Some(include_str!("../system/settings.rsx"))),
];

/// A copy of `app/` with the system pages added as `__system/<name>/page.rsx`,
/// which is what the compiler and the router are given.
fn stage(manifest: &Path, app: &Path) -> Result<PathBuf, String> {
    // One per process: cargo compiles the binary and its test harness at the same time,
    // and two stages of one folder deleted each other's files.
    let tree = manifest
        .join("target")
        .join(format!("rustx-tree-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tree);
    copy_pages(app, app, &tree)?;
    let system = manifest.join("system");
    if let Ok(entries) = std::fs::read_dir(&system) {
        let names: Vec<&str> = SYSTEM_PAGES.iter().map(|(name, _)| *name).collect();
        for entry in entries.flatten() {
            let path = entry.path();
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if path.extension().is_none_or(|e| e != "rsx") || !names.contains(&stem) {
                return Err(format!(
                    "{} is not a system page. `system/` holds the screens the OS asks an application for, under these names only: {}.",
                    path.display(),
                    names
                        .iter()
                        .map(|n| format!("{n}.rsx"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }
    for (name, default) in SYSTEM_PAGES {
        let written = system.join(format!("{name}.rsx"));
        let text = match std::fs::read_to_string(&written) {
            Ok(text) => text,
            Err(_) => match default {
                Some(text) => text.to_string(),
                None => continue,
            },
        };
        let folder = tree.join("__system").join(name);
        std::fs::create_dir_all(&folder)
            .map_err(|e| format!("rustx could not stage {}: {e}", folder.display()))?;
        std::fs::write(folder.join("page.rsx"), text)
            .map_err(|e| format!("rustx could not stage {name}: {e}"))?;
    }
    Ok(tree)
}

fn copy_pages(root: &Path, from: &Path, to: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if path.is_dir() {
            copy_pages(root, &path, to)?;
        } else if path.extension().is_some_and(|e| e == "rsx") {
            let target = to.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("rustx could not stage pages: {e}"))?;
            }
            std::fs::copy(&path, &target)
                .map_err(|e| format!("rustx could not stage {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// What the compiler said about the staged copy, said about the files the
/// developer wrote.
fn unstage(complaint: &str, tree: &Path, manifest: &Path) -> String {
    let (staged, app) = (
        tree.display().to_string(),
        manifest.join("app").display().to_string(),
    );
    let mut text = complaint.replace(&staged, &app);
    for (name, _) in SYSTEM_PAGES {
        for slash in ["/", "\\"] {
            text = text.replace(
                &format!("{app}{slash}__system{slash}{name}{slash}page.rsx"),
                &manifest
                    .join("system")
                    .join(format!("{name}.rsx"))
                    .display()
                    .to_string(),
            );
        }
    }
    text
}

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

/// Every file under `assets/`, as (name inside `assets/`, path on
/// disk).
///
/// Sorted, so that two builds of the same tree emit the same table and
/// the table is a thing a test can read. Directories are walked, and
/// the name always uses forward slashes: it is a file name on this
/// machine and a piece of a URL in the document.
///
/// What is deliberately left out: nothing. An application that puts a
/// 40 MB video in `assets/` gets a 40 MB executable, which is the
/// honest answer -- the alternative is a file that is sometimes there
/// and sometimes not, decided by a size limit nobody wrote down. An
/// application that does not want that puts the file beside the binary
/// instead, where `assets_dir` looks first.
fn assets_of(manifest: &Path) -> Vec<(String, PathBuf)> {
    let root = manifest.join("assets");
    let mut found = named_in_source(manifest);
    if !root.is_dir() {
        return found;
    }
    let mut directories = vec![root.clone()];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let Ok(relative) = path.strip_prefix(&root) else {
                continue;
            };
            let name = relative
                .components()
                .map(|piece| piece.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            found.push((name, path));
        }
    }
    found.sort();
    found.dedup_by(|left, right| left.0 == right.0);
    found
}

/// The files an application names in `Config` and keeps *outside*
/// `assets/`.
///
/// `Config::icon("logo.ico")` and `Config::index("page.html")` are read
/// at run time, and the runtime looks for them in the assets folder
/// first and in the project folder second -- which on a shipped
/// machine does not exist. So an application that put its icon beside
/// `Cargo.toml` instead of under `assets/` had a window with the system
/// icon and one line in the log, on every machine but the one that
/// built it.
///
/// Rather than refusing the path, it travels: the file is carried under
/// the name it was written with, which is the name `resolved_asset`
/// then finds in the unpacked folder. A path under `assets/` is left to
/// the walk above, and an absolute path is nobody's to carry.
fn named_in_source(manifest: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for source in rust_files(&manifest.join("src")) {
        let Ok(text) = std::fs::read_to_string(&source) else {
            continue;
        };
        for opening in [".icon(\"", ".index(\""] {
            let mut rest = text.as_str();
            while let Some(at) = rest.find(opening) {
                rest = &rest[at + opening.len()..];
                let Some(end) = rest.find('"') else { break };
                let written = &rest[..end];
                rest = &rest[end..];
                if written.is_empty() {
                    continue;
                }
                let path = PathBuf::from(written.replace('\\', "/"));
                if path.is_absolute() {
                    continue;
                }
                // Under `assets/` already: the walk carries it, and
                // carrying it twice would mean two `include_bytes!` of
                // the same megabyte.
                if written.starts_with("assets/") || written.starts_with("./assets/") {
                    continue;
                }
                let on_disk = manifest.join(&path);
                if !on_disk.is_file() {
                    continue;
                }
                found.push((written.replace('\\', "/"), on_disk));
            }
        }
    }
    found
}

/// Every `.rs` under a directory, which is where a `Config` is written.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut directories = vec![directory.to_path_buf()];
    while let Some(here) = directories.pop() {
        let Ok(entries) = std::fs::read_dir(&here) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every `.rsx` under `app/`, in a stable order.
fn sources(app: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(app, &mut found);
    found.sort();
    found
}

/// The files the bundler reads: everything script or style under `components/` and `lib/`.
fn bundle_inputs(manifest: &Path) -> Vec<PathBuf> {
    fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "node_modules") {
                    walk(&path, into);
                }
            } else if path.extension().is_some_and(|extension| {
                ["ts", "tsx", "js", "jsx", "mjs", "css"]
                    .iter()
                    .any(|known| extension == *known)
            }) {
                into.push(path);
            }
        }
    }
    let mut found = Vec::new();
    for directory in ["components", "lib"] {
        walk(&manifest.join(directory), &mut found);
    }
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
/// The signature without its `async`, and whether it had one.
///
/// Only the function's own `async` is taken: the first one at the top
/// level, before the parameter list. An `async` inside the parameters
/// (a default value, a closure type) is somebody else's.
fn without_async(signature: TokenStream2) -> (TokenStream2, bool) {
    let mut kept = Vec::new();
    let mut waits = false;
    for token in signature {
        let is_async = matches!(&token, TokenTree::Ident(name) if name == "async");
        if is_async && !waits {
            waits = true;
            continue;
        }
        kept.push(token);
    }
    (kept.into_iter().collect(), waits)
}

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

/// Whether this expansion is an editor's and not a build's.
///
/// It matters because this attribute does real work: it compiles the
/// pages, and it runs the JavaScript bundler, which is `esbuild` in a
/// child process. `rust-analyzer` expands a proc macro every time the
/// file changes, and it gives the expansion a few seconds before it
/// gives up on it -- so a bundler run on every keystroke is a
/// `#[rustx::main]` that times out, an expansion that produces nothing,
/// and every item below it reported as an error. That is what "the
/// completion stops" looks like from the inside.
///
/// Worse, a bundler failure became a `compile_error!`: an application
/// whose `npm install` has not been run yet showed a red
/// `#[rustx::main]` saying to run `npm install`, in an editor, for a
/// file the developer had not touched.
///
/// `rust-analyzer` runs proc macros in a server of its own
/// (`rust-analyzer-proc-macro-srv`, or `rust-analyzer` itself), so the
/// name of the running program is the honest answer. A `cargo check`
/// started *from* the editor is `rustc`, is a real build, and is not
/// this case.
fn inside_an_editor() -> bool {
    let program = std::env::current_exe().ok();
    let name = program
        .as_deref()
        .and_then(std::path::Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    an_editor(
        name.as_deref(),
        std::env::var("RUSTX_MACRO_BUNDLE").ok().as_deref(),
    )
}

/// The decision itself, given the name of the running program and what
/// the environment said -- so that it can be asked, which a function
/// that reads the environment cannot be.
///
/// `RUSTX_MACRO_BUNDLE` is an escape hatch in both directions, because
/// a detector that cannot be turned off is a detector that is wrong on
/// somebody's machine with no way out: `always` bundles even in an
/// editor, `never` bundles in neither.
fn an_editor(program: Option<&str>, told: Option<&str>) -> bool {
    match told {
        Some("always") => return false,
        Some("never") => return true,
        _ => {}
    }
    let Some(program) = program else {
        return false;
    };
    let program = program.to_ascii_lowercase();
    program.contains("rust-analyzer") || program.contains("proc-macro-srv")
}

/// Les pages, telles que le code engendré les apporte.
///
/// Un `include!` est ce qu'il faut à une vraie compilation : une erreur
/// dans une page tombe dans le fichier engendré, à sa ligne, et cargo
/// surveille ce fichier.
///
/// Mais ce fichier est sous `target/`, que tout `.gitignore` ignore, et
/// le VFS de `rust-analyzer` suit `.gitignore` : il ne le charge jamais,
/// l'`include!` échoue, et le corps entier de l'application se retrouve
/// souligné sous un seul `macro-error: failed to load file`. Dans
/// l'éditeur, les pages sont donc recopiées dans l'expansion, où il n'y
/// a aucun fichier à charger — et personne n'a de réglage à poser dans
/// son éditeur pour que son application s'affiche sans rouge.
///
/// Le repli est l'`include!` d'origine : un fichier que l'on n'arrive
/// pas à lire garde le message qu'il avait, qui dit de construire
/// l'application une première fois.
fn pages_brought_in(file: &Path, path: &str) -> TokenStream2 {
    if inside_an_editor()
        && let Ok(source) = std::fs::read_to_string(file)
        && let Ok(tokens) = rooted_at(&source, file).parse::<TokenStream2>()
    {
        return tokens;
    }
    quote! { include!(#path); }
}

/// Le même fichier, dont les `include!` tiennent encore debout ailleurs.
///
/// Le fichier engendré inclut chaque page à côté de lui, par son seul
/// nom (`include!("page.rs")`), et un `include!` se résout depuis le
/// dossier du fichier qui l'écrit. Recopié dans l'expansion, ce nom se
/// mettrait soudain à désigner `src/page.rs`, que personne n'a écrit.
/// Chacun est donc rendu absolu avant d'être recopié.
///
/// shortcut: remplace `include!("` partout dans le texte, donc une page
/// qui écrit elle-même un `include!` relatif se verrait réécrire avec la
/// mauvaise base -- à reprendre si une page a un jour besoin d'inclure
/// un fichier à elle.
fn rooted_at(source: &str, file: &Path) -> String {
    let Some(directory) = file.parent() else {
        return source.to_string();
    };
    let directory = slashed(directory);
    source.replace("include!(\"", &format!("include!(\"{directory}/"))
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

#[cfg(test)]
mod tests {
    /// A file an application names in `Config` and keeps outside
    /// `assets/` has to travel too: the runtime looks in the assets
    /// folder and then in the project folder, and a shipped machine
    /// has no project folder.
    #[test]
    fn a_file_named_outside_the_assets_folder_still_travels() {
        let root =
            std::env::temp_dir().join(format!("rustx-named-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("a src");
        std::fs::create_dir_all(root.join("assets")).expect("an assets");
        std::fs::write(root.join("logo.ico"), b"icon").expect("an icon");
        std::fs::write(root.join("page.html"), b"<p>").expect("a page");
        std::fs::write(root.join("assets/inside.ico"), b"also").expect("an asset");
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() { Config::new(\"X\").icon(\"logo.ico\").index(\"page.html\"); }",
        )
        .expect("a main.rs");

        let carried = assets_of(&root);
        let names: Vec<&str> = carried.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"logo.ico"), "{names:?}");
        assert!(names.contains(&"page.html"), "{names:?}");
        // And the folder's own files are still there, under the name
        // they have inside it.
        assert!(names.contains(&"inside.ico"), "{names:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A path already under `assets/` is carried by the walk, and
    /// carrying it twice would mean two `include_bytes!` of one file.
    #[test]
    fn a_file_named_inside_the_assets_folder_is_carried_once() {
        let root =
            std::env::temp_dir().join(format!("rustx-named-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("a src");
        std::fs::create_dir_all(root.join("assets")).expect("an assets");
        std::fs::write(root.join("assets/icon.ico"), b"icon").expect("an icon");
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() { Config::new(\"X\").icon(\"assets/icon.ico\"); }",
        )
        .expect("a main.rs");

        let carried = assets_of(&root);
        let how_many = carried
            .iter()
            .filter(|(name, _)| name == "icon.ico")
            .count();
        assert_eq!(how_many, 1, "{carried:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A declared path that is not on disk is said once by the runtime
    /// and must not break the build here.
    #[test]
    fn a_named_file_that_is_not_there_carries_nothing() {
        let root =
            std::env::temp_dir().join(format!("rustx-named-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("a src");
        std::fs::write(
            root.join("src/main.rs"),
            "fn main() { Config::new(\"X\").icon(\"nowhere.ico\"); }",
        )
        .expect("a main.rs");
        assert!(assets_of(&root).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `rust-analyzer` expands this attribute on every keystroke and
    /// gives up on an expansion that takes too long, so the bundler --
    /// a child process -- must not run there. These are the names the
    /// two sides actually have.
    #[test]
    fn an_editors_expansion_is_told_apart_from_a_builds() {
        for editor in [
            "rust-analyzer-proc-macro-srv",
            "rust-analyzer-proc-macro-srv.exe",
            "rust-analyzer.exe",
            "Rust-Analyzer-Proc-Macro-Srv.exe",
        ] {
            assert!(
                an_editor(Some(editor), None),
                "{editor} was taken for a build"
            );
        }
        for build in [
            "rustc",
            "rustc.exe",
            "cargo",
            "cargo.exe",
            "build-script-build",
        ] {
            assert!(
                !an_editor(Some(build), None),
                "{build} was taken for an editor"
            );
        }
        // Nothing known: a build, because refusing to bundle is the
        // failure a developer cannot diagnose.
        assert!(!an_editor(None, None));
    }

    #[test]
    fn the_environment_overrides_the_guess_both_ways() {
        assert!(!an_editor(Some("rust-analyzer.exe"), Some("always")));
        assert!(an_editor(Some("rustc"), Some("never")));
        // Anything else is not an instruction.
        assert!(!an_editor(Some("rustc"), Some("yes please")));
    }

    use super::*;

    /// `main` cannot be `async` in Rust, so the attribute takes the
    /// `async` off the signature and puts the body on the framework's
    /// runtime instead.
    ///
    /// Taken off **before** anything can go wrong, which is the whole
    /// point: a page the compiler refuses used to re-emit the signature
    /// exactly as written, so the developer got their own error *and*
    /// `error[E0752]: main function is not allowed to be async`, which
    /// is about nothing they did and points at `src/main.rs`. Item 21 of
    /// the log of gaps.
    #[test]
    fn the_async_comes_off_the_signature() {
        let signature: TokenStream2 = "async fn main()".parse().expect("parses");
        let (kept, waits) = without_async(signature);
        assert!(waits, "the `async` was not noticed");
        assert!(
            !kept.to_string().contains("async"),
            "the `async` is still there: {kept}"
        );
        assert!(kept.to_string().contains("main"), "{kept}");
    }

    #[test]
    fn a_main_that_does_not_wait_is_left_as_it_was() {
        let signature: TokenStream2 = "fn main()".parse().expect("parses");
        let (kept, waits) = without_async(signature);
        assert!(!waits);
        assert_eq!(kept.to_string(), "fn main ()");
    }

    /// Only the function's own `async`. One written inside the
    /// parameters — a closure type, a bound — belongs to somebody else.
    #[test]
    fn an_async_inside_the_parameters_is_not_the_functions_own() {
        let signature: TokenStream2 = "fn main(work: fn() -> impl Future)"
            .parse()
            .expect("parses");
        let (_, waits) = without_async(signature);
        assert!(!waits, "a parameter's tokens were read as the function's");
    }

    #[test]
    fn a_function_is_split_into_its_signature_and_its_body() {
        let item: TokenStream2 = "fn main() { let x = 1; }".parse().expect("parses");
        let (signature, body) = split_body(item).expect("a function has a body");
        assert_eq!(signature.to_string(), "fn main ()");
        assert!(body.stream().to_string().contains("let x"));
    }

    /// Something that is not a function has no body to take off, and
    /// saying so is better than taking the last token off whatever it
    /// was.
    #[test]
    fn something_that_is_not_a_function_is_not_split() {
        let item: TokenStream2 = "struct Window;".parse().expect("parses");
        assert!(split_body(item).is_none());
    }

    #[test]
    fn the_name_of_the_struct_is_read_past_its_visibility() {
        let tokens: Vec<TokenTree> = "pub struct Point { at: f64 }"
            .parse::<TokenStream2>()
            .expect("parses")
            .into_iter()
            .collect();
        assert_eq!(type_name(&tokens).as_deref(), Some("Point"));
    }

    /// The fields a `ToValue` is written from, with the name each one
    /// crosses the JavaScript boundary under.
    #[test]
    fn every_field_is_read_with_the_name_it_crosses_under() {
        let body: TokenStream2 = "at: f64, value: f64".parse().expect("parses");
        assert_eq!(
            fields_of(body),
            vec![
                ("at".to_string(), "at".to_string()),
                ("value".to_string(), "value".to_string()),
            ]
        );
    }

    /// A type may hold commas of its own, and a field's visibility is
    /// not its name.
    #[test]
    fn a_type_with_commas_in_it_is_still_one_field() {
        let body: TokenStream2 = "pub rows: Vec<(usize, String)>, pub(crate) total: usize"
            .parse()
            .expect("parses");
        assert_eq!(
            fields_of(body),
            vec![
                ("rows".to_string(), "rows".to_string()),
                ("total".to_string(), "total".to_string()),
            ]
        );
    }

    /// `#[rustx(rename = "data-key")]`: a field whose JavaScript name is
    /// not a Rust identifier says so.
    #[test]
    fn a_renamed_field_crosses_under_the_name_it_was_given() {
        let body: TokenStream2 = "#[rustx(rename = \"data-key\")] key: String, value: f64"
            .parse()
            .expect("parses");
        assert_eq!(
            fields_of(body),
            vec![
                ("key".to_string(), "data-key".to_string()),
                ("value".to_string(), "value".to_string()),
            ]
        );
    }

    #[test]
    fn an_attribute_that_renames_nothing_renames_nothing() {
        let attribute: TokenStream2 = "rustx(skip)".parse().expect("parses");
        assert_eq!(rename_in(attribute), None);
    }

    /// An `include!` path is written with forward slashes whatever the
    /// platform's separator is: `rustc` reads the path, not the shell.
    /// The modules a page is allowed to name, read out of the crate's
    /// own `main.rs` or `lib.rs`.
    ///
    /// The last helper of this crate with no test. It reads source text
    /// rather than tokens, so every one of its rules is a guess about
    /// how somebody writes a line: `pub mod`, a comment, a `mod` with a
    /// body, a name with a hyphen in it.
    #[test]
    fn every_module_the_crate_declares_is_offered_to_a_page() {
        let directory = std::env::temp_dir().join(format!(
            "rustx-macros-modules-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(directory.join("src")).expect("a directory");
        std::fs::write(
            directory.join("src").join("main.rs"),
            "mod pages;
pub mod kinds;
// mod commented;
mod inline { }
mod pages;
",
        )
        .expect("a main.rs");

        let found = modules_of(&directory);

        assert!(found.contains(&"pages".to_string()), "{found:?}");
        assert!(
            found.contains(&"kinds".to_string()),
            "`pub mod` is a module too: {found:?}"
        );
        assert!(
            !found.contains(&"commented".to_string()),
            "a commented-out declaration is not a declaration: {found:?}"
        );
        assert!(
            !found.contains(&"inline".to_string()),
            "a module with its body in the file is not a file: {found:?}"
        );
        assert_eq!(
            found.iter().filter(|name| *name == "pages").count(),
            1,
            "named twice, offered twice: {found:?}"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_path_for_an_include_is_written_with_forward_slashes() {
        let path = Path::new("target").join("rustx-pages").join("pages.rs");
        assert_eq!(slashed(&path), "target/rustx-pages/pages.rs");
    }

    /// Une page incluse par son seul nom suit le fichier engendré
    /// lorsqu'on le recopie ailleurs.
    ///
    /// Sans cela, `include!("page.rs")` recopié dans l'expansion de
    /// `#[rustx::main]` désigne `src/page.rs`, que personne n'a écrit :
    /// c'est l'erreur qu'un développeur voyait dans son éditeur.
    #[test]
    fn a_page_included_beside_its_table_is_found_from_anywhere() {
        let file = Path::new("tmp")
            .join("video-editor")
            .join("target")
            .join("rustx-pages")
            .join("pages.rs");
        let rewritten = rooted_at("mod page_0 {\n    include!(\"page.rs\");\n}\n", &file);
        assert!(
            rewritten.contains("include!(\"tmp/video-editor/target/rustx-pages/page.rs\")"),
            "got {rewritten}"
        );
    }
}
