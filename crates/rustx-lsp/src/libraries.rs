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

/// The props one tag takes, in the application around `page`.
///
/// Looked up by tag rather than listed for every export: `lucide-react`
/// alone has close to seven thousand of them, and reading every one of
/// their declarations on a keystroke would be felt. Cached on the same
/// stamp as the exports, so `shadcn add` and `npm install` are noticed.
pub fn props_for(application: &Path, tag: &str) -> Vec<Prop> {
    // `ui::Button` is `Button`, said in full.
    let tag = tag.rsplit("::").next().unwrap_or(tag);
    let asked = (application.to_path_buf(), tag.to_string());
    let stamp = changed_at(application);
    let held = props_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((when, found)) = held.get(&asked) {
        if *when == stamp {
            return found.clone();
        }
    }
    drop(held);

    let found = read_props(application, tag);
    props_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(asked, (stamp, found.clone()));
    found
}

fn props_cache() -> &'static Mutex<HashMap<(PathBuf, String), (Stamp, Vec<Prop>)>> {
    static CACHE: OnceLock<Mutex<HashMap<(PathBuf, String), (Stamp, Vec<Prop>)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn read_props(application: &Path, tag: &str) -> Vec<Prop> {
    // The application's own components first, for the same reason the
    // exports are read in that order: they are what a page means when
    // both it and a library have the name.
    for file in rustx_compiler::bundle::own_components(application) {
        let Ok(source) = std::fs::read_to_string(&file) else {
            continue;
        };
        let found = props_in(&source, tag);
        if !found.is_empty() {
            return found;
        }
    }
    for package in packages_of(application) {
        let root = application.join("node_modules").join(&package);
        let Some(types) = declarations_in(&root) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(&types) else {
            continue;
        };
        let found = props_in(&source, tag);
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

/// A prop a tag takes, and the type as it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prop {
    /// `variant`.
    pub name: String,
    /// `"default" | "outline"`, as written, or empty when nothing said.
    pub kind: String,
}

/// Every prop `tag` takes, read out of this TypeScript.
///
/// Three places, in the order a library is likely to have put them:
///
/// 1. `interface ButtonProps { … }` or `type ButtonProps = { … }` —
///    the convention npm follows almost without exception;
/// 2. the destructured parameter of the function itself, which is where
///    `shadcn add button` leaves them and nowhere else;
/// 3. the type the declaration names — `function Table(props: Settings)`
///    — followed to its own `interface` or `type`.
///
/// A prop it misses is one an editor does not offer, which is a smaller
/// loss than a wrong one: the compiler and React resolve what is really
/// there.
pub fn props_in(source: &str, tag: &str) -> Vec<Prop> {
    for named in [format!("{tag}Props"), format!("{tag}Properties")] {
        if let Some(body) = object_named(source, &named) {
            return fields_in(body);
        }
    }
    if let Some(found) = destructured_parameter_of(source, tag) {
        return found;
    }
    let Some(named) = props_type_of(source, tag) else {
        return Vec::new();
    };
    object_named(source, &named)
        .map(fields_in)
        .unwrap_or_default()
}

/// The `{ … }` of `interface NAME` or `type NAME =`.
fn object_named<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    for form in [format!("interface {name}"), format!("type {name}")] {
        let mut from = 0;
        while let Some(at) = source[from..].find(&form) {
            let at = from + at;
            from = at + form.len();
            if !on_a_boundary(source, at, form.len()) {
                continue;
            }
            // `interface X extends Y {` and `type X = { … }` both put
            // the object at the first brace after the name.
            let rest = &source[from..];
            let Some(open) = rest.find('{') else {
                continue;
            };
            // `type X = string;` has no object of its own.
            if rest[..open].contains(';') {
                continue;
            }
            let Some(close) = closing_brace(rest, open) else {
                continue;
            };
            return Some(&rest[open + 1..close]);
        }
    }
    None
}

/// The names a function destructures out of its props parameter.
///
/// `function Button({ className, variant, ...props }: …)`. The rest
/// element is not a prop anybody writes; the names before it are.
fn destructured_parameter_of(source: &str, tag: &str) -> Option<Vec<Prop>> {
    for form in [
        format!("function {tag}("),
        format!("const {tag} = ("),
        format!("const {tag} = function ("),
    ] {
        let Some(at) = source.find(&form) else {
            continue;
        };
        let rest = &source[at + form.len()..];
        let Some(open) = rest.find(|letter: char| !letter.is_whitespace()) else {
            continue;
        };
        if rest.as_bytes()[open] != b'{' {
            continue;
        }
        let Some(close) = closing_brace(rest, open) else {
            continue;
        };
        let found: Vec<Prop> = split_at_depth(&rest[open + 1..close])
            .into_iter()
            .filter_map(|piece| {
                let piece = piece.trim();
                if piece.is_empty() || piece.starts_with("...") {
                    return None;
                }
                // `asChild = false` is one name with a default.
                let name = piece
                    .split(['=', ':'])
                    .next()
                    .unwrap_or(piece)
                    .trim()
                    .to_string();
                is_a_name(&name).then(|| Prop {
                    name,
                    kind: String::new(),
                })
            })
            .collect();
        if !found.is_empty() {
            return Some(found);
        }
    }
    None
}

