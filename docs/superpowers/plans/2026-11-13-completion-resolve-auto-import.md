# Completion resolve & auto-import in `.rsx` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Typing an unqualified name in a `.rsx` (e.g. `read(` for `std::fs::read`) and accepting the completion inserts the real `use` line on the `.rsx`, the way it would in a plain `.rs` file under `rust-analyzer`.

**Architecture:** `rust-analyzer` defers the `additionalTextEdits` of an auto-import to `completionItem/resolve`, which `rustx-lsp` neither advertises nor forwards today — so the edit never exists on the editor's side. The server must: advertise `resolveProvider`, tag each item it hands out with which shadow it came from (LSP's `completionItem/resolve` carries no URI), forward the resolve call downstream, and translate every `additionalTextEdits` range from the shadow's `.rs` coordinates back to the `.rsx`'s with `PositionMap::to_rsx_position` — dropping any edit that lands on a line with no `.rsx` equivalent rather than writing it somewhere wrong.

**Tech Stack:** Rust, the hand-written JSON reader/writer already in `crates/rustx-lsp/src/json.rs`, no new dependency.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` §7.7, and `docs/superpowers/plans/2026-10-31-rust-analyzer-derriere-le-serveur.md` (this plan is the piece its Task 4 did not cover: `completionItem/resolve`).

## Global Constraints

- A position with no `.rsx` equivalent is answered with nothing, never a guess: `PositionMap::to_rsx_position` returning `None` for one of several edits drops that one edit, and returning `None` for the main edit drops the whole resolve.
- `rust-analyzer` absent is not an error: resolve is only advertised when `downstream.exists()`, same as `hoverProvider` and `definitionProvider` today.
- The tag this server adds to a completion item's `data` must be invisible to `rust-analyzer`: resolve must hand back exactly the `data` `rust-analyzer` itself produced, or it answers about the wrong item.

## Review Focus

1. An import whose edit lands on a JSX region or a rewritten line of the shadow: the edit must be dropped, not placed at a nearby wrong line.
2. A completion item with no `data` at all (a markup/tag completion, which answers out of this server's own head): `completionItem/resolve` on it must not crash or forward anything downstream.
3. Two completions requested back to back before either is resolved: each resolve must find its own shadow, not the most recent one.
4. `rust-analyzer` not running when a resolve arrives (died between the list and the pick): answered with the item unchanged, same as any other downstream question with no server.
5. An `additionalTextEdits` edit on a line that *does* map (the common case: inserting a `use` line near the top of the file): the edit's range must come back translated, not left in shadow coordinates, or the editor writes it into the wrong file position.

---

### Task 1: `json.rs` can tag and untag one item in a list

**Files:**
- Modify: `crates/rustx-lsp/src/json.rs`
- Test: `crates/rustx-lsp/tests/json.rs` (create if absent — check first)

**Interfaces:**
- Produces: `json::tag_items(list_result: &str, uri: &str) -> String` — given the raw `result` of a `textDocument/completion` answer (an array, or `{"isIncomplete":...,"items":[...]}`), returns the same shape with `"__rustxUri":"<uri>"` added to every item's own object (not inside `data` — alongside it, so the original `data` travels untouched).
- Produces: `json::untag_item(item: &str) -> (String, Option<String>)` — returns `(item_without_the_tag, uri_if_present)`.

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn every_item_of_a_bare_array_is_tagged() {
    let tagged = json::tag_items(r#"[{"label":"read"},{"label":"write"}]"#, "file:///p.rsx");
    assert!(tagged.contains(r#""label":"read""#));
    assert!(tagged.matches(r#""__rustxUri":"file:///p.rsx""#).count() == 2);
}

#[test]
fn items_under_an_envelope_are_tagged_too() {
    let tagged = json::tag_items(
        r#"{"isIncomplete":false,"items":[{"label":"read"}]}"#,
        "file:///p.rsx",
    );
    assert!(tagged.contains("\"isIncomplete\":false"));
    assert!(tagged.contains("\"__rustxUri\":\"file:///p.rsx\""));
}

#[test]
fn untagging_gives_back_the_uri_and_the_original_item() {
    let tagged = json::tag_items(r#"[{"label":"read","data":{"x":1}}]"#, "file:///p.rsx");
    let item = /* the one element of `tagged`'s array, extracted by the test the way lib.rs will receive it */;
    let (original, uri) = json::untag_item(&item);
    assert_eq!(uri, Some("file:///p.rsx".to_string()));
    assert!(original.contains(r#""data":{"x":1}"#));
    assert!(!original.contains("__rustxUri"));
}

#[test]
fn an_item_nobody_tagged_untags_to_no_uri() {
    let (original, uri) = json::untag_item(r#"{"label":"read"}"#);
    assert_eq!(uri, None);
    assert_eq!(original, r#"{"label":"read"}"#);
}
```

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustx-lsp --test json tag` (or wherever the test file lands)
Expected: FAIL — `tag_items`/`untag_item` not found.

- [x] **Step 3: Implement `tag_items` and `untag_item` in `crates/rustx-lsp/src/json.rs`**

Reuse this crate's existing brace/bracket-balance scanner (the same kind `lib.rs::cut_result` already uses) to find each top-level object in the array, and splice `,"__rustxUri":"<escaped uri>"` just before that object's closing `}`. `untag_item` finds and removes that one field with the same kind of scan this file already does for a named field (see `json::field`), rather than a general parser — there is no other field shaped like it to confuse it with.

- [x] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rustx-lsp --test json`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add crates/rustx-lsp/src/json.rs crates/rustx-lsp/tests/json.rs
git commit -m "feat(lsp): json can tag and untag one item of a completion list"
```

---

### Task 2: Every completion item says which `.rsx` it came from

**Files:**
- Modify: `crates/rustx-lsp/src/lib.rs` (the `"textDocument/completion" | "textDocument/hover" | "textDocument/definition"` arm; `assist`)
- Test: `crates/rustx-lsp/tests/assistance.rs`

**Interfaces:**
- Consumes: `json::tag_items` (Task 1).
- Produces: the `result` returned for `textDocument/completion` now carries `__rustxUri` on every item; `textDocument/hover` and `textDocument/definition` are untouched (resolve exists only for completion).

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn every_completion_item_is_tagged_with_its_page() {
    // Using this file's existing fixture-building helpers (see `a_page_of`
    // and the fake downstream server already started in this file's other
    // tests), request `textDocument/completion` on a page that has one
    // downstream server-provided item, and assert the item in the answer
    // carries `"__rustxUri"` equal to the page's own URI — not the shadow's.
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustx-lsp --test assistance every_completion_item_is_tagged_with_its_page`
Expected: FAIL

