---
status: implemented
---

# Instruction: Fix what just shipped

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-cli/src/package.rs ✏️
crates/rustx-os/src/install.rs ✏️
crates/rustx-macros/src/lib.rs ✏️
crates/rustx/src/setup.rs ✏️
crates/rustx-cli/src/dev.rs ✏️
.github/workflows/ci.yml ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | counter packaged, stamped and signed with a self-signed certificate | signed exe | cli |
| Happy path | run it with --install --silent | installed | system |
| Edge case | start the installed copy then run the newer exe | clear error or close then success | system |
| Teardown | uninstall | clean | system |

## Tasks to do

### `1)` A signed exe still installs

> The stamp must survive signing.

1. Put the stamp in a PE resource, or read it before the security directory.
2. Sign a stamped exe with a self-signed certificate and check that the silent install still works.

### `2)` Apps with main(pages)

> No empty window.

1. List the apps using main(pages); stage system pages for them or return Normal when the route table has none.

### `3)` Licence carried in the binary

> The licence step must appear in a single exe.

1. Carry LICENSE into the binary at package time and read it from there.
2. Add a LICENSE to counter and assert the Licence step in the CI install scenario.

### `4)` Errors and maps name app/

> Staging must be invisible.

1. Check the compiler error path, the .rsx.map and an LSP diagnostic for a deliberate error.

### `5)` rustx dev watches system/

> A saved system page rebuilds.

1. Add system/ to the watched roots.

### `6)` Linux icon and macOS usage descriptions

> No first-use kill on macOS, an icon on Linux.

1. Install an icon and write Icon= in the .desktop.
2. Write the NS usage description keys into Info.plist from Config::permissions.

### `7)` Update over a running copy

> A clear outcome instead of a locked file.

1. Detect the running installed copy through the instance lock and offer to close it.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | the signed exe installs |
| 2 | a main(pages) app starts normally when stamped |
| 3 | the Licence step is shown in the CI scenario |
| 4 | a deliberate error names app/ and the LSP diagnostic is on the right line |
| 5 | editing system/about.rsx under rustx dev rebuilds |
| 6 | desktop-file-validate passes with Icon=, the plist has the usage keys |
| 7 | the update path reports the running copy |
