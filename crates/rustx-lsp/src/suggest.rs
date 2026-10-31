//! What the server offers, out of its own head.
//!
//! The tags and the attributes come from the compiler's own lists — the
//! ones it validates against — so what an editor offers and what the
//! compiler accepts cannot drift apart. The components come from the
//! file being typed in and from what the application installed.
//!
//! Nothing here compiles anything or asks anybody: a keystroke waits for
//! no process, and a developer whose `rust-analyzer` was never installed
//! still gets their tags.

use std::path::Path;

/// What kind of thing is being offered. The editor draws a different
/// icon for each, which is most of what makes a long list readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Tag,
    Component,
    Attribute,
    Event,
    Property,
}

impl Kind {
    /// The number LSP gives this kind.
    pub fn number(self) -> u32 {
        match self {
            // Field, Class, Property, Event, Property: the numbers from
            // the specification's `CompletionItemKind`.
            Self::Tag => 5,
            Self::Component => 7,
            Self::Attribute => 10,
            Self::Event => 23,
            Self::Property => 10,
        }
    }
}

/// One thing offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub label: String,
    pub kind: Kind,
    /// The line the editor shows beside the label.
    pub detail: String,
    /// What to put in the file, when it is not the label itself.
    pub insert: Option<String>,
}

impl Item {
    fn new(label: &str, kind: Kind, detail: &str) -> Self {
        Self {
            label: label.to_string(),
            kind,
            detail: detail.to_string(),
            insert: None,
        }
    }

    fn inserting(mut self, text: &str) -> Self {
        self.insert = Some(text.to_string());
        self
    }
}

/// What the file being typed in, and the application around it, bring.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    /// The components this file declares, each with its parameters as
    /// written.
    pub components: Vec<(String, Vec<String>)>,
    /// The component names the application's libraries export — the
    /// packages of its `package.json` and the files of `components/ui`.
    pub libraries: Vec<String>,
}

/// The tags to offer for what has been typed so far.
pub fn tags(typed: &str, page: &Page) -> Vec<Item> {
    let mut found: Vec<Item> = Vec::new();

    // The file's own components first: they are what a developer wrote
    // minutes ago, and they are the shortest list.
    for (name, parameters) in &page.components {
        if !matches(name, typed) {
            continue;
        }
        let detail = if parameters.is_empty() {
            "a component of this file".to_string()
        } else {
            format!("fn {name}({})", parameters.join(", "))
        };
        found.push(Item::new(name, Kind::Component, &detail));
    }

    for name in &page.libraries {
        if matches(name, typed) && !found.iter().any(|item| &item.label == name) {
            found.push(Item::new(
                name,
                Kind::Component,
                "a component of an installed library",
            ));
        }
    }

    for tag in rustx_compiler::tags::html() {
        if matches(tag, typed) {
            found.push(Item::new(tag, Kind::Tag, "HTML"));
        }
    }
    for tag in rustx_compiler::tags::svg() {
        if matches(tag, typed) && !found.iter().any(|item| item.label == *tag) {
            found.push(Item::new(tag, Kind::Tag, "SVG"));
        }
    }
    found
}

/// The one thing a `</` can become.
pub fn closing(open: &str) -> Vec<Item> {
    if open.is_empty() {
        return Vec::new();
    }
    vec![
        Item::new(&format!("{open}>"), Kind::Tag, "close what is open")
            .inserting(&format!("{open}>")),
    ]
}

/// The attributes to offer on `tag`, minus the ones already written.
pub fn attributes(tag: &str, typed: &str, page: &Page, already: &[String]) -> Vec<Item> {
    let mut found: Vec<Item> = Vec::new();

    // A component of this file takes what its function takes, and
    // nothing else: those are the only names the compiler will accept.
    if let Some((_, parameters)) = page
        .components
        .iter()
        .find(|(name, _)| name == tag.trim_start_matches("js::"))
    {
        for parameter in parameters {
            let (name, kind) = match parameter.split_once(':') {
                Some((name, kind)) => (name.trim(), kind.trim()),
                None => (parameter.as_str(), ""),
            };
            if matches(name, typed) && !already.iter().any(|written| written == name) {
                found
                    .push(Item::new(name, Kind::Property, kind).inserting(&format!("{name}={{}}")));
            }
        }
        return found;
    }

    let installed = page.libraries.iter().any(|name| name == tag);
    for name in rustx_compiler::attributes::known() {
        if matches(name, typed) && !already.iter().any(|written| written == name) {
            found.push(Item::new(
                name,
                Kind::Attribute,
                if installed {
                    "an attribute the library passes on"
                } else {
                    "HTML"
                },
            ));
        }
    }
    for name in rustx_compiler::attributes::events() {
        if matches(name, typed) && !already.iter().any(|written| written == name) {
            found.push(
                Item::new(name, Kind::Event, "a handler: |event| …")
                    .inserting(&format!("{name}={{move |_| }}")),
            );
        }
    }
    found
}

