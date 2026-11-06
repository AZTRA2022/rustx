//! What the JavaScript libraries of an application export.
//!
//! A page writes `<LineChart>` and the name comes from `recharts`;
//! `<CardHeader>` comes from `components/ui/card.tsx`, which
//! `shadcn add` wrote. Neither is in any list the framework keeps, so an
//! editor that offered only HTML tags and the page's own functions was
//! offering the small half.
//!
//! So the names are read where they are: the `package.json` says which
//! libraries, `node_modules/<package>` says what each one exports, and
//! `components/ui/*.tsx` says what the application wrote for itself.
//! Read, not compiled — a keystroke waits for no TypeScript server.
//!
//! # What is read, and what is not
//!
//! A declaration file is TypeScript, and this is not a TypeScript
//! parser. What it understands is the handful of forms every published
//! package uses to say "here is a thing you can import":
//!
//! ```text
//! export { Dialog, DialogTrigger }          // and `A as B`, which gives B
//! export { LineChart } from './LineChart'   // the re-export of an index
//! export declare const Button: …
//! export declare function Card(…)
//! export declare class Chart …
//! export function Separator(…)              // source, not a declaration
//! ```
//!
//! What it skips: `export type`, `export interface`, `export default`,
//! and anything whose name does not start with a capital — a component
//! is capitalised in every library in npm, and `cn` or `buttonVariants`
//! are not tags.
//!
//! A name it misses is a name an editor does not offer, which is a
//! smaller loss than a wrong one: the compiler and the runtime resolve
//! what is actually there, and both say so in a sentence when it is
//! not.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

/// A name a page may write as a tag, and where it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    /// `LineChart`.
    pub name: String,
    /// The library as a page writes it — `recharts`, `ui`,
    /// `radixUiReactDialog`. Also what goes in front of `::` when two
    /// libraries export one name.
    pub from: String,
}

/// How much of a declaration file is read.
///
/// Some packages ship a megabyte of `.d.ts`. The exports of an index are
/// at its top or its bottom; a file this long whose names are in the
/// middle loses a few, and keeps the editor answering in milliseconds.
const ENOUGH: usize = 512 * 1024;

/// What the application around `page` exports, as tags.
///
/// Cached per application, and thrown away when its `package.json` or
/// its `components/ui` changes: reading twenty declaration files on
/// every keystroke would be felt.
pub fn exports_for(application: &Path) -> Vec<Export> {
    let stamp = changed_at(application);
    let held = cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((when, found)) = held.get(application) {
        if *when == stamp {
            return found.clone();
        }
    }
    drop(held);

    let found = read(application);
    cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(application.to_path_buf(), (stamp, found.clone()));
    found
}

fn cache() -> &'static Mutex<HashMap<PathBuf, (Stamp, Vec<Export>)>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (Stamp, Vec<Export>)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// What says that the answer may have changed: the manifest, and the
/// folder the application's own components are in.
type Stamp = (Option<SystemTime>, Option<SystemTime>);

fn changed_at(application: &Path) -> Stamp {
    let when = |path: PathBuf| {
        std::fs::metadata(path)
            .and_then(|about| about.modified())
            .ok()
    };
    // The newest of the whole tree, and not the folder's own stamp: a
    // file added inside `components/ui` moves that folder and leaves
    // `components` untouched, so a stamp on the top folder alone never
    // noticed `shadcn add badge`.
    let own = rustx_compiler::bundle::own_components(application)
        .into_iter()
        .chain(std::iter::once(rustx_compiler::bundle::own_directory(
            application,
        )))
        .filter_map(when)
        .max();
    (when(application.join("package.json")), own)
}

