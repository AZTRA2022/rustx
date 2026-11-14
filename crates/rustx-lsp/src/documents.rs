//! The text the editor is holding, kept in step with it.
//!
//! Everything else in this server reads from here. A diagnostic's line,
//! a completion's position, the Rust written for `rust-analyzer`: all of
//! them are coordinates into *this* text, and if it is not byte for byte
//! the text in the editor's window then every one of them points at the
//! wrong place for the rest of the session. There is no error message
//! for that, and nothing recovers from it but closing the file.
//!
//! So two things are done carefully here.
//!
//! **A change is applied, not assumed.** This server asks for
//! `textDocumentSync: 1`, which means the whole text on every change,
//! because a compiler reads a file from its first byte. But a client is
//! a program somebody else wrote, and more than one has been known to
//! send ranges anyway. A notification carrying a range used to be read
//! for its `text` alone — so the document became the three characters
//! somebody had just typed, and every position after that was nonsense.
//! Both forms are applied correctly here instead, in the order they
//! arrive, which is the only form of this that cannot desynchronise.
//!
//! **A column is a UTF-16 column.** LSP counts characters in UTF-16 code
//! units; Rust counts bytes and this server counts characters. For most
//! text all three agree, which is why it is never noticed — until a page
//! holds an emoji, and then every position after it on that line is one
//! out, in the direction nobody checks.

use std::collections::HashMap;

use crate::wire::Value;

/// One open document.
#[derive(Debug, Clone)]
pub struct Document {
    pub text: String,
    /// What the editor last called it. Kept so that a change arriving
    /// out of order can be recognised rather than applied.
    pub version: i64,
}

/// Every document the editor has open.
#[derive(Debug, Default)]
pub struct Documents {
    held: HashMap<String, Document>,
}

/// What happened to a document when a change was applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// The text changed, or was opened: whoever is listening should
    /// compile it again.
    Fresh,
    /// Nothing to do: a change for a document nobody opened, or one the
    /// editor has already superseded.
    Ignored,
}

impl Documents {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self, uri: &str) -> Option<&str> {
        self.held.get(uri).map(|held| held.text.as_str())
    }

    pub fn version(&self, uri: &str) -> Option<i64> {
        self.held.get(uri).map(|held| held.version)
    }

    pub fn is_open(&self, uri: &str) -> bool {
        self.held.contains_key(uri)
    }

    /// How many documents are held. The editor closes what it opens, so
    /// this is bounded by the windows somebody has open — and a server
    /// that let it grow without bound is a server whose memory only ever
    /// goes one way.
    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn uris(&self) -> impl Iterator<Item = &str> {
        self.held.keys().map(String::as_str)
    }

    /// `textDocument/didOpen`: the whole text, whatever was there before.
    pub fn opened(&mut self, uri: &str, text: String, version: i64) {
        self.held
            .insert(uri.to_string(), Document { text, version });
    }

    /// `textDocument/didClose`: forgotten entirely.
    ///
    /// Not merely for the memory, though a session that opens two
    /// hundred pages and forgets none of them is a session that grows by
    /// every one of them. The editor stops showing a closed file's
    /// underlines only because the server says there are none; and a
    /// question about a document nobody has open must be answered with
    /// nothing rather than from a text that is now out of date.
    pub fn closed(&mut self, uri: &str) -> bool {
        self.held.remove(uri).is_some()
    }

    /// `textDocument/didChange`: every change of the notification, in
    /// the order the editor sent them.
    ///
    /// A change for a document that was never opened is ignored. The
    /// alternative — inventing the document from whatever the change
    /// carried — is how a fragment of a page came to be treated as the
    /// whole of it.
    pub fn changed(&mut self, uri: &str, version: Option<i64>, changes: &[Change]) -> Applied {
        let Some(held) = self.held.get_mut(uri) else {
            return Applied::Ignored;
        };
        // A version that has gone *backwards* is a change the editor has
        // already replaced. LSP delivers notifications in order, so this
        // should never fire; it is here because a change applied out of
        // order leaves the document permanently wrong and nothing
        // afterwards notices.
        //
        // Strictly backwards, and not merely "not forwards": a client
        // that numbers every change the same — which is not conforming,
        // but is cheap to survive — would otherwise have its document
        // frozen at the first keystroke, and a frozen document is the
        // worse of the two failures by far.
        if let Some(version) = version
            && version < held.version
        {
            return Applied::Ignored;
        }
        if changes.is_empty() {
            if let Some(version) = version {
                held.version = version;
            }
            return Applied::Ignored;
        }
        for change in changes {
            match change {
                Change::Whole(text) => {
                    held.text.clear();
                    held.text.push_str(text);
                }
                Change::Ranged { start, end, text } => {
                    let from = offset_of(&held.text, start.line, start.character);
                    let to = offset_of(&held.text, end.line, end.character).max(from);
                    held.text.replace_range(from..to, text);
                }
            }
        }
        if let Some(version) = version {
            held.version = version;
        }
        Applied::Fresh
    }
}

