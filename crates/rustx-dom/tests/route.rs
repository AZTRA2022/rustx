//! Matching a path against the table the compiler generated.

use rustx_dom::Element;
use rustx_dom::route::{Params, Route, match_route};

fn nothing(_params: &Params) -> Element {
    Element::Empty
}

/// A table of static patterns, the way the generated one is shaped.
fn table(patterns: &[&'static str]) -> Vec<Route> {
    patterns
        .iter()
        .map(|pattern| Route {
            pattern,
            render: nothing,
            layouts: &[],
        })
        .collect()
}

/// Which pattern `path` matched, if any.
fn matched(path: &str, patterns: &[&'static str]) -> Option<&'static str> {
    let table = table(patterns);
    match_route(&table, path).map(|(route, _)| route.pattern)
}

#[test]
fn a_static_path_matches_its_own_route() {
    assert_eq!(matched("/device", &["/", "/device"]), Some("/device"));
}

#[test]
fn the_root_matches_the_root() {
    assert_eq!(matched("/", &["/", "/device"]), Some("/"));
}

#[test]
fn a_bracketed_segment_captures_what_is_there() {
    let table = table(&["/device/[id]"]);
    let (_, params) = match_route(&table, "/device/7").expect("a match");
    assert_eq!(params.get("id"), "7");
}

#[test]
fn a_static_segment_wins_over_a_parameter() {
    // Review Focus 4, the second half. Whatever order the table is in.
    assert_eq!(
        matched("/device/new", &["/device/[id]", "/device/new"]),
        Some("/device/new")
    );
    assert_eq!(
        matched("/device/new", &["/device/new", "/device/[id]"]),
        Some("/device/new")
    );
}

#[test]
fn a_path_nobody_declared_matches_nothing() {
    // Review Focus 4, the first half: the router turns this into the
    // not-found page rather than a blank window.
    assert_eq!(matched("/nowhere", &["/", "/device"]), None);
}

#[test]
fn a_trailing_slash_is_the_same_path() {
    assert_eq!(matched("/device/", &["/device"]), Some("/device"));
}

#[test]
fn a_parameter_does_not_match_across_a_slash() {
    assert_eq!(matched("/device/7/edit", &["/device/[id]"]), None);
}

#[test]
fn a_shorter_path_does_not_match_a_longer_pattern() {
    assert_eq!(matched("/device", &["/device/[id]"]), None);
}

#[test]
fn an_empty_segment_does_not_fill_a_parameter() {
    // `/device//` would otherwise hand a page an empty id.
    assert_eq!(matched("/device//", &["/device/[id]"]), None);
}

#[test]
fn two_parameters_are_both_captured() {
    let table = table(&["/device/[id]/stream/[format]"]);
    let (_, params) = match_route(&table, "/device/7/stream/mjpeg").expect("a match");
    assert_eq!(params.get("id"), "7");
    assert_eq!(params.get("format"), "mjpeg");
}

#[test]
fn a_parameter_nobody_captured_reads_as_nothing() {
    // A page asking for a name the path never had gets "", not a panic:
    // this is generated code's caller, and it must not take the window
    // down over a mistake the developer can see on screen.
    let table = table(&["/device/[id]"]);
    let (_, params) = match_route(&table, "/device/7").expect("a match");
    assert_eq!(params.get("nope"), "");
}

#[test]
fn a_path_with_no_leading_slash_is_still_a_path() {
    assert_eq!(matched("device", &["/device"]), Some("/device"));
}
