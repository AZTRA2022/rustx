//! What the editor will offer as tags, for one application.
//!
//! `cargo run -p rustx-lsp --example what_is_installed -- <app directory>`
//!
//! A diagnostic: an editor that offers no `CardHeader` is either an
//! editor that did not ask, or an application whose `node_modules` says
//! nothing. This answers which, in one line per library.

use std::collections::BTreeMap;
use std::path::Path;

fn main() {
    let Some(application) = std::env::args().nth(1) else {
        eprintln!("what_is_installed <app directory>   (the one with package.json in it)");
        std::process::exit(2);
    };
    let application = Path::new(&application);
    let found = rustx_lsp::libraries::read(application);
    if found.is_empty() {
        println!(
            "nothing. {} has no package.json, no node_modules, or no components in them.",
            application.display()
        );
        return;
    }

    let mut by_library: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for export in &found {
        by_library
            .entry(export.from.clone())
            .or_default()
            .push(export.name.clone());
    }
    println!("{} names, from {} libraries:\n", found.len(), by_library.len());
    for (library, mut names) in by_library {
        names.sort();
        println!("{library} ({})", names.len());
        println!("  {}\n", names.join(", "));
    }
}
