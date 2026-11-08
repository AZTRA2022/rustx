//! The JavaScript an application needs, written by the framework.
//!
//! An application declares its libraries the way any JavaScript project
//! does — `npm install recharts` — and writes its components in `.rsx`.
//! What is left is the file that imports those libraries and hands them
//! to the runtime, and that file is the same file every time, so it is
//! not the application's to write: it is generated here, bundled with
//! the application's own esbuild, and put in the page by the engine.
//!
//! It lives in the compiler rather than in the attribute because a
//! proc-macro crate can export nothing but macros, and this wants a test
//! of its own.

use std::path::{Path, PathBuf};

/// Where what the framework generates goes. Ignored by git, like
/// `node_modules`: it is output, and output is not a source.
pub const DIRECTORY: &str = ".rustx";

/// The packages that are tools rather than libraries of the page.
///
/// A page cannot render esbuild. Importing a build tool into the bundle
/// would pull a second copy of it into the window.
const TOOLS: &[&str] = &[
    "esbuild",
    "tailwindcss",
    "postcss",
    "autoprefixer",
    "typescript",
    "vite",
    "webpack",
    "rollup",
    "parcel",
    "@biomejs/biome",
    "prettier",
    "eslint",
];

/// The scopes whose packages are tools, whatever they are called
/// inside.
///
/// `npm install tailwindcss @tailwindcss/cli` is the first line of
/// Tailwind 4's own getting-started page, and it puts the command-line
/// tool in `dependencies`. A bundle that imports it asks a browser to
/// run a compiler.
const TOOL_SCOPES: &[&str] = &["@tailwindcss/", "@esbuild/", "@swc/", "@parcel/", "@rollup/"];

/// Is this package a tool of the build rather than a library of the
/// page?
fn is_a_tool(package: &str) -> bool {
    TOOLS.contains(&package) || TOOL_SCOPES.iter().any(|scope| package.starts_with(scope))
}

/// What `node_modules` holds for a package a page might write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    /// There, and a page can `import * as x from "<package>"`.
    Importable,
    /// There, and there is nothing to import from its root.
    ///
    /// Two shapes, and both used to take the whole build down with an
    /// esbuild error about a path nobody wrote: sub-paths and no root,
    /// which is what `@tiptap/pm` is (a page writes `@tiptap/pm/state`,
    /// never `@tiptap/pm`); and a stylesheet for an entry point, which
    /// is what `tw-animate-css` is, and which belongs to the
    /// stylesheet and not to the bundle.
    ///
    /// Left out of the entry rather than refused: neither is a mistake
    /// the developer made, and a `package.json` written the way the
    /// library's own page says to write it has to build.
    NothingToImport,
    /// Declared and never installed. That one is worth a sentence,
    /// because the fix is a command.
    Absent,
}

/// What `node_modules` holds for `package`.
pub fn installed(app: &Path, package: &str) -> Installed {
    let directory = package
        .split('/')
        .fold(app.join("node_modules"), |at, part| at.join(part));
    let Ok(manifest) = std::fs::read_to_string(directory.join("package.json")) else {
        return Installed::Absent;
    };
    match root_export_of(&manifest) {
        Some(entry) if !entry.ends_with(".css") => Installed::Importable,
        _ => Installed::NothingToImport,
    }
}

/// The root entry of an installed package, as its own manifest declares
/// it — or `None` when it declares none.
///
/// Read by hand, like the dependencies: what is wanted is whether a
/// root entry exists and whether it is a stylesheet, and neither
/// question is worth a JSON parser.
fn root_export_of(manifest: &str) -> Option<String> {
    if let Some(exports) = value_after(manifest, "\"exports\"") {
        // `"exports": "./index.js"` — a root, and that is all of it.
        if exports.starts_with('"') {
            return Some(exports.trim_matches('"').to_string());
        }
        // `"exports": { … }` — a root only if a key is `"."` or a
        // condition (`"import"`, `"default"`). Keys that all begin with
        // `"./"` are sub-paths, and a sub-path is not a root.
        let keys = keys_of(exports);
        if keys.is_empty() {
            return None;
        }
        if keys.iter().any(|key| key == "." || !key.starts_with("./")) {
            // What it resolves to is esbuild's business; that there is
            // one is all this has to know.
            return Some(String::from("."));
        }
        return None;
    }
    for field in ["\"module\"", "\"main\""] {
        if let Some(value) = value_after(manifest, field)
            && value.starts_with('"')
        {
            return Some(value.trim_matches('"').to_string());
        }
    }
    // No `exports`, no `main`: node falls back to `index.js`.
    Some(String::from("index.js"))
}

