//! As much JSON as a language server needs, and no more.
//!
//! What it reads from a client is a method, a path and a text; what it
//! writes is a list of diagnostics. Reading a field by name is enough
//! for the first and escaping a string is enough for the second — and a
//! project that reads Rust tokens by hand does not take a dependency to
//! find a quoted key.
//!
//! The one thing it must get right is escaping, in both directions: a
//! `.rsx` is full of quotes, backslashes and newlines, and a server that
//! loses them sends an editor the wrong line.

/// The string value of the first field called `name`.
pub fn field(message: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\"");
    let mut from = 0;
    while let Some(at) = message[from..].find(&key) {
        let after = from + at + key.len();
        let rest = message[after..].trim_start();
        if let Some(value) = rest.strip_prefix(':') {
            let value = value.trim_start();
            if let Some(text) = value.strip_prefix('"') {
                return Some(unescape(text));
            }
            return None;
        }
        from = after;
    }
    None
}

/// The number value of the first field called `name`.
pub fn number(message: &str, name: &str) -> Option<i64> {
    let key = format!("\"{name}\"");
    let at = message.find(&key)?;
    let rest = message[at + key.len()..].trim_start();
    let value = rest.strip_prefix(':')?.trim_start();
    let digits: String = value
        .chars()
        .take_while(|letter| letter.is_ascii_digit() || *letter == '-')
        .collect();
    digits.parse().ok()
}

/// A string as JSON writes it, without the quotes.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            letter if (letter as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", letter as u32));
            }
            letter => out.push(letter),
        }
    }
    out
}

/// A JSON string body, up to its closing quote, as it was written.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut letters = text.chars();
    while let Some(letter) = letters.next() {
        match letter {
            '"' => break,
            '\\' => match letters.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    let digits: String = letters.by_ref().take(4).collect();
                    if let Some(found) = u32::from_str_radix(&digits, 16)
                        .ok()
                        .and_then(char::from_u32)
                    {
                        out.push(found);
                    }
                }
                Some(other) => out.push(other),
                None => break,
            },
            letter => out.push(letter),
        }
    }
    out
}