/// A position as LSP writes one: a line counted from zero, and a
/// character counted from zero in UTF-16 code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

/// One entry of `contentChanges`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The whole document, which is what `textDocumentSync: 1` asks for.
    Whole(String),
    /// A range of it, which is what a client sends when it has decided
    /// to send ranges.
    Ranged {
        start: Position,
        end: Position,
        text: String,
    },
}

/// The changes a `didChange` carries, read from its `params`.
///
/// A change with a `range` is a ranged one and a change without is the
/// whole text; that is exactly how the specification distinguishes them,
/// and reading the two apart is what keeps a keystroke from being
/// mistaken for a document.
pub fn changes_in(params: &Value) -> Vec<Change> {
    let mut found = Vec::new();
    for change in params.get("contentChanges").items() {
        let Some(text) = change.get("text").as_str() else {
            continue;
        };
        let range = change.get("range");
        if range.is_null() {
            found.push(Change::Whole(text.to_string()));
            continue;
        }
        let (Some(start), Some(end)) = (
            position_in(range.get("start")),
            position_in(range.get("end")),
        ) else {
            // A `range` that is not a range. Taking the text for the
            // whole document here is the mistake this module exists to
            // prevent, so the change is dropped and the document is
            // left as the editor last described it in full.
            continue;
        };
        found.push(Change::Ranged {
            start,
            end,
            text: text.to_string(),
        });
    }
    found
}

/// A `{"line":…,"character":…}`.
pub fn position_in(value: &Value) -> Option<Position> {
    Some(Position {
        line: value.get("line").as_index()?,
        character: value.get("character").as_index()?,
    })
}

/// The byte offset in `text` of a zero-based line and a zero-based
/// UTF-16 character offset within it.
///
/// Clamped, both ways, because an editor is allowed to name a position
/// past the end of a line or past the end of the file and the
/// specification says what that means: the end of the line, and the end
/// of the file.
pub fn offset_of(text: &str, line: usize, character: usize) -> usize {
    let start = line_start(text, line);
    let rest = &text[start..];
    let line_text = match rest.find('\n') {
        Some(at) => &rest[..at],
        None => rest,
    };
    start + utf16_to_byte(line_text, character)
}

/// The byte offset where a zero-based line begins, clamped to the end of
/// the text.
pub fn line_start(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    let mut seen = 0;
    for (at, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            seen += 1;
            if seen == line {
                return at + 1;
            }
        }
    }
    text.len()
}

