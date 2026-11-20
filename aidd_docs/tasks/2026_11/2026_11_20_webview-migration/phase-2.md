---
status: pending
---

# Instruction: tao + wry host

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
✅ crates/rustx-webview/src/host.rs
✅ crates/rustx-webview/src/assets.rs
✅ crates/rustx-webview/tests/host.rs
✏️ crates/rustx-engine/src/guard.rs
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup        | Build a static hello world with the new host in release | An executable of a few MB | cli |
| Happy path   | Start it | A window shows the page, no console window opens, data folder is under the user's local app data | system |
| Edge case    | The user clicks an https link | It opens in the system browser and the app page stays | system |
| Edge case    | The app allows an https origin for in-window navigation and the page navigates there | The remote page loads and its scripts cannot call any native function | system |
| Edge case    | An origin outside the allow-list is navigated to in the window | Navigation is refused with a message naming the origin | system |
| Edge case    | WebView2 runtime missing | A clear message naming the runtime, no crash | system |
| Teardown     | Close the window | Process exits with code 0 | system |

## Tasks to do

### `1)` Open a window with a webview

> The smallest host that draws a page.

1. Pin wry and tao and read their APIs in the pinned version before coding.
2. Create the window and webview in host.rs on the tao event loop thread.
3. Set the WebView2 data folder under the user's local app data through wry's web context.

### `2)` Serve the page without file URLs

> Replace the file:// origin workaround.

1. Register a custom protocol in assets.rs that serves the application's files.
2. Check the origin in the IPC handler: only the application's own origin may send events or call natives.

### `3)` Allow external navigation

> Remote pages are reachable and harmless to the host.

1. Open http and https links in the system browser by default.
2. Let the application configure an allow-list of origins that may load inside the window; refuse any other with a message naming it.
3. Rework guard.rs, whose rule that only local content loads existed because CEF's render process was unsandboxed and no longer holds.

### `4)` No terminal

> The release binary is a GUI program.

1. Confirm the generated main already carries the windows_subsystem attribute and that the hello binary does too.
2. Read the PE subsystem field of the release executable and check it equals 2.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | The window opens and its data folder is not next to the executable. |
| 2 | An IPC message from any origin other than the application's is ignored. |
| 3 | External links open in the system browser, an allow-listed origin loads in the window without native access, and any other origin is refused by name. |
| 4 | The PE subsystem of the release executable is 2. |
