---
status: pending
---

# Instruction: Install, uninstall and update per OS, and the setup API

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-os/src/install.rs ✅
crates/rustx-os/src/lib.rs ✏️
crates/rustx/src/lib.rs ✏️
crates/rustx-cli/src/bin/rustx-setup.rs ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | redirected home and profile folders | clean profile | system |
| Happy path | install then uninstall | files and registration present, then gone | system |
| Edge case | truncated copy | refused, nothing registered | system |
| Teardown | remove the redirected folders | baseline | system |

## Tasks to do

### `1)` One install module, three implementations

> Copy, register, verify, remove, per OS.

1. Create rustx-os/src/install.rs: Windows (LOCALAPPDATA, shortcut, Apps and features key, delayed delete), macOS (bundle copied to the user Applications folder, relaunched with open), Linux (user local share, .desktop, icon, symlink in the user bin).
2. Verify the copy by size and digest before registering.

### `2)` rustx::setup

> What a page calls.

1. Add info(), install(), launch(), uninstall(), quit(); install() is a task writing progress into state.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | each OS installs into a redirected profile and removes itself; a truncated copy is refused |
| 2 | info() reports name, version, destination, mode and progress reaches a page |
