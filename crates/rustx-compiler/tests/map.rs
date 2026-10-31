//! What a position in a `.rsx` is worth in the generated `.rs`.
//!
//! A line survives the transpilation — a region puts back the newlines
//! its expression swallowed — so the interesting question is the column,
//! and the honest answer is sometimes "there isn't one": a JSX region is
//! several lines of `.rsx` and one expression of Rust, and a rewritten
//! `let mut` is a line nobody wrote. A language server that guessed
//! there would complete the wrong word.

use rustx_compiler::map::PositionMap;
use rustx_compiler::transpile::transpile;

const PAGE: &str = "fn Page() -> Element {\n\
                    \x20   let mut n = 0;\n\
                    \x20   let title = \"bonjour\";\n\
                    \x20   return <button\n\
                    \x20       onClick={move |_| n += 1}\n\
                    \x20   >\n\
                    \x20       {n}\n\
                    \x20   </button>;\n\
                    }\n";

fn map_of(source: &str) -> PositionMap {
    transpile(source).expect("transpilable").map
}

#[test]
fn a_line_nothing_touched_keeps_its_columns() {
    // `let title = "bonjour";`: ordinary Rust, copied byte for byte, so
    // the cursor is where it was. This is the case that matters — it is
    // where a developer asks for completion.
    let map = map_of(PAGE);
    assert_eq!(map.to_generated_position(3, 9), Some((3, 9)));
    assert_eq!(map.to_rsx_position(3, 9), Some((3, 9)));
}

#[test]
fn a_position_inside_a_jsx_region_has_no_equivalent() {
    // The region starts at the `<` on line 4 and runs to line 8. Before
    // the `<`, the line is the developer's own Rust; from there on, five
    // lines of `.rsx` are one expression, and a column means nothing.
    let map = map_of(PAGE);
    assert_eq!(map.to_generated_position(4, 5), Some((4, 5)), "`return`");
    assert_eq!(map.to_generated_position(4, 13), None, "the tag's name");
    assert_eq!(map.to_generated_position(7, 10), None, "a child");
    assert_eq!(map.to_generated_position(8, 5), None, "the closing tag");
    assert_eq!(map.to_generated_position(9, 1), Some((9, 1)), "after it");
}

#[test]
fn a_rewritten_line_has_no_equivalent() {
    // `let mut n = 0;` became a cell: the line in the generated file is
    // not the line that was written, and nothing on it lines up.
    let map = map_of(PAGE);
    assert_eq!(map.to_generated_position(2, 9), None);
    assert_eq!(map.to_rsx_position(2, 9), None);
}

#[test]
fn a_map_written_and_read_back_is_the_same_map() {
    let map = map_of(PAGE);
    assert_eq!(PositionMap::from_text(&map.to_text()), map);
}

#[test]
fn a_map_without_columns_still_reads() {
    // The short form is what the map has always written, and a build
    // that already has one on disk must keep working.
    let map = PositionMap::from_text("1:1\n12:30\n");
    assert_eq!(map.to_rsx(12), Some(30));
    // Nothing is known about regions or rewrites, so every position is
    // taken at face value.
    assert_eq!(map.to_generated_position(12, 4), Some((12, 4)));
}
