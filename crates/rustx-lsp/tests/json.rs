//! `completionItem/resolve` carries no URI at all, only whatever `data`
//! a client was handed back. These two functions are how this server
//! smuggles the page's own URI onto every item it offers, and reads it
//! back off one item resolve sends back.

use rustx_lsp::json;

#[test]
fn every_item_of_a_bare_array_is_tagged() {
    let tagged = json::tag_items(r#"[{"label":"read"},{"label":"write"}]"#, "file:///p.rsx");
    assert!(tagged.contains(r#""label":"read""#));
    assert!(tagged.contains(r#""label":"write""#));
    assert_eq!(
        tagged.matches(r#""__rustxUri":"file:///p.rsx""#).count(),
        2,
        "got {tagged}"
    );
}

#[test]
fn items_under_an_envelope_are_tagged_too() {
    let tagged = json::tag_items(
        r#"{"isIncomplete":false,"items":[{"label":"read"}]}"#,
        "file:///p.rsx",
    );
    assert!(tagged.contains("\"isIncomplete\":false"), "got {tagged}");
    assert!(tagged.contains("\"__rustxUri\":\"file:///p.rsx\""), "got {tagged}");
}

#[test]
fn untagging_gives_back_the_uri_and_the_original_item() {
    let tagged = json::tag_items(r#"[{"label":"read","data":{"x":1}}]"#, "file:///p.rsx");
    // The one element of that bare array, the way `lib.rs` will hand a
    // single item to `untag_item`.
    let item = &tagged[1..tagged.len() - 1];
    let (original, uri) = json::untag_item(item);
    assert_eq!(uri, Some("file:///p.rsx".to_string()));
    assert!(original.contains(r#""data":{"x":1}"#), "got {original}");
    assert!(!original.contains("__rustxUri"), "got {original}");
}

#[test]
fn an_item_nobody_tagged_untags_to_no_uri() {
    let (original, uri) = json::untag_item(r#"{"label":"read"}"#);
    assert_eq!(uri, None);
    assert_eq!(original, r#"{"label":"read"}"#);
}
