//! The JSON a client sends, read properly.
//!
//! `json.rs` reads a field by looking for its quoted name in the raw
//! text of a message. That is enough for the messages this server
//! *writes*, which it wrote itself. It is not enough for the messages it
//! *reads*, because those carry the developer's own text — and a `.rsx`
//! is allowed to contain the six characters `"text"`, a `{`, or a
//! `"range":`. A reader that searches for a name finds whichever comes
//! first in the bytes, which may be the one inside the document rather
//! than the key beside it. The whole document then arrives at the server
//! as a fragment, and every position for the rest of the session is
//! wrong.
//!
//! So the messages a client sends are parsed: one pass, one value, no
//! guessing about which `"text"` was meant. It is a small parser because
//! LSP is a small grammar — objects, arrays, strings, numbers, `true`,
//! `false`, `null` — and because a project that reads Rust tokens by
//! hand does not take a dependency to find a quoted key.
//!
//! The one thing it must get right beyond the shape is the escaping. A
//! client is entitled to write any character as `\uXXXX`, and anything
//! outside the basic plane as a *pair* of them: an emoji is
//! `😀`, two values that are not characters on their own.
//! Decoded separately they are both invalid and both lost, and a
//! document that lost two characters has every offset after them wrong.

use std::collections::HashMap;

/// A JSON value, as much of one as a language server ever sees.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// Every number LSP uses is an integer — an id, a line, a character,
    /// a version — and the few that are not are never read here.
    Number(f64),
    Text(String),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
}

impl Value {
    /// The field called `name`, or `Null` when there is none.
    ///
    /// `Null` rather than `Option`, so that a path through a message
    /// reads as one expression: a field of a field of a field, each step
    /// of which may be missing, is how every LSP message is shaped.
    pub fn get(&self, name: &str) -> &Value {
        match self {
            Self::Map(fields) => fields.get(name).unwrap_or(&Value::Null),
            _ => &Value::Null,
        }
    }

    /// The item at `at` of a list, or `Null`.
    pub fn at(&self, at: usize) -> &Value {
        match self {
            Self::List(items) => items.get(at).unwrap_or(&Value::Null),
            _ => &Value::Null,
        }
    }

    /// The items of a list, or nothing at all.
    pub fn items(&self) -> &[Value] {
        match self {
            Self::List(items) => items,
            _ => &[],
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// A whole number, which is what every number in LSP is.
    ///
    /// A value that is not a number at all — a client that sent a string
    /// id, which the specification allows — is `None`, and the caller
    /// decides what that means.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(number) if number.is_finite() => Some(*number as i64),
            _ => None,
        }
    }

    /// A count that cannot be negative: a line, a character, a length.
    /// A negative one is clamped to zero rather than refused, because an
    /// editor that sends `-1` means "the beginning".
    pub fn as_index(&self) -> Option<usize> {
        self.as_i64().map(|number| number.max(0) as usize)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// How deep a value may nest before it is refused.
///
/// A client is not expected to send anything deeper than about five
/// levels; a hostile or broken one that sends ten thousand `[` would
/// otherwise overflow this process's stack, which no `catch_unwind` can
/// recover from.
const DEEPEST: usize = 128;

/// Read one JSON value out of `text`, or `None` when it is not JSON.
///
/// Trailing text after the value is ignored: a frame's body is exactly
/// one message, and a byte after it is a client's problem rather than a
/// reason to lose the message.
pub fn parse(text: &str) -> Option<Value> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        at: 0,
    };
    reader.space();
    let value = reader.value(0)?;
    Some(value)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn word(&mut self, word: &str) -> bool {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Option<Value> {
        if depth > DEEPEST {
            return None;
        }
        self.space();
        match self.peek()? {
            b'{' => self.map(depth),
            b'[' => self.list(depth),
            b'"' => self.text().map(Value::Text),
            b't' => self.word("true").then_some(Value::Bool(true)),
            b'f' => self.word("false").then_some(Value::Bool(false)),
            b'n' => self.word("null").then_some(Value::Null),
            _ => self.number(),
        }
    }

    fn map(&mut self, depth: usize) -> Option<Value> {
        if !self.eat(b'{') {
            return None;
        }
        let mut fields = HashMap::new();
        self.space();
        if self.eat(b'}') {
            return Some(Value::Map(fields));
        }
        loop {
            self.space();
            let name = self.text()?;
            self.space();
            if !self.eat(b':') {
                return None;
            }
            let value = self.value(depth + 1)?;
            fields.insert(name, value);
            self.space();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b'}') {
                return Some(Value::Map(fields));
            }
            return None;
        }
    }

