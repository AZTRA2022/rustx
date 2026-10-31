//! What the compiler refused, as an editor reads it.
//!
//! `CompileError` already points at the `.rsx` — §7.3 asks the compiler
//! to keep the coordinates of the file the developer wrote, and that is
//! exactly what an editor needs. The only translation is the counting:
//! the compiler counts from one, LSP counts from zero.

use crate::json;

/// One underline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Counted from zero, as LSP counts.
    pub line: u32,
    pub column: u32,
    /// How far the underline runs. A word, or what is left of the line.
    pub length: u32,
    pub message: String,
}

/// What this page gets wrong, if anything.
///
/// A panic in the compiler publishes nothing at all: an editor whose
/// server dies on a half-typed tag is worse than an editor that
/// underlines nothing for a moment, and a half-typed tag is what every
/// `.rsx` is most of the time.
pub fn of(source: &str) -> Vec<Diagnostic> {
    let compiled = std::panic::catch_unwind(|| rustx_compiler::transpile::transpile(source));
    match compiled {
        Ok(Ok(_)) => Vec::new(),
        Ok(Err(error)) => vec![Diagnostic {
            line: error.line.saturating_sub(1) as u32,
            column: error.column.saturating_sub(1) as u32,
            length: word_at(source, error.line, error.column),
            message: error.message,
        }],
        Err(_) => Vec::new(),
    }
}

/// How far to underline from where the compiler pointed.
///
/// To the end of the word, which is the tag or the attribute that was
/// refused; to the end of the line when the point is not on a word, so
/// that something is underlined rather than nothing.
fn word_at(source: &str, line: usize, column: usize) -> u32 {
    let Some(text) = source.lines().nth(line.saturating_sub(1)) else {
        return 1;
    };
    let from = column.saturating_sub(1);
    let rest: String = text.chars().skip(from).collect();
    let word = rest
        .chars()
        .take_while(|letter| letter.is_alphanumeric() || *letter == '_' || *letter == '-')
        .count();
    if word > 0 {
        word as u32
    } else {
        rest.chars().count().max(1) as u32
    }
}

/// The notification an editor is waiting for.
pub fn notification(uri: &str, found: &[Diagnostic]) -> String {
    let items: Vec<String> = found
        .iter()
        .map(|item| {
            format!(
                "{{\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\
                 \"end\":{{\"line\":{},\"character\":{}}}}},\
                 \"severity\":1,\"source\":\"rustx\",\"message\":\"{}\"}}",
                item.line,
                item.column,
                item.line,
                item.column + item.length,
                json::escape(&item.message)
            )
        })
        .collect();

    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\
         \"params\":{{\"uri\":\"{}\",\"diagnostics\":[{}]}}}}",
        json::escape(uri),
        items.join(",")
    )
}
