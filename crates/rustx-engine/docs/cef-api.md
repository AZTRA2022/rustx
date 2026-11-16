| Build | Bare native call | DOM → Rust → DOM |
|---|---|---|
| release | 1.06 µs | 19.2 µs |
| debug | 2.68 µs | 31.4 µs |

Measured 2026-10-27 on a Dell OptiPlex 3010 (Ivy Bridge, 2012), averaged
over 200,000 and 20,000 calls. The DOM figure covers `Document::current()`,
`getElementById`, a `textContent` read and a `textContent` write — five V8
operations and two string conversions — so it is a whole round trip, not one
call. For comparison, an IPC `invoke()` costs tens to hundreds of
microseconds and cannot be done synchronously at all.

The handler holds its closure directly rather than looking it up by name on
each call; that one change took the bare call from 9.1 µs to 2.7 µs in debug.
# CEF API map

The implementation plan names CEF concepts by role. This file maps each role
to the identifier the pinned binding actually exposes. Re-check it on every
version bump: `cef-rs` tracks Chromium releases and renames freely.

## Pinned version

| Crate | Version | Notes |
|---|---|---|
| `cef` | `156.2.0+156.0.14` | Safe-ish wrappers, `wrap_*` macros, `Impl*` traits. Published 2026-10-25. |
| `cef-dll-sys` | `156.2.0+156.0.14` | Raw C bindings, pulled in transitively. |

The CEF API version compiled against is `CEF_API_VERSION=15600`.

## Build prerequisites

`cef-dll-sys` downloads the prebuilt CEF distribution and compiles the CEF C++
DLL wrapper from source, so a C++ toolchain is required:

- MSVC (Visual Studio 2022, `HostX64/x64`)
- CMake — installed here via `winget install Kitware.CMake` (4.4.4)
- Ninja — installed here via `winget install Ninja-build.Ninja` (1.13.2),
  which lands in
  `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe`
  and is **not** added to `PATH` automatically. Add it, or the build fails
  with `program not found: is cmake not installed?` — a misleading message,
  since it is Ninja that CMake cannot find.

The downloaded CEF distribution and the built DLL wrapper live under
`target/`, which is git-ignored. Nothing CEF-related is committed.

## Role → identifier

| Role in the plan | Identifier in `cef` 156.2.0 |
|---|---|
| Sub-process entry point | `cef::execute_process(args: Option<&MainArgs>, application: Option<&mut App>, windows_sandbox_info: *mut u8) -> c_int` — returns `>= 0` in a child process: exit immediately with that code. |
| Library init | `cef::initialize(args, settings: Option<&Settings>, application: Option<&mut App>, windows_sandbox_info: *mut u8) -> c_int` |
| Message loop | `cef::run_message_loop()`, `cef::do_message_loop_work()`, `cef::quit_message_loop()` |
| Teardown | `cef::shutdown()` |
| Process arguments | `cef::args::Args::new()` → `MainArgs`. On Windows `MainArgs` carries only the module `HINSTANCE`; CEF reads the real command line from the OS, so it never consumes `std::env::args()`. |
| Application handler | trait `ImplApp`, built with the `wrap_app! { struct X { .. } impl App { .. } }` macro, which produces an `X::new(..) -> App`. |
| Render-process hook | `ImplApp::get_render_process_handler()` |
| Render process handler | trait `ImplRenderProcessHandler`, macro `wrap_render_process_handler!` |
| V8 context created | `ImplRenderProcessHandler::on_context_created(&self, browser: Option<&mut Browser>, frame: Option<&mut Frame>, context: Option<&mut V8Context>)` |
| V8 context released | `ImplRenderProcessHandler::on_context_released(..)` — same shape. This is the hook that invalidates DOM handles (Task 4). |
| Native function body | trait `ImplV8Handler`, macro `wrap_v8_handler!`. Its method: `execute(&self, name: Option<&CefString>, object: Option<&mut V8Value>, arguments: Option<&[Option<V8Value>]>, retval: Option<&mut Option<V8Value>>, exception: Option<&mut CefString>) -> c_int` — return non-zero when handled; write `exception` to raise a JS exception. |
| Make a JS function | `cef::v8_value_create_function(name, handler)` |
| Make JS values | `cef::v8_value_create_string(Option<&CefString>)`, `v8_value_create_int(i32)`, and siblings |
| Current V8 context | `cef::v8_context_get_current_context() -> Option<V8Context>` |
| Create a window + browser | `cef::browser_host_create_browser_sync(window_info: Option<&WindowInfo>, client: Option<&mut Client>, url: Option<&CefString>, settings: Option<&BrowserSettings>, extra_info, request_context) -> Option<Browser>` |
| Window description | `cef::WindowInfo`, with `set_as_popup(parent, title)` for a top-level window and `set_as_windowless(parent)` for off-screen rendering |
| Reference counting | every wrapper implements `cef::rc::Rc`; the `wrap_*` macros generate `add_ref`/`Clone` for you |

## Notes that cost time to rediscover

- `exception` in `ImplV8Handler::execute` is how a native function reports an
  error to JavaScript. Returning zero instead makes V8 report "function does
  not exist", which is wrong and confusing.
- V8 handles may only be touched on the thread that created them — the render
  process main thread. This is the same constraint the design document's
  threading model imposes for its own reasons, so it costs nothing.
- `set_as_windowless` requires `Settings.windowless_rendering_enabled`, and
  switches `runtime_style` to Alloy.

## Measured round-trip

The project's reference figures for "no boundary", kept honest by
`tests/no_boundary.rs`.

