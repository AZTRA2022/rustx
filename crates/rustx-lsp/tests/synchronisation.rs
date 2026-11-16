//! The document store against a second implementation of itself.
//!
//! This is the one part of a language server where being wrong is both
//! invisible and total. A change applied a character to the left leaves
//! a text that still looks like a text; nothing fails; and every
//! position the server computes for the rest of the session — every
//! underline, every completion, every hover — is one character out,
//! silently, until the file is closed. There is no symptom to notice and
//! no log to read.
//!
//! So it is checked against a second implementation that shares no code
//! with the first. `documents.rs` works in Rust `char`s and byte offsets
//! and clamps as it goes; the reference below works entirely in UTF-16
//! code units, which is the unit the specification is written in, and
//! splices a vector. If both agree on ten thousand random edits over
//! text full of emoji, combining marks and carriage returns, then the
//! translation between the two unit systems is right.
//!
//! The generator is a seeded one rather than a fuzzing tool: there is no
//! network here to install one with, and a deterministic sequence is
//! worth more anyway — a failure can be printed with its seed and run
//! again.

use rustx_lsp::documents::{Change, Documents, Position};

/// A reference implementation of "apply this change to this text",
/// written in the units the specification counts in and sharing nothing
/// with the one under test.
fn reference(text: &str, change: &Change) -> String {
    let Change::Ranged {
        start,
        end,
        text: inserted,
    } = change
    else {
        let Change::Whole(whole) = change else {
            unreachable!()
        };
        return whole.clone();
    };

    let units: Vec<u16> = text.encode_utf16().collect();
    let at = |place: &Position| -> usize {
        // The start of the line, counting the `\n`s — which is one unit
        // in UTF-16 as it is one byte in UTF-8.
        let mut offset = 0;
        let mut seen = 0;
        while offset < units.len() && seen < place.line {
            if units[offset] == u16::from(b'\n') {
                seen += 1;
            }
            offset += 1;
        }
        if seen < place.line {
            return units.len();
        }
        // The end of it.
        let mut end = offset;
        while end < units.len() && units[end] != u16::from(b'\n') {
            end += 1;
        }
        // A column past the end of the line is the end of the line, and
        // for a line that ends in a carriage return that is in front of
        // it.
        let mut wanted = (offset + place.character).min(end);
        if wanted == end && end > offset && units[end - 1] == u16::from(b'\r') {
            wanted = end - 1;
        }
        // A position inside a surrogate pair is the character it is
        // inside, which is the only one that is a position in a text.
        while wanted > offset && wanted < units.len() && (0xDC00..0xE000).contains(&units[wanted]) {
            wanted -= 1;
        }
        wanted
    };

    let from = at(start);
    let to = at(end).max(from);
    let mut out: Vec<u16> = units[..from].to_vec();
    out.extend(inserted.encode_utf16());
    out.extend_from_slice(&units[to..]);
    String::from_utf16(&out).expect("a text that is still a text")
}

/// A random number generator with no dependency and no surprises.
struct Dice(u64);

impl Dice {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }

    fn next(&mut self) -> u64 {
        // xorshift64*, which is four lines and plenty random enough to
        // land an edit in the middle of a surrogate pair.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn upto(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        (self.next() % bound as u64) as usize
    }

    fn one_of<'a, T>(&mut self, from: &'a [T]) -> &'a T {
        &from[self.upto(from.len())]
    }
}

/// The characters an edit may insert. Every one of them is a different
/// number of UTF-16 units, bytes, or both.
const LETTERS: &[&str] = &[
    "a",
    "z",
    " ",
    "\n",
    "\r\n",
    "<",
    ">",
    "/",
    "{",
    "}",
    "\"",
    // Two bytes, one unit.
    "é",
    "ü",
    // Three bytes, one unit.
    "€",
    "中",
    // Four bytes, two units: the case everything turns on.
    "😀",
    "𝕏",
    "🇫🇷",
    // A combining mark, which is a character of its own.
    "\u{301}",
    // And a few at once, which is what a paste is.
    "<div>",
    "</div>",
    "fn f() {}",
    "",
];

