//! Where the cursor is.
//!
//! One read of the text, from its first byte to the cursor, which says
//! which half of the server answers: the markup's own lists, or
//! `rust-analyzer` through the generated file. Everything downstream
//! depends on getting this right, and getting it right on a file that
//! does not compile — which is what a `.rsx` is while somebody is
//! typing in it.
//!
//! It is not the compiler's parser. The parser reads a finished tag and
//! refuses what is not one; this reads an unfinished one and says what
//! it is in the middle of. A `<di` with no `>` is the whole point.

/// What the cursor is in the middle of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The developer's own Rust: a `let`, a signature, a function body
    /// outside any markup. `rust-analyzer` answers, through the map.
    Rust,
    /// A tag name being written: `<di`.
    TagName { typed: String },
    /// A closing tag being written: `</`, with the tag it would close.
    ClosingTag { open: String },
    /// An attribute name being written, the tag it is on, and what is
    /// already written on that tag — so that nothing is offered twice.
    AttributeName {
        tag: String,
        typed: String,
        already: Vec<String>,
    },
    /// After the `=` of an attribute.
    AttributeValue { tag: String, attribute: String },
    /// A key of a `style={{ … }}` object.
    StyleKey { typed: String },
    /// A Rust expression inside the markup, with where it starts: the
    /// byte offset of the first character after the `{`.
    Expression { from: usize },
}

/// Read the text up to `line`:`column` (both one-based) and say where
/// that is.
pub fn at(source: &str, line: usize, column: usize) -> Place {
    let Some(cursor) = offset_of(source, line, column) else {
        return Place::Rust;
    };
    read(&source[..cursor])
}

/// The byte offset of a one-based line and column.
///
/// Columns are counted in characters, which is what an editor sends, and
/// a `.rsx` is full of accented text — counting bytes would land in the
/// middle of a letter.
pub fn offset_of(source: &str, line: usize, column: usize) -> Option<usize> {
    if line == 0 || column == 0 {
        return None;
    }
    let mut at = 0;
    for (number, text) in source.split_inclusive('\n').enumerate() {
        if number + 1 == line {
            let mut letters = text.char_indices();
            for _ in 1..column {
                match letters.next() {
                    // The column is past the end of the line: the end of
                    // the line is where the cursor is.
                    None => return Some(at + text.trim_end_matches('\n').len()),
                    Some((_, '\n')) => return Some(at + text.trim_end_matches('\n').len()),
                    Some(_) => {}
                }
            }
            return Some(at + letters.next().map_or(text.len(), |(by, _)| by));
        }
        at += text.len();
    }
    // Past the last line.
    Some(source.len())
}

