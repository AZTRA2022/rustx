//! The first five minutes of the framework.
//!
//! What a developer sees when they type `rustx new` is the framework's
//! argument, made of files: a `main.rs` of four lines, a page, and
//! nothing else. No `build.rs`, no plumbing, nothing to configure
//! before the window opens — because a project that opens with forty
//! lines of ceremony has already said what the framework is about, and
//! said the wrong thing.

use std::path::{Path, PathBuf};

/// Write a project called `name` inside `into`.
///
/// Nothing is written until the name has been accepted and the
/// directory found free: half a project on disk, named something cargo
/// will not build, is worse than a sentence.
pub fn write(name: &str, into: &Path) -> Result<Vec<PathBuf>, String> {
    let name = name.trim();
    refuse_a_bad_name(name)?;

    let root = into.join(name);
    if root.exists() {
        return Err(format!(
            "`{}` already exists. Give the project another name, or another place to live.",
            root.display()
        ));
    }

    let files: Vec<(PathBuf, String)> = vec![
        (root.join("Cargo.toml"), manifest(name)),
        (root.join("src").join("main.rs"), main_for(name)),
        (root.join("app").join("page.rsx"), page()),
        (root.join(".gitignore"), ignored()),
        (root.join("assets").join("readme.md"), assets_note()),
        (root.join("package.json"), package_json(name)),
        (root.join("tsconfig.json"), tsconfig()),
        (root.join("components.json"), components_json()),
        (root.join("lib").join("utils.ts"), utils_ts()),
        (root.join("components").join("readme.md"), components_note()),
        (root.join("assets").join("tailwind.css"), tailwind_css()),
    ];

    let mut written = Vec::new();
    for (path, text) in files {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| why(parent, &error))?;
        }
        std::fs::write(&path, text).map_err(|error| why(&path, &error))?;
        written.push(path);
    }
    Ok(written)
}

/// A name cargo would refuse, refused here first and in words.
///
/// Every refusal here is a failure a developer would otherwise meet one
/// step later and in worse words: cargo's own, npm's own, or Windows's
/// — and by then there is a directory on disk with half a project in
/// it. The promise of [`write`] is that nothing is written until the
/// name is known to be a name, so this function is where every known
/// way for a name to be wrong is written down.
pub fn refuse_a_bad_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("A project needs a name: `rustx new mon-app`.".to_string());
    }
    // A crate name's first character has to be a letter. Cargo says
    // "the first character must be a Unicode XID_Start" about a digit;
    // about a `-` or a `_` it says the same thing, and npm refuses a
    // package whose name begins with `_` outright — so one rule covers
    // all three, and `rustx new --help` stops being a directory called
    // `--help`.
    if !name.starts_with(|letter: char| letter.is_ascii_alphabetic()) {
        let first = name.chars().next().unwrap_or(' ');
        return Err(format!(
            "`{name}` starts with `{first}`, and a crate name starts with a letter. \
             Try `app-{}`.",
            name.trim_start_matches(['-', '_', '.'])
        ));
    }
    if let Some(wrong) = name
        .chars()
        .find(|letter| !(letter.is_ascii_alphanumeric() || *letter == '-' || *letter == '_'))
    {
        return Err(format!(
            "`{name}` holds a `{wrong}`, which a crate name cannot. Letters, digits, `-` and \
             `_` only."
        ));
    }
    // crates.io takes sixty-four characters, and a path on Windows is
    // bounded too: a name that long makes a project whose deepest
    // generated file cannot be written, and the error for *that* is an
    // IO error about a path nobody typed.
    if name.len() > 64 {
        return Err(format!(
            "`{name}` is {} characters long, and a crate name stops at 64.",
            name.len()
        ));
    }
    // The framework's own names — every one of the eight crates, by
    // package name and by directory name, because a project that shares
    // one has to depend on itself and cargo's message for that is not
    // one anybody should have to read. `_` and `-` are compared the
    // same, because a library called `rustx_dom` collides with
    // `rustx-dom`'s library whichever way the package was spelled.
    let normalised = name.to_ascii_lowercase().replace('_', "-");
    if OURS.contains(&normalised.as_str()) {
        return Err(format!(
            "`{name}` is the framework's own name, and a project cannot share it — it would \
             have to depend on itself."
        ));
    }
    // A keyword is not a module name, and a crate is a module. Cargo
    // refuses these, late, with a message about Unicode identifiers.
    if KEYWORDS.contains(&normalised.replace('-', "_").as_str()) {
        return Err(format!(
            "`{name}` is a Rust keyword, and a crate is a module whose name has to be \
             an identifier. `{name}-app` is not."
        ));
    }
    // And the names of the crates every Rust program already has. Cargo
    // refuses these too: "it conflicts with Rust's built-in crate
    // names".
    if BUILT_IN.contains(&normalised.replace('-', "_").as_str()) {
        return Err(format!(
            "`{name}` is the name of a crate every Rust program already has, and cargo \
             refuses it for that reason. `{name}-app` is free."
        ));
    }
    // Windows has nine file names that are devices, at every level of
    // every directory. `mkdir nul` does not make a directory, and what
    // it says instead is not about a project name.
    let without_extension = normalised.split('.').next().unwrap_or(&normalised);
    if RESERVED_ON_WINDOWS.contains(&without_extension) {
        return Err(format!(
            "`{name}` is a device name on Windows, not a file name — a directory \
             cannot be called that on any machine that runs one. `{name}-app` can."
        ));
    }
    Ok(())
}