/// The text of the value that follows `key`, trimmed, or `None`.
fn value_after<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let at = text.find(key)? + key.len();
    let rest = text[at..].trim_start().strip_prefix(':')?.trim_start();
    if let Some(inside) = rest.strip_prefix('"') {
        // The closing quote, counted from the start of `rest` so that
        // what comes back includes both quotes.
        let end = inside.find('"')? + 2;
        return Some(&rest[..end]);
    }
    if rest.starts_with('{') {
        let mut depth = 0usize;
        let mut inside = false;
        for (index, letter) in rest.char_indices() {
            match letter {
                '"' => inside = !inside,
                '{' if !inside => depth += 1,
                '}' if !inside => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&rest[..=index]);
                    }
                }
                _ => {}
            }
        }
        return None;
    }
    None
}

/// The keys of a JSON object, at its first level only.
fn keys_of(object: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    let mut inside = false;
    let mut expecting_key = false;
    for letter in object.chars() {
        match letter {
            '"' if !inside => {
                inside = true;
                current.clear();
            }
            '"' if inside => {
                inside = false;
                if depth == 1 && expecting_key {
                    keys.push(current.clone());
                    expecting_key = false;
                }
            }
            _ if inside => current.push(letter),
            '{' => {
                depth += 1;
                if depth == 1 {
                    expecting_key = true;
                }
            }
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 1 => expecting_key = true,
            ':' => {}
            _ => {}
        }
    }
    keys
}

/// What React is imported under, and what it is called in the page.
const REACT: &[(&str, &str)] = &[
    ("react", "react"),
    ("reactDom", "react-dom"),
    ("reactDomClient", "react-dom/client"),
];

/// The entry point for an application with these dependencies.
/// Where an application keeps the React components it owns.
///
/// What `npx shadcn add button` leaves behind: TypeScript in the
/// project itself, not in `node_modules`. Nobody installs those and
/// nobody imports them, so a page that wrote `<Button />` would find
/// nothing — unless the framework puts them in the bundle, which is
/// what this is.
const OWN: &str = "components";

/// What Tailwind reads, as one comma-separated list.
///
/// The pages, the application's own Rust — a class name can be written
/// in a `const` beside the code that uses it — and the application's own
/// React components, which is where every shadcn component keeps its
/// classes.
pub fn content_globs() -> String {
    sources()
        .iter()
        .map(|glob| format!("./{glob}"))
        .collect::<Vec<String>>()
        .join(",")
}

/// The same, as the globs themselves: one list, written once, whichever
/// way the installed Tailwind wants to be told about it.
fn sources() -> [&'static str; 4] {
    [
        "app/**/*.rsx",
        "src/**/*.rs",
        "components/**/*.{ts,tsx,js,jsx}",
        "lib/**/*.{ts,tsx,js,jsx}",
    ]
}

/// Every component file the application wrote itself, as an absolute
/// path.
///
/// The one walk both halves of the framework read: the bundle, which
/// has to re-export them, and the editor, which has to offer them. They
/// disagreed once — the bundle read `components/ui` and the Tailwind
/// globs read `components/**` — and what fell in the gap was a
/// developer's own `components/Editor.tsx`, a tag that compiled and
/// found nothing at runtime.
pub fn own_components(app: &Path) -> Vec<PathBuf> {
    let directory = app.join(OWN);
    let mut files = Vec::new();
    collect_components(&directory, &directory, &mut files);
    files.sort();
    files
        .iter()
        .map(|relative| directory.join(relative))
        .collect()
}

/// Where the application keeps the components it owns.
pub fn own_directory(app: &Path) -> PathBuf {
    app.join(OWN)
}

