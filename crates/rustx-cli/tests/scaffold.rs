//! `rustx new` is consistent with itself.
//!
//! A test that asserts a list of file names stops being worth anything
//! the day the scaffold changes, and the scaffold changes — it grew
//! five files when the framework learned to take npm libraries. So
//! these laws are about agreement rather than content: every file it
//! says it wrote is on disk, every import it makes is declared, every
//! path it names is a path it created, and the entry point the
//! framework generates for its dependencies imports all of them.
//!
//! Three of the laws read their expectation out of `rustx-compiler`
//! rather than writing it down — the generated bundle's own name, the
//! directories Tailwind is told to scan, the stylesheet the framework
//! hands to Tailwind — so the scaffold and the bundler cannot drift
//! apart without something here failing. That is the one kind of
//! JavaScript-interoperability test this machine can run: there is no
//! network, so nothing may depend on `npm install` succeeding.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rustx_compiler::bundle;
use rustx_cli::new;

// ---------------------------------------------------------------- setup

fn somewhere(what: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TURN: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rustx-scaffold-{what}-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0),
        TURN.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("a directory to write into");
    path
}

/// A project, and the files it said it wrote.
struct Scaffold {
    into: PathBuf,
    root: PathBuf,
    written: Vec<PathBuf>,
}

impl Scaffold {
    fn new(what: &str) -> Self {
        let into = somewhere(what);
        let written = new::write("mon-app", &into).expect("a project");
        let root = into.join("mon-app");
        Self { into, root, written }
    }

    fn read(&self, relative: &str) -> String {
        let path = self.root.join(relative);
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} could not be read: {error}", path.display()))
    }

    fn json(&self, relative: &str) -> Json {
        parse(&self.read(relative))
            .unwrap_or_else(|why| panic!("{relative} is not valid JSON: {why}"))
    }
}

impl Drop for Scaffold {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.into);
    }
}