/// The keys of a `style={{ … }}`, in the spelling JavaScript uses.
pub fn style_keys(typed: &str) -> Vec<Item> {
    PROPERTIES
        .iter()
        .filter(|name| matches(name, typed))
        .map(|name| Item::new(name, Kind::Property, "CSS").inserting(&format!("{name}: ")))
        .collect()
}

/// What an application's own components are called, read off the files
/// `shadcn add` and friends leave behind.
///
/// `components/ui/button.tsx` exports `Button`; a page writes
/// `<Button>`. The file name is enough — reading the TypeScript to find
/// its exports is a TypeScript server's work, and that is another plan.
pub fn page_of(source: &str, path: &Path) -> Page {
    let components = rustx_compiler::promote::component_signatures(source);
    Page {
        components,
        libraries: installed(path),
    }
}

/// The component names the application around `page` installed.
fn installed(page: &Path) -> Vec<String> {
    let Some(application) = crate::crate_for_pages::application_of(page) else {
        return Vec::new();
    };
    let Some(root) = application.parent() else {
        return Vec::new();
    };
    let mut found = Vec::new();

    // The components installed as source — `components/ui/button.tsx`.
    if let Ok(entries) = std::fs::read_dir(root.join("components").join("ui")) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_source = path
                .extension()
                .and_then(|kind| kind.to_str())
                .is_some_and(|kind| matches!(kind, "ts" | "tsx" | "js" | "jsx"));
            if !is_source {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|name| name.to_str()) {
                // `button.tsx` is `Button`, `alert-dialog.tsx` is
                // `AlertDialog`: the spelling shadcn itself uses.
                found.push(capitalised(stem));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// `alert-dialog` becomes `AlertDialog`.
fn capitalised(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut next_is_capital = true;
    for letter in name.chars() {
        match letter {
            '-' | '_' | '.' => next_is_capital = true,
            letter if next_is_capital => {
                out.extend(letter.to_uppercase());
                next_is_capital = false;
            }
            letter => out.push(letter),
        }
    }
    out
}

/// Does `candidate` answer what has been typed?
///
/// A prefix, ignoring case, which is what an editor's own filter does
/// after us — being stricter here would hide what it would have kept.
fn matches(candidate: &str, typed: &str) -> bool {
    typed.is_empty() || candidate.to_lowercase().starts_with(&typed.to_lowercase())
}

/// The LSP answer for a list of items.
pub fn json(items: &[Item]) -> String {
    let mut out = String::from("[");
    for (at, item) in items.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"label\":\"{}\",\"kind\":{},\"detail\":\"{}\"",
            crate::json::escape(&item.label),
            item.kind.number(),
            crate::json::escape(&item.detail)
        ));
        if let Some(insert) = &item.insert {
            out.push_str(&format!(
                ",\"insertText\":\"{}\"",
                crate::json::escape(insert)
            ));
        }
        out.push('}');
    }
    out.push(']');
    out
}

/// The CSS properties a style object takes, in JavaScript spelling.
///
/// The ones a page actually writes. A complete list of every property
/// in every draft specification would be a thousand lines of noise in
/// front of somebody looking for `background`.
const PROPERTIES: &[&str] = &[
    "alignItems",
    "alignSelf",
    "background",
    "backgroundColor",
    "backgroundImage",
    "border",
    "borderBottom",
    "borderColor",
    "borderLeft",
    "borderRadius",
    "borderRight",
    "borderStyle",
    "borderTop",
    "borderWidth",
    "bottom",
    "boxShadow",
    "boxSizing",
    "color",
    "columnGap",
    "cursor",
    "display",
    "filter",
    "flex",
    "flexBasis",
    "flexDirection",
    "flexGrow",
    "flexShrink",
    "flexWrap",
    "font",
    "fontFamily",
    "fontSize",
    "fontStyle",
    "fontWeight",
    "gap",
    "grid",
    "gridColumn",
    "gridRow",
    "gridTemplateColumns",
    "gridTemplateRows",
    "height",
    "inset",
    "justifyContent",
    "justifySelf",
    "left",
    "letterSpacing",
    "lineHeight",
    "listStyle",
    "margin",
    "marginBottom",
    "marginLeft",
    "marginRight",
    "marginTop",
    "maxHeight",
    "maxWidth",
    "minHeight",
    "minWidth",
    "objectFit",
    "opacity",
    "outline",
    "overflow",
    "overflowX",
    "overflowY",
    "padding",
    "paddingBottom",
    "paddingLeft",
    "paddingRight",
    "paddingTop",
    "placeItems",
    "pointerEvents",
    "position",
    "right",
    "rowGap",
    "textAlign",
    "textDecoration",
    "textOverflow",
    "textTransform",
    "top",
    "transform",
    "transition",
    "userSelect",
    "verticalAlign",
    "visibility",
    "whiteSpace",
    "width",
    "wordBreak",
    "zIndex",
];
