---
status: pending
---

# Instruction: Natives over IPC and task wake-up

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
✅ crates/rustx-webview/src/natives.rs
✅ crates/rustx-webview/tests/natives.rs
✏️ crates/rustx-engine/src/task.rs
✏️ crates/rustx-engine/src/frame.rs
✏️ crates/rustx-engine/src/renderer/mod.rs
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Register a native that adds two numbers | Listed in the page's native table | system |
| Happy path   | Call it from the page | A Promise resolves with the sum | browser |
| Edge case    | The native returns an error or panics | The Promise rejects, the application keeps running | browser |
| Edge case    | A tokio task finishes while the window is idle | The loop wakes through the proxy and renders | system |
| Teardown     | Close the window | Pending requests are dropped without a panic | system |

## Tasks to do

### `1)` Natives as Promises

> Same registration, asynchronous call.

1. Keep register_native's signature and run the function on the loop thread when the request arrives.
2. Reply with the request id; the page resolves or rejects the Promise.
3. Reject names starting with the reserved prefix as today.

### `2)` Wake the loop from tasks

> Replace the current render thread wake-up.

1. Read task.rs and replace its wake-up with a tao event loop proxy.
2. Port on_frame users, which today are only in apps/video-editor, and note animation driven from Rust per frame as a known limit.

### `3)` Run apps/hybrid

> Prove async main works.

1. Start apps/hybrid on the new engine and list what differs from CEF.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | A native call from the page returns its value or rejects with its error. |
| 2 | A task completing on a worker thread triggers a render with no user input. |
| 3 | apps/hybrid runs its async main and the differences are written down. |
