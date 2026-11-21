---
status: done
---

# Instruction: Patch applier and event round trip

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
✅ crates/rustx-webview/assets/applier.js
✅ crates/rustx-webview/src/session.rs
✅ crates/rustx-webview/tests/counter.rs
✏️ crates/rustx-engine/src/render.rs
✏️ crates/rustx-engine/src/router.rs
✏️ crates/rustx-engine/src/dom.rs
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Start apps/counter on the new engine | First frame drawn from one patch batch | browser |
| Happy path   | Click the button three times | The counter shows 3 and each click was one IPC event | browser |
| Edge case    | Type fast in a controlled input | No keystroke lost and the caret does not jump | browser |
| Edge case    | Click an internal link | The router navigates and the browser does not reload | browser |
| Edge case    | Call a synchronous Node read such as text() on the new engine | A documented error naming the async replacement | system |
| Teardown     | Close the window | Process exits cleanly | system |

## Tasks to do

### `1)` Apply patches in JavaScript

> The page holds the DOM, Rust sends patches.

1. Write applier.js to apply a patch batch once per frame.
2. Do not rewrite the value of a focused field when it is already identical.

### `2)` Send events back

> Handlers stay in Rust, called by id.

1. Attach one delegated listener per event kind that posts the event record and handler id.
2. Run the Rust handler on the loop thread and render the resulting batch.
3. Decide default prevention in JavaScript from a flag declared in the patch, and intercept router links in JavaScript.

### `3)` Handle synchronous DOM reads

> Public Node reads cannot answer synchronously.

1. Read dom.rs and list every caller of text, attribute, tag_name, query_selector and count_matching.
2. Replace each with an async equivalent or remove it, and document the break.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | The initial render and later updates reach the DOM from patches alone. |
| 2 | apps/counter works end to end and typing in a controlled input loses nothing. |
| 3 | No synchronous read remains on the new engine and the caller list is written down. |
