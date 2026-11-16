//! Two questions about nesting, answered by measurement rather than by
//! guesswork. A probe, not a test.
//!
//! `depth_probe` alone: the deepest tag nesting of every `.rsx` under
//! `apps/`, so the limit can be set with room above what is written.
//!
//! `depth_probe <depth> <stack-kib>`: whether a tree that deep survives
//! a stack that size.
use std::path::{Path, PathBuf};

fn main() {
    let mut given = std::env::args().skip(1);
    match (given.next(), given.next()) {
        (Some(depth), stack) => {
            let depth: usize = depth.parse().unwrap_or(32);
            let stack: usize = stack.and_then(|a| a.parse().ok()).unwrap_or(2048);
            let worker = std::thread::Builder::new()
                .stack_size(stack * 1024)
                .spawn(move || {
                    let source = format!(
                        "fn Page() -> Element {{ return {}{}; }}",
                        "<div>".repeat(depth),
                        "</div>".repeat(depth)
                    );
                    let built = rustx_compiler::transpile::transpile(&source);
                    eprintln!(
                        "depth {depth} on {stack} KiB: {}",
                        if built.is_ok() { "ok" } else { "refused" }
                    );
                })
                .expect("a worker thread");
            let _ = worker.join();
        }
        _ => survey(),
    }
}

fn survey() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut pages = Vec::new();
    gather(&root.join("apps"), &mut pages);
    gather(&root.join("site"), &mut pages);
    let mut deepest: Vec<(usize, PathBuf)> = pages
        .into_iter()
        .map(|page| {
            let source = std::fs::read_to_string(&page).unwrap_or_default();
            (deepest_nesting(&source), page)
        })
        .collect();
    deepest.sort_by_key(|(depth, _)| std::cmp::Reverse(*depth));
    for (depth, page) in deepest.iter().take(10) {
        println!("{depth:4}  {}", page.display());
    }
}

/// How deep the tags of this source nest, counted by the scanner's own
/// rule so that a `<` which is a comparison does not count.
fn deepest_nesting(source: &str) -> usize {
    let Ok(regions) = rustx_compiler::scan::find_jsx(source) else {
        return 0;
    };
    let mut worst = 0;
    for region in regions {
        let text = &source[region.start..region.end];
        let bytes = text.as_bytes();
        let mut depth = 0usize;
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'<' {
                if bytes.get(index + 1) == Some(&b'/') {
                    depth = depth.saturating_sub(1);
                } else {
                    // Self-closing tags open nothing.
                    let close = text[index..].find('>').map_or(text.len(), |at| index + at);
                    if !text[index..close].trim_end().ends_with('/') {
                        depth += 1;
                        worst = worst.max(depth);
                    }
                    index = close;
                }
            }
            index += 1;
        }
    }
    worst
}

fn gather(at: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            gather(&path, into);
        } else if path.extension().is_some_and(|kind| kind == "rsx") {
            into.push(path);
        }
    }
}
