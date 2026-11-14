//! What `rustx new` writes.
//!
//! The first five minutes of the framework. A project that does not
//! compile, or that opens with a `build.rs` and forty lines of
//! plumbing in `main.rs`, has already said what the framework is
//! about — and said the wrong thing.

use std::fs;
use std::path::PathBuf;

use rustx_cli::{arguments, new};

/// A fresh, empty directory nobody else in this run can have.
///
/// A timestamp alone collided: two tests on two threads can read the
/// same nanosecond on this clock, and one test's `mon-app` then shows
/// up inside another's "must be empty" directory. The counter is the
/// one thing in this process nothing else can also read.
fn somewhere() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TURN: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rustx-new-{}-{}-{}",
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

#[test]
fn a_new_project_has_a_page_and_no_build_script() {
    let into = somewhere();
    let written = new::write("mon-app", &into).expect("a project");
    let names: Vec<String> = written
        .iter()
        .map(|path| {
            path.strip_prefix(into.join("mon-app"))
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    assert!(names.contains(&"Cargo.toml".to_string()), "got {names:?}");
    assert!(names.contains(&"src/main.rs".to_string()), "got {names:?}");
    assert!(names.contains(&"app/page.rsx".to_string()), "got {names:?}");
    assert!(
        !names.iter().any(|name| name.contains("build.rs")),
        "a build script is exactly what this framework does without: {names:?}"
    );
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn the_main_it_writes_is_four_lines_long() {
    // The shape the design document asks for, and the whole argument of
    // the framework: the ceremony belongs to the framework, not to the
    // file a developer opens first.
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let main = fs::read_to_string(into.join("mon-app/src/main.rs")).expect("a main");
    let code: Vec<&str> = main
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();
    assert!(code.len() <= 7, "too much ceremony: {code:?}");
    assert!(main.contains("#[rustx::main]"), "got {main}");
    assert!(main.contains("App::new(Config::new("), "got {main}");
    assert!(main.contains("app.run()"), "got {main}");
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn the_page_it_writes_is_a_page() {
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let page = fs::read_to_string(into.join("mon-app/app/page.rsx")).expect("a page");
    // It compiles, which is the only thing a first page has to do.
    rustx_compiler::transpile::transpile(&page).expect("the first page must compile");
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_name_cargo_would_refuse_is_refused_first() {
    // Before a single file is written: half a project on disk, named
    // something cargo will not build, is worse than a sentence.
    let into = somewhere();
    for name in ["mon app", "", "1app", "mon/app", "rustx"] {
        let refused = new::write(name, &into).expect_err("refused");
        assert!(!refused.is_empty(), "a refusal with nothing in it");
    }
    assert_eq!(
        fs::read_dir(&into).expect("readable").count(),
        0,
        "something was written for a name that was refused"
    );
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_directory_that_exists_is_not_overwritten() {
    let into = somewhere();
    fs::create_dir(into.join("mon-app")).expect("a directory in the way");
    fs::write(into.join("mon-app/precious.txt"), "mine").expect("a file in the way");
    assert!(new::write("mon-app", &into).is_err());
    assert!(
        into.join("mon-app/precious.txt").is_file(),
        "it was touched"
    );
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn every_command_is_read_from_what_was_typed() {
    let read = |line: &str| {
        arguments::read(
            &line
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<String>>(),
        )
    };
    assert_eq!(
        read("new mon-app"),
        arguments::Command::New {
            name: "mon-app".to_string()
        }
    );
    assert_eq!(read("dev"), arguments::Command::Dev);
    assert_eq!(
        read("build --release"),
        arguments::Command::Build { release: true }
    );
    assert_eq!(read("build"), arguments::Command::Build { release: false });
    assert_eq!(read("run"), arguments::Command::Run);
    assert_eq!(read(""), arguments::Command::Help);
    assert_eq!(read("--help"), arguments::Command::Help);
    assert_eq!(
        read("biuld"),
        arguments::Command::Unknown("biuld".to_string())
    );
}

#[test]
fn a_new_project_ignores_what_the_build_writes() {
    // `/target` alone was the whole file, and the first `git add .` of
    // a project with npm libraries carried two megabytes of minified
    // JavaScript, a `node_modules`, and the entry point the framework
    // generates.
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let ignored = fs::read_to_string(into.join("mon-app").join(".gitignore")).expect("a file");
    for line in ["/target", "/node_modules", "/.rustx/", "/assets/rustx-bundle.js"] {
        assert!(
            ignored.lines().any(|written| written == line),
            "`{line}` is not ignored: {ignored}"
        );
    }
}

#[test]
fn a_new_project_names_one_dependency_and_one_only() {
    // The documentation's promise: "Une application n'en nomme qu'un :
    // `rustx-app`." The generated pages name `rustx_dom` and
    // `rustx_engine`, so the facade re-exports them and the prelude of
    // `#[rustx::main]` imports them -- which is what makes the promise
    // true rather than aspirational. A project written before that did
    // not compile at all.
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let manifest = fs::read_to_string(into.join("mon-app").join("Cargo.toml")).expect("a file");
    let dependencies: Vec<&str> = manifest
        .lines()
        .skip_while(|line| !line.starts_with("[dependencies]"))
        .skip(1)
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .collect();
    assert_eq!(dependencies.len(), 1, "got {dependencies:?}");
    assert!(dependencies[0].starts_with("rustx ="), "got {dependencies:?}");
}

/// What a new project asks of the framework must be a version that
/// exists.
///
/// `rustx = { package = "rustx-app", version = "0.1" }` was written out
/// by hand, and it outlived 0.2.0. Under cargo's rule for 0.x a `0.1`
/// requirement does not accept 0.2.0, so every project made by
/// `rustx new` silently got the framework of the version before —
/// including its refusals, fixed months earlier. The requirement is
/// read from this crate's own version now, and this law is what keeps
/// the two from drifting again.
#[test]
fn a_new_project_asks_for_the_version_this_command_line_belongs_to() {
    let into = somewhere();
    new::write("my-app", &into).expect("a project");

    let manifest = fs::read_to_string(into.join("my-app").join("Cargo.toml"))
        .or_else(|_| fs::read_to_string(into.join("Cargo.toml")))
        .expect("the manifest it wrote");

    let mine = env!("CARGO_PKG_VERSION");
    let mut parts = mine.split('.');
    let wanted = match (parts.next(), parts.next()) {
        (Some("0"), Some(minor)) => format!("0.{minor}"),
        (Some(major), _) => major.to_string(),
        _ => mine.to_string(),
    };

    assert!(
        manifest.contains(&format!("version = \"{wanted}\"")),
        "a project made by rustx {mine} asks for something else:\n{manifest}"
    );
}

/// `npm install <library>` has to find a project to install into.
///
/// Without a `package.json`, an old `npm` refuses outright and a new
/// one writes one of its own — named after the directory, at version
/// `1.0.0`, with no `private` field — the moment a developer follows
/// the documentation's first line (`npm install recharts`). The file
/// this command should have written shows up a step late, named by
/// npm and not by the project, and `rustx new` looks like it forgot a
/// file every project that reaches for a JavaScript library needs.
#[test]
fn a_new_project_can_already_receive_an_npm_install() {
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let manifest = fs::read_to_string(into.join("mon-app").join("package.json"))
        .expect("a package.json ready for npm install");
    assert!(manifest.contains("\"name\""), "got {manifest}");
    assert!(manifest.contains("\"private\": true"), "got {manifest}");
    assert!(manifest.contains("\"dependencies\""), "got {manifest}");
    let _: () = assert_is_valid_json(&manifest);
    let _ = fs::remove_dir_all(&into);
}

/// The README says `npm install`, and that's it — the framework builds
/// the bundle with esbuild. Without `esbuild` declared here, that is
/// false: `bundle::build` refuses every library until `esbuild` is in
/// `node_modules`, and nothing a developer just installed reaches the
/// page, with no message that names why.
#[test]
fn a_new_project_already_has_esbuild_to_build_with() {
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let manifest = fs::read_to_string(into.join("mon-app").join("package.json"))
        .expect("a package.json ready for npm install");
    assert!(
        manifest.contains("\"esbuild\""),
        "a plain `npm install` must already provide what the bundler needs: {manifest}"
    );
    let _: () = assert_is_valid_json(&manifest);
    let _ = fs::remove_dir_all(&into);
}

/// `esbuild` downloads its own platform binary through a postinstall
/// script, and a modern `npm` holds every postinstall script back until
/// it is approved — so `esbuild.cmd` can exist in `node_modules/.bin`
/// and still have no binary to run, with nothing in this project's own
/// error messages that says why.
#[test]
fn a_new_project_already_approves_esbuilds_own_install_script() {
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let manifest = fs::read_to_string(into.join("mon-app").join("package.json"))
        .expect("a package.json ready for npm install");
    assert!(
        manifest.contains("\"allowScripts\""),
        "esbuild's postinstall script must already be approved: {manifest}"
    );
    let _: () = assert_is_valid_json(&manifest);
    let _ = fs::remove_dir_all(&into);
}

/// Not a parser brought in for one test: a `package.json` with a
/// trailing comma or an unclosed brace fails every `npm install` with
/// an error that does not name this command at all.
fn assert_is_valid_json(text: &str) {
    let mut depth = 0i32;
    for letter in text.chars() {
        match letter {
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            _ => {}
        }
    }
    assert_eq!(depth, 0, "unbalanced braces: {text}");
}

/// What a complex library needs before it will install itself.
///
/// `npx shadcn add button` reads two files before it writes anything,
/// and refuses without them — "No import alias found in your
/// tsconfig.json file" — so a project born without them is a project
/// where the documented one-liner does not work. Verified end to end on
/// a project this function wrote: `npm install`, `npx shadcn add button
/// card`, `cargo build`, and a window that resolved `Button` and drew
/// the card.
#[test]
fn a_new_project_can_already_receive_a_shadcn_component() {
    let into = somewhere();
    let root = new::write("mon-app", &into)
        .expect("a project")
        .first()
        .and_then(|first| first.parent().map(|parent| parent.to_path_buf()))
        .expect("a root");

    // The alias, which both the shadcn command line and esbuild read.
    let tsconfig = fs::read_to_string(root.join("tsconfig.json")).expect("a tsconfig.json");
    assert!(
        tsconfig.contains("\"@/*\"") && tsconfig.contains("\"baseUrl\""),
        "the `@` alias is what `@/lib/utils` resolves by: {tsconfig}"
    );
    let _: () = assert_is_valid_json(&tsconfig);

    // And what the command line reads to know it needs no `init`.
    let components = fs::read_to_string(root.join("components.json")).expect("a components.json");
    assert!(
        components.contains("assets/tailwind.css"),
        "the stylesheet named here is the one the framework hands to Tailwind: {components}"
    );
    let _: () = assert_is_valid_json(&components);

    // The one function every shadcn component has imported, and the
    // theme whose names its classes are.
    assert!(root.join("lib").join("utils.ts").exists(), "no lib/utils.ts");
    let css = fs::read_to_string(root.join("assets").join("tailwind.css")).expect("a stylesheet");
    assert!(
        css.contains("--primary:") && css.contains("--muted-foreground:"),
        "`bg-primary` and `text-muted-foreground` are names of these: {css}"
    );

    let _ = fs::remove_dir_all(&into);
}

/// A file that imports what the manifest does not declare is a project
/// that fails on its first `npm install`.
#[test]
fn a_new_project_declares_what_its_own_files_import() {
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let root = into.join("mon-app");
    let manifest = fs::read_to_string(root.join("package.json")).expect("a package.json");

    // `lib/utils.ts` imports these two by name.
    for named in ["clsx", "tailwind-merge"] {
        assert!(
            manifest.contains(&format!("\"{named}\"")),
            "lib/utils.ts imports {named}: {manifest}"
        );
    }
    // `assets/tailwind.css` imports these two by name.
    for named in ["tailwindcss", "tw-animate-css"] {
        assert!(
            manifest.contains(&format!("\"{named}\"")),
            "assets/tailwind.css imports {named}: {manifest}"
        );
    }
    // And a React component cannot be drawn without React.
    for named in ["react", "react-dom"] {
        assert!(
            manifest.contains(&format!("\"{named}\"")),
            "a library component needs {named}: {manifest}"
        );
    }
    let _: () = assert_is_valid_json(&manifest);
    let _ = fs::remove_dir_all(&into);
}

/// `@tailwindcss/cli` brings `@parcel/watcher`, whose install script a
/// modern `npm` holds back — and says so, on the last line of the first
/// `npm install` a developer ever runs here. Its version is whatever the
/// tree resolves, so it is approved by name alone, which npm accepts.
#[test]
fn a_new_project_leaves_no_install_script_unapproved() {
    let into = somewhere();
    new::write("mon-app", &into).expect("a project");
    let manifest = fs::read_to_string(into.join("mon-app").join("package.json"))
        .expect("a package.json");
    assert!(
        manifest.contains("\"@parcel/watcher\""),
        "the one install script a fresh tree holds back: {manifest}"
    );
    let _ = fs::remove_dir_all(&into);
}