/// The name of the type a declaration says the props are.
fn props_type_of(source: &str, tag: &str) -> Option<String> {
    for form in [
        format!("function {tag}("),
        format!("const {tag}:"),
        format!("let {tag}:"),
    ] {
        let Some(at) = source.find(&form) else {
            continue;
        };
        let rest = &source[at + form.len()..];
        // `(props: Settings)` and `React.FC<ButtonProps>` both put the
        // name after the punctuation that introduces it.
        let Some(after) = rest.find([':', '<']) else {
            continue;
        };
        let name = name_at(rest[after + 1..].trim_start());
        if is_a_name(name) && name.starts_with(|letter: char| letter.is_ascii_uppercase()) {
            return Some(name.to_string());
        }
    }
    None
}

/// The fields of an object type, at its own depth and not deeper.
fn fields_in(body: &str) -> Vec<Prop> {
    split_at_depth(body)
        .into_iter()
        .filter_map(|piece| {
            let piece = piece.trim();
            if piece.is_empty() || piece.starts_with("//") || piece.starts_with('[') {
                return None;
            }
            let (written, kind) = match piece.split_once(':') {
                Some((written, kind)) => (written, kind.trim()),
                None => (piece, ""),
            };
            let name = written.trim().trim_end_matches('?').trim();
            is_a_name(name).then(|| Prop {
                name: name.to_string(),
                kind: kind.trim_end_matches(';').trim().to_string(),
            })
        })
        .collect()
}

/// One object's own pieces: split at the separators that belong to it,
/// so a nested `{ … }` or a `Map<a, b>` stays in one piece.
fn split_at_depth(body: &str) -> Vec<&str> {
    let bytes = body.as_bytes();
    let mut pieces = Vec::new();
    let mut depth = 0i32;
    let mut from = 0;
    let mut inside: Option<u8> = None;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(quote) = inside {
            if *byte == b'\\' {
                escaped = true;
            } else if *byte == quote {
                inside = None;
            }
            continue;
        }
        match *byte {
            b'"' | b'\'' | b'`' => inside = Some(*byte),
            b'{' | b'[' | b'(' | b'<' => depth += 1,
            b'}' | b']' | b')' | b'>' => depth -= 1,
            b';' | b',' | b'\n' if depth <= 0 => {
                pieces.push(&body[from..index]);
                from = index + 1;
            }
            _ => {}
        }
    }
    if from < body.len() {
        pieces.push(&body[from..]);
    }
    pieces
}

/// The brace matching the one at `open`.
fn closing_brace(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Is the text at `at` a whole word rather than the tail of one?
fn on_a_boundary(source: &str, at: usize, length: usize) -> bool {
    let bytes = source.as_bytes();
    let is_word = |byte: Option<&u8>| {
        byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'$')
    };
    !is_word(at.checked_sub(1).and_then(|before| bytes.get(before)))
        && !is_word(bytes.get(at + length))
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
    fn the_props_of_an_interface_are_read() {
        let found = props_in(
            "interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
                 variant?: \"default\" | \"outline\";
                 size?: Size;
                 asChild?: boolean;
             }
             export declare const Button: React.FC<ButtonProps>;",
            "Button",
        );
        let names: Vec<&str> = found.iter().map(|prop| prop.name.as_str()).collect();
        assert_eq!(names, vec!["variant", "size", "asChild"]);
        assert_eq!(found[2].kind, "boolean");
    }

    #[test]
    fn the_props_of_a_type_alias_are_read() {
        let found = props_in("type CardProps = {
  title: string;
  rows?: number;
};", "Card");
        let names: Vec<&str> = found.iter().map(|prop| prop.name.as_str()).collect();
        assert_eq!(names, vec!["title", "rows"]);
    }

    #[test]
    fn what_shadcn_destructures_is_read_off_the_function_itself() {
        // `components/ui/button.tsx`, as `shadcn add button` writes it:
        // no props type anywhere, the names are in the parameter.
        let found = props_in(
            "function Button({ className, variant, size, asChild = false, ...props }:              React.ComponentProps<\"button\">) {
  return null;
}",
            "Button",
        );
        let names: Vec<&str> = found.iter().map(|prop| prop.name.as_str()).collect();
        assert_eq!(names, vec!["className", "variant", "size", "asChild"]);
    }

    #[test]
    fn a_props_type_named_by_the_declaration_is_followed() {
        let found = props_in(
            "interface Settings {
  dense?: boolean;
}
             export declare function Table(props: Settings): JSX.Element;",
            "Table",
        );
        let names: Vec<&str> = found.iter().map(|prop| prop.name.as_str()).collect();
        assert_eq!(names, vec!["dense"]);
    }

    #[test]
    fn a_tag_with_no_props_anywhere_offers_none() {
        assert!(props_in("export declare const Spinner: React.FC;", "Spinner").is_empty());
    }

    #[test]
    fn a_nested_object_does_not_leak_its_own_fields() {
        let found = props_in(
            "type ChartProps = {
  margin?: { top: number; left: number };
  data: Row[];
};",
            "Chart",
        );
        let names: Vec<&str> = found.iter().map(|prop| prop.name.as_str()).collect();
        assert_eq!(names, vec!["margin", "data"]);
    }

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