/// The whole read, without the cache in front of it.
pub fn read(application: &Path) -> Vec<Export> {
    let mut found: Vec<Export> = Vec::new();

    // The application's own components first: they are the ones a page
    // means when both it and a library have the name.
    for file in rustx_compiler::bundle::own_components(application) {
        let Ok(source) = std::fs::read_to_string(&file) else {
            continue;
        };
        for name in exported_in(&source) {
            found.push(Export {
                name,
                from: "ui".to_string(),
            });
        }
        // And the default export, under the name of the file, which is
        // what the generated index calls it and therefore what a page
        // writes. A component a developer wrote is almost always one.
        if !rustx_compiler::bundle::has_a_default(&file) {
            continue;
        }
        let named = file
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(rustx_compiler::bundle::component_name);
        if let Some(name) = named {
            found.push(Export {
                name,
                from: "ui".to_string(),
            });
        }
    }

    for package in packages_of(application) {
        let from = rustx_compiler::bundle::name_for(&package);
        for name in exports_of_package(application, &package) {
            found.push(Export {
                name,
                from: from.clone(),
            });
        }
    }

    found.sort_by(|one, two| one.name.cmp(&two.name).then(one.from.cmp(&two.from)));
    found.dedup();
    found
}

/// The packages this application declares, minus the ones that are not
/// libraries of components.
fn packages_of(application: &Path) -> Vec<String> {
    let Ok(manifest) = std::fs::read_to_string(application.join("package.json")) else {
        return Vec::new();
    };
    rustx_compiler::bundle::dependencies_in(&manifest)
        .into_iter()
        .filter(|package| !NOT_COMPONENTS.contains(&package.as_str()))
        .collect()
}

/// Packages nobody writes as a tag. React itself is the boundary, not a
/// component; the rest are functions a page's TypeScript calls.
const NOT_COMPONENTS: &[&str] = &[
    "react",
    "react-dom",
    "clsx",
    "tailwind-merge",
    "class-variance-authority",
    "tailwindcss",
    "@tailwindcss/cli",
    "esbuild",
    "typescript",
];

/// What one installed package exports.
fn exports_of_package(application: &Path, package: &str) -> Vec<String> {
    let root = application.join("node_modules").join(package);
    let Some(types) = declarations_in(&root) else {
        return Vec::new();
    };
    let Ok(source) = std::fs::read_to_string(&types) else {
        return Vec::new();
    };
    let source = if source.len() > ENOUGH {
        // The top and the tail, which is where an index puts its
        // exports.
        let head = &source[..floor(&source, ENOUGH / 2)];
        let tail = &source[ceiling(&source, source.len() - ENOUGH / 2)..];
        format!("{head}\n{tail}")
    } else {
        source
    };
    exported_in(&source)
}

/// The declaration file a package points at, or the usual places.
fn declarations_in(root: &Path) -> Option<PathBuf> {
    if let Ok(manifest) = std::fs::read_to_string(root.join("package.json")) {
        for field in ["\"types\"", "\"typings\""] {
            if let Some(path) = value_of(&manifest, field) {
                let named = root.join(path.trim_start_matches("./"));
                if named.is_file() {
                    return Some(named);
                }
            }
        }
    }
    for usual in [
        "index.d.ts",
        "dist/index.d.ts",
        "dist/index.d.mts",
        "types/index.d.ts",
        "lib/index.d.ts",
    ] {
        let named = root.join(usual);
        if named.is_file() {
            return Some(named);
        }
    }
    None
}

/// The string a JSON field holds, read without a JSON parser: the field
/// is one line of a manifest, and a manifest is not arbitrary JSON when
/// npm wrote it.
fn value_of(manifest: &str, field: &str) -> Option<String> {
    let at = manifest.find(field)?;
    let rest = &manifest[at + field.len()..];
    let colon = rest.find(':')?;
    let rest = &rest[colon + 1..];
    let open = rest.find('"')?;
    let rest = &rest[open + 1..];
    let close = rest.find('"')?;
    Some(rest[..close].to_string())
}

