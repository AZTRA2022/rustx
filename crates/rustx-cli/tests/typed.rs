//! What a developer typed, and what the command line does with a word
//! it was not expecting.
//!
//! Every law here is a thing that used to happen silently. Run against
//! the version of these two files committed before them, the whole of
//! this file fails:
//!
//! - `rustx new --help` was a project called `--help`: the name check
//!   accepted it (every character is a letter, a digit, a `-` or a `_`,
//!   and it does not start with a digit), so a directory called
//!   `--help` was created, with a `Cargo.toml` naming a crate cargo
//!   would never build.
//! - `rustx build --relese` built in debug and said nothing, because an
//!   unknown word was tested for with `contains("--release")` and
//!   anything else simply was not there.
//! - `rustx run --release` ran the debug binary and said nothing, for
//!   the same reason: `run` read no flags at all.
//! - `rustx new foo bar` wrote `foo` and ignored `bar`.
//! - `rustx new test`, `rustx new crate`, `rustx new nul`,
//!   `rustx new rustx-os` and four more of the framework's own crate
//!   names were all accepted, and all of them make a project that does
//!   not build — or, on Windows, a directory that cannot be made.
//! - `rustx --version` was not a command.
//!
//! A silent wrong answer is the worst failure a command line has, and
//! these were six of them.

use std::fs;
use std::path::PathBuf;

use rustx_cli::{arguments, help, new};
use rustx_cli::arguments::Command;

fn read(line: &str) -> Command {
    arguments::read(
        &line
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<String>>(),
    )
}

