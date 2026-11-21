---
status: implemented
---

# Instruction: Bootstrapper and auto-updater

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx-cli/src/package.rs ✏️
crates/rustx/src/setup.rs ✏️
crates/rustx-macros/system/settings.rsx ✏️
crates/rustx-macros/system/permissions.rsx ✏️
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | WEBVIEW2_BROWSER_EXECUTABLE_FOLDER on an empty folder | no runtime | system |
| Happy path | a local manifest over loopback names a newer version | downloaded, verified, update wizard | system |
| Edge case | digest mismatch | refused | system |
| Teardown | stop the server | clean | system |

## Tasks to do

### `1)` Bootstrapper

> The installer must run without WebView2.

1. Detect a missing runtime before opening a window; show a native box.
2. Download and run Microsoft's Evergreen bootstrapper and continue; on Linux name the missing package.

### `2)` Auto-updater UI

> Check, download, verify, update.

1. Add Check for updates to settings.rsx through rustx::update.
2. Download to a temp file, verify the digest, run it, and show update.rsx.

### `3)` Real permission state

> Say what the OS says.

1. Read the macOS authorization status and the Windows consent store; otherwise show asked on first use.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | a missing runtime takes the prompt path and not a crash |
| 2 | the update flow runs against a loopback manifest in CI |
| 3 | the page shows a real state where the OS exposes it |