/// A module re-exporting every component the application wrote, or
/// `None` when it wrote none.
///
/// Generated rather than asked for: `shadcn add` writes one file per
/// component and no index, so the index is the framework's to keep up
/// to date — and it is regenerated at every build, which is what makes
/// `shadcn add dialog` take effect without anybody editing anything.
pub fn index_for(app: &Path) -> Option<String> {
    let directory = app.join(OWN);
    let mut files = Vec::new();
    collect_components(&directory, &directory, &mut files);
    if files.is_empty() {
        return None;
    }
    // In a fixed order, so that two builds of the same project write the
    // same file and nothing downstream sees a change that is not one.
    files.sort();

    let mut out = String::from(
        "// Generated by rustx from this application's own components.\n\
         // Nobody edits this file, and nobody commits it.\n\n",
    );
    let mut named: Vec<String> = Vec::new();
    for file in &files {
        out.push_str(&format!("export * from \"../{OWN}/{file}\";\n"));
        // And the default export, under the name of the file. A
        // developer's own React component is almost always
        // `export default function Editor()`, and `export *` does not
        // carry a default: without this a page that wrote `<Editor>`
        // found nothing, while shadcn's own components — which export
        // by name — worked, and the difference was invisible.
        let Some(name) = component_name(file) else {
            continue;
        };
        if named.contains(&name) {
            continue;
        }
        if has_a_default(&directory.join(file)) {
            out.push_str(&format!(
                "export {{ default as {name} }} from \"../{OWN}/{file}\";\n"
            ));
            named.push(name);
        }
    }
    Some(out)
}

/// Every component file under `at`, by its path relative to `root`,
/// with `/` for a separator whatever the system writes.
///
/// Recursive, because `components/ui` is where `shadcn add` leaves its
/// own and `components/` is where a developer leaves theirs -- and the
/// Tailwind globs beside this have always read both.
fn collect_components(root: &Path, at: &Path, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            collect_components(root, &path, into);
            continue;
        }
        let is_a_component = path
            .extension()
            .and_then(|kind| kind.to_str())
            .is_some_and(|kind| matches!(kind, "ts" | "tsx" | "js" | "jsx"));
        if !is_a_component {
            continue;
        }
        // A declaration file declares; it exports no value a page could
        // render.
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".d.ts"))
        {
            continue;
        }
        if let Ok(relative) = path.strip_prefix(root) {
            into.push(
                relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .join("/"),
            );
        }
    }
}

/// The name a page would write for the default export of this file.
///
/// `Editor.tsx` is `Editor`; `my-editor.tsx` is `MyEditor`. A file
/// whose name is not a name -- `index.ts` -- gets none, because
/// `<Index>` is nothing anybody means.
pub fn component_name(file: &str) -> Option<String> {
    let base = file.rsplit('/').next()?;
    let stem = base.split('.').next()?;
    if stem.is_empty() || stem.eq_ignore_ascii_case("index") {
        return None;
    }
    let mut name = String::new();
    let mut capitalise = true;
    for letter in stem.chars() {
        match letter {
            '-' | '_' | '.' => capitalise = true,
            _ if capitalise => {
                name.extend(letter.to_uppercase());
                capitalise = false;
            }
            _ => name.push(letter),
        }
    }
    if name.starts_with(|letter: char| letter.is_ascii_digit()) {
        return None;
    }
    Some(name)
}

/// Does this file have a default export?
pub fn has_a_default(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|text| {
            text.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("export default") || line.starts_with("export { default")
            })
        })
        .unwrap_or(false)
}

pub fn entry_for(dependencies: &[String]) -> String {
    entry_for_with(dependencies, false)
}

/// The same, for an application whose own components are to be imported
/// when it has any.
pub fn entry_for_in(dependencies: &[String], app: &Path) -> String {
    entry_for_with(dependencies, index_for(app).is_some())
}