/// A text to start from, with every awkwardness in it at once.
fn starting_text(dice: &mut Dice) -> String {
    let mut text = String::new();
    for _ in 0..dice.upto(40) + 1 {
        for _ in 0..dice.upto(10) {
            text.push_str(dice.one_of(LETTERS));
        }
        text.push('\n');
    }
    text
}

/// A change somewhere in or around `text`.
///
/// Deliberately not always a valid one: a quarter of these name a
/// position past the end of a line or past the end of the file, and a
/// few are the wrong way round. An editor sends all of those, and the
/// specification says what each of them means.
fn a_change(dice: &mut Dice, text: &str) -> Change {
    let lines = text.lines().count().max(1);
    let line = match dice.upto(8) {
        0 => dice.upto(lines + 3),
        _ => dice.upto(lines),
    };
    let width = text
        .lines()
        .nth(line)
        .map_or(0, |line| line.encode_utf16().count());
    let pick = |dice: &mut Dice| match dice.upto(8) {
        0 => dice.upto(width + 4),
        _ => dice.upto(width + 1),
    };
    let first = pick(dice);
    let second = pick(dice);
    let second_line = if dice.upto(4) == 0 {
        (line + dice.upto(3)).min(lines)
    } else {
        line
    };
    let mut inserted = String::new();
    for _ in 0..dice.upto(4) {
        inserted.push_str(dice.one_of(LETTERS));
    }
    Change::Ranged {
        start: Position {
            line,
            character: first.min(second),
        },
        end: Position {
            line: second_line,
            character: if second_line == line {
                first.max(second)
            } else {
                second
            },
        },
        text: inserted,
    }
}

#[test]
fn ten_thousand_random_changes_agree_with_the_reference() {
    let mut checked = 0usize;
    for seed in 0..200u64 {
        let mut dice = Dice::new(seed);
        let mut held = Documents::new();
        let mut expected = starting_text(&mut dice);
        held.opened("file:///c:/p/app/page.rsx", expected.clone(), 1);

        for step in 0..50 {
            let change = a_change(&mut dice, &expected);
            expected = reference(&expected, &change);
            held.changed(
                "file:///c:/p/app/page.rsx",
                Some(step + 2),
                std::slice::from_ref(&change),
            );
            let got = held.text("file:///c:/p/app/page.rsx").expect("a text");
            assert_eq!(got, expected, "seed {seed}, step {step}, change {change:?}");
            checked += 1;
        }
    }
    assert_eq!(checked, 10_000);
}

#[test]
fn several_changes_in_one_notification_agree_with_the_reference() {
    // A client may send a list, and the specification says each is
    // applied to the text the one before it produced. A server that
    // computed every position against the original text would be wrong
    // from the second change onward.
    for seed in 200..260u64 {
        let mut dice = Dice::new(seed);
        let mut held = Documents::new();
        let mut expected = starting_text(&mut dice);
        held.opened("u", expected.clone(), 1);

        for step in 0..20 {
            let how_many = dice.upto(4) + 1;
            let mut changes = Vec::new();
            let mut walking = expected.clone();
            for _ in 0..how_many {
                let change = a_change(&mut dice, &walking);
                walking = reference(&walking, &change);
                changes.push(change);
            }
            expected = walking;
            held.changed("u", Some(step + 2), &changes);
            assert_eq!(
                held.text("u").expect("a text"),
                expected,
                "seed {seed}, step {step}, changes {changes:?}"
            );
        }
    }
}

#[test]
fn a_whole_text_change_is_always_exactly_the_text() {
    let mut dice = Dice::new(7);
    let mut held = Documents::new();
    held.opened("u", String::new(), 1);
    for step in 0..500 {
        let text = starting_text(&mut dice);
        held.changed("u", Some(step + 2), &[Change::Whole(text.clone())]);
        assert_eq!(held.text("u"), Some(text.as_str()));
    }
}

