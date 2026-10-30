//! The route table, read from the directory the pages live in.
//!
//! `app/page.rsx` is `/`, `app/device/page.rsx` is `/device`, and
//! `app/device/[id]/page.rsx` is `/device/[id]` — the folder names are
//! the URL, which is the comfort of Next.js and costs nothing at
//! runtime: the table is generated at build time and is a `static`.
//!
//! A `layout.rsx` wraps every page below it, outermost first. That is
//! the whole nesting rule.
//!
//! Each page is included in a module of its own, because two pages both
//! declaring `fn Page` must not collide, and each module opens with
//! `use super::*;` so that what the application's `main.rs` brought into
//! scope — `Element`, its own types — is in scope in the page too.

use std::path::{Path, PathBuf};

use crate::error::CompileError;
use crate::promote::component_signatures;

/// One page, and the layouts that enclose it.
struct Page {
    /// `/device/[id]`, as the matcher will see it.
    pattern: String,
    /// The generated `.rs` to include, relative to the table.
    include: String,
    /// The parameters `fn Page` declared, in order.
    parameters: Vec<String>,
    /// Every component this file declares that takes nothing, which
    /// is every component React could be given the name of.
    components: Vec<String>,
    /// Indices into the layout list, outermost first.
    layouts: Vec<usize>,
}

/// The text of the `routes.rs` for the pages under `from`.
pub fn table_for(from: &Path) -> Result<String, CompileError> {
    let mut sources = Vec::new();
    collect(from, &mut sources)?;
    // Sorted, so the table is the same whatever order the filesystem
    // hands the entries over in: §7.3 asks for a deterministic
    // transpilation and a route table is part of it.
    sources.sort();

    // Outermost first, by how deep the folder is — not by name. Sorted
    // alphabetically, `device/layout.rsx` comes before `layout.rsx`,
    // which both inverts the nesting and moves the root layout from
    // index 0 under `/` to index 1 under `/device`. The router keys a
    // layout by its depth, so a layout whose index changed between two
    // pages was released and rebuilt on navigation — the opposite of
    // what a layout is for.
    let mut layouts: Vec<PathBuf> = sources
        .iter()
        .filter(|path| file_name(path) == "layout.rsx")
        .cloned()
        .collect();
    layouts.sort_by_key(|path| (path.components().count(), path.clone()));
    for layout in &layouts {
        // A layout is a component that wraps what is below it, so it
        // has to take it. Reading the parameters and discarding them
        // sent the developer a `rustc` arity error about a call written
        // in `OUT_DIR` — a file they never wrote.
        let parameters = declares(layout, "Layout")?;
        if parameters != ["children"] {
            return Err(flag(
                layout,
                format!(
                    "`fn Layout` takes `children: Element` and nothing else — it takes {}. \
                     A layout wraps the screens below it, and `children` is what it wraps.",
                    if parameters.is_empty() {
                        "nothing".to_string()
                    } else {
                        parameters.join(", ")
                    }
                ),
            ));
        }
    }

    let mut pages = Vec::new();
    for source in sources.iter().filter(|path| file_name(path) == "page.rsx") {
        let parameters = declares(source, "Page")?;
        let relative = source.strip_prefix(from).unwrap_or(source);
        let pattern = pattern_of(relative);

        for parameter in &parameters {
            if !pattern.contains(&format!("[{parameter}]")) {
                return Err(flag(
                    source,
                    format!(
                        "`fn Page` takes `{parameter}`, and `{pattern}` has no `[{parameter}]` \
                         segment to fill it from. A page's parameters are its dynamic segments."
                    ),
                ));
            }
        }

        let directory = relative.parent().unwrap_or(Path::new(""));
        let enclosing = layouts
            .iter()
            .enumerate()
            .filter(|(_, layout)| {
                let layout_directory = layout
                    .strip_prefix(from)
                    .unwrap_or(layout)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_path_buf();
                directory.starts_with(&layout_directory)
            })
            .map(|(at, _)| at)
            .collect();

        let components = component_signatures(&std::fs::read_to_string(source).unwrap_or_default())
            .into_iter()
            .filter(|(_, parameters)| parameters.is_empty())
            .map(|(name, _)| name)
            .collect();

        pages.push(Page {
            pattern,
            include: include_path(relative),
            parameters,
            components,
            layouts: enclosing,
        });
    }

    Ok(render(&pages, &layouts, from))
}

