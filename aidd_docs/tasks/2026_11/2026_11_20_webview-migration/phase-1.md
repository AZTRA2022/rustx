---
status: in-progress
---

# Instruction: Safety net and wire protocol

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
✅ crates/rustx-webview/Cargo.toml
✅ crates/rustx-webview/src/lib.rs
✅ crates/rustx-webview/src/protocol.rs
✅ crates/rustx-webview/tests/protocol.rs
✅ aidd_docs/tasks/2026_11/2026_11_20_webview-migration/baseline.md
✏️ crates/rustx-dom/src/diff.rs
✏️ Cargo.toml
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Build apps/counter in release with CEF and record size, release build time, time to first frame | Numbers written in baseline.md | cli |
| Happy path   | Serialize every Patch variant and decode it back | Decoded patch equals the original | system |
| Edge case    | Decode a message with an unknown tag or a truncated body | A typed error, no panic | system |
| Teardown     | Remove the build output of the baseline run | Working tree holds only the baseline file | cli |

## Tasks to do

### `1)` Record the baseline

> Fix the numbers phase 7 is judged against.

1. Build apps/counter in release with the current CEF engine.
2. Record delivered size, release build time, time to first frame and click-to-paint latency in baseline.md.
3. List which tests in crates/rustx-engine/tests, including common, depend on CEF, and read crates/rustx/src/lib.rs where it calls register_native and on_frame.

### `2)` Define the wire protocol

> One schema for patches, events and native requests.

✅ Create the rustx-webview crate and add it to the workspace.
2. Write protocol.rs with the host-to-page message (patch batch, native reply) and the page-to-host message (event with handler id, native request with request id).
3. Reuse Patch from rustx-dom/src/diff.rs and Event::as_value for events; add only the serialization the Patch type lacks.
4. Reject unknown tags and truncated messages with a typed error.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | baseline.md holds three measured numbers and the list of CEF dependent tests. |
| 2 | Every Patch variant survives a round trip and a malformed message returns an error. |
