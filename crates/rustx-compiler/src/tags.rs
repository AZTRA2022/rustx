//! What may be written as a tag.
//!
//! Attributes fail closed, and tags have to as well. `<dvi>` is accepted
//! by every browser alive: it builds an `HTMLUnknownElement`, styles
//! nothing, and leaves a blank area on screen with no error anywhere.
//! That is the silence the design document forbids.

/// HTML elements, sorted.
const HTML: &[&str] = &[
    "a",
    "abbr",
    "address",
    "area",
    "article",
    "aside",
    "audio",
    "b",
    "bdi",
    "bdo",
    "blockquote",
    "br",
    "button",
    "canvas",
    "caption",
    "cite",
    "code",
    "col",
    "colgroup",
    "data",
    "datalist",
    "dd",
    "del",
    "details",
    "dfn",
    "dialog",
    "div",
    "dl",
    "dt",
    "em",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "legend",
    "li",
    "main",
    "map",
    "mark",
    "menu",
    "meter",
    "nav",
    "noscript",
    "object",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "section",
    "select",
    "slot",
    "small",
    "source",
    "span",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "track",
    "u",
    "ul",
    "var",
    "video",
    "wbr",
];

/// SVG elements, so that an icon can be drawn.
const SVG: &[&str] = &[
    "circle",
    "clipPath",
    "defs",
    "ellipse",
    "g",
    "line",
    "linearGradient",
    "mask",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "stop",
    "svg",
    "text",
    "tspan",
    "use",
];

/// Is this a tag the framework knows how to build?
pub fn is_known(name: &str) -> bool {
    HTML.contains(&name) || SVG.contains(&name)
}

/// Does this look like a component rather than an element?
///
/// Worth telling apart, because every TSX author will write `<Card />` on
/// their first day, and "unknown tag" would send them hunting for a typo
/// that is not there.
pub fn looks_like_a_component(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_uppercase())
}

/// The known tag closest to `name`.
pub fn nearest(name: &str) -> Option<&'static str> {
    crate::attributes::closest(name, HTML.iter().chain(SVG).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fragment_is_not_a_tag_to_look_up() {
        assert!(!is_known(""));
    }

    #[test]
    fn a_typo_finds_its_neighbour() {
        assert_eq!(nearest("dvi"), Some("div"));
        assert_eq!(nearest("buton"), Some("button"));
    }

    #[test]
    fn a_pascal_case_name_is_a_component() {
        assert!(looks_like_a_component("Card"));
        assert!(!looks_like_a_component("div"));
    }
}
