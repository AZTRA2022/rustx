//! `rustx-lsp`: the language server an editor starts.
//!
//! Standard input and standard output, which is how every editor speaks
//! to a language server. Everything it does is in the library beside
//! this file, so that a session can be tested without an editor.

fn main() {
    let input = std::io::stdin();
    let output = std::io::stdout();
    rustx_lsp::serve(&mut input.lock(), &mut output.lock());
}