/// The generated file, as text.
fn render(pages: &[Page], layouts: &[PathBuf], from: &Path) -> String {
    let mut out = String::from(
        "// Generated by rustx from the pages in `app/`. Nobody edits this file.\n\
         // Each page is a module of its own so that two `fn Page`s cannot collide,\n\
         // and every module opens on the application's own scope.\n\n",
    );

    // Each file gets a module, and each module one entry point of its
    // own: the components a `.rsx` declares are private to it, so the
    // call has to be written inside the module that can see them.
    for (at, layout) in layouts.iter().enumerate() {
        let relative = layout.strip_prefix(from).unwrap_or(layout);
        out.push_str(&format!(
            "mod layout_{at}_file {{\n    use super::*;\n    include!(\"{}\");\n\n\
             \x20   pub(super) fn frame(children: rustx_dom::Element) -> rustx_dom::Element {{\n\
             \x20       Layout(children)\n    }}\n}}\n\n\
             fn layout_{at}(children: rustx_dom::Element) -> rustx_dom::Element {{\n\
             \x20   layout_{at}_file::frame(children)\n}}\n\n",
            include_path(relative)
        ));
    }

    for (at, page) in pages.iter().enumerate() {
        let arguments: Vec<String> = page
            .parameters
            .iter()
            .map(|parameter| format!("params.get(\"{parameter}\").to_string()"))
            .collect();
        let components: Vec<String> = page
            .components
            .iter()
            .map(|name| {
                format!("(\"{name}\", {name} as fn() -> rustx_dom::Element)")
            })
            .collect();
        out.push_str(&format!(
            "mod page_{at} {{\n    use super::*;\n    include!(\"{}\");\n\n\
             \x20   pub(super) fn screen(params: &rustx_dom::route::Params) \
             -> rustx_dom::Element {{\n\
             \x20       let _ = params;\n        Page({})\n    }}\n\n\
             \x20   pub(super) fn components() -> Vec<(&'static str, fn() -> rustx_dom::Element)> \
             {{\n        vec![{}]\n    }}\n}}\n\n",
            page.include,
            arguments.join(", "),
            components.join(", ")
        ));
    }

    out.push_str(
        "/// Every route this application has, in the order the matcher\n\
         /// should try them.\n\
         pub fn route_table() -> &'static [rustx_dom::route::Route] {\n    &[\n",
    );
    for (at, page) in pages.iter().enumerate() {
        let enclosing: Vec<String> = page
            .layouts
            .iter()
            .map(|at| format!("layout_{at}"))
            .collect();
        out.push_str(&format!(
            "        rustx_dom::route::Route {{\n            pattern: \"{}\",\n\
             \x20           render: page_{at}::screen,\n\
             \x20           layouts: &[{}],\n        }},\n",
            page.pattern,
            enclosing.join(", ")
        ));
    }
    out.push_str("    ]\n}\n\n");

    // Every component the application declares, by name. An application
    // does not say which ones React may mount: a component written in
    // `app/` is a component, and a name nobody wrote is an error React
    // can catch.
    out.push_str(
        "/// Every component of this application that takes nothing, by name.\n\
         pub fn component_table() -> Vec<(&'static str, fn() -> rustx_dom::Element)> {\n\
         \x20   let mut all = Vec::new();\n",
    );
    for at in 0..pages.len() {
        out.push_str(&format!("    all.extend(page_{at}::components());\n"));
    }
    out.push_str("    all\n}\n");
    out
}

/// `device/[id]/page.rsx` → `/device/[id]`, and `page.rsx` → `/`.
fn pattern_of(relative: &Path) -> String {
    let segments: Vec<String> = relative
        .parent()
        .unwrap_or(Path::new(""))
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", segments.join("/"))
    }
}

/// How the generated table reaches a page's `.rs`, which sits beside it
/// under the same relative path. `include!` resolves relatively to the
/// file that writes it, so one `include!` in `main.rs` is enough for an
/// application.
fn include_path(relative: &Path) -> String {
    relative
        .with_extension("rs")
        .to_string_lossy()
        .replace('\\', "/")
}

/// The parameters of the component `wanted`, or a complaint naming the
/// declaration the file is missing.
fn declares(path: &Path, wanted: &str) -> Result<Vec<String>, CompileError> {
    let source = std::fs::read_to_string(path).map_err(|error| flag(path, error.to_string()))?;
    component_signatures(&source)
        .into_iter()
        .find(|(name, _)| name == wanted)
        .map(|(_, parameters)| parameters)
        .ok_or_else(|| {
            flag(
                path,
                format!(
                    "a `{}` has to declare `fn {wanted}`: that is what the route table calls.",
                    file_name(path)
                ),
            )
        })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn flag(path: &Path, message: String) -> CompileError {
    CompileError {
        offset: 0,
        file: path.to_path_buf(),
        line: 1,
        column: 1,
        message,
    }
}

fn collect(directory: &Path, into: &mut Vec<PathBuf>) -> Result<(), CompileError> {
    let entries =
        std::fs::read_dir(directory).map_err(|error| flag(directory, error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| flag(directory, error.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into)?;
        } else if path.extension().is_some_and(|extension| extension == "rsx") {
            into.push(path);
        }
    }
    Ok(())
}
