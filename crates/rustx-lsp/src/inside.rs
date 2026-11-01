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
/// What is being written at the end of `before`.
///
/// A small state machine over the text, because that is what reading an
/// unfinished file is: no tree to walk, and the last few characters
/// decide everything.
///
/// It carries a depth, which is the whole difference between a reader
/// that works on one region and a reader that works on a file: markup
/// ends when the tag that opened it closes, not at the first `>` or the
/// first `/`. Without it, everything after the first region in a file
/// looked like the inside of a tag, and a page of several components
/// offered nothing from its second function on.
fn read(before: &str) -> Place {
    let mut state = State::Rust;
    let mut letters = before.char_indices().peekable();

    while let Some((at, letter)) = letters.next() {
        state = match state {
            State::Rust if letter == '<' && !starts_markup(&before[..at]) => State::Rust,
            State::Rust => match letter {
                // `<` is also less-than and the start of a generic.
                // What tells them apart is what follows — a letter, a
                // `/` or a `>` — and what *precedes* it, which is what
                // `starts_markup` reads.
                '<' => match letters.peek().map(|(_, next)| *next) {
                    Some('/') => {
                        letters.next();
                        State::Closing {
                            typed: String::new(),
                            from: at,
                            depth: 1,
                        }
                    }
                    Some(next) if next.is_alphabetic() || next == '>' => State::Tag {
                        typed: String::new(),
                        depth: 0,
                    },
                    // Nothing after it, because the cursor is right
                    // there: a `<` just typed is a tag being opened,
                    // which is the one moment a list is wanted. A
                    // less-than that was about to be a comparison gets
                    // an unwanted list, which is cheap; the other way
                    // round costs the feature.
                    None => State::Tag {
                        typed: String::new(),
                        depth: 0,
                    },
                    _ => State::Rust,
                },
                _ => State::Rust,
            },

            State::Tag { mut typed, depth } => match letter {
                letter if letter.is_alphanumeric() || letter == ':' || letter == '-' => {
                    typed.push(letter);
                    State::Tag { typed, depth }
                }
                // The tag is open: what follows it is inside it.
                '>' => State::Inside {
                    tag: typed,
                    depth: depth + 1,
                },
                // `<br/>`: open and closed at once, so the depth does
                // not change — and the markup around it goes on.
                '/' => State::Closed { depth },
                letter if letter.is_whitespace() => State::Attributes {
                    tag: typed,
                    typed: String::new(),
                    already: Vec::new(),
                    depth,
                },
                _ => State::Tag { typed, depth },
            },

            State::Closing {
                mut typed,
                from,
                depth,
            } => match letter {
                letter if letter.is_alphanumeric() || letter == ':' || letter == '-' => {
                    typed.push(letter);
                    State::Closing { typed, from, depth }
                }
                // One tag closed. If it was the one that opened the
                // markup, the file is Rust again.
                '>' => {
                    if depth <= 1 {
                        State::Rust
                    } else {
                        State::Inside {
                            tag: String::new(),
                            depth: depth - 1,
                        }
                    }
                }
                _ => State::Closing { typed, from, depth },
            },

            State::Attributes {
                tag,
                mut typed,
                mut already,
                depth,
            } => match letter {
                letter if letter.is_alphanumeric() || letter == '-' || letter == '_' => {
                    typed.push(letter);
                    State::Attributes {
                        tag,
                        typed,
                        already,
                        depth,
                    }
                }
                '=' => {
                    already.push(typed.clone());
                    State::Value {
                        tag,
                        attribute: typed,
                        already,
                        depth,
                    }
                }
                '>' => State::Inside {
                    tag,
                    depth: depth + 1,
                },
                '/' => State::Closed { depth },
                letter if letter.is_whitespace() => {
                    // A bare attribute, written and finished.
                    if !typed.is_empty() {
                        already.push(typed);
                    }
                    State::Attributes {
                        tag,
                        typed: String::new(),
                        already,
                        depth,
                    }
                }
                _ => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                    depth,
                },
            },

            // After the `/` of a `<br/>`: the `>` is next, and then we
            // are wherever we were.
            State::Closed { depth } => match letter {
                '>' => {
                    if depth == 0 {
                        State::Rust
                    } else {
                        State::Inside {
                            tag: String::new(),
                            depth,
                        }
                    }
                }
                _ => State::Closed { depth },
            },

            State::Value {
                tag,
                attribute,
                already,
                depth,
            } => match letter {
                '"' => State::Text {
                    tag,
                    attribute,
                    already,
                    depth,
                },
                '{' => {
                    // `style={{ … }}` is an object, and its keys are not
                    // attributes. Anything else is an expression.
                    if letters.peek().map(|(_, next)| *next) == Some('{') {
                        letters.next();
                        State::Style {
                            typed: String::new(),
                            tag,
                            already,
                            depth,
                        }
                    } else {
                        State::Expression {
                            from: at + letter.len_utf8(),
                            braces: 1,
                            tag,
                            depth,
                            in_attribute: true,
                            already,
                        }
                    }
                }
                letter if letter.is_whitespace() => State::Value {
                    tag,
                    attribute,
                    already,
                    depth,
                },
                _ => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                    depth,
                },
            },

            State::Text {
                tag,
                attribute,
                already,
                depth,
            } => match letter {
                '"' => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                    depth,
                },
                _ => State::Text {
                    tag,
                    attribute,
                    already,
                    depth,
                },
            },

            State::Style {
                mut typed,
                tag,
                already,
                depth,
            } => match letter {
                letter if letter.is_alphanumeric() || letter == '-' || letter == '_' => {
                    typed.push(letter);
                    State::Style {
                        typed,
                        tag,
                        already,
                        depth,
                    }
                }
                // The first `}` of the `}}` that ends the object; the
                // second is eaten as an ordinary character of the
                // attribute list.
                '}' => State::Attributes {
                    tag,
                    typed: String::new(),
                    already,
                    depth,
                },
                _ => State::Style {
                    typed: String::new(),
                    tag,
                    already,
                    depth,
                },
            },

            State::Inside { tag, depth } => match letter {
                '<' => match letters.peek().map(|(_, next)| *next) {
                    Some('/') => {
                        letters.next();
                        State::Closing {
                            typed: String::new(),
                            from: at,
                            depth,
                        }
                    }
                    _ => State::Tag {
                        typed: String::new(),
                        depth,
                    },
                },
                '{' => State::Expression {
                    from: at + letter.len_utf8(),
                    braces: 1,
                    tag,
                    depth,
                    in_attribute: false,
                    already: Vec::new(),
                },
                _ => State::Inside { tag, depth },
            },

            State::Expression {
                from,
                braces,
                tag,
                depth,
                in_attribute,
                already,
            } => match letter {
                '{' => State::Expression {
                    from,
                    braces: braces + 1,
                    tag,
                    depth,
                    in_attribute,
                    already,
                },
                '}' if braces == 1 => {
                    if in_attribute {
                        State::Attributes {
                            tag,
                            typed: String::new(),
                            already,
                            depth,
                        }
                    } else {
                        State::Inside { tag, depth }
                    }
                }
                '}' => State::Expression {
                    from,
                    braces: braces - 1,
                    tag,
                    depth,
                    in_attribute,
                    already,
                },
                _ => State::Expression {
                    from,
                    braces,
                    tag,
                    depth,
                    in_attribute,
                    already,
                },
            },
        };
    }

    match state {
        State::Rust => Place::Rust,
        State::Tag { typed, .. } => Place::TagName { typed },
        State::Closing { typed, from, .. } => Place::ClosingTag {
            // What is open *before* this `</`: the `</` being typed is
            // the one about to close it, so it must not count as closed
            // already.
            open: if typed.is_empty() {
                innermost_open(&before[..from.min(before.len())])
            } else {
                typed
            },
        },
        State::Attributes { tag, typed, already, .. } => Place::AttributeName {
            tag,
            typed,
            already,
        },
        State::Value { tag, attribute, .. } | State::Text { tag, attribute, .. } => {
            Place::AttributeValue { tag, attribute }
        }
        State::Style { typed, .. } => Place::StyleKey { typed },
        // A tag that closed itself, mid-`/>`: nothing to offer, and the
        // next character decides where we are.
        State::Closed { .. } => Place::AttributeValue {
            tag: String::new(),
            attribute: String::new(),
        },
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

/// Does a `<` at the end of `before` open a tag, or is it a generic?
///
/// `Vec<Point>` is not a tag and `return <main>` is; what tells them
/// apart is the token before the `<`. The compiler decides this the
/// same way, in `scan.rs` — a `<` may open a tag after `return`, `=>`,
/// `{`, `(`, `,`, `=`, `|` or nothing at all, and after anything else
/// it is Rust. The rule is read backwards here because this is called
/// at one position rather than while walking a whole file.
///
/// Getting this wrong is not a small matter: read as a tag, a
/// `Vec<Point>` leaves the reader inside markup for the rest of the
/// file, and every tag in every function below it offers nothing.
fn starts_markup(before: &str) -> bool {
    let trimmed = before.trim_end();
    let Some(last) = trimmed.chars().next_back() else {
        // The start of the file.
        return true;
    };
    match last {
        '{' | '(' | ',' | '=' | '|' | '[' | '>' | '&' | '?' | '!' | ':' | ';' => {
            // `=>` ends in `>`, which is also the end of a tag; both are
            // places a tag may follow, so neither has to be told apart.
            true
        }
        letter if letter.is_alphanumeric() || letter == '_' => {
            // An identifier — or a keyword after which markup is
            // ordinary. `return` above all, and the ones a page writes
            // in a loop or a branch.
            let word: String = trimmed
                .chars()
                .rev()
                .take_while(|letter| letter.is_alphanumeric() || *letter == '_')
                .collect::<Vec<char>>()
                .into_iter()
                .rev()
                .collect();
            matches!(
                word.as_str(),
                "return" | "else" | "in" | "match" | "yield" | "await" | "and" | "or"
            )
        }
        _ => true,
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
///
/// `depth` is how many tags are open around this position. Markup ends
/// when it reaches zero, and not before.
enum State {
    Rust,
    Tag {
        typed: String,
        depth: usize,
    },
    Closing {
        typed: String,
        /// Where the `<` of this `</` is.
        from: usize,
        depth: usize,
    },
    Attributes {
        tag: String,
        typed: String,
        already: Vec<String>,
        depth: usize,
    },
    Value {
        tag: String,
        attribute: String,
        already: Vec<String>,
        depth: usize,
    },
    Text {
        tag: String,
        attribute: String,
        already: Vec<String>,
        depth: usize,
    },
    Style {
        typed: String,
        tag: String,
        already: Vec<String>,
        depth: usize,
    },
    /// Between the `/` and the `>` of a tag that closes itself.
    Closed {
        depth: usize,
    },
    Inside {
        tag: String,
        depth: usize,
    },
    Expression {
        from: usize,
        braces: usize,
        tag: String,
        depth: usize,
        /// An expression in an attribute goes back to the attribute
        /// list; one between tags goes back inside the tag.
        in_attribute: bool,
        already: Vec<String>,
    },
}
