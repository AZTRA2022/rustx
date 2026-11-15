# Known debt

What we know is imperfect, why we left it, and what would change our
mind. A line here is a decision, not an oversight: an entry with no
"we deal with it when" left in it has to be fixed or deleted.

> Translated from the French on 2026-11-14, item by item and in the same
> order, because two other documents and one test cite these items by
> their number. Where an item has since been overtaken by the code, the
> entry says so under **Since then** and keeps the original text above
> it: a debt that was paid is still worth reading, and an entry that
> quietly changed its story is not a record.

---

## Engine (plan 1, branch review of 2026-10-28)

The review sorted sixteen points. The first eight — two critical, six
important — are fixed in commits `3a04e63`, `cd2ec2f` and `e1ee60e`.
The next eight are here.

### 1. `__rustx_frame` is a public global function

`renderer::install` registers it through the public `register_native`.
So it lands in the application's native namespace (which the
application can overwrite) and on the JavaScript global (the page's own
script can call it, which re-enters `tick()`, or shadow it, which stops
the loop).

This is not corruption: `tick()`'s take-then-give-back survives
re-entrancy without losing callbacks. It is robustness.

**We deal with it when** we do hot reload, which needs an internal
channel between the engine and the page in any case. The right fix is a
separate internal slot and a non-enumerable property, not a rename.

### 2. `shutdown()` throws away the jobs already queued

`*guard = None` drops the waiting `Vec<Job>`. The documentation only
promises that *later* posts are refused, so the behaviour is not a lie
— but it is not a choice anybody made either.

**We deal with it when** we know what a capture application wants:
drain the queue before closing (the last frames are arriving) or throw
it away (the window has gone, nobody will see them). The answer comes
from the application that proves it, not from here.

### 3. `Poster::post` from the browser process succeeds into the void

`tick()` only runs in the render process. A `post` made from the
browser process is accepted and never drained: the queue grows without
bound and without a word.

**We deal with it when** the OS layer exists, because that is what will
give developers a reason to have code in both processes. `poster()`
will refuse outside `Role::Renderer` then.

### 4. Hooks registered after the first context never run

`HOOKS_RUN` is deliberate: hooks run once per process rather than once
per page, so that the state a closure captured survives a navigation.
But `on_context_created`'s documentation does not say so.

**We deal with it** at the next pass over the public API's
documentation. It is a contract to write, not code to change.

### 5. No compile-time guard against `panic = "abort"`

All of the engine's panic tolerance rests on `catch_unwind`. An
application compiled with `panic = "abort"` turns every one of those
`catch_unwind`s into a process abort, silently, and everything this
review just had fixed comes back.

**We deal with it when** we write the build tooling (`rustx build`),
which is the only place a profile can be refused from in any case.

### 6. `Document::current()` does not assert its thread

It is `pub` and callable from a worker thread. In practice libcef's
`CEF_V8_REQUIRE_ISOLATE_RETURN` returns null rather than misbehaving —
so this is not undefined behaviour, just a `None` nobody can explain. An
explicit `cef::currently_on(RENDERER)` would make the contract
readable.

**We deal with it when** we touch `dom.rs` for the reconciler.

### 7. No test covers a DOM call that throws

`dom.rs` checks `has_exception()` after `execute_function`, and it
works. But nothing proves that an exception raised by the DOM comes out
as `DomError::Failed` rather than as a silent `None`.

**We deal with it when** the reconciler writes to the DOM for real:
that is when a call that throws stops being hypothetical.

### 8. `App::run` does not `cef::shutdown()` on the error path

If `run_message_loop` panics, CEF's clean shutdown does not happen.
Cosmetic for a demonstration binary; to be revisited when `App` is the
public API that third-party software uses.

---

## The `.rsx` compiler (plan 2, branch review of 2026-10-28)

The review sorted sixteen points. The two critical and the six
important ones are fixed. The eight minor ones are here, plus one limit
the fixing pass deliberately left in place.

### 9. A `rustc` error names the right line number, not the right file

The generated `.rs` now has exactly as many lines as the `.rsx`, and
every line carries the same number: a `rustc` error on line 10 really is
line 10 of the `.rsx`. But the path shown is still the one in `OUT_DIR`,
which the developer did not write.

What is missing is the path rewrite in `cargo`'s output.

**We deal with it at plan 8**, with the `rustx` command line: it is the
thing that wraps `cargo` and the only place its messages can be
rewritten from. Until then the line number is right, which is the part
that matters.

**Since then:** still true of the command line — nothing in
`rustx-cli` reads `cargo`'s diagnostics. The editor's half is done: the
language server carries the map and reports in the `.rsx`'s own
coordinates (`rustx-lsp/src/lib.rs::retarget_edits`, `shadow.rs`), so
the path is wrong only for somebody reading raw `cargo` output.

