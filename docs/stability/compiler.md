# Stability — the compiler

The subsystem: `crates/rustx-compiler`, `crates/rustx-dom`,
`crates/rustx-macros`. Nothing outside those three paths is touched here;
where a fix belongs elsewhere it is recorded under
[Belongs to another subsystem](#belongs-to-another-subsystem).

This document is kept current as the work proceeds. It is written so that
someone could resume from it cold: every defect carries its root cause,
the smallest correction that was made, and the test that would catch it
coming back.

## How to run this subsystem's tests

```powershell
$env:CARGO_TARGET_DIR = "C:\Users\aztra\AppData\Local\Temp\rustx-target-compiler"
cargo test -p rustx-compiler -p rustx-dom -p rustx-macro
```

The crate is published as `rustx-macro`, not `rustx-macros`; the
directory name and the library name differ from the package name.

Cargo must be driven from PowerShell on this machine: under Git Bash,
`/usr/bin/link` shadows MSVC's `link.exe` and every link fails. A target
directory of its own keeps this subsystem from blocking on the lock that
five other agents share.

---

## 1. The audit

### Pipeline, as it actually runs

```
.rsx source
  │
  ├─ rewrite::promote_state            Rust → Rust. Decides nothing; applies
  │   └─ promote::analyse              what promote decided. Edits only.
  │       ├─ functions_of              every outermost `fn`, and whether it
  │       │                            returns an `Element` (→ a component)
  │       ├─ locals_of                 `let mut` statements
  │       ├─ shared_of                 `let mut` outside every `fn`
  │       ├─ handler_closures          the `{…}` of every `on*=` attribute
  │       ├─ calls_of                  `keep` / `effect` / `task` / bare `async`
  │       ├─ awaited_of                `let x = ….await;`
  │       └─ mutations_in / names_used reads and writes, per name
  │
  ├─ scan::find_jsx                    where the JSX regions are
  ├─ parse::parse_jsx                  one region → a `Node` tree
  ├─ emit::emit                        a `Node` tree → a `TokenStream`
  └─ map::PositionMap                  generated line ↔ `.rsx` line
```

`build::compile_directory` drives it per file and writes `<name>.rsx.map`
beside each `.rs`. `rustx-macros` calls `build::compile_app` /
`compile_pages` while `#[rustx::main]` expands.

### The invariant everything rests on

> **The compiler never loses or moves a character it was not asked to
> move, and never changes a line number it was not asked to change.**

Concretely: `promote_state` replaces declarations, calls and writes *in
place*, and every replacement has the same number of newlines as the text
it replaces. `transpile` then relies on that twice — once to decide which
lines a language server may not offer a column in (it diffs the two
texts line by line), and once to keep every `rustc` diagnostic on the
line the developer wrote. A single lost newline moves every diagnostic
below it, silently and permanently.

This is now enforced mechanically rather than by discipline: see defect
**C-2**.

### Determinism

No `HashMap`, `HashSet`, `SystemTime` or randomness anywhere in
`rustx-compiler` or `rustx-macros`; `build::compile_directory` sorts its
inputs and `rustx-macros::sources` sorts the files it names. Byte-identical
output for identical input is asserted on every fuzz case
(`tests/fuzz.rs`, law 4).

### Findings, by severity

| # | Severity | Finding | State |
|---|----------|---------|-------|
| C-1 | **P0** crash | Deeply nested JSX overflows the stack: no diagnostic, the process dies | **fixed** |
| C-2 | **P0** miscompilation | A promoted `let mut` with a multi-line initializer loses lines; every diagnostic below it drifts | **fixed** |
| C-3 | **P0** crash | A JSX region inside `{…}` is scanned with no ceiling; a tag closed after the brace yields a reversed byte range and panics | **fixed** |
| C-4 | **P1** miscompilation | `is_statement` reads `};` as "the block was a statement", so a read at the end of a `let`'s block is taken for a write | **fixed** |
| C-5 | **P2** diagnostics | Refusals carry the line, column and byte offset of the *rewritten* text, not of the file the developer has open | **fixed** |

More to come as the campaign continues; the table is the index.

---

## 2. Defects, root causes and fixes

### C-1 — a deep tree is a stack overflow, not a refusal (P0)

**Symptom.** `transpile` on a page whose tags nest a few hundred deep
kills the process with `STATUS_STACK_OVERFLOW`. No line, no column, no
message — in `cargo build`, in `rustx dev`, and in the language server
answering a keystroke.

**Root cause.** `parse::Cursor::element` is recursive descent and
`emit::expression` recurses over the tree it produces, as does
`TokenStream::to_string` over the nested groups, as does their `Drop`.
Measured cost: **about 12 KiB of stack per level of nesting** in an
unoptimised build (`examples/depth_probe.rs`). A 1 MiB thread therefore
dies at about 128 levels and a 2 MiB thread at about 300. Nothing bounded
the depth.

**Fix.** A limit, `parse::NESTING_LIMIT = 64`, refused with a sentence
that names the real problem and says what to do (give part of the tree a
component of its own). Enforced in two places for two different reasons:

- `scan::region` counts the open tags in a `Vec` as it loops over bytes,
  so it can refuse a document nested ten thousand deep **without pushing
  a single frame**. Left to the parser alone, reaching the refusal itself
  cost a quarter of a megabyte.
- `parse::Cursor::element` carries a `depth` through `parse_nested`, which
  catches the nesting the scanner cannot see: `<div>{ <div>{ … } }</div>`
  nests through braces, and each `{…}` is a region found by a scan of its
  own.

**Why 64.** The deepest page in this repository nests **7** (measured,
`examples/depth_probe.rs` with no arguments surveys `apps/` and `site/`).
64 is nine times that, and half of what kills the smallest stack the
compiler runs on.

**Tests.** `crates/rustx-compiler/tests/parse.rs`:
`a_tree_too_deep_to_descend_is_refused_rather_than_crashing`,
`a_very_deep_tree_is_refused_on_a_small_stack` (a 256 KiB thread, which
is less than a quarter of what the limit itself costs — it would die if
the refusal were not raised before the descent),
`nesting_through_an_expression_counts_towards_the_same_limit`,
`a_tree_at_the_limit_is_still_accepted`.

### C-2 — a multi-line initializer loses its lines (P0)

**Symptom.** A page with

```rust
let mut agents: Vec<Agent> = vec![
    Agent::new("scout"),
    Agent::new("relay"),
];
```

came out three lines shorter than it went in. Every `rustc` diagnostic
below that declaration was reported three lines higher than it was
written, and the position map — which assumes the two files have the same
lines — silently agreed.

**Root cause.** `rewrite::opening` deliberately collapses the initializer
onto one line, because the same text is also *copied* to wherever a
future reaches the value and a newline inserted there would move lines
too. That is right. What was wrong is that the declaration's own edit was
one of **five of eight** edit sites that never gave the newlines back:
`call_edit` and `awaited_edits` called `pad_to`, the promoted
declaration, the handler rewrite and the three mutation shapes did not.
`read_patched` and `arguments_of` also `trim()` the text they copy, which
drops a leading or trailing newline.

**Fix.** Padding moved out of the individual edit sites and into
`rewrite::apply`, which is the one place every edit of the pass goes
through. An edit's text is now padded to the newline count of the text it
replaces, always. `pad_to` is idempotent, so the three sites that already
padded are unaffected. The same function also stopped indexing the source
with a raw range: a span that is not on a character boundary is skipped
rather than panicking.

This makes the invariant structural instead of a thing eight call sites
have to remember.

**Tests.** `crates/rustx-compiler/tests/fuzz.rs` law 2 asserts the line
count on every generated case (4 000 plausible pages, 6 000 half-typed
ones, 4 000 pieces of syntax soup). The pre-existing
`every_page_keeps_its_line_count` and `the_file_keeps_its_line_count`
cover the repository's own pages.

### C-3 — a region inside braces is scanned with no ceiling (P0)

**Symptom.** `byte range starts at 865 but ends at 859` — a panic in
`parse::braced_rust`, on a page somebody was halfway through typing.

**Root cause.** `scan::find_jsx_range(source, from, to)` is asked for the
regions of one `{…}`. `scan_range` bounded its own loop with `to`, but
the sub-scanners it calls did not: `region` and `end_of_tag` both read to
the end of the **file**. So a `<div>` opened between the braces and
closed after them gave back a span whose `end` was past the closing
brace. `braced_rust` then sliced `self.source[cursor..to]` with
`cursor > to`.

That is the same blind spot as the rest of this subsystem's history: a
scan that does not know the bounds of what it is scanning.

**Fix.** `region` and `end_of_tag` take a `ceiling` and stop at it. A tag
that does not close inside the expression it was opened in is unclosed,
and that is now what it is told — placed at the opening `<`.

**Tests.** `crates/rustx-compiler/tests/fuzz.rs`
`no_half_typed_page_makes_the_compiler_panic` reproduces it (it was found
there); `tests/scan.rs` carries the focused case.

### C-4 — `};` read as "the block was a statement" (P1)

**Symptom.** `apps/utility-hub/app/search/page.rsx` was refused with

> `departments` is written while `Page` is being drawn.

about a page that only ever reads `departments`.

**Root cause.** `promote::is_statement` decides whether a method call is
a write by walking **forward** from the call to the first `;` or `}` at
bracket depth zero. On a `}` it asked what followed, and read a following
`;` as "the block was a statement, so its last expression is one too".

But **a statement block takes no semicolon in Rust.** A `};` is almost
always a `let` whose value is a block:

```rust
let units: Vec<Department> = if needle.is_empty() {
    Vec::new()
} else {
    departments.iter().filter(…).cloned().collect()
};
```

so `.collect()` was taken for a write to `departments`. The earlier fix
for this function (`vault-storage`, in `BUG.md`) added brace awareness
but kept deciding from what *follows* the brace, which is the wrong side:
what says whether a block is a statement is what comes **before** it.

**Fix.** `block_in_statement_position` replaces the look-ahead. Given the
`}` that closes the block whose tail the expression is, it finds the
matching `{`, finds the start of the statement that `{` sits in, and
reads the *head* — everything from the statement's start to the `{`:

| head | the block is |
|---|---|
| nothing | the first thing in a block: a statement |
| `if …`, `match …`, `for …`, `while …`, `loop`, `unsafe`, `else` | a construct, whose own position decides — and the head *is* that position |
| `let x =`, `x =`, `return` | a value |
| `f(`, `vec![`, `a +` | a value |
| `… =>` | a match arm's value, and so the match's: ask again one level out |
| `… \|x\|` | a closure body, whose tail its caller discards or does not |

The statement's start is found by `statement_start`, which scans
**forward** from the enclosing block and is aware of strings, raw
strings, character literals and both kinds of comment. Forward is the
only direction in which `"a;b"` and `// a;` can be stepped over without
guessing — which is the whole class of defect this subsystem keeps
producing, and the reason the new scan is written this way.

`if let Some(x) = y { … }` is why the construct-keyword test comes before
the test for a binding.

**Tests.** `crates/rustx-compiler/tests/shared.rs`:
`a_page_value_read_in_the_else_of_a_let_is_not_a_write`,
`a_page_value_read_in_a_block_assigned_to_something_is_not_a_write`,
`a_semicolon_in_a_string_does_not_begin_a_statement`, and the
counterpart in the other direction,
`a_write_that_ends_a_statement_block_is_still_a_write`.

All four were watched failing against the old logic and passing against
the new one. The fourth failed against the old logic too: `if c {
rows.push(1) }` written without a semicolon was already being read as a
value, so the fix closes both directions of the same hole. The two
pre-existing tests that constrain the opposite direction —
`a_method_on_a_value_inside_an_expression_is_a_read_not_a_write` and
`a_method_that_is_a_statement_of_its_own_is_still_a_write` — still pass.

### C-5 — a refusal placed in a file the developer does not have (P2)

**Symptom.** A `CompileError` from `transpile` carried a byte offset past
the end of the `.rsx`, and a column counted in a line the developer never
wrote.

**Root cause.** `transpile` runs `promote_state` first and then scans,
parses and emits the **rewritten** text. Every refusal below that point
was resolved against the rewritten text. Line numbers survive that — the
rewrite keeps every edit on its own line, which is C-2's invariant — but
nothing else does: a `let mut n = 0;` becomes a hundred and twenty
characters of cell, so a column counted there names a different
character, and the offset of a file that has grown by a thousand
characters can point past the end of the one the developer has open. A
caller that reads the `.rsx` and slices at that offset to underline the
mistake — which is what a language server does — slices at random.

**Fix.** `transpile::as_written` puts each refusal back in the
developer's coordinates: the line is kept, the column is kept only where
the line was not rebuilt (the rewritten lines are already known, they are
what `PositionMap` is given), and the offset is counted afresh in the
written file.

**Test.** `crates/rustx-compiler/tests/fuzz.rs` `judge` asserts
`error.offset <= source.len()` and a non-empty message for every refusal
of every generated case. Found there.

---

## 3. Testing

### Added

- **`crates/rustx-compiler/tests/fuzz.rs`** — a seeded property/fuzz
  harness, as ordinary `#[test]`s so that it runs in CI on every push.
  SplitMix64 as the generator, ten lines and no dependency, identical
  stream on every platform, with the seed printed beside any failure so
  that it replays.

  Four laws, applied to every case:

  1. **No panic, on any input.** A refusal is an answer; it only has to
     be an honest one, with a message and an offset inside the file.
  2. **The line count is preserved**, by `promote_state` and by
     `transpile`.
  3. **Determinism**: the same input twice, byte for byte, map included.
  4. **What is outside a JSX region is copied through** unchanged.

  Four generators:

  | test | cases | what it generates |
  |---|---|---|
  | `no_soup_of_syntax_makes_the_compiler_panic` | 4 000 | piles of the pieces a scanner trips on — `</`, `r#"`, `{`, a lone `\`, `é`, `🎉`, a BOM, `\r` |
  | `no_plausible_page_loses_a_line_or_panics` | 4 000 | pages the compiler will agree to compile: state, handlers, `keep`, `effect`, bare `async`, markup, multi-line initializers, `';'` and `"a;b"` as initializers |
  | `no_half_typed_page_makes_the_compiler_panic` | 6 000 | those pages with one to three wounds: head or tail cut off, a character dropped, a piece inserted, a stretch duplicated |
  | `the_pathological_shapes_are_answers_and_not_crashes` | 25 | written out by hand: empty, 20 000 unclosed braces, 20 000 quotes, 500 nested `div`s, mixed line endings, non-ASCII where an offset lands |

- `crates/rustx-compiler/tests/parse.rs` — four tests for C-1.
- `crates/rustx-compiler/tests/shared.rs` — four tests for C-4.
- `crates/rustx-compiler/examples/depth_probe.rs` — a measurement tool,
  not a test: it surveys the nesting depth of the repository's pages, and
  it answers "does a tree this deep survive a stack this big".

### Still to do

- `rustx-macros` has **no tests at all**. Its helpers (`without_async`,
  `fields_of`, `rename_in`, `modules_of`, `type_name`) are pure and
  testable from a `#[cfg(test)]` module inside the crate.
- Coverage-guided fuzzing with `cargo-fuzz`, now that the network is
  available. The seeded harness keeps its place regardless: it runs in
  CI, a fuzzer does not.
- `scan.rs` focused cases for C-3.
- The reconciliation against `docs/manques-du-framework.md`,
  `docs/chasse-aux-bugs.md` and `docs/dette-connue.md`.

---

## 4. Belongs to another subsystem

Nothing yet.

---

## 5. Open, unverified, or deliberately left

- `attributes::style_string` accepts quoted string literals only, so
  `style={{ width: n }}` is refused. The refusal now distinguishes a
  missing unit from a computed value, but `BUG.md`'s entry is still open
  and the README does not say so. Under audit.
- `scan::skip_comment` does not nest `/* /* */ */`. Rust's block comments
  do nest. No input has been found where this corrupts output rather than
  producing a refusal; not yet proven safe.
- `promote::matching` counts parentheses without stepping over strings,
  so `keep(format!("a)"))` may be cut short. Under audit; a case is in
  `tests/fuzz.rs`'s pathological list and currently passes, which needs
  explaining rather than trusting.