/// What is being written at the end of `before`.
///
/// A small state machine over the text, because that is what reading an
/// unfinished file is: no tree to walk, and the last few characters
/// decide everything.
fn read(before: &str) -> Place {
    let mut state = State::Rust;
    let mut letters = before.char_indices().peekable();

    while let Some((at, letter)) = letters.next() {
        state = match state {
            State::Rust => match letter {
                '<' => {
                    // `<` is also less-than and the start of a generic.
                    // What tells them apart is what follows: a letter, a
                    // `/`, or a `>` is markup; anything else is Rust.
                    match letters.peek().map(|(_, next)| *next) {
                        Some('/') => {
                            letters.next();
                            State::Closing {
                                typed: String::new(),
                                from: at,
                            }
                        }
                        Some(next) if next.is_alphabetic() || next == '>' => State::Tag {
                            typed: String::new(),
                        },
                        // Nothing after it, because the cursor is right
                        // there: a `<` just typed is a tag being opened,
                        // which is the one moment a list is wanted. A
                        // less-than that was about to be a comparison
                        // gets an unwanted list, which is cheap; the
                        // other way round costs the feature.
                        None => State::Tag {
                            typed: String::new(),
                        },
                        _ => State::Rust,
                    }
                }
                _ => State::Rust,
            },

            State::Tag { mut typed } => match letter {
                letter if letter.is_alphanumeric() || letter == ':' || letter == '-' => {
                    typed.push(letter);
                    State::Tag { typed }
                }
                '>' => State::Inside { tag: typed },
                '/' => State::Rust,
                letter if letter.is_whitespace() => State::Attributes {
                    tag: typed,
                    typed: String::new(),
                    already: Vec::new(),
                },
                _ => State::Tag { typed },
            },

            State::Closing { mut typed, from } => match letter {
                letter if letter.is_alphanumeric() || letter == ':' || letter == '-' => {
                    typed.push(letter);
                    State::Closing { typed, from }
                }
                // A closing tag that is finished puts us back in the
                // markup of whatever encloses it, which for the purpose
                // of a suggestion is the same as plain markup.
                '>' => State::Inside { tag: String::new() },
                _ => State::Closing { typed, from },
            },

            State::Attributes {
                tag,
                mut typed,
                mut already,
            } => match letter {
                letter if letter.is_alphanumeric() || letter == '-' || letter == '_' => {
                    typed.push(letter);
                    State::Attributes {
                        tag,
                        typed,
                        already,
                    }
                }
                '=' => {
                    already.push(typed.clone());
                    State::Value {
                        tag,
                        attribute: typed,
                        already,
                    }
                }
                '>' => State::Inside { tag },
                '/' => State::Rust,
                letter if letter.is_whitespace() => {
                    // A bare attribute, written and finished.
                    if !typed.is_empty() {
                        already.push(typed);
                    }
                    State::Attributes {
                        tag,
                        typed: String::new(),
                        already,
                    }
                }
                _ => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                },
            },

            State::Value {
                tag,
                attribute,
                already,
            } => match letter {
                '"' => State::Text {
                    tag,
                    attribute,
                    already,
                },
                '{' => {
                    // `style={{ … }}` is an object, and its keys are not
                    // attributes. Anything else is an expression.
                    if letters.peek().map(|(_, next)| *next) == Some('{') {
                        letters.next();
                        State::Style {
                            typed: String::new(),
                        }
                    } else {
                        State::Expression {
                            from: at + letter.len_utf8(),
                            depth: 1,
                            tag,
                        }
                    }
                }
                letter if letter.is_whitespace() => State::Value {
                    tag,
                    attribute,
                    already,
                },
                _ => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                },
            },

            State::Text {
                tag,
                attribute,
                already,
            } => match letter {
                '"' => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                },
                _ => State::Text {
                    tag,
                    attribute,
                    already,
                },
            },

            State::Style { mut typed } => match letter {
                letter if letter.is_alphanumeric() || letter == '-' || letter == '_' => {
                    typed.push(letter);
                    State::Style { typed }
                }
                '}' => State::Attributes {
                    tag: String::new(),
                    typed: String::new(),
                    already: Vec::new(),
                },
                _ => State::Style {
                    typed: String::new(),
                },
            },

            State::Inside { tag } => match letter {
                '<' => match letters.peek().map(|(_, next)| *next) {
                    Some('/') => {
                        letters.next();
                        State::Closing {
                            typed: String::new(),
                            from: at,
                        }
                    }
                    _ => State::Tag {
                        typed: String::new(),
                    },
                },
                '{' => State::Expression {
                    from: at + letter.len_utf8(),
                    depth: 1,
                    tag,
                },
                _ => State::Inside { tag },
            },

            State::Expression { from, depth, tag } => match letter {
                '{' => State::Expression {
                    from,
                    depth: depth + 1,
                    tag,
                },
                '}' if depth == 1 => State::Inside { tag },
                '}' => State::Expression {
                    from,
                    depth: depth - 1,
                    tag,
                },
                _ => State::Expression { from, depth, tag },
            },
        };
    }

    match state {
        State::Rust => Place::Rust,
        State::Tag { typed } => Place::TagName { typed },
        State::Closing { typed, from } => Place::ClosingTag {
            // What is open *before* this `</`: the `</` being typed is
            // the one about to close it, so it must not count as closed
            // already.
            open: if typed.is_empty() {
                innermost_open(&before[..from.min(before.len())])
            } else {
                typed
            },
        },
        State::Attributes {
            tag,
            typed,
            already,
        } => Place::AttributeName {
            tag,
            typed,
            already,
        },
        State::Value { tag, attribute, .. } | State::Text { tag, attribute, .. } => {
            Place::AttributeValue { tag, attribute }
        }
        State::Style { typed } => Place::StyleKey { typed },
        // Inside the markup but not in a name: between two tags, where
        // what a developer types is text. Nothing to offer, and
        // offering tags there would fill a paragraph with `<div>`.
        State::Inside { .. } => Place::AttributeValue {
            tag: String::new(),
            attribute: String::new(),
        },
        State::Expression { from, .. } => Place::Expression { from },
    }
}

/// The tag a `</` would close: the last one opened and not yet closed.
fn innermost_open(before: &str) -> String {
    let mut open: Vec<String> = Vec::new();
    let mut letters = before.char_indices().peekable();
    while let Some((_, letter)) = letters.next() {
        if letter != '<' {
            continue;
        }
        match letters.peek().map(|(_, next)| *next) {
            Some('/') => {
                letters.next();
                open.pop();
            }
            Some(next) if next.is_alphabetic() => {
                let mut name = String::new();
                let mut closed_itself = false;
                for (_, letter) in letters.by_ref() {
                    match letter {
                        letter if letter.is_alphanumeric() || letter == ':' || letter == '-' => {
                            name.push(letter)
                        }
                        '/' => closed_itself = true,
                        '>' => break,
                        _ => {}
                    }
                }
                if !closed_itself && !name.is_empty() {
                    open.push(name);
                }
            }
            _ => {}
        }
    }
    open.pop().unwrap_or_default()
}

/// What the reader is in the middle of, as it goes.
enum State {
    Rust,
    Tag {
        typed: String,
    },
    Closing {
        typed: String,
        /// Where the `<` of this `</` is.
        from: usize,
    },
    Attributes {
        tag: String,
        typed: String,
        already: Vec<String>,
    },
    Value {
        tag: String,
        attribute: String,
        already: Vec<String>,
    },
    Text {
        tag: String,
        attribute: String,
        already: Vec<String>,
    },
    Style {
        typed: String,
    },
    Inside {
        tag: String,
    },
    Expression {
        from: usize,
        depth: usize,
        tag: String,
    },
}
