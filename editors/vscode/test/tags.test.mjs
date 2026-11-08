// What a `>` finishes, and what it does not.
//
// The interesting cases are all Rust. A `.rsx` is Rust with JSX inside
// it, so an editor that closed every `>` would answer `Vec<u32>` with
// `</u32>` — which is why most of this file is about the things that
// must stay untouched.

import { test } from "node:test";
import assert from "node:assert/strict";

import { closingFor } from "../src/tags.mjs";

test("an_opening_tag_is_closed", () => {
  assert.equal(closingFor("return <div>"), "</div>");
  assert.equal(closingFor("    <Card>"), "</Card>");
});

test("a_tag_with_attributes_is_closed", () => {
  assert.equal(
    closingFor('<Button variant="outline" onClick={move |_| n += 1}>'),
    "</Button>",
  );
});

test("a_tag_written_across_lines_is_closed", () => {
  assert.equal(
    closingFor('<section\n    id="attente"\n    className="p-8">'),
    "</section>",
  );
});

test("a_tag_said_in_full_is_closed_under_the_name_it_was_written_with", () => {
  assert.equal(
    closingFor("<js::recharts::LineChart data={data}>"),
    "</js::recharts::LineChart>",
  );
});

test("a_fragment_is_closed_with_a_fragment", () => {
  assert.equal(closingFor("return <>"), "</>");
});

test("a_tag_that_closed_itself_is_left_alone", () => {
  assert.equal(closingFor("<br />"), undefined);
  assert.equal(closingFor('<AArrowDown size={18} />'), undefined);
});

test("a_closing_tag_is_not_closed_again", () => {
  assert.equal(closingFor("</div>"), undefined);
});

test("a_rust_generic_is_not_a_tag", () => {
  assert.equal(closingFor("let rows = Vec::<u32>"), undefined);
  assert.equal(closingFor("let seen: HashMap<String, u32>"), undefined);
  assert.equal(closingFor("fn one<T>"), undefined);
  assert.equal(closingFor("Result<(), rustx::Error>"), undefined);
});

test("a_comparison_is_not_a_tag", () => {
  assert.equal(closingFor("if a < b >"), undefined);
});

test("a_closure_arrow_is_not_a_tag", () => {
  assert.equal(closingFor("move |_| ok =>"), undefined);
});

test("a_shift_is_not_a_tag", () => {
  assert.equal(closingFor("let wide = 1 << 3 >"), undefined);
});

test("a_tag_that_already_closed_earlier_on_the_line_is_not_reopened", () => {
  // The `>` of `<p>` was handled when it was typed; typing one after
  // the text is not a second tag.
  assert.equal(closingFor("<p>ok>"), undefined);
});

test("an_attribute_expression_does_not_count_as_the_tag", () => {
  // The `>` here closes nothing: it is inside the braces of a handler.
  assert.equal(closingFor("<p onClick={move |_| { n > 1 }}>"), "</p>");
  assert.equal(closingFor("<p onClick={move |_| n > 1"), undefined);
});

test("text_with_no_angle_bracket_at_the_end_is_not_a_tag", () => {
  assert.equal(closingFor("return <div"), undefined);
  assert.equal(closingFor(""), undefined);
});

test("a_string_attribute_holding_an_angle_bracket_is_stepped_over", () => {
  assert.equal(closingFor('<p title="a > b">'), "</p>");
});