/// How far into `line_text` a UTF-16 offset reaches, in bytes.
///
/// A character outside the basic plane — an emoji, most of the symbols a
/// design file is full of — is two UTF-16 units and one Rust `char`. A
/// position that lands between its two halves is rounded down to the
/// character itself, which is the only answer that is a position in a
/// Rust string at all.
pub fn utf16_to_byte(line_text: &str, character: usize) -> usize {
    let mut units = 0;
    for (at, letter) in line_text.char_indices() {
        if units >= character {
            return at;
        }
        units += letter.len_utf16();
        if units > character {
            // The position was inside this character.
            return at;
        }
    }
    // Carriage returns are part of the line as far as this server is
    // concerned — the text is kept exactly as the editor sent it — but a
    // column past the end of the line clamps to the end of the line, and
    // for a CRLF file that is in front of the `\r`.
    line_text
        .strip_suffix('\r')
        .map_or(line_text.len(), str::len)
}

/// The reverse: how many UTF-16 units of `line_text` come before the
/// byte offset `at`.
///
/// Used on the way out. The compiler counts characters and an editor
/// counts UTF-16 units, so a diagnostic on a line holding an emoji was
/// published one column to the left of where it belonged.
pub fn byte_to_utf16(line_text: &str, at: usize) -> usize {
    line_text
        .get(..at.min(line_text.len()))
        .unwrap_or(line_text)
        .chars()
        .map(char::len_utf16)
        .sum()
}

/// The same, from a count of characters rather than a count of bytes —
/// which is the unit the compiler's own columns are in.
pub fn chars_to_utf16(line_text: &str, characters: usize) -> usize {
    line_text
        .chars()
        .take(characters)
        .map(char::len_utf16)
        .sum()
}

/// A position an editor sent, as a count of characters into its line.
///
/// The unit everything inside this server counts in: `inside`, the
/// position map and the compiler's own columns are all characters, and
/// an editor's are UTF-16 code units. The two differ by one for every
/// emoji earlier on the line, which is why a completion on a line with
/// one in it was being read a word to the left.
pub fn character_in(text: &str, line: usize, character: usize) -> usize {
    let line_text = line_at(text, line);
    let at = utf16_to_byte(line_text, character);
    line_text.get(..at).unwrap_or(line_text).chars().count()
}

/// The reverse, for a position on its way out to an editor.
pub fn utf16_in(text: &str, line: usize, character: usize) -> usize {
    chars_to_utf16(line_at(text, line), character)
}

