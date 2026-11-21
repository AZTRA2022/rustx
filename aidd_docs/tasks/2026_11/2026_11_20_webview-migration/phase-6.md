---
status: done
---

# Instruction: Window controls on tao

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
✅ crates/rustx-webview/src/pane.rs
✅ crates/rustx-webview/tests/pane.rs
✏️ crates/rustx-os/src/window.rs
✏️ crates/rustx-engine/src/window.rs
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Open a frameless window | Pane built from the tao window handle | system |
| Happy path   | Drag, resize, set on top and close through rustx-os | The window follows each call | system |
| Edge case    | Call a control on macOS or Linux | Verified in CI only, otherwise reported as unverified | system |
| Teardown     | Close the window | Handles released | system |

## Tasks to do

### `1)` Feed tao's handle to rustx-os

> Pane::from_handle already takes a raw handle.

1. Read window.rs in both crates and map each control to a tao call or to rustx-os.
2. Build the Pane from the tao window's handle in pane.rs.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | Every control the CEF engine exposed works on Windows, and macOS and Linux status is stated. |