#[test]
fn the_text_is_always_valid_utf8_and_never_cut_inside_a_character() {
    // The property a `String` guarantees and a `Vec<u16>` does not: a
    // store that spliced at an arbitrary UTF-16 offset could leave half
    // a surrogate pair, which is not a text at all. The one under test
    // cannot, and this says so on positions chosen to land inside one.
    let mut held = Documents::new();
    held.opened("u", "😀😀😀".to_string(), 1);
    for (step, character) in [1usize, 3, 5, 0, 2, 4, 6, 99].iter().enumerate() {
        held.changed(
            "u",
            Some(step as i64 + 2),
            &[Change::Ranged {
                start: Position {
                    line: 0,
                    character: *character,
                },
                end: Position {
                    line: 0,
                    character: *character,
                },
                text: "x".to_string(),
            }],
        );
        let text = held.text("u").expect("a text");
        assert!(
            !text.contains('\u{fffd}'),
            "a character was cut in half at {character}: {text:?}"
        );
    }
}

#[test]
fn the_server_publishes_what_the_compiler_says_about_the_text_it_holds() {
    // The end-to-end form of the same property, through the whole
    // server: whatever sequence of changes arrives, the last thing the
    // editor is told must be exactly what the compiler says about the
    // text the editor is holding. If the store has drifted by one
    // character, this is where it shows.
    use rustx_lsp::{diagnostics, json, serve, wire};

    const URI: &str = "file:///c:/p/app/page.rsx";
    let base = "fn Page() -> Element {\n    return <p>bonjour</p>;\n}\n";

    for seed in 300..340u64 {
        let mut dice = Dice::new(seed);
        // The text the editor would be holding, kept by the reference.
        let mut expected = base.to_string();
        let mut stream: Vec<u8> = Vec::new();
        let push = |stream: &mut Vec<u8>, body: String| {
            stream.extend_from_slice(
                format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes(),
            );
        };
        push(
            &mut stream,
            format!(
                r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{URI}","languageId":"rsx","version":1,"text":"{}"}}}}}}"#,
                json::escape(&expected)
            ),
        );
        for step in 0..12 {
            let change = a_change(&mut dice, &expected);
            expected = reference(&expected, &change);
            let Change::Ranged { start, end, text } = &change else {
                unreachable!()
            };
            push(
                &mut stream,
                format!(
                    r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{URI}","version":{}}},"contentChanges":[{{"range":{{"start":{{"line":{},"character":{}}},"end":{{"line":{},"character":{}}}}},"text":"{}"}}]}}}}"#,
                    step + 2,
                    start.line,
                    start.character,
                    end.line,
                    end.character,
                    json::escape(text)
                ),
            );
        }
        push(
            &mut stream,
            r#"{"jsonrpc":"2.0","id":1,"method":"shutdown","params":null}"#.to_string(),
        );

        let mut output: Vec<u8> = Vec::new();
        serve(&mut std::io::Cursor::new(stream), &mut output);
        let said = String::from_utf8_lossy(&output).to_string();

        // The last thing published about this page.
        let pieces: Vec<&str> = said
            .split("Content-Length: ")
            .filter(|piece| piece.contains("publishDiagnostics"))
            .collect();
        let last = pieces
            .last()
            .unwrap_or_else(|| panic!("seed {seed}: nothing was published"));
        let message = wire::parse(&last[last.find('{').unwrap_or(0)..])
            .unwrap_or_else(|| panic!("seed {seed}: {last}"));
        let published: Vec<String> = message
            .get("params")
            .get("diagnostics")
            .items()
            .iter()
            .map(|found| {
                format!(
                    "{}:{}-{}",
                    found
                        .get("range")
                        .get("start")
                        .get("line")
                        .as_i64()
                        .unwrap_or(-1),
                    found
                        .get("range")
                        .get("start")
                        .get("character")
                        .as_i64()
                        .unwrap_or(-1),
                    found
                        .get("range")
                        .get("end")
                        .get("character")
                        .as_i64()
                        .unwrap_or(-1),
                )
            })
            .collect();

        let truth: Vec<String> = diagnostics::of(&expected)
            .iter()
            .map(|found| {
                format!(
                    "{}:{}-{}",
                    found.line,
                    found.column,
                    found.column + found.length
                )
            })
            .collect();
        assert_eq!(
            published, truth,
            "seed {seed}: what was published is not what the compiler says \
             about the text the editor holds.\ntext: {expected:?}"
        );
    }
}
