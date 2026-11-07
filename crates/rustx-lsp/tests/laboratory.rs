//! What the server offers for a real application, against what that
//! application actually has on disk.
//!
//! Every other test here builds its own fixture, which proves the rule
//! it was built for and nothing about the files a developer really has.
//! `labo-rustx` is the application this framework is read through: nine
//! components written by `shadcn add`, one written by hand, and a page
//! that uses them. If a name or a prop is missing in the editor, it is
//! missing here first.
//!
//! The reading below is deliberately naive and deliberately separate
//! from the server's: a test that called `libraries::exports_for` to
//! decide what to expect would be comparing the server to itself.

use std::path::{Path, PathBuf};

use rustx_lsp::{libraries, suggest};

/// The laboratory, when the sibling checkout is there.
fn laboratory() -> Option<PathBuf> {
    let beside = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../labo-rustx");
    beside.canonicalize().ok().filter(|here| here.is_dir())
}

/// Every `.tsx` the application wrote, however deep.
fn components_of(laboratory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut look = vec![laboratory.join("components")];
    while let Some(at) = look.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                look.push(path);
            } else if path.extension().is_some_and(|kind| kind == "tsx") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Is this a name a page could write as a tag?
fn is_a_tag_name(word: &str) -> bool {
    word.starts_with(|letter: char| letter.is_ascii_uppercase())
        && word
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '_')
}

/// The names a `.tsx` exports, read without the server's help.
///
/// `export { A, B }` across any number of lines, and
/// `export function A` / `export const A`.
fn exported_naively(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in source.match_indices("export") {
        let rest = source[at + "export".len()..].trim_start();
        if let Some(list) = rest.strip_prefix('{') {
            let Some(end) = list.find('}') else {
                continue;
            };
            for piece in list[..end].split(',') {
                let name = piece.rsplit(" as ").next().unwrap_or(piece).trim();
                if is_a_tag_name(name) {
                    found.push(name.to_string());
                }
            }
            continue;
        }
        for word in ["function ", "const ", "class "] {
            if let Some(after) = rest.strip_prefix(word) {
                let name: String = after
                    .trim_start()
                    .chars()
                    .take_while(|letter| letter.is_ascii_alphanumeric() || *letter == '_')
                    .collect();
                if is_a_tag_name(&name) {
                    found.push(name);
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The names `function NAME({ … })` pulls out of its props, read
/// without the server's help. The rest element is not a prop anybody
/// writes; the names before it are.
fn destructured_naively(source: &str, tag: &str) -> Vec<String> {
    let opening = format!("function {tag}(");
    let Some(at) = source.find(&opening) else {
        return Vec::new();
    };
    let rest = &source[at + opening.len()..];
    let rest = rest.trim_start();
    let Some(inner) = rest.strip_prefix('{') else {
        return Vec::new();
    };
    let Some(end) = inner.find('}') else {
        return Vec::new();
    };
    inner[..end]
        .split(',')
        .filter_map(|piece| {
            let piece = piece.trim();
            if piece.is_empty() || piece.starts_with("...") {
                return None;
            }
            let name = piece.split(['=', ':']).next().unwrap_or(piece).trim();
            let plain = name
                .chars()
                .all(|letter| letter.is_ascii_alphanumeric() || letter == '_');
            (plain && !name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// The page the server is asked about, and the laboratory it belongs to.
fn page_of(laboratory: &Path) -> (suggest::Page, PathBuf) {
    let path = laboratory.join("app/page.rsx");
    let source = std::fs::read_to_string(&path).expect("the laboratory has a page");
    (suggest::page_of(&source, &path), path)
}

#[test]
fn every_component_the_laboratory_installed_is_offered_as_a_tag() {
    let Some(laboratory) = laboratory() else {
        eprintln!("note: ../labo-rustx is not beside the repository; this proves nothing today");
        return;
    };

    let offered: Vec<String> = libraries::exports_for(&laboratory)
        .into_iter()
        .map(|export| export.name)
        .collect();

    let mut missing = Vec::new();
    for file in components_of(&laboratory) {
        let source = std::fs::read_to_string(&file).expect("a component reads");
        for name in exported_naively(&source) {
            if !offered.contains(&name) {
                missing.push(format!("{name} (from {})", file.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "installed and not offered as a tag: {missing:#?}"
    );
}

#[test]
fn every_prop_a_component_takes_is_offered_inside_its_tag() {
    let Some(laboratory) = laboratory() else {
        eprintln!("note: ../labo-rustx is not beside the repository; this proves nothing today");
        return;
    };
    let (page, _) = page_of(&laboratory);

    let mut missing = Vec::new();
    let mut checked = 0;
    for file in components_of(&laboratory) {
        let source = std::fs::read_to_string(&file).expect("a component reads");
        for tag in exported_naively(&source) {
            let wanted = destructured_naively(&source, &tag);
            if wanted.is_empty() {
                continue;
            }
            let offered = suggest::attributes(&tag, "", &page, &[]);
            for prop in &wanted {
                let found = offered
                    .iter()
                    .find(|item| &item.label == prop)
                    .map(|item| item.kind);
                match found {
                    Some(suggest::Kind::Property) => {}
                    Some(other) => missing.push(format!("<{tag} {prop}> offered as {other:?}")),
                    None => missing.push(format!("<{tag} {prop}> not offered at all")),
                }
            }
            checked += 1;
        }
    }

    assert!(checked > 0, "no component of the laboratory names its props");
    assert!(
        missing.is_empty(),
        "{checked} tags read, and these props are wrong or absent: {missing:#?}"
    );
}

#[test]
fn an_icon_off_npm_offers_the_props_its_declaration_leads_to() {
    // `<AArrowDown size={18} />` is on the laboratory's page, and
    // `size` is the only prop anybody ever writes on an icon. The
    // declaration does not hold its props: `declare const AArrowDown:
    // LucideIcon`, and `LucideIcon` is an alias for a wrapper around
    // `LucideProps`. Reading props has to follow that hop, or seven
    // thousand icons take nothing at all.
    let Some(laboratory) = laboratory() else {
        eprintln!("note: ../labo-rustx is not beside the repository; this proves nothing today");
        return;
    };
    if !laboratory
        .join("node_modules/lucide-react")
        .is_dir()
    {
        eprintln!("note: lucide-react is not installed in the laboratory");
        return;
    }

    let found = libraries::props_for(&laboratory, "AArrowDown");
    let names: Vec<&str> = found.iter().map(|prop| prop.name.as_str()).collect();
    assert!(
        names.contains(&"size"),
        "an icon's own props were not read: {names:?}"
    );
}
