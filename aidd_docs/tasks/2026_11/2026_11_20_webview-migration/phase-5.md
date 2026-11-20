---
status: pending
---

# Instruction: React boundary in JavaScript

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
✅ crates/rustx-webview/assets/foreign.js
✅ crates/rustx-webview/tests/foreign.rs
✏️ crates/rustx-engine/src/foreign.rs
✏️ crates/rustx-engine/src/bridge.rs
✏️ crates/rustx-engine/src/mount.rs
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Mount a shadcn Textarea through the boundary | The component renders | browser |
| Happy path   | Type in it with onInput taking an annotated rustx_dom::Event | The handler receives the typed text | browser |
| Edge case    | A library calls a prop with a plain string | The closure receives a Value, not an Event | browser |
| Edge case    | A prop is given a function value | The description carries a callback id, not a function | system |
| Teardown     | Unmount the page | React roots are released | browser |

## Tasks to do

### `1)` Describe the tree as data

> Functions cannot cross; ids can.

1. Port describe and describe_node from foreign.rs to a JSON description with callback ids.
2. Build the React tree in foreign.js and bind each callback id to a function that posts to the host.

### `2)` Serialize events for components

> A React synthetic event is circular.

1. Extract type, target value, checked, key and modifiers in foreign.js before posting, as read_event does today.
2. Check the result against Event::from_value so the earlier onClick fix still holds.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | A component with nested components and callbacks renders and its callbacks fire. |
| 2 | event.value holds the typed text and the typed closure compiles and runs. |