| Build | Bare native call | DOM → Rust → DOM |
|---|---|---|
| release | 1.06 µs | 19.2 µs |
| debug | 2.68 µs | 31.4 µs |

Measured 2026-10-27 on a Dell OptiPlex 3010 (Ivy Bridge, 2012), averaged over
200,000 and 20,000 calls. The DOM figure covers `Document::current()`,
`getElementById`, a `textContent` read and a `textContent` write — five V8
operations and two string conversions — so it is a whole round trip, not one
call. For comparison, an IPC `invoke()` costs tens to hundreds of
microseconds and cannot be made synchronously at all.

The V8 handler holds its closure directly instead of looking it up by name on
every call; that one change took the bare call from 9.1 µs to 2.7 µs in
debug.

The same run proves where the code runs: the browser process reports its own
pid, the page reports the pid and role of the process its Rust executes in,
and they differ — `Renderer`, the same process as V8.

## The one that costs an afternoon

`cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0)` must be called **in every
process, before any other CEF call**. CEF's C API is versioned: libcef reads
the version the client declares before accepting any structure the client
implements. Skip it and the process aborts with exit code 3 and

```
FATAL:cef\libcef_dll\ctocpp\app_ctocpp.cc:118] CefApp_0_CToCpp called with invalid version -1
```

the first time a Rust-implemented handler is handed to CEF — while a run that
passes `None` for every handler works fine, which is what makes it so
misleading. The crate only does this in a `#[cfg(test)]` helper, so it is on
the embedder. `rustx_engine::declare_api_version()` is that call.

Two more that cost less but still cost:

- The browser must be created from `ImplBrowserProcessHandler::on_context_initialized`,
  not straight after `cef::initialize`. Calling it early aborts inside
  `browser_host_create_browser_sync`.
- `WindowInfo::runtime_style` must be `RuntimeStyle::ALLOY` for a browser
  hosted in a window we own. The default, Chrome style, brings the whole
  browser UI with it.
- `Settings::root_cache_path` must be set per application. The default is
  shared, and Chromium enforces a process singleton per cache directory: a
  second application hands its window to the first and exits.

## The GPU process on legacy hardware

On the development machine (Dell OptiPlex 3010, Intel HD Graphics, driver
10.18.10.4252) Chromium's GPU process dies on startup with an access
violation, four times, before falling back to software rendering. The window
stays blank for those few seconds.

This is not something the host can fix, and it is not the sandbox: the crash
is identical with `--disable-gpu-sandbox`, and with the `sandbox` feature on
or off. What does avoid it:

| Switch | Result |
|---|---|
| _(none)_ | 4 GPU process crashes, then software fallback |
| `--disable-gpu-sandbox` | 4 crashes — rules the sandbox out |
| `--in-process-gpu` | no crash |
| `--use-angle=swiftshader` | no crash |
| `--disable-gpu-compositing` | no crash |

`rustx` does **not** set any of these. Hardware acceleration is what the
design document's performance bar depends on, and crippling it for every
machine because of one 2012 graphics driver would be the wrong trade. Pass
`--disable-gpu-compositing` on the command line when developing on such a
machine.

The `sandbox` feature of the `cef` crate is nonetheless left **off**, for its
own reason: it is a default feature, it builds the wrapper with
`USE_SANDBOX=ON`, and CEF then expects the host to create Windows sandbox
info and pass it to `execute_process` and `initialize`. `rustx` deliberately
runs an unsandboxed render process (design document §7.1), so declaring a
sandbox it does not set up would be a lie waiting to crash.

## Calling Rust from the page

Three things are not obvious, and each one costs a debugging session.

**Sub-process command lines are rebuilt from scratch.** Nothing the
application was launched with reaches the render process by itself — the
renderer's command line is Chromium's own (`--type=renderer`,
`--user-data-dir=…`, field trial handles). Since the render process is where
the application's Rust runs, the engine forwards the host's arguments in
`ImplBrowserProcessHandler::on_before_child_process_launch`.

**A sub-process does not reliably inherit the parent's stdout.** Its stderr
arrives (a Rust panic in the renderer shows up on the parent's stderr), its
stdout does not. Anything a test needs out of the render process goes through
a file, not a pipe.

**`exception` must be written with `try_set`, not assigned.** The handler
signature hands over `Option<&mut CefString>`, and that `CefString` is a
`BorrowedMut` view of the caller's `cef_string_t`. Writing
`*slot = CefString::from(msg)` replaces the Rust-side wrapper and leaves the
caller's buffer untouched: no exception ever reaches JavaScript, and the call
silently returns `undefined`. `slot.try_set(&msg)` writes through.

And one that is merely worth knowing: an exception thrown inside a DOM event
listener does not propagate to whoever called `element.click()`. A page
driving itself has to read the result out of the DOM rather than expecting a
throw.

## Driving a frame loop

`requestAnimationFrame` is the compositor's own vsync signal and it already
runs on the thread that owns the DOM, so the engine's frame loop rides on it
rather than inventing a timer that would drift against the screen.

Getting the loop started is the fiddly part. Two ways do **not** work from
`on_context_created`:

- `V8Context::eval` called bare — it only works inside a V8 callback, or
  with `enter()`/`exit()` around it.
- `V8Context::eval` with `enter()`/`exit()` — it runs, but a
  `requestAnimationFrame` registered that early is dropped and the loop
  never ticks. This one is particularly unhelpful: nothing fails, no
  exception appears, the callback simply never fires.

What works is `ImplFrame::execute_java_script`, which schedules the script
to run in the page itself at a point where rAF registrations stick.
