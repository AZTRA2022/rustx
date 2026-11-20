---
status: done
---

# Instruction: Remove CEF

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
❌ crates/rustx-engine/src/cef_app.rs
❌ crates/rustx-engine/src/bin/hello.rs
✏️ crates/rustx-engine/Cargo.toml
✏️ crates/rustx-cli/src/package.rs
✏️ crates/rustx-cli/src/installer.rs
✅ aidd_docs/tasks/2026_11/2026_11_20_webview-migration/result.md
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Run the full workspace tests on the new engine | All green or each failure explained | cli |
| Happy path   | Package apps/counter in release | Delivered folder has no libcef, locales or helper copies, and its size is compared with baseline.md | cli |
| Edge case    | Package on a machine without Inno Setup | Falls back as today; external tool removal is a separate plan | cli |
| Teardown     | Delete build outputs | Clean tree | cli |

## Tasks to do

### `1)` Delete the CEF code and dependency

> Nothing links to Chromium any more.

1. Move the tests that relied on hello.rs to the new engine, then delete hello.rs and cef_app.rs.
2. Remove the cef dependency and make the new engine the only one behind the rustx facade.

### `2)` Stop shipping CEF

> The package holds the executable and the app.

1. Remove the locale copy, helper bundles and libcef handling from package.rs and installer.rs.
2. Add the Linux package dependency on libwebkit2gtk-4.1.
3. Compare delivered size, build time, start time and click-to-paint latency with baseline.md in result.md.

### `3)` Update the documentation

> The documents describe the new model.

1. Rewrite the README and design document passages about CEF and in-process natives.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | The workspace builds with no cef crate in the dependency tree. |
| 2 | result.md holds the four comparisons and the delivered size is a few MB plus the app. |
| 3 | No document claims that a native call is a same-thread call. |
