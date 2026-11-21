---
status: pending
---

# Instruction: Remove the stub and its payload, update docs

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-cli/src/bin/rustx-setup.rs ❌
crates/rustx-cli/src/bin/setup-install.html ❌
crates/rustx-cli/src/bin/setup-uninstall.html ❌
crates/rustx-cli/src/selfextract.rs ❌
site/content/docs/tooling/packaging.mdx ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | phase 5 green | none | cli |
| Happy path | cargo build workspace | no stub binary | cli |
| Edge case | docs build | no broken link | cli |
| Teardown | none | none | cli |

## Tasks to do

### `1)` Delete

> Only after phase 5 is green.

1. Delete the stub, its pages and selfextract.

### `2)` Docs

> One account.

1. Rewrite the packaging page, whats-left and result.md for the 2-in-1 app and the system/ folder.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | nothing references rustx-setup or selfextract |
| 2 | the packaging page names every file of system/ |