/// Every capitalised name this TypeScript says can be imported.
pub fn exported_in(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in source.match_indices("export") {
        if !on_its_own(source, at) {
            continue;
        }
        let rest = source[at + "export".len()..].trim_start();
        if let Some(list) = rest.strip_prefix('{') {
            let Some(end) = list.find('}') else {
                continue;
            };
            for piece in list[..end].split(',') {
                // `Dialog as Root` is imported as `Root`.
                let name = piece
                    .rsplit(" as ")
                    .next()
                    .unwrap_or(piece)
                    .trim()
                    .trim_start_matches("type ")
                    .trim();
                keep(name, &mut found);
            }
            continue;
        }
        // `export declare const Button`, `export function Card`, and the
        // same without `declare`.
        let rest = rest.strip_prefix("declare").map_or(rest, str::trim_start);
        let rest = rest
            .strip_prefix("abstract")
            .map_or(rest, |rest| rest.trim_start());
        for word in ["const ", "function ", "class ", "let ", "var "] {
            if let Some(after) = rest.strip_prefix(word) {
                keep(name_at(after.trim_start()), &mut found);
                break;
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// A name worth offering as a tag.
fn keep(name: &str, found: &mut Vec<String>) {
    let name = name.trim();
    if name.is_empty() || !is_a_name(name) {
        return;
    }
    // A component is capitalised, everywhere in npm. `cn`, `cva` and
    // `buttonVariants` are not tags.
    if !name.starts_with(|letter: char| letter.is_ascii_uppercase()) {
        return;
    }
    found.push(name.to_string());
}

/// The identifier at the start of this text.
fn name_at(text: &str) -> &str {
    let end = text
        .find(|letter: char| !(letter.is_alphanumeric() || letter == '_' || letter == '$'))
        .unwrap_or(text.len());
    &text[..end]
}

fn is_a_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|letter| letter.is_alphanumeric() || letter == '_' || letter == '$')
}

/// Is the `export` at `at` a word, rather than the end of another one?
fn on_its_own(source: &str, at: usize) -> bool {
    let before = source[..at].chars().next_back();
    let after = source[at + "export".len()..].chars().next();
    let is_word = |letter: Option<char>| {
        letter.is_some_and(|letter| letter.is_alphanumeric() || letter == '_' || letter == '$')
    };
    !is_word(before) && !is_word(after)
}

/// A character boundary at or before `at`.
fn floor(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// A character boundary at or after `at`.
fn ceiling(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while at < text.len() && !text.is_char_boundary(at) {
        at += 1;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_of_exports_is_read() {
        let found = exported_in("declare const Dialog: unknown;\nexport { Dialog, DialogTrigger };");
        assert_eq!(found, vec!["Dialog", "DialogTrigger"]);
    }

    #[test]
    fn a_renamed_export_is_read_under_the_name_it_is_imported_by() {
        let found = exported_in("export { Root as Dialog, Trigger as DialogTrigger };");
        assert_eq!(found, vec!["Dialog", "DialogTrigger"]);
    }

    #[test]
    fn an_index_that_re_exports_is_read() {
        // How recharts says it.
        let found = exported_in("export { LineChart } from './chart/LineChart';");
        assert_eq!(found, vec!["LineChart"]);
    }

    #[test]
    fn a_declaration_is_read() {
        let found = exported_in(
            "export declare const Button: React.FC;\n\
             export declare function Card(props: P): JSX.Element;\n\
             export declare abstract class Chart {}\n\
             export function Separator(): JSX.Element {}",
        );
        assert_eq!(found, vec!["Button", "Card", "Chart", "Separator"]);
    }

    #[test]
    fn what_is_not_a_component_is_not_offered() {
        // Types are not tags; neither is a helper, and neither is a
        // default nobody can name.
        let found = exported_in(
            "export type ButtonProps = {};\n\
             export interface CardProps {}\n\
             export declare const buttonVariants: unknown;\n\
             export default Button;\n\
             export { cn };",
        );
        assert!(found.is_empty(), "got {found:?}");
    }

    #[test]
    fn the_word_export_inside_another_word_is_not_one() {
        let found = exported_in("declare const reexportedThing: unknown;\nexports.Button = 1;");
        assert!(found.is_empty(), "got {found:?}");
    }

    #[test]
    fn a_types_field_is_read_out_of_a_manifest() {
        assert_eq!(
            value_of("{\n  \"name\": \"recharts\",\n  \"types\": \"./types/index.d.ts\"\n}", "\"types\""),
            Some("./types/index.d.ts".to_string())
        );
    }

    #[test]
    fn an_application_with_nothing_installed_exports_nothing() {
        let nowhere = std::env::temp_dir().join("rustx-no-such-application");
        assert!(read(&nowhere).is_empty());
    }
}