/// Every file under `at`, by its path relative to it, with `/` for a
/// separator whatever the system writes.
fn files_under(at: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut left = vec![at.to_path_buf()];
    while let Some(directory) = left.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                left.push(path);
            } else if let Ok(relative) = path.strip_prefix(at) {
                found.insert(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found
}

// ------------------------------------------------------------ the laws

/// No file written and not named, and none named and not written.
///
/// The count is printed to the developer — `mon-app is ready: 11
/// files.` — so a file that is written and not reported is a lie, and a
/// file that is reported and not written is one too.
#[test]
fn every_file_it_says_it_wrote_is_on_disk_and_nothing_else_is() {
    let project = Scaffold::new("reported");

    let reported: BTreeSet<String> = project
        .written
        .iter()
        .map(|path| {
            path.strip_prefix(&project.root)
                .expect("every file it writes is inside the project")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();

    assert_eq!(
        reported,
        files_under(&project.root),
        "what `rustx new` says it wrote and what is on disk are not the same set"
    );
}

/// A `package.json` with a trailing comma fails every `npm install`
/// with an error that does not name this command at all.
#[test]
fn every_json_file_it_writes_parses() {
    let project = Scaffold::new("json");
    for named in ["package.json", "tsconfig.json", "components.json"] {
        let text = project.read(named);
        parse(&text).unwrap_or_else(|why| panic!("{named} is not valid JSON ({why}):\n{text}"));
    }
}

/// And the parser above is a parser, not a brace counter.
///
/// The check this replaces counted `{` against `}`, so a trailing
/// comma, an unquoted key and a missing colon all passed it — which is
/// to say it would not have caught any of the three mistakes anybody
/// actually makes in a hand-written JSON file. A law is worth what its
/// instrument can see.
#[test]
fn the_parser_this_test_uses_refuses_what_npm_refuses() {
    for wrong in [
        "{\"a\": 1,}",
        "{\"a\": [1, 2,]}",
        "{a: 1}",
        "{\"a\" 1}",
        "{\"a\": 1} {",
        "{\"a\": }",
        "{\"a\": 'b'}",
        "{\"a\": 01}",
    ] {
        assert!(
            parse(wrong).is_err(),
            "`{wrong}` is not JSON and the parser took it"
        );
    }
    assert!(parse("{\"a\": [1, {\"b\": null}], \"c\": -1.5e3}").is_ok());
}

/// A file that imports what the manifest does not declare is a project
/// that fails on its first `npm install` — or worse, installs and then
/// cannot bundle, which is the failure that says nothing useful.
#[test]
fn every_library_its_own_files_import_is_one_it_declares() {
    let project = Scaffold::new("imports");
    let manifest = project.json("package.json");
    let declared: BTreeSet<String> = ["dependencies", "devDependencies"]
        .iter()
        .flat_map(|table| manifest.keys_of(table))
        .collect();

    let mut imported: BTreeSet<String> = BTreeSet::new();
    for file in &project.written {
        let Some(kind) = file.extension().and_then(|kind| kind.to_str()) else {
            continue;
        };
        if !matches!(kind, "ts" | "tsx" | "js" | "jsx" | "css") {
            continue;
        }
        let text = fs::read_to_string(file).expect("a file it just wrote");
        for specifier in specifiers_in(&text) {
            // A path is not a package: `./x`, `../x` and `@/x` resolve
            // on disk, and the law about those is the next one.
            if specifier.starts_with('.') || specifier.starts_with("@/") {
                continue;
            }
            imported.insert(package_of(&specifier));
        }
    }

    assert!(
        !imported.is_empty(),
        "no imports were found at all, so this law proved nothing"
    );
    for package in &imported {
        assert!(
            declared.contains(package),
            "a file `rustx new` writes imports `{package}`, and the `package.json` it \
             writes does not declare it. Imported: {imported:?}. Declared: {declared:?}"
        );
    }
}

/// Nothing it writes may refer to a path it did not create.
///
/// This is the law that found the missing `components/`:
/// `tsconfig.json` compiled it, `components.json` aliased it as
/// `@/components`, and the bundler walked it looking for what to
/// re-export — and no file in a fresh project had ever made it.
#[test]
fn every_path_its_own_files_name_is_a_path_it_created() {
    let project = Scaffold::new("paths");

    // `tsconfig.json`: what it compiles, and what the alias points at.
    let tsconfig = project.json("tsconfig.json");
    let options = tsconfig.at("compilerOptions").expect("compilerOptions");
    for named in tsconfig.list_of("include") {
        assert!(
            project.root.join(&named).exists(),
            "tsconfig.json compiles `{named}`, which `rustx new` did not create"
        );
    }
    let base = options.text_at("baseUrl").expect("a baseUrl");
    assert!(
        project.root.join(&base).is_dir(),
        "the baseUrl every alias resolves against is `{base}`, which is not there"
    );

    // `components.json`: the one stylesheet the framework hands to
    // Tailwind, named by the shadcn command line.
    let components = project.json("components.json");
    let tailwind = components.at("tailwind").expect("a tailwind table");
    let css = tailwind.text_at("css").expect("a css path");
    assert!(
        project.root.join(&css).is_file(),
        "components.json names `{css}` as the stylesheet, and it is not there"
    );
}

/// The `@` alias is not a package: it is a path, declared in
/// `tsconfig.json` and nowhere else, and it is what every component
/// `npx shadcn add` writes imports `cn` through.
#[test]
fn the_alias_every_installed_component_imports_resolves_to_a_file() {
    let project = Scaffold::new("alias");
    let tsconfig = project.json("tsconfig.json");
    let options = tsconfig.at("compilerOptions").expect("compilerOptions");
    let base = options.text_at("baseUrl").expect("a baseUrl");
    let paths = options.at("paths").expect("a paths table");

    let patterns = paths.list_of("@/*");
    assert!(
        !patterns.is_empty(),
        "`@/*` is what `@/lib/utils` resolves by, and it is not declared"
    );

    // `@/lib/utils`, resolved the way a bundler resolves it: the `*` of
    // the pattern stands for what the `*` of the alias matched.
    let mut resolved = false;
    for pattern in &patterns {
        let candidate = project
            .root
            .join(&base)
            .join(pattern.replace('*', "lib/utils").trim_start_matches("./"));
        for end in ["ts", "tsx", "js", "jsx"] {
            if candidate.with_extension(end).is_file() {
                resolved = true;
            }
        }
    }
    assert!(
        resolved,
        "`@/lib/utils` is the one import every shadcn component makes, and it \
         resolves to nothing: patterns {patterns:?}, baseUrl `{base}`"
    );
}

/// The first page has one thing to do, and it is to compile.
#[test]
fn the_page_it_writes_compiles() {
    let project = Scaffold::new("page");
    let page = project.read("app/page.rsx");
    rustx_compiler::transpile::transpile(&page).expect("the first page must compile");
}

/// The manifest it writes is the manifest the build reads back.
#[test]
fn the_manifest_it_writes_is_the_one_the_build_reads() {
    let project = Scaffold::new("manifest");
    let read = rustx_cli::build::read(&project.root).expect("a readable project");
    assert_eq!(read.binary, "mon-app", "the binary cargo will write");
    assert_eq!(read.about.version, "0.1.0");
    assert_eq!(
        read.about.name, "Mon app",
        "the window's title is read out of the `Config` in main.rs"
    );
}

/// Everything the build writes inside the project is ignored by git,
/// and the names are the framework's own rather than copies of them.
///
/// A project whose first `git add .` carried two megabytes of minified
/// JavaScript learned this; the names being read from
/// `rustx-compiler` is what keeps it learned when one of them changes.
#[test]
fn the_gitignore_covers_what_the_build_writes_by_the_names_it_writes_it_under() {
    let project = Scaffold::new("ignored");
    let ignored: Vec<String> = project
        .read(".gitignore")
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    let has = |what: &str| {
        ignored
            .iter()
            .any(|line| line.trim_matches('/') == what.trim_matches('/'))
    };

    assert!(has("target"), "cargo's own output: {ignored:?}");
    assert!(has("node_modules"), "npm's own output: {ignored:?}");
    assert!(
        has(bundle::DIRECTORY),
        "`{}` is where the framework writes the entry point and the bundle: {ignored:?}",
        bundle::DIRECTORY
    );
    assert!(
        has(&format!("assets/{}", bundle::SERVED)),
        "`assets/{}` is the copy of the bundle the document loads: {ignored:?}",
        bundle::SERVED
    );
}

/// Every directory Tailwind is told to read is a directory the project
/// has.
///
/// The globs are the compiler's, not this test's: `bundle::sources` is
/// the one list both halves of the framework read, and a glob whose
/// root does not exist is a class name nobody will ever find — silently,
/// because an empty scan is not an error to Tailwind.
#[test]
fn every_directory_the_bundler_scans_is_one_the_scaffold_made() {
    let project = Scaffold::new("globs");
    let globs = bundle::content_globs();
    assert!(!globs.is_empty(), "there are no globs, so this proved nothing");

    for glob in at_top_level(&globs) {
        let root = glob
            .as_str()
            .trim()
            .trim_start_matches("./")
            .split(['/', '*'])
            .find(|part| !part.is_empty())
            .expect("a glob has a first directory");
        assert!(
            project.root.join(root).is_dir(),
            "`{glob}` is scanned for class names and `{root}/` is not in a fresh \
             project, so nothing written there would ever be seen"
        );
    }
}

/// The stylesheet the scaffold writes is the one the framework imports.
///
/// Two files name it: `components.json`, so the shadcn command line
/// writes its theme into the right place, and the generated Tailwind
/// input, which `@import`s it. If those two ever name different files,
/// a developer's theme is compiled by nobody and every shadcn component
/// has no colours at all.
#[test]
fn the_stylesheet_the_scaffold_writes_is_the_one_the_framework_compiles() {
    let project = Scaffold::new("stylesheet");
    let named = project
        .json("components.json")
        .at("tailwind")
        .expect("a tailwind table")
        .text_at("css")
        .expect("a css path");

    let generated = bundle::input_for(bundle::Dialect::Source, true)
        .expect("Tailwind 4 is handed a generated stylesheet");
    let file = named
        .rsplit('/')
        .next()
        .expect("the stylesheet has a file name");
    assert!(
        generated.contains(file),
        "components.json names `{named}` and the generated input imports something \
         else:\n{generated}"
    );
    assert!(project.root.join(&named).is_file());
}

/// Every library the scaffold declares reaches the page, under the name
/// a page would write for it.
///
/// This is the interoperability law, run end to end over the real
/// `package.json` that `rustx new` writes and the real entry point the
/// compiler generates for it — no `npm install`, no network, and
/// nothing hard-coded on either side. A library declared and absent
/// from `window.rustxLibraries` is the failure that surfaces five
/// seconds later as *the page asked for 'X', and nothing has it*.
#[test]
fn the_entry_point_exposes_every_library_the_scaffold_declares() {
    let project = Scaffold::new("entry");
    let manifest = project.read("package.json");

    let declared = bundle::dependencies_in(&manifest);
    assert!(
        !declared.is_empty(),
        "the scaffold declares no dependencies, so this law proved nothing"
    );

    let entry = bundle::entry_for(&declared);
    for package in &declared {
        let name = bundle::name_for(package);
        assert!(
            entry.contains(&format!("\"{package}\"")),
            "`{package}` is declared and the entry point does not import it:\n{entry}"
        );
        assert!(
            entry.contains(&format!("  {name},")),
            "`{package}` is imported and not handed to the page as `{name}`:\n{entry}"
        );
    }
    // React is in the object whether or not it is declared, because the
    // boundary cannot draw a component without it.
    for name in ["react", "reactDom", "reactDomClient"] {
        assert!(
            entry.contains(&format!("  {name},")),
            "`{name}` is what a React boundary is drawn with:\n{entry}"
        );
    }
}

/// And no two of them come to the same name.
///
/// Two packages whose identifiers collide would be written over each
/// other in one object literal — legal JavaScript, invisible, and a
/// page gets the components of a library it did not mean.
#[test]
fn no_two_libraries_the_scaffold_declares_collide() {
    let project = Scaffold::new("collision");
    let declared = bundle::dependencies_in(&project.read("package.json"));
    assert_eq!(
        bundle::a_collision_in(&declared),
        None,
        "two of the scaffold's own dependencies come to the same name"
    );
}

/// The build's tools are not libraries of the page.
///
/// Everything under `devDependencies` is a tool — esbuild, Tailwind's
/// command line, TypeScript, the React type declarations — and a bundle
/// that imported one would ask a browser to run a compiler.
#[test]
fn nothing_the_scaffold_installs_as_a_tool_reaches_the_page() {
    let project = Scaffold::new("tools");
    let manifest = project.json("package.json");
    let entry = bundle::entry_for(&bundle::dependencies_in(&project.read("package.json")));

    for tool in manifest.keys_of("devDependencies") {
        assert!(
            !entry.contains(&format!("\"{tool}\"")),
            "`{tool}` is a tool of the build and the entry point imports it:\n{entry}"
        );
    }
    // And the manifest says which tools it is: a `devDependencies` that
    // is empty would make the law above vacuous.
    assert!(
        manifest.keys_of("devDependencies").len() >= 3,
        "the scaffold declares almost no tools, so this law proved nothing"
    );
}

/// The globs of a comma-separated list, split on the commas that
/// separate globs and not on the commas inside one.
///
/// `./components/**/*.{ts,tsx,js,jsx}` holds three commas of its own.
/// Splitting naively gave `tsx`, `js` and `jsx}` as directories to look
/// for, which is how this test found that the list `bundle::content_globs`
/// hands to Tailwind 3's `--content` flag cannot be read by it either —
/// see `docs/stability/cli-and-distribution.md`, *Found elsewhere*.
fn at_top_level(list: &str) -> Vec<String> {
    let mut globs = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for letter in list.chars() {
        match letter {
            '{' => {
                depth += 1;
                current.push(letter);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.push(letter);
            }
            ',' if depth == 0 => {
                globs.push(std::mem::take(&mut current));
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        globs.push(current);
    }
    globs
}

// ----------------------------------------------------- reading imports

/// Every module specifier in a TypeScript or CSS file.
///
/// `import … from "x"`, `import "x"`, `require("x")` and `@import "x"`
/// — the four shapes the files this command writes actually use.
fn specifiers_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (marker, quoted) in [
        ("from ", true),
        ("import(", false),
        ("require(", false),
        ("@import ", true),
        ("import ", true),
    ] {
        let mut rest = text;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at + marker.len()..];
            let looking = rest.trim_start();
            let looking = if quoted {
                looking
            } else {
                looking.trim_start_matches(['(', ' '])
            };
            let Some(opening) = looking.chars().next() else {
                continue;
            };
            if opening != '"' && opening != '\'' {
                continue;
            }
            let inside = &looking[1..];
            if let Some(end) = inside.find(opening) {
                let specifier = inside[..end].trim().to_string();
                if !specifier.is_empty() {
                    found.push(specifier);
                }
            }
        }
    }
    found
}

/// The package a specifier names, without the sub-path:
/// `react-dom/client` is `react-dom`, `@radix-ui/react-dialog/x` is
/// `@radix-ui/react-dialog`.
fn package_of(specifier: &str) -> String {
    let parts: Vec<&str> = specifier.split('/').collect();
    if specifier.starts_with('@') && parts.len() >= 2 {
        format!("{}/{}", parts[0], parts[1])
    } else {
        parts[0].to_string()
    }
}

// ------------------------------------------------------- a JSON parser
//
// Written here rather than taken as a dependency, and written strictly:
// the check this replaces counted braces, so a trailing comma passed it
// — and a trailing comma is the one mistake a hand-written JSON file
// actually has. `rustx-cli` has one dependency and a command line is
// not where it starts taking more, so the test carries the parser.

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
    List(Vec<Json>),
    Map(Vec<(String, Json)>),
}

impl Json {
    fn at(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Map(pairs) => pairs.iter().find(|(named, _)| named == key).map(|(_, it)| it),
            _ => None,
        }
    }

    fn text_at(&self, key: &str) -> Option<String> {
        match self.at(key)? {
            Json::Text(text) => Some(text.clone()),
            _ => None,
        }
    }

    /// The keys of the object at `key`, or nothing when there is none.
    fn keys_of(&self, key: &str) -> Vec<String> {
        match self.at(key) {
            Some(Json::Map(pairs)) => pairs.iter().map(|(named, _)| named.clone()).collect(),
            _ => Vec::new(),
        }
    }

    /// The strings of the list at `key` — or the one string, since
    /// `include` and `paths` both take either shape.
    fn list_of(&self, key: &str) -> Vec<String> {
        match self.at(key) {
            Some(Json::List(items)) => items
                .iter()
                .filter_map(|item| match item {
                    Json::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .collect(),
            Some(Json::Text(text)) => vec![text.clone()],
            _ => Vec::new(),
        }
    }
}

fn parse(text: &str) -> Result<Json, String> {
    let bytes: Vec<char> = text.chars().collect();
    let mut at = 0usize;
    let value = value(&bytes, &mut at)?;
    skip_blank(&bytes, &mut at);
    if at != bytes.len() {
        return Err(format!("something after the end, at {at}"));
    }
    Ok(value)
}

fn skip_blank(text: &[char], at: &mut usize) {
    while *at < text.len() && text[*at].is_whitespace() {
        *at += 1;
    }
}

fn value(text: &[char], at: &mut usize) -> Result<Json, String> {
    skip_blank(text, at);
    match text.get(*at) {
        None => Err("nothing where a value goes".to_string()),
        Some('{') => object(text, at),
        Some('[') => list(text, at),
        Some('"') => text_value(text, at).map(Json::Text),
        Some('t') => word(text, at, "true").map(|()| Json::Bool(true)),
        Some('f') => word(text, at, "false").map(|()| Json::Bool(false)),
        Some('n') => word(text, at, "null").map(|()| Json::Null),
        Some(letter) if *letter == '-' || letter.is_ascii_digit() => number(text, at),
        Some(letter) => Err(format!("`{letter}` where a value goes, at {at}")),
    }
}

fn object(text: &[char], at: &mut usize) -> Result<Json, String> {
    *at += 1; // the `{`
    let mut pairs = Vec::new();
    skip_blank(text, at);
    if text.get(*at) == Some(&'}') {
        *at += 1;
        return Ok(Json::Map(pairs));
    }
    loop {
        skip_blank(text, at);
        // A key is a quoted string, and nothing else: `{a: 1}` is
        // JavaScript, not JSON, and npm refuses it.
        let key = text_value(text, at)?;
        skip_blank(text, at);
        if text.get(*at) != Some(&':') {
            return Err(format!("no `:` after the key `{key}`, at {at}"));
        }
        *at += 1;
        pairs.push((key, value(text, at)?));
        skip_blank(text, at);
        match text.get(*at) {
            Some(',') => {
                *at += 1;
                skip_blank(text, at);
                // A trailing comma: the mistake this parser exists for.
                if text.get(*at) == Some(&'}') {
                    return Err(format!("a comma before the `}}`, at {at}"));
                }
            }
            Some('}') => {
                *at += 1;
                return Ok(Json::Map(pairs));
            }
            _ => return Err(format!("no `,` and no `}}` after a value, at {at}")),
        }
    }
}

fn list(text: &[char], at: &mut usize) -> Result<Json, String> {
    *at += 1; // the `[`
    let mut items = Vec::new();
    skip_blank(text, at);
    if text.get(*at) == Some(&']') {
        *at += 1;
        return Ok(Json::List(items));
    }
    loop {
        items.push(value(text, at)?);
        skip_blank(text, at);
        match text.get(*at) {
            Some(',') => {
                *at += 1;
                skip_blank(text, at);
                if text.get(*at) == Some(&']') {
                    return Err(format!("a comma before the `]`, at {at}"));
                }
            }
            Some(']') => {
                *at += 1;
                return Ok(Json::List(items));
            }
            _ => return Err(format!("no `,` and no `]` after an item, at {at}")),
        }
    }
}

fn text_value(text: &[char], at: &mut usize) -> Result<String, String> {
    if text.get(*at) != Some(&'"') {
        return Err(format!("no string where one goes, at {at}"));
    }
    *at += 1;
    let mut out = String::new();
    while let Some(letter) = text.get(*at) {
        *at += 1;
        match letter {
            '"' => return Ok(out),
            '\\' => {
                let Some(escaped) = text.get(*at) else {
                    return Err("a string that ends on a backslash".to_string());
                };
                *at += 1;
                match escaped {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'u' => {
                        let mut number = 0u32;
                        for _ in 0..4 {
                            let Some(digit) = text.get(*at).and_then(|it| it.to_digit(16)) else {
                                return Err(format!("a short \\u escape, at {at}"));
                            };
                            *at += 1;
                            number = number * 16 + digit;
                        }
                        out.push(char::from_u32(number).unwrap_or('\u{fffd}'));
                    }
                    '"' | '\\' | '/' => out.push(*escaped),
                    other => return Err(format!("`\\{other}` is not an escape")),
                }
            }
            other => out.push(*other),
        }
    }
    Err("a string that never closes".to_string())
}

fn number(text: &[char], at: &mut usize) -> Result<Json, String> {
    let start = *at;
    if text.get(*at) == Some(&'-') {
        *at += 1;
    }
    // A leading zero is not followed by another digit: `01` is not JSON.
    if text.get(*at) == Some(&'0') {
        *at += 1;
        if text.get(*at).is_some_and(char::is_ascii_digit) {
            return Err(format!("a number with a leading zero, at {start}"));
        }
    } else {
        let mut digits = 0;
        while text.get(*at).is_some_and(char::is_ascii_digit) {
            *at += 1;
            digits += 1;
        }
        if digits == 0 {
            return Err(format!("no digits where a number goes, at {start}"));
        }
    }
    if text.get(*at) == Some(&'.') {
        *at += 1;
        let mut digits = 0;
        while text.get(*at).is_some_and(char::is_ascii_digit) {
            *at += 1;
            digits += 1;
        }
        if digits == 0 {
            return Err(format!("a point with no digits after it, at {start}"));
        }
    }
    if matches!(text.get(*at), Some('e') | Some('E')) {
        *at += 1;
        if matches!(text.get(*at), Some('+') | Some('-')) {
            *at += 1;
        }
        let mut digits = 0;
        while text.get(*at).is_some_and(char::is_ascii_digit) {
            *at += 1;
            digits += 1;
        }
        if digits == 0 {
            return Err(format!("an exponent with no digits, at {start}"));
        }
    }
    let written: String = text[start..*at].iter().collect();
    written
        .parse::<f64>()
        .map(Json::Number)
        .map_err(|error| format!("`{written}` is not a number: {error}"))
}

fn word(text: &[char], at: &mut usize, expected: &str) -> Result<(), String> {
    for letter in expected.chars() {
        if text.get(*at) != Some(&letter) {
            return Err(format!("`{expected}` was expected, at {at}"));
        }
        *at += 1;
    }
    Ok(())
}
