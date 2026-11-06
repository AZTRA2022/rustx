//! What `rustx new` writes.
//!
//! The first five minutes of the framework. A project that does not
//! compile, or that opens with a `build.rs` and forty lines of
//! plumbing in `main.rs`, has already said what the framework is
//! about — and said the wrong thing.

use std::fs;
use std::path::PathBuf;

use rustx_cli::{arguments, new};

fn somewhere() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rustx-new-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
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