- [x] **Step 3: Wire `json::tag_items` into the completion path only**

In the match arm, after `assist`/`offer` produce `answer`, tag it only when `method == "textDocument/completion"` — `offer`'s own markup items (tag/attribute names) have no downstream `data` to preserve, but they are tagged the same way so resolve on one of them is still well-formed (Review Focus 2 is answered in Task 4, by `untag_item` succeeding regardless of what else is in the item).

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p rustx-lsp --test assistance`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add crates/rustx-lsp/src/lib.rs crates/rustx-lsp/tests/assistance.rs
git commit -m "feat(lsp): a completion item says which page it came from"
```

---

### Task 3: `completionItem/resolve` is answered

**Files:**
- Modify: `crates/rustx-lsp/src/lib.rs`
- Test: `crates/rustx-lsp/tests/assistance.rs`

**Interfaces:**
- Consumes: `json::untag_item` (Task 1), `shadows: &HashMap<String, shadow::Shadow>`, `downstream.held(&shadows)`, `PositionMap::to_rsx_position` (already exists in `rustx_compiler::map`).
- Produces: a new match arm `"completionItem/resolve"` that: untags the incoming item; if there was no tag or no shadow for that URI or no live downstream server, answers the item back unchanged (Review Focus 2 and 4); otherwise forwards the untagged item to `server.ask("completionItem/resolve", &item)`, and on a result runs every entry of `additionalTextEdits` through a new `retarget_edits(edits: &str, map: &PositionMap) -> String` that converts each `range` with `to_rsx_position` and **removes** (does not zero out) any edit whose start or end has no equivalent (Review Focus 1, 5).

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn a_resolve_with_no_tag_answers_the_item_unchanged() { /* … */ }

#[test]
fn a_resolve_whose_edit_lands_on_a_mapped_line_comes_back_translated() { /* … */ }

#[test]
fn a_resolve_whose_edit_lands_inside_a_jsx_region_drops_that_edit() { /* … */ }

#[test]
fn a_resolve_with_downstream_gone_answers_the_item_unchanged() { /* Review Focus 4 */ }
```

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustx-lsp --test assistance resolve`
Expected: FAIL

- [x] **Step 3: Implement the `"completionItem/resolve"` arm and `retarget_edits`**

`retarget_edits` reuses this file's existing brace-balanced item scanner (Task 1) to walk `additionalTextEdits`' array rather than parsing JSON generally, matching every other function in this module.

- [x] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rustx-lsp --test assistance && cargo test --workspace && cargo clippy --workspace --all-targets`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add crates/rustx-lsp/src/lib.rs crates/rustx-lsp/tests/assistance.rs
git commit -m "feat(lsp): completionItem/resolve brings the import back onto the .rsx"
```

---

### Task 4: The editor is told resolve exists

**Files:**
- Modify: `crates/rustx-lsp/src/lib.rs` (the `initialize` response, line ~226-232)
- Test: `crates/rustx-lsp/tests/assistance.rs` or `crates/rustx-lsp/tests/session.rs`

**Interfaces:**
- Consumes: Task 3.
- Produces: `completionProvider` gains `"resolveProvider":true` **only** inside the branch already guarded by `downstream.exists()`, next to `hoverProvider`/`definitionProvider` — a developer with no `rust-analyzer` on their machine must not see a promise this server cannot keep (Global Constraint 2).

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn resolve_is_promised_only_with_a_downstream_server() {
    // initialize with the fake downstream command available: result contains
    // "\"resolveProvider\":true".
    // initialize with no downstream command at all: result does not.
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustx-lsp --test assistance resolve_is_promised`
Expected: FAIL

- [x] **Step 3: Add `,"resolveProvider":true` to the `completion` string only in the `downstream.exists()` branch**

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --workspace`
Expected: PASS

- [x] **Step 5: Commit**

```bash
git add crates/rustx-lsp/src/lib.rs crates/rustx-lsp/tests/assistance.rs
git commit -m "feat(lsp): advertise completion resolve when there is a server to resolve with"
```

---

## What this plan delivers

Typing a short, unqualified name in a `.rsx` and accepting it from the completion list inserts the real `use` line on the `.rsx` itself, at a line that genuinely exists there — closing the last open item of `BUG.md` ("no auto-import for installed crates, everything must be fully qualified"). Where the import would land somewhere that has no `.rsx` equivalent, nothing is written instead of something wrong.
