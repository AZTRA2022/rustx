---
status: pending
---

# Instruction: system/*.rsx compiled, never routed, with default pages

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-macros/src/lib.rs ✏️
crates/rustx-compiler/src/build.rs ✏️
crates/rustx/assets/system/install.rsx ✅
crates/rustx/assets/system/uninstall.rsx ✅
crates/rustx/assets/system/update.rsx ✅
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | an app with only system/install.rsx | compiles | cli |
| Happy path | the install page calls setup install | progress then done | system |
| Edge case | a file with another name in system/ | refused with the locked names listed | cli |
| Teardown | none | none | cli |

## Tasks to do

### `1)` Compile system/

> Locked names, optional, never wrapped by app/layout.rsx.

1. Compile each locked file as a page under a reserved route the router never lists; refuse another name in system/ with a sentence naming the locked ones.
2. Fall back to a default RSX page for a missing file.

### `2)` Default install, uninstall and update pages

> Replace the HTML defaults.

1. Write the three defaults using rustx::setup.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | a custom install.rsx is shown and app/layout.rsx is not applied to it |
| 2 | an app with no system/ folder shows the defaults |
