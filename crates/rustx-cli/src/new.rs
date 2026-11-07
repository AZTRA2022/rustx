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
fn refuse_a_bad_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("A project needs a name: `rustx new mon-app`.".to_string());
    }
    if name.starts_with(|letter: char| letter.is_ascii_digit()) {
        return Err(format!(
            "`{name}` starts with a digit, and a Rust crate cannot. Try `app-{name}`."
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
    // The framework's own names. A project called `rustx` cannot depend
    // on `rustx`, and the error cargo gives for that is not one anybody
    // should have to read.
    if matches!(name, "rustx" | "rustx-dom" | "rustx-engine" | "rustx-cli") {
        return Err(format!(
            "`{name}` is the framework's own name, and a project cannot share it — it would \
             have to depend on itself."
        ));
    }
    Ok(())
}

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
     \x20       <h1 style={{ margin: \"0\", fontSize: \"34px\" }}>Bonjour.</h1>\n\
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
     \x20           Cliqué {clicks} fois\n\
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

fn assets_note() -> String {
    "What the window needs and the pages use: an `icon.ico`, images, a\n\
     stylesheet. `Config::new(\"…\").icon(\"assets/icon.ico\")` names the\n\
     icon; nothing else here has to be declared.\n"
        .to_string()
}
