---
status: pending
---

# Instruction: Visual proof and editor support for system/

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-lsp/src/shadow.rs ✏️
.github/workflows/ci.yml ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | CI runners with a display | none | system |
| Happy path | open each screen and capture it | 7 images per OS | system |
| Edge case | a screen that fails to draw | the job fails naming it | system |
| Teardown | none | none | system |

## Tasks to do

### `1)` Screenshots in CI

> The user can look at the 7 screens without a Mac.

1. Capture the 7 default screens on the three systems and upload them as an artifact.

### `2)` system/ in the language server

> Completion and diagnostics.

1. Give system/*.rsx shadows and completion.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | the artifact holds 21 images |
| 2 | a diagnostic on a system page lands on the right line |
