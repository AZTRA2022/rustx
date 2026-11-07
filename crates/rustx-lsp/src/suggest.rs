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
    ///
    /// Two kinds offered in the same list must not share one, or the
    /// distinction exists only here: a prop the component itself takes
    /// drew the same icon as `accesskey` for as long as both said `10`.
    /// A tag name and a prop may share one — they never meet.
    pub fn number(self) -> u32 {
        match self {
            // The numbers from the specification's `CompletionItemKind`.
            // Field, Class, Property, Event, Field.
            Self::Tag => 5,
            Self::Component => 7,
            Self::Attribute => 10,
            Self::Event => 23,
            // A prop is a field of this component, and not a property
            // every tag has: that is the whole reason it is offered
            // first.
            Self::Property => 5,
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
    /// packages of its `package.json` and the files of `components/ui`
    /// — each with the library it comes from.
    pub libraries: Vec<crate::libraries::Export>,
    /// The application this page belongs to, which is where the props of
    /// an installed tag are read from. Looked up when asked rather than
    /// listed up front: a library with seven thousand exports has
    /// seven thousand sets of props.
    pub application: Option<std::path::PathBuf>,
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

    // Then the tags the framework itself knows, before the installed
    // libraries. An application with `lucide-react` in it has seven
    // thousand names, and a list that long is cut somewhere: better at
    // the tail of the icons than at `div`.
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

    // Then what the application installed. The bare name, with the
    // library it comes from on the line beside it — because that is
    // what a page writes, and `ui::Button` is not something anybody
    // should have to type.
    for export in &page.libraries {
        if !matches(&export.name, typed) {
            continue;
        }
        if let Some(already) = found.iter_mut().find(|item| item.label == export.name) {
            // Two libraries export it, or the page declares it too. The
            // line says both, and the explicit form is offered below.
            if already.kind == Kind::Component && !already.detail.contains(&export.from) {
                already.detail = format!("{} · {}", already.detail, export.from);
            }
            continue;
        }
        found.push(Item::new(&export.name, Kind::Component, &export.from));
    }

    // And the explicit form, for a name two libraries export: a page
    // that wants the other one writes `recharts::LineChart`, and an
    // editor that only offered the bare name would not say so.
    for export in &page.libraries {
        let qualified = format!("{}::{}", export.from, export.name);
        let shared = page
            .libraries
            .iter()
            .filter(|each| each.name == export.name)
            .count()
            > 1;
        // Offered to whoever typed the library's name, and — when two
        // libraries share the name — to whoever typed the name itself,
        // because that is where the question comes up.
        let asked = matches(&qualified, typed) || (shared && matches(&export.name, typed));
        if !asked {
            continue;
        }
        if !shared && !typed.contains("::") {
            continue;
        }
        if found.iter().any(|item| item.label == qualified) {
            continue;
        }
        found.push(Item::new(
            &qualified,
            Kind::Component,
            &format!("{} from {}, said in full", export.name, export.from),
        ));
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

    let installed = page
        .libraries
        .iter()
        .any(|export| export.name == tag || format!("{}::{}", export.from, export.name) == tag);

    // What the library says this tag takes, before the HTML attributes
    // it passes on. First because they are the answer: `variant` and
    // `size` are what somebody opening a `<Button` is looking for, and
    // a list that began with `accept` and `accesskey` buried them.
    if installed {
        if let Some(application) = &page.application {
            for prop in crate::libraries::props_for(application, tag) {
                if !matches(&prop.name, typed)
                    || already.iter().any(|written| written == &prop.name)
                {
                    continue;
                }
                let detail = if prop.kind.is_empty() {
                    tag.to_string()
                } else {
                    prop.kind.clone()
                };
                found.push(
                    Item::new(&prop.name, Kind::Property, &detail)
                        .inserting(&format!("{}={{}}", prop.name)),
                );
            }
        }
    }

    for name in rustx_compiler::attributes::known() {
        if matches(name, typed)
            && !already.iter().any(|written| written == name)
            && !found.iter().any(|item| item.label == *name)
        {
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
        if matches(name, typed)
            && !already.iter().any(|written| written == name)
            && !found.iter().any(|item| item.label == *name)
        {
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

/// What this file declares, and what the application around it
/// installed.
///
/// The libraries are read from `node_modules` and from
/// `components/ui/*.tsx` — see [`crate::libraries`]. `CardHeader` and
/// `LineChart` are in no list the framework keeps, and they are most of
/// what a page made of installed components is written with.
pub fn page_of(source: &str, path: &Path) -> Page {
    let components = rustx_compiler::promote::component_signatures(source);
    let application = crate::crate_for_pages::application_of(path);
    Page {
        components,
        libraries: installed(path),
        application,
    }
}

/// The component names the application around `page` installed.
///
/// The application's own directory, which is where `package.json`,
/// `node_modules` and `components/` are — and not its parent, which is
/// what this read for a while: `apps/zoo/app/page.rsx` was answered
/// from `apps/`, and `labo-rustx/app/page.rsx` from `projects/`. Both
/// have no `node_modules`, so the list came back empty and the editor
/// offered HTML tags and nothing else. Every other reader in the
/// server already used the application itself.
fn installed(page: &Path) -> Vec<crate::libraries::Export> {
    let Some(application) = crate::crate_for_pages::application_of(page) else {
        return Vec::new();
    };
    crate::libraries::exports_for(&application)
}

/// Does `candidate` answer what has been typed?
///
/// A prefix, ignoring case, which is what an editor's own filter does
/// after us — being stricter here would hide what it would have kept.
fn matches(candidate: &str, typed: &str) -> bool {
    typed.is_empty() || candidate.to_lowercase().starts_with(&typed.to_lowercase())
}

/// How many things are offered at once.
///
/// `lucide-react` alone exports close to seven thousand names, and an
/// application that installs it would otherwise have every keystroke
/// answered with a megabyte of JSON. What is cut is the tail of the
/// list, and the editor is told the answer is partial — so it asks
/// again on the next letter, when the prefix has narrowed it.
pub const AT_MOST: usize = 800;

/// The LSP answer, as a list that says whether it is the whole of one.
pub fn list(items: &[Item]) -> String {
    let partial = items.len() > AT_MOST;
    let shown = if partial { &items[..AT_MOST] } else { items };
    format!(
        "{{\"isIncomplete\":{partial},\"items\":{}}}",
        json(shown)
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// An application on disk, because what the props are read from is
    /// files: a `components/ui/button.tsx` the way `shadcn add button`
    /// writes it, and a manifest.
    fn an_application_with_a_button() -> std::path::PathBuf {
        let root = std::env::temp_dir().join("rustx-lsp-props-test");
        let ui = root.join("components").join("ui");
        std::fs::create_dir_all(&ui).expect("a place to put the fixture");
        std::fs::write(root.join("package.json"), "{
  \"dependencies\": {}
}")
            .expect("a manifest");
        std::fs::write(
            ui.join("button.tsx"),
            "import * as React from \"react\";
             function Button({ className, variant, size, asChild = false, ...props }) {
                 return null;
             }
             export { Button };
",
        )
        .expect("a component");
        root
    }

    #[test]
    fn the_props_of_an_installed_tag_are_offered_before_the_html_ones() {
        let root = an_application_with_a_button();
        let page = Page {
            components: Vec::new(),
            libraries: vec![crate::libraries::Export {
                name: "Button".to_string(),
                from: "ui".to_string(),
            }],
            application: Some(root),
        };

        let offered = attributes("Button", "", &page, &[]);
        let labels: Vec<&str> = offered.iter().map(|item| item.label.as_str()).collect();
        assert!(
            labels.starts_with(&["className", "variant", "size", "asChild"]),
            "the component own props were not offered first: {labels:?}"
        );
        // And what the library passes on is still there, after them.
        assert!(labels.contains(&"id"), "got {labels:?}");
        assert!(labels.contains(&"onClick"), "got {labels:?}");
    }

    #[test]
    fn a_prop_already_written_is_not_offered_again() {
        let root = an_application_with_a_button();
        let page = Page {
            components: Vec::new(),
            libraries: vec![crate::libraries::Export {
                name: "Button".to_string(),
                from: "ui".to_string(),
            }],
            application: Some(root),
        };

        let offered = attributes("Button", "var", &page, &["variant".to_string()]);
        assert!(
            !offered.iter().any(|item| item.label == "variant"),
            "it was offered twice: {offered:?}"
        );
    }
}