### 10. A line inside a region resolves to the start of the region

A region is still one Rust expression on one line, followed by the
newlines it swallowed. So an error written on line 10 of a region that
starts on line 6 is reported on line 6 — at the start of the `return`,
where the whole thing is visible, rather than at its end as before.

To land on the exact line, every emitted child would have to be put on
the line its source started on, which needs an offset on `Node::Text`
and `Node::Expression`. **We deal with it when** plan 8 needs exact
columns for the editor extension.

### 11. An orphaned closing tag after a region is copied through as it is

`return <div></div></span>;` produces `… .build()</span>;`, so a
`rustc` syntax error in the generated file instead of a message in the
`.rsx`'s coordinates. **We deal with it** at the next pass over
`scan.rs`.

### 12. A repeated attribute is accepted silently

`<div id="a" id="b"/>` emits both; the last one wins in the DOM. And
`value={x} defaultValue="y"` both resolve to `value` and tread on each
other without a word. **We deal with it when** we touch `emit.rs` for
components.

### 13. A dead `trim_end` in the self-closing tag check

`source[..index].trim_end().ends_with("/>")` can never see `<div/ >`,
because `index` is always a position after the `>`. No effect today;
the `trim_end` suggests a space is tolerated there when it is not.

**Still true** at `crates/rustx-compiler/src/scan.rs:264`.

### 14. `PositionMap::from_text` has neither a caller nor a test

Dead until point 9 is dealt with.

**Since then: no longer true, and the entry was stale rather than
wrong.** `from_text` is covered by `crates/rustx-compiler/tests/map.rs`
(it round-trips `to_text`), and `PositionMap` itself has real readers in
`rustx-lsp` — `shadow.rs` holds one per open document and
`retarget_edits` puts `rust-analyzer`'s edits back into the `.rsx`'s
coordinates.

### 15. `mount_in` has no way back

A failure halfway leaves half a tree attached. Combined with the mount
being retried on every frame in `apps/catalogue`, the prefix that
succeeded would be attached a second time. Unlikely, cheap to keep:
build the whole subtree and attach it in one go. **We deal with it at
plan 3**, which rewrites mounting for re-rendering in any case.

### 16. Every attribute goes through `setAttribute`

So `value` and `checked` set the *default* value rather than the live
property. Correct for a static mount; wrong as soon as a controlled
field is re-rendered. **We deal with it at plan 3.**

### 17. `tag_name` and `Cursor::name` disagree about non-ASCII

`scan.rs` accepts bytes `>= 0x80` in a tag name, `parse.rs` does not.
It only produces a slightly confusing error. The two should share one
function.

### 18. A rustx component written *inside* a JS component is refused

`<ui::CardContent>{Feature("…")}</ui::CardContent>` does not compile: a
`{…}` inside a boundary is emitted as `ForeignChild::Value`, so it goes
through `ToValue`, and `Element` is not a value. What the developer sees
is `the trait bound rustx::Element: ToValue is not satisfied` in a
generated file — which is to say, incomprehensible.

What works today: a node rustx owns (`#panel-host`) and
`rustx_component("Name", "id")` from the page's JavaScript, which
`apps/hybrid` proves. What is missing is the natural form, and it will
be missed quickly: a page made of installed components is made of
components inside components.

What it would take: a `ForeignChild::Ours(Element)` variant, a slot the
runtime renders in its place (`<span data-rustx-slot>`), and the rustx
subtree mounted inside it on the same frame — with the subtree's life
tied to the React node's, since React can unmount it whenever it likes.
The compiler cannot choose the variant on a type it does not know, so it
needs a trait (`IntoForeignChild`) with autoref specialisation, or a
method on `Element` that the `{…}` calls when it exists.

**To do**: its own small plan, after the media work. Until then, the
refusal should at least be a sentence from the compiler naming the form
that does work, instead of a trait error in a generated file.

**Since then: done, by the trait this entry predicted.**
`IntoForeignChild` is in `crates/rustx-dom/src/element.rs` with four
markers — `Element`, anything `ToValue`, `Vec<Element>` and
`Option<Element>` — and an `Element` inside a boundary becomes
`ForeignChild::Element`, a `Vec<Element>` an `Element::Fragment` the
engine describes to React as an array with every `key` intact. Covered
by `a_list_of_elements_may_be_written_inside_a_library_component` and
`an_optional_element_may_be_written_inside_a_library_component` in
`crates/rustx-dom/tests/foreign.rs`. The subtree's life is still the
page's rather than the React node's, which is the half of the design
above that was not needed to make the form compile.
