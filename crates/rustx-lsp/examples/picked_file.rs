//! The file an expression question is asked against, printed.
//!
//! `cargo run -p rustx-lsp --example picked_file -- <page.rsx> <line> <column>`
//!
//! The generated half is transpiled here and now, exactly as the server
//! holds it: a file left in `target/` by an earlier build is not what a
//! question is asked against, and reading one is how a working server
//! comes to look broken.

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [page, line, column] = arguments.as_slice() else {
        eprintln!("picked_file <page.rsx> <line> <column>");
        std::process::exit(2);
    };
    let (line, column) = (
        line.parse::<usize>().unwrap(),
        column.parse::<usize>().unwrap(),
    );
    let source = std::fs::read_to_string(page).unwrap();
    let made = rustx_compiler::transpile::transpile(&source)
        .expect("the page transpiles")
        .rust;
    match rustx_lsp::picked::file(&source, &made, line, column) {
        Some(file) => {
            for (which, text) in file.lines().enumerate() {
                println!("{:>4} {text}", which + 1);
            }
        }
        None => println!("no file: the cursor is not in an expression"),
    }
}