/// The framework's own names: each crate's package name and its
/// directory name, since both are things a developer might type.
const OURS: &[&str] = &[
    "rustx",
    "rustx-app",
    "rustx-cli",
    "rustx-compiler",
    "rustx-dom",
    "rustx-engine",
    "rustx-lsp",
    "rustx-macro",
    "rustx-macros",
    "rustx-os",
];

/// Rust's keywords, the reserved ones included: a crate name is a
/// module name, and `use crate::…` is already something else.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "union",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// The crates every Rust program already has, which cargo refuses as a
/// package name for exactly that reason.
const BUILT_IN: &[&str] = &["std", "core", "alloc", "test", "proc_macro"];

/// The nine names Windows gives to devices rather than to files, at
/// every level of every directory.
const RESERVED_ON_WINDOWS: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn why(path: &Path, error: &std::io::Error) -> String {
    format!("`{}` could not be written: {error}", path.display())
}

fn manifest(name: &str) -> String {
    let wanted = wanted_version();
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\n\
         [dependencies]\n\
         # The package is called `rustx-app` on crates.io, because\n\
         # `rustx` is taken there. The library is called `rustx`, so the\n\
         # code and the documentation write `rustx::` as if nothing had\n\
         # happened.\n\
         rustx = {{ package = \"rustx-app\", version = \"{wanted}\" }}\n"
    )
}

/// What a new project asks for: this command line's own minor.
///
/// Read from the version of this crate rather than written out, because
/// a written-out one goes stale in silence. `version = "0.1"` outlived
/// 0.2.0, and under cargo's rule for 0.x a `0.1` requirement does not
/// accept 0.2.0 — so `rustx new` kept making projects pinned to the
/// framework of the version before, and what they got was a refusal
/// that had been fixed months earlier.
fn wanted_version() -> String {
    let full = env!("CARGO_PKG_VERSION");
    let mut parts = full.split('.');
    match (parts.next(), parts.next()) {
        (Some("0"), Some(minor)) => format!("0.{minor}"),
        (Some(major), _) => major.to_string(),
        _ => full.to_string(),
    }
}

/// A project ready for the documentation's first line: `npm install
/// recharts`, `npm install shadcn`, whatever the page reaches for.
///
/// Without this file, `npm install` still runs — but on an old `npm`
/// it refuses outright, and on a new one it writes a `package.json` of
/// its own: named after the directory rather than the project, at
/// `"1.0.0"`, with no `"private"`. That file shows up a step late and
/// looks like something `rustx new` forgot, when the fix is one file
/// written on the day the project is born.
///
/// `esbuild` is declared here too, under `devDependencies` — not because
/// a developer is meant to know it exists, but because the README says
/// the opposite: "`npm install`, and that's it — the framework builds
/// the bundle with esbuild." Without this line that sentence is false:
/// `bundle::build` refuses every library in `node_modules` until
/// `esbuild` is one of them, and nothing a developer installed shows up
/// on the page, silently, because the tool that would have said why is
/// the one that is missing.
///
/// `allowScripts` goes with it. Esbuild fetches its own platform binary
/// through a postinstall script, and a modern `npm` holds every
/// postinstall script back until it is approved — one more silent way
/// for `esbuild.cmd` to exist and still have nothing to run. Esbuild is
/// named with its version, because the version above is exact and the
/// two are edited together. `@parcel/watcher`, which arrives under
/// `@tailwindcss/cli`, is named without one: its version is whatever
/// the tree resolves and nobody here can pin it. npm accepts both
/// forms, and without that second line a fresh `npm install` ends on a
/// warning about a script it held back.
///
/// And the libraries the files beside it already import.
///
/// `lib/utils.ts` imports `clsx` and `tailwind-merge` on the day the
/// project is born; `assets/tailwind.css` imports `tailwindcss` and
/// `tw-animate-css`; every component `npx shadcn add` writes imports
/// React and most of them `class-variance-authority`. A project whose
/// files import what its manifest does not declare is a project that
/// fails on its first `npm install` — or worse, installs and then
/// cannot bundle, which is the failure that says nothing useful.
///
/// So the manifest declares them, and `npm install` in a fresh project
/// is enough for `npx shadcn add button` to work. The heavier libraries
/// a page might reach for — Convex, Supabase, a charting library —
/// need no files beyond this: `npm install convex` and the generated
/// entry point picks them up. shadcn was the one that needed more than
/// a dependency, because it reads `components.json` and `tsconfig.json`
/// before it writes anything.
fn package_json(name: &str) -> String {
    let npm_name = name.to_lowercase();
    format!(
        "{{\n\
         \x20   \"name\": \"{npm_name}\",\n\
         \x20   \"private\": true,\n\
         \x20   \"version\": \"0.1.0\",\n\
         \x20   \"dependencies\": {{\n\
         \x20       \"class-variance-authority\": \"^0.7.1\",\n\
         \x20       \"clsx\": \"^2.1.1\",\n\
         \x20       \"react\": \"18.3.1\",\n\
         \x20       \"react-dom\": \"18.3.1\",\n\
         \x20       \"tailwind-merge\": \"^3.7.0\"\n\
         \x20   }},\n\
         \x20   \"devDependencies\": {{\n\
         \x20       \"@tailwindcss/cli\": \"^4.3.3\",\n\
         \x20       \"@types/react\": \"^18.3.31\",\n\
         \x20       \"@types/react-dom\": \"^18.3.7\",\n\
         \x20       \"esbuild\": \"0.28.2\",\n\
         \x20       \"tailwindcss\": \"^4.3.3\",\n\
         \x20       \"tw-animate-css\": \"^1.4.0\",\n\
         \x20       \"typescript\": \"^5.9.3\"\n\
         \x20   }},\n\
         \x20   \"allowScripts\": {{\n\
         \x20       \"@parcel/watcher\": true,\n\
         \x20       \"esbuild@0.28.2\": true\n\
         \x20   }}\n\
         }}\n"
    )
}

/// What makes `@/lib/utils` resolve — for esbuild and for the shadcn
/// command line both.
///
/// Every component `npx shadcn add` writes starts with
/// `import { cn } from "@/lib/utils"`. That `@` is not a package: it is
/// a path alias, declared here and nowhere else. Two things read this
/// file, and a project without it fails at both — the shadcn command
/// line refuses before writing anything ("No import alias found in your
/// tsconfig.json file"), and if the files are copied in by hand esbuild
/// then refuses to bundle them ("Could not resolve `@/lib/utils`"), so
/// the library is installed, the component is on disk, and the page is
/// empty.
///
/// esbuild finds it by walking up from each file it reads, which is why
/// it sits at the root of the project rather than beside the entry point
/// the framework generates.
fn tsconfig() -> String {
    "{\n\
     \x20 \"compilerOptions\": {\n\
     \x20   \"target\": \"ES2020\",\n\
     \x20   \"lib\": [\"ES2020\", \"DOM\", \"DOM.Iterable\"],\n\
     \x20   \"jsx\": \"react-jsx\",\n\
     \x20   \"module\": \"ESNext\",\n\
     \x20   \"moduleResolution\": \"Bundler\",\n\
     \x20   \"allowImportingTsExtensions\": true,\n\
     \x20   \"strict\": true,\n\
     \x20   \"noEmit\": true,\n\
     \x20   \"skipLibCheck\": true,\n\
     \x20   \"baseUrl\": \".\",\n\
     \x20   \"paths\": { \"@/*\": [\"./*\"] }\n\
     \x20 },\n\
     \x20 \"include\": [\"components\", \"lib\"]\n\
     }\n"
    .to_string()
}

/// What `npx shadcn add` reads before it writes anything.
///
/// Without this file the command line runs its own `init` — which asks
/// questions in a terminal that may not be interactive, guesses a
/// Next.js project, and puts the stylesheet somewhere the framework does
/// not look. With it, `npx shadcn add button` is the whole of the
/// installation, and the component lands in `components/ui/` where the
/// bundler's own index already looks for it.
///
/// `"rsc": false` because there is no server here, and `"css"` points at
/// `assets/tailwind.css` because that is the one file the framework
/// hands to Tailwind.
fn components_json() -> String {
    "{\n\
     \x20 \"$schema\": \"https://ui.shadcn.com/schema.json\",\n\
     \x20 \"style\": \"new-york\",\n\
     \x20 \"rsc\": false,\n\
     \x20 \"tsx\": true,\n\
     \x20 \"tailwind\": {\n\
     \x20   \"config\": \"\",\n\
     \x20   \"css\": \"assets/tailwind.css\",\n\
     \x20   \"baseColor\": \"zinc\",\n\
     \x20   \"cssVariables\": true,\n\
     \x20   \"prefix\": \"\"\n\
     \x20 },\n\
     \x20 \"aliases\": {\n\
     \x20   \"components\": \"@/components\",\n\
     \x20   \"utils\": \"@/lib/utils\",\n\
     \x20   \"ui\": \"@/components/ui\",\n\
     \x20   \"lib\": \"@/lib\",\n\
     \x20   \"hooks\": \"@/hooks\"\n\
     \x20 }\n\
     }\n"
    .to_string()
}

/// The one function every shadcn component imports.
///
/// Six lines, and they are the same six lines in every project that uses
/// them. Written on the day the project is born rather than left for the
/// first `shadcn add` to notice, because a component that cannot find
/// `cn` is a component that does not compile — and the file is harmless
/// in a project that never installs one.
fn utils_ts() -> String {
    "import { clsx, type ClassValue } from \"clsx\";\n\
     import { twMerge } from \"tailwind-merge\";\n\n\
     /** What every shadcn component uses to merge its classes. */\n\
     export function cn(...inputs: ClassValue[]) {\n\
     \x20 return twMerge(clsx(inputs));\n\
     }\n"
    .to_string()
}

/// The stylesheet an application owns, and the theme shadcn reads.
///
/// Writing this file is how an application says it wants the whole of
/// Tailwind — its reset and its component layer, not the utilities
/// alone. The variables under it are shadcn/ui's theme at base colour
/// zinc: `bg-primary`, `text-muted-foreground` and `border-input` are
/// names of *these*, so a component installed into a project without
/// them is a component with no colours at all.
///
/// Tailwind 4 is configured in CSS — `@theme` is the configuration —
/// so there is no `tailwind.config.js` to write and none to keep in
/// step with this.
fn tailwind_css() -> String {
    "/* The whole of Tailwind for this application: its reset, its\n\
     \x20  component layer and its utilities. Delete this file and the\n\
     \x20  framework asks Tailwind for the utilities alone, so a page that\n\
     \x20  brings its own stylesheet is not reset underneath it.\n\n\
     \x20  The variables are shadcn/ui's theme, base colour zinc. Every\n\
     \x20  component `npx shadcn add` installs reads them, so they belong\n\
     \x20  to the application and not to the framework. */\n\
     @import \"tailwindcss\";\n\
     @import \"tw-animate-css\";\n\n\
     @theme inline {\n\
     \x20 --color-background: var(--background);\n\
     \x20 --color-foreground: var(--foreground);\n\
     \x20 --color-card: var(--card);\n\
     \x20 --color-card-foreground: var(--card-foreground);\n\
     \x20 --color-popover: var(--popover);\n\
     \x20 --color-popover-foreground: var(--popover-foreground);\n\
     \x20 --color-primary: var(--primary);\n\
     \x20 --color-primary-foreground: var(--primary-foreground);\n\
     \x20 --color-secondary: var(--secondary);\n\
     \x20 --color-secondary-foreground: var(--secondary-foreground);\n\
     \x20 --color-muted: var(--muted);\n\
     \x20 --color-muted-foreground: var(--muted-foreground);\n\
     \x20 --color-accent: var(--accent);\n\
     \x20 --color-accent-foreground: var(--accent-foreground);\n\
     \x20 --color-destructive: var(--destructive);\n\
     \x20 --color-destructive-foreground: var(--destructive-foreground);\n\
     \x20 --color-border: var(--border);\n\
     \x20 --color-input: var(--input);\n\
     \x20 --color-ring: var(--ring);\n\
     \x20 --radius-lg: var(--radius);\n\
     \x20 --radius-md: calc(var(--radius) - 2px);\n\
     \x20 --radius-sm: calc(var(--radius) - 4px);\n\
     \x20 /* A desktop application\x27s font is the system\x27s own: it is\n\
     \x20    already installed, and it is the one that is recognised.\n\
     \x20    Declaring the token is not enough on its own -- the rule\n\
     \x20    in `@layer base` below is what puts it on the document. */\n\
     \x20 --font-sans: \"Segoe UI Variable Text\", \"Segoe UI\",\n\
     \x20   ui-sans-serif, system-ui, -apple-system, sans-serif;\n\
     \x20 --font-mono: ui-monospace, \"Cascadia Mono\", Consolas,\n\
     \x20   \"Liberation Mono\", monospace;\n\
     }\n\n\
     :root {\n\
     \x20 --radius: 0.625rem;\n\
     \x20 --background: oklch(1 0 0);\n\
     \x20 --foreground: oklch(0.141 0.005 285.823);\n\
     \x20 --card: oklch(1 0 0);\n\
     \x20 --card-foreground: oklch(0.141 0.005 285.823);\n\
     \x20 --popover: oklch(1 0 0);\n\
     \x20 --popover-foreground: oklch(0.141 0.005 285.823);\n\
     \x20 --primary: oklch(0.21 0.006 285.885);\n\
     \x20 --primary-foreground: oklch(0.985 0 0);\n\
     \x20 --secondary: oklch(0.967 0.001 286.375);\n\
     \x20 --secondary-foreground: oklch(0.21 0.006 285.885);\n\
     \x20 --muted: oklch(0.967 0.001 286.375);\n\
     \x20 --muted-foreground: oklch(0.552 0.016 285.938);\n\
     \x20 --accent: oklch(0.967 0.001 286.375);\n\
     \x20 --accent-foreground: oklch(0.21 0.006 285.885);\n\
     \x20 --destructive: oklch(0.577 0.245 27.325);\n\
     \x20 --destructive-foreground: oklch(0.985 0 0);\n\
     \x20 --border: oklch(0.92 0.004 286.32);\n\
     \x20 --input: oklch(0.92 0.004 286.32);\n\
     \x20 --ring: oklch(0.705 0.015 286.067);\n\
     }\n\n\
     .dark {\n\
     \x20 --background: oklch(0.141 0.005 285.823);\n\
     \x20 --foreground: oklch(0.985 0 0);\n\
     \x20 --card: oklch(0.21 0.006 285.885);\n\
     \x20 --card-foreground: oklch(0.985 0 0);\n\
     \x20 --popover: oklch(0.21 0.006 285.885);\n\
     \x20 --popover-foreground: oklch(0.985 0 0);\n\
     \x20 --primary: oklch(0.92 0.004 286.32);\n\
     \x20 --primary-foreground: oklch(0.21 0.006 285.885);\n\
     \x20 --secondary: oklch(0.274 0.006 286.033);\n\
     \x20 --secondary-foreground: oklch(0.985 0 0);\n\
     \x20 --muted: oklch(0.274 0.006 286.033);\n\
     \x20 --muted-foreground: oklch(0.705 0.015 286.067);\n\
     \x20 --accent: oklch(0.274 0.006 286.033);\n\
     \x20 --accent-foreground: oklch(0.985 0 0);\n\
     \x20 --destructive: oklch(0.704 0.191 22.216);\n\
     \x20 --destructive-foreground: oklch(0.985 0 0);\n\
     \x20 --border: oklch(1 0 0 / 10%);\n\
     \x20 --input: oklch(1 0 0 / 15%);\n\
     \x20 --ring: oklch(0.552 0.016 285.938);\n\
     }\n\n\
     @layer base {\n\
     \x20 * {\n\
     \x20   border-color: var(--border);\n\
     \x20 }\n\
     \x20 body {\n\
     \x20   background-color: var(--background);\n\
     \x20   color: var(--foreground);\n\
     \x20   font-family: var(--font-sans);\n\
     \x20 }\n\
     }\n"
    .to_string()
}

/// The `main.rs` of the design document, and not one line more.
fn main_for(name: &str) -> String {
    let title = title_of(name);
    format!(
        "//! `{name}`.\n\
         //!\n\
         //! The pages are in `app/`. This file is the window.\n\n\
         use rustx::{{App, Config}};\n\n\
         #[rustx::main]\n\
         fn main() -> Result<(), rustx::Error> {{\n\
         \x20   let app = App::new(Config::new(\"{title}\").size(900, 640))?;\n\n\
         \x20   app.run()\n\
         }}\n"
    )
}

/// `mon-app` becomes `Mon app`: a window title is read by a person.
fn title_of(name: &str) -> String {
    let spaced = name.replace(['-', '_'], " ");
    let mut letters = spaced.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
        None => spaced,
    }
}

