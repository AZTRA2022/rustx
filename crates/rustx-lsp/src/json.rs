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

/// Tags every item of a `textDocument/completion` result — a bare
/// array, or `{"isIncomplete":...,"items":[...]}` — with the page it
/// came from.
///
/// `completionItem/resolve` carries only a `CompletionItem` and nothing
/// that says which document it belongs to; this is how the answer to
/// that question travels there and back, beside whatever `data`
/// `rust-analyzer` itself put on the item.
pub fn tag_items(list_result: &str, uri: &str) -> String {
    let Some(open) = list_result.find('[') else {
        return list_result.to_string();
    };
    let Some(close) = matching(list_result, open, '[', ']') else {
        return list_result.to_string();
    };
    let inside = &list_result[open + 1..close];
    let tag = format!(",\"__rustxUri\":\"{}\"", escape(uri));

    let mut rebuilt = String::with_capacity(inside.len() + tag.len() * 4);
    let mut at = 0;
    for (start, end) in objects_in(inside) {
        let _ = start;
        rebuilt.push_str(&inside[at..end - 1]);
        rebuilt.push_str(&tag);
        rebuilt.push('}');
        at = end;
    }
    rebuilt.push_str(&inside[at..]);

    format!(
        "{}{rebuilt}{}",
        &list_result[..=open],
        &list_result[close..]
    )
}

/// The reverse of [`tag_items`], on one item: the tag removed, and the
/// URI it carried — `None` when there was none, which is any item that
/// answers out of this server's own head rather than `rust-analyzer`'s.
pub fn untag_item(item: &str) -> (String, Option<String>) {
    let key = "\"__rustxUri\"";
    let Some(at) = item.find(key) else {
        return (item.to_string(), None);
    };
    let before = item[..at].trim_end();
    let before = before.strip_suffix(',').unwrap_or(before);

    let rest = item[at + key.len()..].trim_start();
    let rest = rest.strip_prefix(':').unwrap_or(rest).trim_start();
    let Some(value) = rest.strip_prefix('"') else {
        return (item.to_string(), None);
    };
    let Some(end) = value.find('"') else {
        return (item.to_string(), None);
    };
    let uri = unescape(value);
    let after = &value[end + 1..];

    (format!("{before}{after}"), Some(uri))
}

/// Where the `{…}` or `[…]` value of the first field called `name`
/// is, delimiters included — `None` when there is no such field, or
/// its value is not that kind of delimiter.
fn value_span(text: &str, name: &str, opening: char, closing: char) -> Option<(usize, usize)> {
    let key = format!("\"{name}\"");
    let at = text.find(&key)?;
    let after_key = at + key.len();
    let colon = text[after_key..].find(':')?;
    let after_colon = after_key + colon + 1;
    let value = text[after_colon..].trim_start();
    if !value.starts_with(opening) {
        return None;
    }
    let value_start = after_colon + (text[after_colon..].len() - value.len());
    let close = matching(text, value_start, opening, closing)?;
    Some((value_start, close + 1))
}

/// The raw text of the `{…}` value of the first field called `name`,
/// such as `completionItem/resolve`'s own `params` — a `CompletionItem`
/// with no envelope, the same shape this server already hands out.
pub fn object_field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let (start, end) = value_span(text, name, '{', '}')?;
    Some(&text[start..end])
}

/// The raw text of the `[…]` value of the first field called `name`.
pub fn array_field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let (start, end) = value_span(text, name, '[', ']')?;
    Some(&text[start..end])
}

/// `text` with the `{…}` or `[…]` value of `name` replaced by
/// `replacement`, whole. `text` unchanged when there is no such field.
pub fn replace_field(
    text: &str,
    name: &str,
    opening: char,
    closing: char,
    replacement: &str,
) -> String {
    match value_span(text, name, opening, closing) {
        Some((start, end)) => format!("{}{replacement}{}", &text[..start], &text[end..]),
        None => text.to_string(),
    }
}

/// The index of the delimiter that closes the one at `open_at`, which
/// must itself be `opening`. Quoted text is skipped whole, the same way
/// every other reader in this module does it.
fn matching(text: &str, open_at: usize, opening: char, closing: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut inside_text = false;
    let mut escaped = false;
    for (at, letter) in text[open_at..].char_indices() {
        if inside_text {
            match letter {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => inside_text = false,
                _ => {}
            }
            continue;
        }
        match letter {
            '"' => inside_text = true,
            letter if letter == opening => depth += 1,
            letter if letter == closing => {
                // A closing delimiter with nothing open. Not reachable
                // from a well-formed value, which is why it used to be
                // written as `depth -= 1` — and a `usize` that goes
                // below zero is a panic in a debug build and nonsense in
                // a release one. Both of those are a language server
                // that stops answering, so it is said out loud instead.
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open_at + at);
                }
            }
            _ => {}
        }
    }
    None
}

/// Every top-level `{…}` of `text`, as `(start, end)` byte ranges —
/// `end` just past the closing brace. A brace inside a quoted string,
/// or nested one level down, is not a top-level one.
pub fn objects_in(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut inside_text = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (at, letter) in text.char_indices() {
        if inside_text {
            match letter {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => inside_text = false,
                _ => {}
            }
            continue;
        }
        match letter {
            '"' => inside_text = true,
            '{' => {
                if depth == 0 {
                    start = at;
                }
                depth += 1;
            }
            '}' => {
                // A `}` with nothing open: the text is not a list of
                // objects. What was found so far is kept and the rest is
                // left alone, rather than counting below zero.
                let Some(less) = depth.checked_sub(1) else {
                    return found;
                };
                depth = less;
                if depth == 0 {
                    found.push((start, at + 1));
                }
            }
            _ => {}
        }
    }
    found
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