    fn list(&mut self, depth: usize) -> Option<Value> {
        if !self.eat(b'[') {
            return None;
        }
        let mut items = Vec::new();
        self.space();
        if self.eat(b']') {
            return Some(Value::List(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.space();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b']') {
                return Some(Value::List(items));
            }
            return None;
        }
    }

    fn number(&mut self) -> Option<Value> {
        let from = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
        ) {
            self.at += 1;
        }
        if from == self.at {
            return None;
        }
        std::str::from_utf8(&self.bytes[from..self.at])
            .ok()?
            .parse()
            .ok()
            .map(Value::Number)
    }

    /// A JSON string, from its opening quote, with every escape decoded.
    fn text(&mut self) -> Option<String> {
        if !self.eat(b'"') {
            return None;
        }
        let mut out = String::new();
        loop {
            let byte = self.peek()?;
            match byte {
                b'"' => {
                    self.at += 1;
                    return Some(out);
                }
                b'\\' => {
                    self.at += 1;
                    match self.peek()? {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            self.at += 1;
                            out.push(self.escaped_char()?);
                            // `escaped_char` left `at` past what it read.
                            continue;
                        }
                        // An escape nobody defined. Kept as what was
                        // written, which loses nothing.
                        other => out.push(other as char),
                    }
                    self.at += 1;
                }
                // Raw UTF-8, which is how a client sends anything it did
                // not bother to escape. Taken whole, by the length its
                // own lead byte declares, so a three-byte character is
                // never cut into three — and read from that many bytes
                // only, because decoding the rest of the document once
                // per character would make a long page quadratic.
                _ => {
                    let wide = match byte {
                        0x00..=0x7F => 1,
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        0xF0..=0xF7 => 4,
                        // A continuation byte with nothing in front of
                        // it. The frame was decoded from UTF-8 already,
                        // so this cannot happen; refused rather than
                        // guessed at.
                        _ => return None,
                    };
                    let piece = self.bytes.get(self.at..self.at + wide)?;
                    let letter = std::str::from_utf8(piece).ok()?.chars().next()?;
                    out.push(letter);
                    self.at += wide;
                }
            }
        }
    }

    /// The character a `\u` names, a surrogate pair counting as one.
    ///
    /// `at` is on the first of the four digits when this is called, and
    /// is left just past everything this consumed — which is why the
    /// caller `continue`s rather than stepping over one more byte.
    fn escaped_char(&mut self) -> Option<char> {
        let first = self.four_digits()?;
        // A high surrogate is half a character: the low half follows it,
        // written as its own escape. Decoded apart, both are invalid and
        // both would be dropped — and an emoji that vanished takes every
        // position after it with it.
        if (0xD800..0xDC00).contains(&first) {
            let saved = self.at;
            if self.bytes[self.at..].starts_with(b"\\u") {
                self.at += 2;
                if let Some(second) = self.four_digits() {
                    if (0xDC00..0xE000).contains(&second) {
                        let code =
                            0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                        if let Some(letter) = char::from_u32(code) {
                            return Some(letter);
                        }
                    }
                }
            }
            self.at = saved;
            // A high surrogate with nothing after it. Not a character,
            // and it must not disappear silently either: the replacement
            // character keeps every offset after it unchanged, which is
            // the property a document store depends on.
            return Some(char::REPLACEMENT_CHARACTER);
        }
        if (0xDC00..0xE000).contains(&first) {
            return Some(char::REPLACEMENT_CHARACTER);
        }
        Some(char::from_u32(first).unwrap_or(char::REPLACEMENT_CHARACTER))
    }

    /// Four hexadecimal digits, `at` left just past them.
    fn four_digits(&mut self) -> Option<u32> {
        let digits = self.bytes.get(self.at..self.at + 4)?;
        let text = std::str::from_utf8(digits).ok()?;
        let value = u32::from_str_radix(text, 16).ok()?;
        self.at += 4;
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, parse};

    #[test]
    fn a_field_is_read_by_its_name_and_not_by_where_it_appears() {
        // The bug this module exists for: the document says `"text"`,
        // and a reader that searched the bytes found the document's one
        // first.
        let message = r#"{"params":{"contentChanges":[{"text":"let s = \"\\\"text\\\": 1\";"}]}}"#;
        let read = parse(message).expect("a message");
        assert_eq!(
            read.get("params")
                .get("contentChanges")
                .at(0)
                .get("text")
                .as_str(),
            Some("let s = \"\\\"text\\\": 1\";")
        );
    }

    #[test]
    fn an_emoji_written_as_a_surrogate_pair_is_one_character() {
        let read = parse(r#"{"text":"a😀b"}"#).expect("a message");
        assert_eq!(read.get("text").as_str(), Some("a😀b"));
    }

    #[test]
    fn a_lone_surrogate_keeps_its_place_rather_than_vanishing() {
        let read = parse(r#"{"text":"a\ud83db"}"#).expect("a message");
        let text = read.get("text").as_str().expect("text");
        assert_eq!(text.chars().count(), 3, "got {text:?}");
        assert!(text.starts_with('a') && text.ends_with('b'), "got {text:?}");
    }

    #[test]
    fn every_escape_json_defines_is_decoded() {
        let read = parse(r#"{"t":"\"\\\/\b\f\n\r\té"}"#).expect("a message");
        assert_eq!(
            read.get("t").as_str(),
            Some("\"\\/\u{8}\u{c}\n\r\t\u{e9}")
        );
    }

    #[test]
    fn a_number_is_read_and_a_missing_field_is_null() {
        let read = parse(r#"{"id":42,"version":-1,"f":1.5}"#).expect("a message");
        assert_eq!(read.get("id").as_i64(), Some(42));
        assert_eq!(read.get("version").as_i64(), Some(-1));
        assert_eq!(read.get("version").as_index(), Some(0));
        assert_eq!(read.get("f").as_i64(), Some(1));
        assert!(read.get("nothing").is_null());
        assert_eq!(read.get("nothing").get("deeper").as_i64(), None);
    }

    #[test]
    fn nothing_that_is_not_json_is_read_as_json() {
        for broken in [
            "", "{", "[", "{\"a\"}", "{\"a\":}", "nul", "{,}", "[1,]", "\"unterminated",
        ] {
            assert!(parse(broken).is_none(), "{broken:?} was read as JSON");
        }
    }

    #[test]
    fn a_value_nested_past_all_reason_is_refused_rather_than_overflowing() {
        let deep = format!("{}1{}", "[".repeat(5000), "]".repeat(5000));
        assert!(parse(&deep).is_none());
    }

    #[test]
    fn raw_utf8_in_a_string_is_kept_whole() {
        let read = parse("{\"t\":\"é😀ç\"}").expect("a message");
        assert_eq!(read.get("t").as_str(), Some("é😀ç"));
    }

    #[test]
    fn an_empty_object_and_an_empty_list_are_values() {
        assert_eq!(parse("{}"), Some(Value::Map(Default::default())));
        assert_eq!(parse("[]"), Some(Value::List(Vec::new())));
        assert_eq!(parse("  null  "), Some(Value::Null));
    }
}
