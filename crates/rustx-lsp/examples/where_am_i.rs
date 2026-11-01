//! What the server thinks the cursor is in the middle of.
//!
//! `cargo run -p rustx-lsp --example where_am_i -- <page.rsx> <line> <column>`
//!
//! A diagnostic, like the device layer's `explain()`: when an editor
//! offers nothing, the question is always whether the server read the
//! position differently from the developer, and this is the shortest way
//! to find out.

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [page, line, column] = arguments.as_slice() else {
        eprintln!("where_am_i <page.rsx> <line> <column>   (both one-based)");
        std::process::exit(2);
    };
    let (Ok(line), Ok(column)) = (line.parse::<usize>(), column.parse::<usize>()) else {
        eprintln!("the line and the column are numbers, counted from one");
        std::process::exit(2);
    };
    let Ok(source) = std::fs::read_to_string(page) else {
        eprintln!("{page} could not be read");
        std::process::exit(1);
    };

    let text = source.lines().nth(line - 1).unwrap_or_default();
    println!("line {line}: {text:?}");
    println!(
        "up to column {column}: {:?}",
        text.chars().take(column - 1).collect::<String>()
    );
    println!("{:?}", rustx_lsp::inside::at(&source, line, column));
}