fn entry_for_with(dependencies: &[String], own: bool) -> String {
    let libraries: Vec<(String, &String)> = dependencies
        .iter()
        .filter(|package| !is_a_tool(package))
        .filter(|package| !REACT.iter().any(|(_, from)| from == package))
        .map(|package| (name_for(package), package))
        .collect();

    let mut out = String::from(
        "// Generated by rustx from this application's package.json.\n\
         // Nobody edits this file, and nobody commits it.\n\n",
    );
    for (name, from) in REACT {
        out.push_str(&format!("import * as {name} from \"{from}\";\n"));
    }
    for (name, package) in &libraries {
        out.push_str(&format!("import * as {name} from \"{package}\";\n"));
    }

    if own {
        out.push_str("\n// The components this application wrote itself — `components/ui`,\n");
        out.push_str("// which is where `shadcn add` leaves them. The index beside this\n");
        out.push_str("// file is generated too, so a component added today is a tag today.\n");
        out.push_str("import * as ui from \"./ui.ts\";\n");
    }

    out.push_str("\n// What the pages may write as a tag. Read by the framework's own\n");
    out.push_str("// runtime, which the engine puts in the page before this runs.\n");
    out.push_str("window.rustxLibraries = {\n");
    if own {
        out.push_str("  ui,\n");
    }
    for (name, _) in REACT {
        out.push_str(&format!("  {name},\n"));
    }
    for (name, _) in &libraries {
        out.push_str(&format!("  {name},\n"));
    }
    out.push_str("};\n");
    out
}

/// What a package is called where a page can write it.
///
/// `recharts` stays `recharts`; `@radix-ui/react-dialog` becomes
/// `radixUiReactDialog`, because a page writes a Rust path and `@` and
/// `/` and `-` are not in one.
pub fn name_for(package: &str) -> String {
    let mut name = String::new();
    let mut capitalise = false;
    for letter in package.chars() {
        match letter {
            '@' => {}
            '/' | '-' | '.' | '_' => capitalise = !name.is_empty(),
            _ if capitalise => {
                name.extend(letter.to_uppercase());
                capitalise = false;
            }
            _ => name.push(letter),
        }
    }
    name
}

/// Two packages whose names, once made into identifiers, are the same.
///
/// `@tiptap/react` and `tiptap-react` both come to `tiptapReact`. The
/// entry point would write the second over the first in one object
/// literal, which JavaScript allows and nobody can see.
pub fn a_collision_in(packages: &[String]) -> Option<(String, String)> {
    for (at, one) in packages.iter().enumerate() {
        let name = name_for(one);
        for two in &packages[at + 1..] {
            if name_for(two) == name {
                return Some((one.clone(), two.clone()));
            }
        }
    }
    None
}

/// What the framework built for an application.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Built {
    /// The application's libraries, bundled.
    pub bundle: Option<PathBuf>,
    /// The stylesheet Tailwind wrote from the classes in the pages.
    pub stylesheet: Option<PathBuf>,
}

