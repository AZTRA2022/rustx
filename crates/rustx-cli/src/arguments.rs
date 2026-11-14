//! What was typed.
//!
//! Five commands and a flag. A parser library would be more code to
//! read than this is, and the project already reads Rust tokens by
//! hand — a command line is not where it starts taking dependencies.
//!
//! What it will not do is ignore a word. Every argument this does not
//! understand comes back as [`Command::Misused`] with a sentence about
//! it, because the alternative is what was here before: `rustx build
//! --relese` built in debug and said nothing, `rustx run --release` ran
//! the debug binary and said nothing, and `rustx new --help` wrote a
//! project into a directory called `--help` that cargo would never
//! build.

/// What the developer asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    New {
        name: String,
    },
    Dev,
    Build {
        release: bool,
    },
    Run {
        release: bool,
    },
    Help,
    /// `rustx --version`: the one question every command line is asked
    /// and the only one whose answer a script reads.
    Version,
    /// Something that is none of the above, kept so it can be answered
    /// with the nearest thing that is.
    Unknown(String),
    /// A command that exists, given something it does not take. The
    /// sentence is already written, because the thing that knows what
    /// went wrong is the thing that read it.
    Misused(String),
}

/// Every command there is, in the order the help lists them.
pub const COMMANDS: &[&str] = &["new", "dev", "build", "run", "help", "version"];

/// Read the arguments, without the program's own name.
pub fn read(arguments: &[String]) -> Command {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();

    // Asking for the help is asking for the help, wherever it is typed.
    // `rustx new --help` is a question about `new`; it used to be a
    // project called `--help`.
    if words.iter().any(|word| matches!(*word, "--help" | "-h")) {
        return Command::Help;
    }
    if words.iter().any(|word| matches!(*word, "--version" | "-V")) {
        return Command::Version;
    }

    let Some((first, rest)) = words.split_first() else {
        return Command::Help;
    };
    match *first {
        "new" => a_name_in(rest),
        "dev" => nothing_after("dev", rest).unwrap_or(Command::Dev),
        "build" => with_a_profile("build", rest, |release| Command::Build { release }),
        "run" => with_a_profile("run", rest, |release| Command::Run { release }),
        "help" => Command::Help,
        "version" => Command::Version,
        other => Command::Unknown((*other).to_string()),
    }
}

/// `rustx new <name>`: one name, and nothing else at all.
fn a_name_in(rest: &[&str]) -> Command {
    let mut names = rest.iter().filter(|word| !word.starts_with('-'));
    let Some(name) = names.next() else {
        if let Some(flag) = rest.iter().find(|word| word.starts_with('-')) {
            return Command::Misused(format!(
                "`rustx new` takes a project name, and `{flag}` is an option — it \
                 would have become the name of the directory. `rustx new mon-app` \
                 writes a project called `mon-app` here."
            ));
        }
        // `rustx new` with nothing after it is a question, not a
        // mistake: the help says what goes there.
        return Command::Help;
    };
    if let Some(also) = names.next() {
        return Command::Misused(format!(
            "`rustx new` writes one project, and two names were given: `{name}` and \
             `{also}`. A crate name cannot hold a space, so a name in two words is \
             two names — `rustx new {name}-{also}`, perhaps."
        ));
    }
    if let Some(flag) = rest.iter().find(|word| word.starts_with('-')) {
        return Command::Misused(format!(
            "`rustx new` takes no options, and `{flag}` is one. The project is \
             written into the directory you are in, under the name you gave it: \
             `cd` first, then `rustx new {name}`."
        ));
    }
    Command::New {
        name: (*name).to_string(),
    }
}

/// `rustx build` and `rustx run`: `--release` or nothing.
fn with_a_profile(command: &str, rest: &[&str], make: impl Fn(bool) -> Command) -> Command {
    let mut release = false;
    for word in rest {
        match *word {
            "--release" => release = true,
            // The two every developer tries, because cargo takes them.
            "--debug" => {
                return Command::Misused(format!(
                    "`rustx {command}` builds in debug unless it is told `--release`, \
                     so there is nothing for `--debug` to do and it is not a flag."
                ));
            }
            other => return Command::Misused(about(command, other)),
        }
    }
    make(release)
}

/// `rustx dev`: the command, and that is all of it.
fn nothing_after(command: &str, rest: &[&str]) -> Option<Command> {
    let word = rest.first()?;
    if *word == "--release" {
        return Some(Command::Misused(
            "`rustx dev` always builds in debug: a release build of a browser engine \
             is not something to wait for between two keystrokes. `rustx build \
             --release` makes the build that ships."
                .to_string(),
        ));
    }
    Some(Command::Misused(about(command, word)))
}

/// What to say about a word a command does not take.
fn about(command: &str, word: &str) -> String {
    let takes = match command {
        "build" | "run" => "`--release`, and nothing else",
        _ => "nothing after it",
    };
    let mut said = format!("`rustx {command}` takes {takes}, and `{word}` was given.");
    if let Some(near) = nearest_flag(word) {
        said.push_str(&format!(" Did you mean `{near}`?"));
    }
    said.push_str(" `rustx --help` says what each command takes.");
    said
}

/// The command `typed` was most likely meant to be.
///
/// One letter wrong, two letters swapped: the two mistakes a keyboard
/// makes. Anything further apart is left alone, because a wrong
/// suggestion is worse than none.
pub fn nearest(typed: &str) -> Option<&'static str> {
    nearest_of(typed, COMMANDS, 2)
}

/// The same, for the one flag there is to mistype.
fn nearest_flag(typed: &str) -> Option<&'static str> {
    nearest_of(typed, &["--release"], 2)
}

fn nearest_of(typed: &str, known: &[&'static str], apart_at_most: usize) -> Option<&'static str> {
    known
        .iter()
        .map(|known| (distance(typed, known), *known))
        .filter(|(apart, _)| *apart <= apart_at_most)
        .min_by_key(|(apart, _)| *apart)
        .map(|(_, known)| known)
}

/// How many single-letter changes turn one word into the other.
fn distance(from: &str, to: &str) -> usize {
    let from: Vec<char> = from.chars().collect();
    let to: Vec<char> = to.chars().collect();
    let mut row: Vec<usize> = (0..=to.len()).collect();
    for (at, letter) in from.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = at + 1;
        for (to_at, other) in to.iter().enumerate() {
            let cost = usize::from(letter != other);
            let next = (row[to_at] + 1)
                .min(row[to_at + 1] + 1)
                .min(diagonal + cost);
            diagonal = row[to_at + 1];
            row[to_at + 1] = next;
        }
    }
    row[to.len()]
}