/// The first page: a counter, because it is the shortest thing that
/// shows what the framework does that nothing else does — a `let mut`
/// that is state, with no annotation anywhere.
fn page() -> String {
    "let mut clicks = 0;\n\
     \n\
     fn Page() -> Element {\n\
     \x20   return <main style={{\n\
     \x20       display: \"grid\",\n\
     \x20       placeItems: \"center\",\n\
     \x20       height: \"100vh\",\n\
     \x20       gap: \"16px\",\n\
     \x20       fontFamily: \"system-ui, sans-serif\",\n\
     \x20       background: \"#0b0b10\",\n\
     \x20       color: \"#f4f4f5\",\n\
     \x20   }}>\n\
     \x20       <h1 style={{ margin: \"0\", fontSize: \"34px\" }}>Hello.</h1>\n\
     \x20\n\
     \x20       <button\n\
     \x20           onClick={move |_| clicks += 1}\n\
     \x20           style={{\n\
     \x20               padding: \"10px 18px\",\n\
     \x20               borderRadius: \"10px\",\n\
     \x20               border: \"0\",\n\
     \x20               background: \"#7f5af0\",\n\
     \x20               color: \"#ffffff\",\n\
     \x20               fontSize: \"16px\",\n\
     \x20           }}\n\
     \x20       >\n\
     \x20           Clicked {clicks} times\n\
     \x20       </button>\n\
     \x20   </main>;\n\
     }\n"
    .to_string()
}