/// Write and build this application's JavaScript and CSS.
///
/// Everything empty when the application has no `package.json`: it uses
/// no library, so there is nothing to bundle and nothing to install.
pub fn build(app: &Path) -> Result<Built, String> {
    let manifest = app.join("package.json");
    let Ok(declared) = std::fs::read_to_string(&manifest) else {
        return Ok(Built::default());
    };
    let stylesheet = styles(app)?;
    // The build tools go first, and before anything else is asked of
    // the disk: an application whose only dependencies are Tailwind and
    // its command line wants no esbuild, no entry point and no bundle.
    let libraries: Vec<String> = dependencies_in(&declared)
        .into_iter()
        .filter(|package| !is_a_tool(package))
        .collect();
    if libraries.is_empty() {
        return Ok(Built {
            bundle: None,
            stylesheet,
        });
    }

    let esbuild = esbuild_in(app).ok_or_else(|| {
        format!(
            "this application declares JavaScript libraries in {} and has no `node_modules` to load them from. Run `npm install` in {}.",
            manifest.display(),
            app.display()
        )
    })?;

    // And what each library is: in the bundle, left out of it, or
    // declared and never installed.
    let mut dependencies = Vec::new();
    let mut absent = Vec::new();
    for package in libraries {
        match installed(app, &package) {
            Installed::Importable => dependencies.push(package),
            Installed::NothingToImport => {}
            Installed::Absent => absent.push(package),
        }
    }
    if !absent.is_empty() {
        return Err(format!(
            "{} names `{}` under `dependencies`, and `node_modules` has nothing of that name. Run `npm install` in {}.",
            manifest.display(),
            absent.join("`, `"),
            app.display()
        ));
    }
    // Two libraries that come to the same name. The entry point would
    // declare the second one over the first in `window.rustxLibraries`,
    // and a page would get the components of a library it did not mean
    // — silently, because a duplicate key in an object literal is legal
    // JavaScript.
    if let Some((one, two)) = a_collision_in(&dependencies) {
        return Err(format!(
            "`{one}` and `{two}` both come to `{}`, which is the name a page writes. One of \
             the two has to go, or be imported from a component of the application instead.",
            name_for(&one)
        ));
    }
    if dependencies.is_empty() {
        return Ok(Built {
            bundle: None,
            stylesheet,
        });
    }

    let out = app.join(DIRECTORY);
    std::fs::create_dir_all(&out)
        .map_err(|error| format!("rustx could not write {}: {error}", out.display()))?;
    // The index of the application's own components, first: the entry
    // imports it, and esbuild reads what is on disk.
    if let Some(index) = index_for(app) {
        let beside = out.join("ui.ts");
        std::fs::write(&beside, index)
            .map_err(|error| format!("rustx could not write {}: {error}", beside.display()))?;
    }

    let entry = out.join("entry.js");
    std::fs::write(&entry, entry_for_in(&dependencies, app))
        .map_err(|error| format!("rustx could not write {}: {error}", entry.display()))?;

    let bundle = out.join("bundle.js");
    let finished = std::process::Command::new(&esbuild)
        .arg(&entry)
        .arg("--bundle")
        .arg("--format=iife")
        .arg("--target=chrome120")
        .arg("--minify")
        // The page is a browser, whatever the machine running the build
        // is. `--platform=node` left `require("fs")` in an IIFE and
        // picked the server build of a library that has two.
        .arg("--platform=browser")
        // React reads it on its first line, and a shell is what used to
        // eat the quotes: there is no shell here.
        .arg("--define:process.env.NODE_ENV=\"production\"")
        .arg(format!("--outfile={}", bundle.display()))
        .current_dir(app)
        .output()
        .map_err(|error| format!("rustx could not run {}: {error}", esbuild.display()))?;

    if !finished.status.success() {
        return Err(format!(
            "esbuild refused this application's libraries:\n{}",
            String::from_utf8_lossy(&finished.stderr).trim()
        ));
    }

    // Beside the document, under a name nobody would write by hand.
    //
    // A bundle is a megabyte of JavaScript, and a megabyte handed to
    // `execute_java_script` as a string is a script that sometimes
    // simply does not run — the page then has no libraries and every
    // boundary in it is an empty container. A browser loads a script
    // that size from a URL, which is what it is good at, so the file
    // goes where the document can reach it and the framework adds the
    // one `<script src>` the developer does not have to write.
    let beside = app.join("assets").join(SERVED);
    if let Some(assets) = beside.parent() {
        std::fs::create_dir_all(assets)
            .map_err(|error| format!("rustx could not write {}: {error}", assets.display()))?;
    }
    crate::build::copy_if_changed(&bundle, &beside)
        .map_err(|error| format!("rustx could not write {}: {error}", beside.display()))?;

    Ok(Built {
        bundle: Some(beside),
        stylesheet,
    })
}

/// What the bundle is called beside the application's document.
pub const SERVED: &str = "rustx-bundle.js";

