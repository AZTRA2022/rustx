---
status: implemented
---

# Instruction: Package stamp and mode detection

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-cli/src/package.rs ✏️
crates/rustx-webview/src/app.rs ✏️
crates/rustx/src/lib.rs ✏️
crates/rustx-os/src/lib.rs ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | a stamped and an unstamped binary | both built | system |
| Happy path | run the stamped binary outside the install location | install mode | system |
| Edge case | cargo run or rustx dev | normal start | system |
| Teardown | remove installs | baseline | system |

## Tasks to do

### `1)` Stamp

> Only rustx package marks a binary installable.

1. Append a trailer with the install metadata before signing; read it at start.

### `2)` Modes

> A silent path for CI.

1. Install mode when stamped and outside the install location; update mode when a different version is installed.
2. Add --install --silent, --uninstall with a folder and --silent, and Config::portable().
3. Release the single-instance lock before relaunching from the install location.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | the trailer is read back; an unstamped binary has none |
| 2 | debug and dev builds never enter install mode; the installed copy takes the instance lock |