/// What a project does not commit.
///
/// Four lines rather than one, because three of them are output the
/// framework writes on every build: the entry point and the bundle in
/// `.rustx`, the copy of the bundle the document loads from `assets`,
/// and `node_modules`. A project whose first `git add .` carried two
/// megabytes of minified JavaScript learned that from here.
fn ignored() -> String {
    "/target\n/node_modules\n/.rustx/\n/assets/rustx-bundle.js\n".to_string()
}

/// Why there is a `components/` on the day the project is born.
///
/// Three files written by this command name that directory before
/// anything has put a component in it: `tsconfig.json` compiles it,
/// `components.json` aliases it as `@/components`, and the bundler
/// walks it looking for what to re-export. A project whose own
/// configuration names a directory that is not there is a project a
/// developer cannot check for themselves — and `npx shadcn add button`
/// is the first thing that writes into it, so it may as well be waiting.
fn components_note() -> String {
    "The React components this application owns. `npx shadcn add button`\n\
     writes into `components/ui/`; anything you write yourself goes here\n\
     beside it.\n\n\
     A page writes the file's name as a tag — `components/Editor.tsx` is\n\
     `<Editor />` — and the framework builds the index that makes that\n\
     true at every build. Nothing here has to be declared anywhere.\n\n\
     `tsconfig.json` compiles this directory and `components.json` calls\n\
     it `@/components`, so an import of `@/components/ui/button` resolves\n\
     for the shadcn command line, for esbuild and for an editor alike.\n"
        .to_string()
}

fn assets_note() -> String {
    "What the window needs and the pages use: an `icon.ico`, images, a\n\
     stylesheet. `Config::new(\"…\").icon(\"assets/icon.ico\")` names the\n\
     icon; nothing else here has to be declared.\n"
        .to_string()
}
