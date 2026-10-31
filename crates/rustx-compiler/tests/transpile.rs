//! A whole file in, a whole file out, and the positions kept.

use rustx_compiler::transpile::transpile;

#[test]
fn rust_outside_jsx_is_copied_through_untouched() {
    let source = "use std::fmt;\n\nfn helper(x: u8) -> u8 { x + 1 }\n";
    assert_eq!(transpile(source).unwrap().rust, source);
}

#[test]
fn every_rsx_line_maps_to_the_line_it_generated() {
    // Review Focus 3: a rustc error must point at the .rsx line.
    let source =
        "fn App() -> Element {\n    let name = missing_fn();\n    return <div>{name}</div>;\n}\n";
    let out = transpile(source).unwrap();

    let generated = out
        .rust
        .lines()
        .position(|l| l.contains("missing_fn"))
        .unwrap()
        + 1;
    assert_eq!(out.map.to_rsx(generated), Some(2));
}

#[test]
fn the_mapping_holds_after_jsx_that_expands_over_several_lines() {
    let source = "fn App() -> Element {\n    return <ul>\n        <li>a</li>\n    </ul>;\n}\nfn after() -> u8 { 7 }\n";
    let out = transpile(source).unwrap();

    let generated = out
        .rust
        .lines()
        .position(|l| l.contains("fn after"))
        .unwrap()
        + 1;
    assert_eq!(out.map.to_rsx(generated), Some(6));
}

#[test]
fn transpiling_is_deterministic() {
    let source = "fn App() -> Element { return <div><span>a</span><b>c</b></div>; }";
    assert_eq!(
        transpile(source).unwrap().rust,
        transpile(source).unwrap().rust
    );
}

#[test]
fn a_line_inside_a_region_maps_to_the_start_of_that_region() {
    // A region used to collapse onto one generated line and everything
    // after it slid up, so a line inside the region resolved to the line
    // of its *closing* tag — for a 60-line page, up to 60 lines past the
    // mistake. It now resolves to the line the region opened on, which is
    // where the developer can see the whole thing at once.
    //
    // Landing on the exact line inside a region would mean placing each
    // emitted child on the line its source started on; the region is one
    // expression today. See docs/dette-connue.md.
    let source =
        "fn App() -> Element {\n    return <ul>\n        <li>{bad()}</li>\n    </ul>;\n}\n";
    let out = transpile(source).unwrap();

    let generated = out.rust.lines().position(|l| l.contains("bad")).unwrap() + 1;
    assert_eq!(out.map.to_rsx(generated), Some(2));
}

#[test]
fn every_line_of_the_generated_file_lines_up_with_the_one_it_came_from() {
    let source = "fn App() -> Element {\n    return <ul>\n        <li>a</li>\n    </ul>;\n}\nfn after() -> u8 { 7 }\n";
    let out = transpile(source).unwrap();

    assert_eq!(
        out.rust.lines().count(),
        source.lines().count(),
        "the two files no longer have the same number of lines:\n{}",
        out.rust
    );
    for line in 1..=source.lines().count() {
        assert_eq!(out.map.to_rsx(line), Some(line), "line {line}");
    }
}

#[test]
fn a_line_inside_a_region_resolves_forwards_too() {
    let source = "fn App() -> Element {\n    return <ul>\n        <li>a</li>\n    </ul>;\n}\n";
    let out = transpile(source).unwrap();
    assert_eq!(out.map.to_generated(3), Some(3));
}
