//! The file a question about an expression is asked against.
//!
//! A JSX region becomes one long line of Rust, so a column inside it has
//! no counterpart: that is why the position map calls those positions
//! opaque, and why completion stopped at the `<`. What `rust-analyzer`
//! *can* be asked about is a file where the region is blank except for
//! the one expression under the cursor, laid at its own line and its own
//! column.
//!
//! Everything else in that file is the real generated code — the
//! signature, the `let` above, the shared state, the other functions —
//! so the types are the real types. And since the expression keeps its
//! position, nothing has to be translated in either direction: the
//! question goes out at the position the editor sent, and the answer
//! comes back at a position that is already right.
//!
//! The file does not type-check: an expression standing where an element
//! was has the wrong type. That is fine, and deliberate —
//! `rust-analyzer` completes through a type error without blinking. What
//! it cannot do is complete through a file it cannot *parse*, which is
//! why the braces that followed the region are kept.

use crate::inside::{self, Place};

/// Build that file, or `None` when the cursor is not in an expression.
pub fn file(source: &str, generated: &str, line: usize, column: usize) -> Option<String> {
    let Place::Expression { from } = inside::at(source, line, column) else {
        return None;
    };
    let cursor = inside::offset_of(source, line, column)?;
    let expression = source.get(from..end_of(source, from, cursor))?;
    let (first, last, prefix, trailing) = region_around(source, cursor)?;

    let mut lines: Vec<String> = generated.lines().map(str::to_string).collect();
    while lines.len() < last {
        lines.push(String::new());
    }

    // The region's own text goes, and what surrounded it on its first
    // and last lines stays: a `return ` before it, a `; }` after it. An
    // unbalanced brace is a file nobody can parse, and then nothing
    // works anywhere in it.
    for number in first..=last {
        let at = number - 1;
        lines[at] = match (number == first, number == last) {
            (true, true) => format!("{prefix}{}{trailing}", " ".repeat(gap(&prefix, &trailing))),
            (true, false) => prefix.clone(),
            (false, true) => trailing.clone(),
            (false, false) => String::new(),
        };
    }

    // And the expression where the developer wrote it.
    let (expression_line, expression_column) = place_of(source, from);
    for (which, piece) in expression.split('\n').enumerate() {
        let number = expression_line + which;
        let at = number - 1;
        while lines.len() <= at {
            lines.push(String::new());
        }
        let column = if which == 0 { expression_column } else { 1 };
        lines[at] = laid(&lines[at], column, piece.trim_end_matches('\r'));
    }

    let mut out = lines.join("\n");
    out.push('\n');
    Some(out)
}

/// Put `piece` at `column` of `line`, keeping what is before it and
/// whatever of the line reaches past the two.
fn laid(line: &str, column: usize, piece: &str) -> String {
    let letters: Vec<char> = line.chars().collect();
    let mut out: String = letters.iter().take(column - 1).collect();
    while out.chars().count() < column - 1 {
        out.push(' ');
    }
    out.push_str(piece);
    // What the piece did not cover, if the line was longer: the `; }`
    // that followed a one-line region lives there.
    let after = column - 1 + piece.chars().count();
    if letters.len() > after {
        out.extend(letters.iter().skip(after));
    }
    out
}

/// How many spaces stand between a one-line region's prefix and what
/// followed it.
///
/// Enough that the expression, which is laid down afterwards, has room
/// to land at its own column. The expression is inside the region, so
/// its column is past the prefix; the spaces only have to reach that
/// far, and `laid` keeps whatever of the line is past the expression.
fn gap(prefix: &str, trailing: &str) -> usize {
    let _ = trailing;
    // A region is at least `<a/>`; in practice the expression is well
    // inside it. The width of the prefix is the one thing that must not
    // be disturbed, so the gap is counted from there.
    prefix.chars().count().max(1)
}

/// The region around `cursor`: its first and last line, what precedes it
/// on the first line, and what follows it on the last.
///
/// Read from the source with the compiler's own scanner when the file
/// scans — which is most of the time, because an unfinished tag inside a
/// region does not stop the region from being found. When it does not
/// scan at all, the lines are found by looking for the `<` that opened
/// the region and the `>` that closed it, which is enough to lay an
/// expression down.
fn region_around(source: &str, cursor: usize) -> Option<(usize, usize, String, String)> {
    let (start, end) = match rustx_compiler::scan::find_jsx(source) {
        Ok(spans) => spans
            .into_iter()
            .map(|span| (span.start, span.end))
            .find(|(start, end)| cursor >= *start && cursor <= *end)?,
        Err(_) => guessed(source, cursor)?,
    };

    let (first, first_column) = place_of(source, start);
    let (last, _) = place_of(source, end.min(source.len()));
    let prefix: String = line_text(source, first)
        .chars()
        .take(first_column - 1)
        .collect();
    let trailing: String = {
        let (_, end_column) = place_of(source, end.min(source.len()));
        line_text(source, last)
            .chars()
            .skip(end_column - 1)
            .collect()
    };
    Some((first, last, prefix, trailing))
}

/// Where a region starts and ends, on a file the scanner refuses.
///
/// Backwards to the `<` that opened it, forwards to the end of the line
/// that holds the last `>` — a guess, used only when the file is in a
/// state nothing can parse, which is the state a file is in while
/// somebody types in it.
fn guessed(source: &str, cursor: usize) -> Option<(usize, usize)> {
    let start = source
        .get(..cursor)?
        .char_indices()
        .rfind(|(_, letter)| *letter == '<')
        .map(|(at, _)| at)?;
    let end = source[cursor..]
        .find('\n')
        .map_or(source.len(), |at| cursor + at);
    Some((start, end))
}

/// Where the expression ends: its matching `}`, or the cursor when
/// nobody has closed it yet.
fn end_of(source: &str, from: usize, cursor: usize) -> usize {
    let mut depth = 1_usize;
    let mut inside_text: Option<char> = None;
    let mut escaped = false;
    for (at, letter) in source[from..].char_indices() {
        let at = from + at;
        if let Some(quote) = inside_text {
            match letter {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                letter if letter == quote => inside_text = None,
                _ => {}
            }
            continue;
        }
        match letter {
            '"' | '\'' => inside_text = Some(letter),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return at;
                }
            }
            _ => {}
        }
    }
    // Nobody closed it. What there is of it is what is before the
    // cursor, which is exactly what completion is about.
    cursor.max(from)
}

/// The one-based line and column of a byte offset.
fn place_of(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rfind('\n')
        .map_or(before.chars().count(), |newline| {
            before[newline + 1..].chars().count()
        })
        + 1;
    (line, column)
}

/// One line of a text, by its one-based number.
fn line_text(source: &str, line: usize) -> &str {
    source.lines().nth(line - 1).unwrap_or("")
}
