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
    assert_eq!(transpile(source).unwrap().rust, transpile(source).unwrap().rust);
}