/// The stylesheet Tailwind writes from the classes the pages use.
///
/// Run when the application installed Tailwind, and not otherwise. The
/// input is the application's own `assets/tailwind.css` when it wrote
/// one; otherwise the framework asks for the utilities alone — a page
/// writes `className="bg-violet-500"`, and Tailwind's base layer would
/// reset the stylesheet the page already has.
///
/// What it scans is `app/**/*.rsx`: Tailwind reads the pages it styles,
/// which is the one thing it has to be told here, because nothing else
/// in its world has that extension.
fn styles(app: &Path) -> Result<Option<PathBuf>, String> {
    let Some(tailwind) = tool_in(app, "tailwindcss") else {
        // Tailwind in `node_modules` and no command line to run it:
        // Tailwind 4 moved the binary into a package of its own, and an
        // application that installed only the library got a window with
        // every `className` in it and not one rule — silently, which is
        // the worst of the two ways to be wrong.
        if app.join("node_modules").join("tailwindcss").exists() {
            return Err(format!(
                "this application installed tailwindcss and nothing that can run it. \
                 Tailwind 4 keeps its command line in a package of its own: run \
                 `npm install @tailwindcss/cli` in {} (on Tailwind 3, `npm install \
                 tailwindcss@3`, which carries its own).",
                app.display()
            ));
        }
        return Ok(None);
    };
    let out = app.join(DIRECTORY);
    std::fs::create_dir_all(&out)
        .map_err(|error| format!("rustx could not write {}: {error}", out.display()))?;

    // Whichever Tailwind the application installed, spoken to the way
    // that one is spoken to.
    let dialect = dialect_in(app);
    let written = app.join("assets").join("tailwind.css");
    let input = match input_for(dialect, written.exists()) {
        None => written,
        Some(text) => {
            let generated = out.join("tailwind.in.css");
            std::fs::write(&generated, text).map_err(|error| {
                format!("rustx could not write {}: {error}", generated.display())
            })?;
            generated
        }
    };

    let stylesheet = out.join("tailwind.css");
    // Tailwind writes where it is told, and it writes every time. The
    // sheet is `include_str!`d by the attribute, so a new date on it is
    // a crate cargo believes out of date — and a project that rebuilds
    // for ever. So Tailwind writes beside it, and the sheet itself is
    // touched only when the bytes differ.
    let fresh = out.join("tailwind.next.css");
    let finished = std::process::Command::new(&tailwind)
        .args(arguments_for(dialect, &input, &fresh))
        .current_dir(app)
        .output()
        .map_err(|error| format!("rustx could not run {}: {error}", tailwind.display()))?;

    if !finished.status.success() {
        return Err(format!(
            "tailwindcss refused this application's pages:\n{}",
            String::from_utf8_lossy(&finished.stderr).trim()
        ));
    }
    crate::build::copy_if_changed(&fresh, &stylesheet)
        .map_err(|error| format!("rustx could not write {}: {error}", stylesheet.display()))?;
    Ok(Some(stylesheet))
}

/// How the installed Tailwind is told which files to read.
///
/// The one thing about a JavaScript library that the framework cannot
/// discover at runtime: Tailwind is not imported by the page, it is a
/// program the build *runs*, and a program has a command line. That
/// command line changed between 3 and 4 — `--content` became `@source`,
/// `@tailwind utilities` became `@import "tailwindcss"` — so there are
/// two contracts, and which one is spoken is decided by what the
/// application installed, never by what the framework prefers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// Tailwind 3 and before: the sources on the command line.
    Content,
    /// Tailwind 4: the sources in the stylesheet.
    Source,
}

/// Which contract this application's Tailwind speaks.
///
/// Read from the version `node_modules` actually holds rather than from
/// the range in `package.json`: `^3.4` and `^4.0` are ranges, and what
/// is installed is a fact. When there is no version to read, the
/// command line itself answers: 4 moved it into `@tailwindcss/cli`, so a
/// project that has that package is a project on 4.
pub fn dialect_in(app: &Path) -> Dialect {
    match installed_major(app, "tailwindcss") {
        Some(major) if major >= 4 => Dialect::Source,
        Some(_) => Dialect::Content,
        None => {
            let own_cli = app
                .join("node_modules")
                .join("@tailwindcss")
                .join("cli")
                .exists();
            if own_cli {
                Dialect::Source
            } else {
                Dialect::Content
            }
        }
    }
}

/// The major version of an installed package, from its own manifest.
fn installed_major(app: &Path, package: &str) -> Option<u32> {
    let manifest = package
        .split('/')
        .fold(app.join("node_modules"), |at, part| at.join(part))
        .join("package.json");
    let text = std::fs::read_to_string(manifest).ok()?;
    let at = text.find("\"version\"")? + "\"version\"".len();
    let rest = text[at..].trim_start().strip_prefix(':')?.trim_start();
    let version = rest.strip_prefix('"')?;
    let version = &version[..version.find('"')?];
    version.split('.').next()?.parse().ok()
}

