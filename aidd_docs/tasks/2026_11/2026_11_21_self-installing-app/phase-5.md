---
status: implemented
---

# Instruction: Package and prove it on Windows, macOS and Linux in CI

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-cli/src/installer.rs ✏️
crates/rustx-cli/tests/setup_ui.rs ✏️
.github/workflows/ci.yml ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | packaged counter per OS | artifact | cli |
| Happy path | install, start, uninstall | clean | system |
| Edge case | the installed copy is running during update | the page says so | system |
| Teardown | uninstall | baseline | system |

## Tasks to do

### `1)` Artifact

> The stamped binary plus the archive, per OS, no stub.

1. rustx package emits the stamped binary or bundle and signs it once.

### `2)` CI on three systems

> Install, update and uninstall paths.

1. Run the packaged binary with --install --silent, start the installed copy, assert it stays up.
2. Run a newer build over it for the update path.
3. Run the uninstall and assert nothing is left.
4. One test where install.rsx calls setup install against a redirected profile.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | no separate setup program in dist |
| 2 | the three CI jobs pass the three paths |
