//! Finding the JSX in a file that is otherwise Rust.
//!
//! `<` means two things. In `a < b` and in `Vec<u8>` it is an operator; in
//! `return <div>` it opens a tag. Nothing in the character itself says
//! which, so the scanner decides from its neighbours:
//!
//! **A `<` opens a tag when the token just before it is one of `return`,
//! `=>`, `{`, `(`, `,`, `=`, or the start of the file, and the character
//! just after it is an identifier, `/`, or `>` with no space between.**
//!
//! Generics fail the first half — they follow an identifier or a `:` —
//! and comparisons fail the second, because people write `a < b` with
//! spaces and `a<b` only when `a` is a type. The rule is a convention, not
//! a theorem, and it is written here rather than spread through the code
//! so that the day it needs widening there is one place to widen.

use crate::error::CompileError;

/// A byte range in the `.rsx` source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// The token before a `<`, reduced to what the rule needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Previous {
    StartOfFile,
    Return,
    FatArrow,
    OpenBrace,
    OpenParen,
    Comma,
    Equals,
    Anything,
}

impl Previous {
    /// Does a `<` after this token open a tag?
    fn may_open_a_tag(self) -> bool {
        !matches!(self, Previous::Anything)
    }
}

/// The top-level JSX regions of `source`, in the order they appear.
pub fn find_jsx(source: &str) -> Result<Vec<Span>, CompileError> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut previous = Previous::StartOfFile;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if let Some(after) = skip_comment(bytes, index) {
            index = after;
            continue;
        }
        if byte == b'"' {
            index = skip_string(source, index)?;
            previous = Previous::Anything;
            continue;
        }
        if byte == b'\'' {
            // A character literal, or a lifetime. Both end this token.
            index = skip_char_or_lifetime(bytes, index);
            previous = Previous::Anything;
            continue;
        }
        if is_word_byte(byte) {
            let start = index;
            while index < bytes.len() && is_word_byte(bytes[index]) {
                index += 1;
            }
            previous = if &source[start..index] == "return" {
                Previous::Return
            } else {
                Previous::Anything
            };
            continue;
        }
        if byte == b'<' && previous.may_open_a_tag() && opens_a_tag(bytes, index) {
            let span = region(source, index)?;
            index = span.end;
            spans.push(span);
            previous = Previous::Anything;
            continue;
        }

        previous = match byte {
            b'{' => Previous::OpenBrace,
            b'(' => Previous::OpenParen,
            b',' => Previous::Comma,
            b'=' if bytes.get(index + 1) == Some(&b'>') => Previous::FatArrow,
            // `==`, `=>` and `=` all end in a state where a tag may follow;
            // only a bare `=` and a `=>` are in the rule, and `==` is rare
            // enough before JSX that treating it the same costs nothing.
            b'=' => Previous::Equals,
            _ => Previous::Anything,
        };
        if previous == Previous::FatArrow {
            index += 1;
        }
        index += 1;
    }

    Ok(spans)
}

/// The second half of the rule: what follows the `<`.
fn opens_a_tag(bytes: &[u8], index: usize) -> bool {
    match bytes.get(index + 1) {
        Some(&next) => next.is_ascii_alphabetic() || next == b'_' || next == b'/' || next == b'>',
        None => false,
    }
}

/// The whole region, from the opening `<` to the `>` that closes the
/// outermost tag.
fn region(source: &str, start: usize) -> Result<Span, CompileError> {
    let bytes = source.as_bytes();
    // Open tags, by name and by where they were opened, so an unclosed one
    // can be named and placed.
    let mut open: Vec<(String, usize)> = Vec::new();
    let mut index = start;

    while index < bytes.len() {
        match bytes[index] {
            b'<' if bytes.get(index + 1) == Some(&b'/') => {
                index = end_of_tag(source, index)?;
                if open.pop().is_none() {
                    return Err(CompileError::at(
                        source,
                        index,
                        "a closing tag with nothing open",
                    ));
                }
                if open.is_empty() {
                    return Ok(Span { start, end: index });
                }
            }
            b'<' => {
                let at = index;
                let name = tag_name(source, index);
                index = end_of_tag(source, index)?;
                let self_closing = source[..index].trim_end().ends_with("/>");
                if !self_closing {
                    open.push((name, at));
                } else if open.is_empty() {
                    return Ok(Span { start, end: index });
                }
            }
            b'{' => index = skip_braces(source, index)?,
            _ => index += 1,
        }
    }

    let (name, at) = open
        .first()
        .cloned()
        .unwrap_or_else(|| (String::new(), start));
    Err(CompileError::at(
        source,
        at,
        format!("unclosed tag <{name}>"),
    ))
}

/// The name right after a `<`, or the empty string for a fragment.
fn tag_name(source: &str, open: usize) -> String {
    let bytes = source.as_bytes();
    let mut index = open + 1;
    let start = index;
    while index < bytes.len() && (is_word_byte(bytes[index]) || bytes[index] == b'-') {
        index += 1;
    }
    source[start..index].to_string()
}

/// One byte past the `>` that ends this tag's header, stepping over
/// strings and over the braces of attribute expressions — a closure body
/// may hold `>` and `<` of its own.
fn end_of_tag(source: &str, open: usize) -> Result<usize, CompileError> {
    let bytes = source.as_bytes();
    let mut index = open + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'>' => return Ok(index + 1),
            b'"' => index = skip_string(source, index)?,
            b'{' => index = skip_braces(source, index)?,
            _ => index += 1,
        }
    }
    Err(CompileError::at(
        source,
        open,
        format!("unclosed tag <{}>", tag_name(source, open)),
    ))
}

/// One byte past the `}` matching the `{` at `open`.
pub(crate) fn skip_braces(source: &str, open: usize) -> Result<usize, CompileError> {
    let bytes = source.as_bytes();
    let mut depth = 0_usize;
    let mut index = open;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            b'"' => index = skip_string(source, index)?,
            b'\'' => index = skip_char_or_lifetime(bytes, index),
            _ => index += 1,
        }
    }
    Err(CompileError::at(source, open, "unclosed `{`"))
}

/// One byte past the closing quote, honouring `\"`.
pub(crate) fn skip_string(source: &str, open: usize) -> Result<usize, CompileError> {
    let bytes = source.as_bytes();
    let mut index = open + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Ok(index + 1),
            _ => index += 1,
        }
    }
    Err(CompileError::at(source, open, "unterminated string"))
}

/// A `'` starts either a character literal or a lifetime, and the two are
/// told apart by whether a quote closes it within a few bytes.
fn skip_char_or_lifetime(bytes: &[u8], open: usize) -> usize {
    let mut index = open + 1;
    if bytes.get(index) == Some(&b'\\') {
        index += 2;
    } else if index < bytes.len() {
        index += 1;
    }
    if bytes.get(index) == Some(&b'\'') {
        return index + 1;
    }
    // A lifetime: just the tick, the name follows as an ordinary word.
    open + 1
}

/// One byte past a `//` or `/* */` comment, or `None` if there is none.
fn skip_comment(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) != Some(&b'/') {
        return None;
    }
    match bytes.get(index + 1) {
        Some(b'/') => {
            let mut index = index + 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            Some(index)
        }
        Some(b'*') => {
            let mut index = index + 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            Some((index + 2).min(bytes.len()))
        }
        _ => None,
    }
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}