fn somewhere(what: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TURN: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "rustx-typed-{what}-{}-{}-{}",
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
fn asking_for_the_help_is_asking_for_the_help_wherever_it_is_typed() {
    for line in [
        "--help",
        "-h",
        "help",
        "new --help",
        "new -h",
        "build --help",
        "dev --help",
        "run --help",
    ] {
        assert_eq!(read(line), Command::Help, "`rustx {line}` is a question");
    }
}

#[test]
fn the_version_is_a_question_every_command_line_is_asked() {
    for line in ["--version", "-V", "version"] {
        assert_eq!(read(line), Command::Version, "`rustx {line}`");
    }
}

#[test]
fn a_flag_that_was_mistyped_is_said_and_not_ignored() {
    // The whole defect: `--relese` built in debug, silently, and a
    // developer shipped a debug build of a browser engine.
    let said = match read("build --relese") {
        Command::Misused(said) => said,
        other => panic!("`rustx build --relese` built something: {other:?}"),
    };
    assert!(said.contains("--relese"), "the word is not named: {said}");
    assert!(
        said.contains("Did you mean `--release`?"),
        "one letter wrong is a suggestion, not a riddle: {said}"
    );
}

#[test]
fn an_argument_no_command_takes_is_said_and_not_ignored() {
    for line in [
        "build --target=x86_64-pc-windows-msvc",
        "build -j4",
        "run --quiet",
        "dev app/page.rsx",
        "dev --watch",
    ] {
        assert!(
            matches!(read(line), Command::Misused(_)),
            "`rustx {line}` was accepted, and whatever it did was not what was asked"
        );
    }
}

#[test]
fn the_release_flag_is_read_by_run_as_well_as_by_build() {
    // `rustx run --release` ran the debug build and printed nothing
    // about it. Cargo takes the flag; a developer types it.
    assert_eq!(read("build --release"), Command::Build { release: true });
    assert_eq!(read("build"), Command::Build { release: false });
    assert_eq!(read("run --release"), Command::Run { release: true });
    assert_eq!(read("run"), Command::Run { release: false });
}

#[test]
fn the_dev_loop_says_why_it_has_no_release() {
    let said = match read("dev --release") {
        Command::Misused(said) => said,
        other => panic!("{other:?}"),
    };
    assert!(
        said.contains("rustx build --release"),
        "a refusal that does not say what to do instead: {said}"
    );
}

#[test]
fn an_option_where_a_project_name_goes_is_not_a_project_name() {
    let said = match read("new --in ../elsewhere") {
        Command::Misused(said) => said,
        other => panic!("`rustx new --in …` made something: {other:?}"),
    };
    assert!(said.contains("--in"), "got {said}");
    assert!(
        said.contains("cd"),
        "there is no flag for where a project goes, and the refusal has to say what \
         there is instead: {said}"
    );
}

#[test]
fn two_names_are_two_names_and_not_one_project() {
    let said = match read("new mon app") {
        Command::Misused(said) => said,
        other => panic!("`rustx new mon app` wrote a project and dropped a word: {other:?}"),
    };
    assert!(said.contains("mon") && said.contains("app"), "got {said}");
}

#[test]
fn one_name_is_still_one_project() {
    assert_eq!(
        read("new mon-app"),
        Command::New {
            name: "mon-app".to_string()
        }
    );
}

#[test]
fn nothing_typed_is_a_question_not_a_mistake() {
    let nothing: Vec<String> = Vec::new();
    assert_eq!(arguments::read(&nothing), Command::Help);
    assert_eq!(
        read("new"),
        Command::Help,
        "`rustx new` with no name is somebody asking what goes there"
    );
}

#[test]
fn a_command_that_is_not_one_suggests_the_nearest() {
    assert_eq!(read("biuld"), Command::Unknown("biuld".to_string()));
    assert_eq!(arguments::nearest("biuld"), Some("build"));
    assert_eq!(arguments::nearest("ne"), Some("new"));
    assert_eq!(
        arguments::nearest("publish"),
        None,
        "a suggestion nobody meant is worse than none"
    );
}

/// Every name the old check accepted and cargo, npm or Windows then
/// refused — each with the reason it is refused here instead, one step
/// earlier and in words.
#[test]
fn a_name_that_cannot_work_is_refused_before_a_directory_is_made() {
    let into = somewhere("names");
    let impossible = [
        // An option typed where a name goes: the directory would have
        // been called `--help`.
        ("--help", "letter"),
        ("-h", "letter"),
        // npm refuses a package whose name starts with `_`, and cargo
        // wants a letter first too.
        ("_app", "letter"),
        // Cargo: "it conflicts with Rust's built-in crate names".
        ("test", "already has"),
        ("std", "already has"),
        ("proc_macro", "already has"),
        // A crate is a module, and a module's name is an identifier.
        ("crate", "keyword"),
        ("self", "keyword"),
        ("async", "keyword"),
        // Windows device names: `mkdir nul` does not make a directory.
        ("nul", "device name"),
        ("con", "device name"),
        ("com1", "device name"),
        // Every one of the framework's own crates, by package name and
        // by directory name.
        ("rustx", "framework's own name"),
        ("rustx-app", "framework's own name"),
        ("rustx-os", "framework's own name"),
        ("rustx-cli", "framework's own name"),
        ("rustx-dom", "framework's own name"),
        ("rustx-lsp", "framework's own name"),
        ("rustx-engine", "framework's own name"),
        ("rustx-macro", "framework's own name"),
        ("rustx-macros", "framework's own name"),
        ("rustx-compiler", "framework's own name"),
        ("rustx_dom", "framework's own name"),
        ("RustX-OS", "framework's own name"),
    ];
    for (name, because) in impossible {
        let refused = new::write(name, &into)
            .map(|written| written.len())
            .expect_err(&format!("`rustx new {name}` wrote a project that cannot build"));
        assert!(
            refused.contains(because),
            "`{name}` is refused, and the sentence does not say why ({because}): {refused}"
        );
    }
    assert_eq!(
        fs::read_dir(&into).expect("readable").count(),
        0,
        "something was written for a name that was refused"
    );
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_name_that_is_merely_long_is_refused_by_length_and_not_by_the_filesystem() {
    let into = somewhere("long");
    let name = "a".repeat(65);
    let refused = new::write(&name, &into).expect_err("refused");
    assert!(refused.contains("64"), "got {refused}");
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn the_names_that_do_work_still_work() {
    // The refusals above are a list, and a list is a thing that grows
    // too far: these have to stay possible.
    let into = somewhere("fine");
    for name in [
        "mon-app",
        "mon_app",
        "app",
        "rustx-app-of-mine",
        "testing",
        "Counter",
        "a",
        "crates",
        "selfie",
        "console",
    ] {
        new::write(name, &into)
            .unwrap_or_else(|why| panic!("`rustx new {name}` is a reasonable thing to type: {why}"));
    }
    let _ = fs::remove_dir_all(&into);
}

/// A flag the command line takes and the help does not mention is a
/// feature nobody can find; a flag the help promises and the command
/// line refuses is worse. Both have happened: `--release` was read by
/// `build` and named nowhere.
#[test]
fn the_help_names_every_command_and_every_flag_there_is() {
    let said = help::text();
    for command in arguments::COMMANDS {
        // `help` and `version` answer to a word and to a flag both, and
        // the help shows whichever form a developer is likelier to
        // type. Naming it one way is naming it.
        assert!(
            said.contains(&format!("rustx {command}"))
                || said.contains(&format!("rustx --{command}")),
            "`rustx {command}` is a command and the help does not name it:\n{said}"
        );
    }
    for flag in ["--release", "--version", "--help"] {
        assert!(
            said.contains(flag),
            "`{flag}` is accepted and the help does not name it:\n{said}"
        );
    }
    // And the other way: every flag the help shows is a flag that is
    // read, tested by handing each of them back to the parser.
    assert_eq!(read("build --release"), Command::Build { release: true });
    assert_eq!(read("run --release"), Command::Run { release: true });
    assert_eq!(read("--version"), Command::Version);
    assert_eq!(read("--help"), Command::Help);
}

#[test]
fn the_help_still_says_what_a_reload_keeps() {
    let said = help::text().to_lowercase();
    assert!(
        said.contains("does not reload native code"),
        "the limit is not said:\n{said}"
    );
    assert!(
        said.contains("shared state") && said.contains("page that was open"),
        "what is kept is not said:\n{said}"
    );
    assert!(
        said.contains("cells inside a component"),
        "what is not kept is not said:\n{said}"
    );
}

/// The one thing on this machine the command line cannot do without,
/// said as a sentence rather than as `program not found`.
#[test]
fn a_machine_without_cargo_is_told_what_is_missing_and_where_to_get_it() {
    let said = rustx_cli::build::says_no_cargo(&std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "program not found",
    ));
    assert!(said.contains("PATH"), "got {said}");
    assert!(said.contains("rustup.rs"), "got {said}");
    assert!(
        said.contains("cargo --version"),
        "the one command that answers the question is not given: {said}"
    );
}