/// One line of a text, by its zero-based number, without its ending.
pub fn line_at(text: &str, line: usize) -> &str {
    let start = line_start(text, line);
    let rest = &text[start..];
    match rest.find('\n') {
        Some(at) => &rest[..at],
        None => rest,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Applied, Change, Documents, Position, byte_to_utf16, chars_to_utf16, line_at, offset_of,
        utf16_to_byte,
    };

    fn whole(text: &str) -> Vec<Change> {
        vec![Change::Whole(text.to_string())]
    }

    fn ranged(start: (usize, usize), end: (usize, usize), text: &str) -> Vec<Change> {
        vec![Change::Ranged {
            start: Position {
                line: start.0,
                character: start.1,
            },
            end: Position {
                line: end.0,
                character: end.1,
            },
            text: text.to_string(),
        }]
    }

    #[test]
    fn a_whole_text_change_replaces_the_document() {
        let mut held = Documents::new();
        held.opened("u", "one".to_string(), 1);
        assert_eq!(held.changed("u", Some(2), &whole("two")), Applied::Fresh);
        assert_eq!(held.text("u"), Some("two"));
    }

    #[test]
    fn a_ranged_change_is_applied_to_the_text_and_not_mistaken_for_it() {
        // The bug: a client that sends ranges had its three characters
        // taken for the whole document.
        let mut held = Documents::new();
        held.opened("u", "let n = 0;\nlet m = 1;\n".to_string(), 1);
        held.changed("u", Some(2), &ranged((1, 8), (1, 9), "42"));
        assert_eq!(held.text("u"), Some("let n = 0;\nlet m = 42;\n"));
    }

    #[test]
    fn several_changes_of_one_notification_are_applied_in_order() {
        let mut held = Documents::new();
        held.opened("u", "abcd".to_string(), 1);
        let changes = vec![
            Change::Ranged {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
                text: "X".to_string(),
            },
            Change::Ranged {
                start: Position {
                    line: 0,
                    character: 3,
                },
                end: Position {
                    line: 0,
                    character: 4,
                },
                text: "Y".to_string(),
            },
        ];
        held.changed("u", Some(2), &changes);
        assert_eq!(held.text("u"), Some("XbcY"));
    }

    #[test]
    fn a_change_for_a_document_nobody_opened_is_ignored() {
        let mut held = Documents::new();
        assert_eq!(held.changed("u", Some(2), &whole("x")), Applied::Ignored);
        assert_eq!(held.text("u"), None);
    }

    #[test]
    fn a_change_the_editor_has_already_replaced_is_ignored() {
        let mut held = Documents::new();
        held.opened("u", "one".to_string(), 7);
        assert_eq!(
            held.changed("u", Some(6), &whole("stale")),
            Applied::Ignored
        );
        assert_eq!(held.text("u"), Some("one"));
        assert_eq!(held.changed("u", Some(8), &whole("fresh")), Applied::Fresh);
        assert_eq!(held.text("u"), Some("fresh"));
    }

    #[test]
    fn a_closed_document_is_forgotten() {
        let mut held = Documents::new();
        held.opened("u", "one".to_string(), 1);
        assert!(held.closed("u"));
        assert_eq!(held.text("u"), None);
        assert_eq!(held.len(), 0);
        assert!(!held.closed("u"));
    }

    #[test]
    fn a_column_is_counted_in_utf16_units() {
        // `😀` is one Rust char, two UTF-16 units, four bytes.
        let line = "😀ab";
        assert_eq!(utf16_to_byte(line, 0), 0);
        // Inside the emoji: the emoji itself.
        assert_eq!(utf16_to_byte(line, 1), 0);
        assert_eq!(utf16_to_byte(line, 2), 4);
        assert_eq!(utf16_to_byte(line, 3), 5);
        assert_eq!(byte_to_utf16(line, 4), 2);
        assert_eq!(chars_to_utf16(line, 1), 2);
    }

    #[test]
    fn an_edit_after_an_emoji_lands_where_the_editor_meant_it() {
        let mut held = Documents::new();
        held.opened("u", "<p>😀</p>".to_string(), 1);
        // The editor says character 5: `<p>` is three units, the emoji
        // two, so that is just after the emoji.
        held.changed("u", Some(2), &ranged((0, 5), (0, 5), "!"));
        assert_eq!(held.text("u"), Some("<p>😀!</p>"));
    }

    #[test]
    fn a_position_past_the_end_of_a_line_is_the_end_of_the_line() {
        assert_eq!(offset_of("ab\ncd\n", 0, 99), 2);
        assert_eq!(offset_of("ab\ncd\n", 1, 99), 5);
        assert_eq!(offset_of("ab\ncd\n", 99, 0), 6);
    }

    #[test]
    fn a_crlf_line_ends_in_front_of_its_carriage_return() {
        assert_eq!(offset_of("ab\r\ncd", 0, 99), 2);
        assert_eq!(line_at("ab\r\ncd", 0), "ab\r");
        assert_eq!(line_at("ab\r\ncd", 1), "cd");
    }

    #[test]
    fn an_insertion_at_the_very_end_of_a_document_works() {
        let mut held = Documents::new();
        held.opened("u", "fn f() {}".to_string(), 1);
        held.changed("u", Some(2), &ranged((0, 9), (0, 9), "\n"));
        assert_eq!(held.text("u"), Some("fn f() {}\n"));
    }

    #[test]
    fn a_reversed_range_does_not_panic() {
        let mut held = Documents::new();
        held.opened("u", "abcdef".to_string(), 1);
        held.changed("u", Some(2), &ranged((0, 4), (0, 2), "X"));
        // Nothing is removed and the text is inserted at the start of
        // the range, which is the only reading that cannot lose text.
        assert_eq!(held.text("u"), Some("abcdXef"));
    }
}
