# Baseline: apps/counter on the CEF engine

Measured 2026-11-20 on the development machine (Windows 10 Pro 10.0.19045), commit `1ceb94c` on branch `webview-migration`. Phase 7 repeats each measure the same way.

## How it was measured

- Build and delivery: `rustx package --no-zip --out <dir>`, run in `apps/counter`, with the debug build of the `rustx` command line.
- Build state: cold for the `counter` release target and for `cef`, `rustx-engine`, `rustx-app` and `counter`. The machine was also compiling `rustx-webview` in the same cargo workspace during that run, so the time is an upper bound, not a clean figure.
- Window time: a PowerShell stopwatch from `Start-Process` of the delivered `counter.exe` until a process of that name has `MainWindowHandle -ne 0`, then the process is stopped. Three runs. Reuse this script as is in phase 7.

## Numbers

| Measure | Value |
| --- | --- |
| Release build and packaging, cold | 1757 s (29 min 17 s); cargo itself reported 19 min 43 s for the compile |
| Delivered folder, without installer | 411 MB, 237 files |
| Installer (Inno Setup, installed on this machine) | 132.4 MB |
| Folder plus installer | 543.7 MB |
| `counter.exe` itself | 1.14 MB |
| `libcef.dll` | 281 MB |
| `locales/` | 49.8 MB, 220 files |
| `dxcompiler.dll` | 24.8 MB |
| `resources.pak` | 23.5 MB |
| `icudtl.dat` | 10.3 MB |
| `.pdb` files delivered | 0 |
| Time to window handle, runs 1 / 2 / 3 | 348 ms / 118 ms / 136 ms |

The user's "about 200 MB" in BUG.md matches the installer and the compressed folder, not the folder on disk. The Rust executable is 1.14 MB: the weight is entirely Chromium.

## Not measured

- Time to first paint and click-to-paint latency. The window handle exists well before the page is drawn (118 ms is the window, not the page). The only existing reporting channel, `REPORT key=value` through a native function, exists in the `hello` binary and not in an ordinary application. Adding CEF instrumentation to measure it would be thrown away in phase 7, so the criterion "click-to-paint" is partially met: phase 3 defines it on the new engine and phase 7 compares against a measurement taken on CEF by then with the same page script.
- Rebuild time after editing a page.

## A finding about slowness

Each launch of the delivered counter prints, on this machine, `GPU process exited unexpectedly: exit_code=-2147483645` three times and `Failed to create shared context for virtualization`. Chromium falls back to software rendering. This is a probable cause of the "extremely slow application" in BUG.md, independent of the Rust code. It is unverified as the cause: nothing was compared with a machine whose GPU process stays up.

## Tests that depend on CEF

`crates/rustx-engine/tests/common/mod.rs` starts the `hello` binary (`CARGO_BIN_EXE_hello`) and reads its `REPORT` lines.

- 20 test files include `common` and drive `hello`: architecture, bootstrap, closing, dom, early_navigate, foreign, frame, heavy, lifecycle, local_only, mount, native_call, nested_layout, no_boundary, no_frames, render, router, subframe, task, without_a_window.
- 10 test files do not: awaited, a_new_context_reopens_the_mailbox, bridge, dropped, runtime, runtime_is_lazy, runtime_stops_with_the_window, second_window, watchdog, window. Whether they link CEF in another way is unchecked.

## Callers of register_native and on_frame in the facade

`crates/rustx/src/lib.rs`: `on_frame` at lines 825 and 1045; `register_native` at line 1063 (`rustx_shared`) and `rustx_engine::register_native` at line 1125 (`rustx_about`). Beyond the facade: `apps/video-editor/src/main.rs` (3 sites) and `crates/rustx-engine/src/render.rs:129`.