/// The stylesheet to hand the command line, or `None` to hand it the
/// one the application wrote.
///
/// On 3, an application's own stylesheet goes in as it is: it holds the
/// `@tailwind` layers itself, and the sources travel on the command
/// line. On 4 the sources are `@source` lines, which have to live in the
/// stylesheet being compiled — so the application's own is imported
/// from a generated one rather than replaced, because the theme a
/// developer wrote is theirs.
pub fn input_for(dialect: Dialect, own: bool) -> Option<String> {
    match (dialect, own) {
        (Dialect::Content, true) => None,
        // The utilities alone: a page writes `className="bg-violet-500"`
        // and Tailwind's base layer would reset the stylesheet the page
        // already has.
        (Dialect::Content, false) => Some("@tailwind utilities;\n".to_string()),
        (Dialect::Source, own) => {
            let first = if own {
                // Relative to the generated stylesheet, which sits in
                // the framework's own directory beside `assets`.
                "@import \"../assets/tailwind.css\";"
            } else {
                "@import \"tailwindcss\";"
            };
            let sources: String = sources()
                .iter()
                .map(|glob| format!("@source \"../{glob}\";\n"))
                .collect();
            Some(format!("{first}\n{sources}"))
        }
    }
}

/// The command line for one run of Tailwind.
pub fn arguments_for(dialect: Dialect, input: &Path, output: &Path) -> Vec<String> {
    let mut arguments = vec![
        "--input".to_string(),
        input.display().to_string(),
        "--output".to_string(),
        output.display().to_string(),
    ];
    if dialect == Dialect::Content {
        // One `--content`, with every glob in it: the flag does not
        // repeat, and a second one silently replaced the first — the
        // pages were never scanned and every utility was missing while
        // the reset was there.
        arguments.push("--content".to_string());
        arguments.push(content_globs());
    }
    arguments.push("--minify".to_string());
    arguments
}

/// The names under `dependencies` in a `package.json`.
///
/// Read by hand: a `package.json` is the one JSON file this crate ever
/// reads, and the keys of one object are not worth a dependency on a
/// JSON parser. `devDependencies` are left out on purpose — a tool is
/// not a library of the page.
pub fn dependencies_in(manifest: &str) -> Vec<String> {
    let Some(at) = manifest.find("\"dependencies\"") else {
        return Vec::new();
    };
    let Some(open) = manifest[at..].find('{') else {
        return Vec::new();
    };
    let open = at + open;
    let Some(close) = manifest[open..].find('}') else {
        return Vec::new();
    };

    let mut names = Vec::new();
    let body = &manifest[open + 1..open + close];
    for pair in body.split(',') {
        let Some(key) = pair.split(':').next() else {
            continue;
        };
        let name = key.trim().trim_matches('"').trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    names
}

/// The application's own esbuild, if it installed one.
fn esbuild_in(app: &Path) -> Option<PathBuf> {
    tool_in(app, "esbuild")
}

/// A tool from the application's own `node_modules`, under whichever
/// name this system gave it.
fn tool_in(app: &Path, tool: &str) -> Option<PathBuf> {
    let binaries = app.join("node_modules").join(".bin");
    for name in [
        format!("{tool}.cmd"),
        format!("{tool}.exe"),
        tool.to_string(),
    ] {
        let path = binaries.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scoped_package_becomes_an_identifier() {
        assert_eq!(name_for("@radix-ui/react-dialog"), "radixUiReactDialog");
        assert_eq!(name_for("recharts"), "recharts");
        assert_eq!(name_for("react-dom"), "reactDom");
    }

    #[test]
    fn the_dependencies_are_read_and_the_dev_ones_are_not() {
        let manifest = "{\n  \"dependencies\": {\n    \"recharts\": \"2.12.7\",\n    \
             \"@radix-ui/react-dialog\": \"1.1.1\"\n  },\n  \
             \"devDependencies\": { \"esbuild\": \"0.23.1\" }\n}";
        assert_eq!(
            dependencies_in(manifest),
            vec!["recharts".to_string(), "@radix-ui/react-dialog".to_string()]
        );
    }
}
