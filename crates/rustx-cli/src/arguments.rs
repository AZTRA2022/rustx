//! What was typed.
//!
//! Four commands and a flag. A parser library would be more code to
//! read than this is, and the project already reads Rust tokens by
//! hand — a command line is not where it starts taking dependencies.

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
    Run,
    Help,
    /// Something that is none of the above, kept so it can be answered
    /// with the nearest thing that is.
    Unknown(String),
}

/// Read the arguments, without the program's own name.
pub fn read(arguments: &[String]) -> Command {
    let mut words = arguments.iter().map(String::as_str);
    let Some(first) = words.next() else {
        return Command::Help;
    };
    let rest: Vec<&str> = words.collect();

    match first {
        "new" => match rest.first() {
            Some(name) => Command::New {
                name: (*name).to_string(),
            },
            // `rustx new` with nothing after it is a question, not a
            // mistake: the help says what goes there.
            None => Command::Help,
        },
        "dev" => Command::Dev,
        "build" => Command::Build {
            release: rest.contains(&"--release"),
        },
        "run" => Command::Run,
        "help" | "--help" | "-h" => Command::Help,
        other => Command::Unknown(other.to_string()),
    }
}

/// The command `typed` was most likely meant to be.
///
/// One letter wrong, two letters swapped: the two mistakes a keyboard
/// makes. Anything further apart is left alone, because a wrong
/// suggestion is worse than none.
pub fn nearest(typed: &str) -> Option<&'static str> {
    ["new", "dev", "build", "run", "help"]
        .into_iter()
        .map(|known| (distance(typed, known), known))
        .filter(|(apart, _)| *apart <= 2)
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
