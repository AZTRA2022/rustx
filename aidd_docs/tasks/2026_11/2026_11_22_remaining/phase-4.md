---
status: pending
---

# Instruction: The rest of BUG.md

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-cli/src/dev.rs ✏️
crates/rustx-webview/src/window.rs ✏️
crates/rustx-webview/src/router.rs ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | a plan per item | approved | cli |
| Happy path | each item ships with its check | green CI | system |
| Edge case | an item that does not pay off | recorded and dropped | cli |
| Teardown | none | none | cli |

## Tasks to do

### `1)` Fast refresh

> Measure, then decide.

1. Try rust-lld and record the gain on counter and video-editor; spike a native hot patch behind a flag only if the numbers justify it.

### `2)` Second window

> Controlled from the first.

1. Send Orders to the child over the stdin pipe that links the two processes.

### `3)` Next.js features

> Per-route metadata and redirects.

1. Per-route window title and router redirects, then what the inventory ranks next.

### `4)` Editor and LSP audit

> A ranked list against rust-analyzer and ES7+.

1. Run the audit skill on completion latency, diagnostics, go to definition and snippets.

### `5)` Tauri gaps

> In the user's order.

1. Per-native capability scopes, macOS signing and notarisation, sidecar binaries, window effects, cross compiling from one host.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | a number before and after for the edit-to-window time |
| 2 | a window opened by a page is resized from the first |
| 3 | a route sets the window title |
| 4 | a ranked audit list exists |
| 5 | the first gap the user ranks ships |
