# Runtime and systems — stability log

The runtime half of the mission in `STABILITY.md`: `crates/rustx-engine`,
`crates/rustx` (published as `rustx-app`) and `crates/rustx-os`. Four
other agents own the compiler, the LSP, the CLI and `apps/`
concurrently; nothing outside those three directories is touched here,
and anything found elsewhere is reported rather than fixed.

This document is written so that the work can be resumed cold. It says
what was audited, what was wrong, what the root cause was, what the fix
is, which test pins it, and — explicitly — what this machine cannot
verify.

**How to run anything here**

```powershell
$env:CARGO_TARGET_DIR = "C:\Users\aztra\AppData\Local\Temp\rustx-target-runtime"
$env:RUSTX_SOFTWARE = "1"      # there is no GPU on this machine
cargo test --offline -p rustx-engine --lib
cargo test --offline -p rustx-engine --test architecture --test without_a_window
cargo test --offline -p rustx-os --lib
cargo test --offline -p rustx-app
```

PowerShell, never Git Bash: `/usr/bin/link` shadows MSVC `link.exe`.
`--offline` always: there is no network. A running app holds the CEF
DLLs, so the next build fails with `os error 5` until it is killed.

---

## 1. The map

```
rustx-app (crates/rustx)         the one crate an application depends on
  │  Config → App → window; the route table, the components, the bundle,
  │  the stylesheet; the `#[rustx::main]` prelude.
  ▼
rustx-engine                     the CEF host and the frame loop
  │  app/bootstrap/cef_app/client   the processes and the window
  │  renderer/ + value + dom        the V8 boundary
  │  frame                          the frame loop, the one door for threads
  │  render + mount + tree          the reconciler's output into the DOM
  │  router                         one screen at a time
  │  task + runtime                 threads and futures, with tokens
  │  foreign                        the React boundary, both directions
  │  watchdog + panics + guard      the three safety nets
  ▼
rustx-dom (NOT OURS — compiler agent)   Element, diff, state, store, route
rustx-os                         the platform: clipboard, dialogs, tray, …
```

Critical execution paths, in the order a frame takes them:

1. `renderer::install` — a V8 context appears; natives are installed,
   the frame queue and the remote mailbox are opened, the driver script
   and the application's scripts go in.
2. `__rustx_frame` → `frame::tick` — drain `state::apply_remote` (what
   futures posted), drain the frame queue (what threads posted), then
   run the frame callbacks.
3. `render::render_dirty` (the one frame callback) — hand over pending
   React boundaries, take the dirty instances, map each to its root,
   re-render those roots, diff, patch.
4. `router::screen` — the one component the router mounts; reads the
   `Current` store, matches the table, draws the page inside its
   layouts.
5. `cef_app::on_context_released` / `client::on_before_close` — the
   teardown.

---

## 2. Audit findings

Severity uses `STABILITY.md` §4. "Fixed" entries have a regression test
named in §3 unless the row says otherwise.

| # | Severity | Where | Finding | State |
|---|----------|-------|---------|-------|
| R1 | **P0** | `render.rs` | The root registry was iterated under `RefCell::borrow_mut` while components and React ran. A React subtree that mounts a rustx component (`rustx_component`) or a React cleanup that releases one (`rustx_release`) reaches back into that registry → `already mutably borrowed: BorrowMutError`, i.e. a panic in the middle of a frame, leaving the page half-patched. | **Fixed** |
| R2 | **P1** | `render.rs` | `render_dirty` took the dirty set *before* checking for a document and returned if there was none. `take_dirty` clears what it hands back and nothing marks an instance twice, so every write made on a frame that ran without a context was lost for good — a silently stale screen. | **Fixed** |
| R3 | **P1** | `router.rs`, `rustx/src/lib.rs` | There was no default screen for a page that panicked. Without an `error.rsx` the panic went to the frame boundary, which caught it and carried on: the window kept the *previous* render and the reason went only to stderr. `STABILITY.md` §3 asks for this screen by name. | **Fixed** |
| R4 | **P1** | `frame.rs`, `cef_app.rs` | `render::unmount_all()` existed and was called from nowhere. Nothing released the mounted roots when a window closed or a context was replaced, so no component's `Drop` ran: a `task(…)` thread kept looking at a flag nobody would ever flip, and a file or capture session stayed open. | **Fixed** (verification is window-only — see §4) |
| R5 | **P1** | `router.rs` | The history only ever grew. An application that writes a filter into the address on every keystroke, or switches panel on a timer, adds a `String` per navigation for as long as the window is open. | **Fixed** |
| R6 | **P1** | `router.rs` | A navigation asked for before the router mounted was held in a *thread-local*. A `task(async …)` started from `main` runs on a runtime worker, where the router is not mounted and never will be, so the ask was written into that worker's own cell and never read: the screen never came up and nothing said why. | **Fixed** |
| R7 | **P1** | `windows/clipboard.rs` | `read_dib` reached `chunks_exact(0)` — which panics — for a clipboard header claiming a zero width or height. The header comes from whichever program the user last copied from, so the panic was reachable from outside the application entirely. | **Fixed** |
| R8 | **P1** | `windows/clipboard.rs` | `written_dib` panicked the same way for `set_image` of an image with no pixels, and computed `width * 4 * height` unchecked (wraps in release on a 32-bit target, making a short buffer pass the length check). | **Fixed** |
| R9 | **P1** | `rustx-os` (all of it) | Only `terminal::run` documented that it blocks. A file dialog waits for a person, the clipboard waits for whichever application holds it, a device enumeration waits for COM — called from an `onClick`, each stops the window, and the symptom a developer sees is "the framework hung". | **Fixed** |
| R10 | **P2** | `renderer/mod.rs` | `in_context` entered the page's V8 context, ran the frame, and exited — with no guard. A panic between the two left V8 with a context entered for ever, and every frame after it entered another on top. | **Fixed** (no headless test — see §4) |
| R11 | **P2** | `render.rs` | The page's one frame callback was registered when the root count reached 1. An application whose last root is released and whose next one is mounted is back at one root, so it registered a second callback and rendered itself twice per frame from then on. | **Fixed** (no headless test — see §4) |
| R12 | **P2** | `rustx/src/lib.rs` | Two default not-found screens existed (the facade's `no_such_page`, and nothing in the engine). One abstraction now, in the engine, which the facade overrides. | **Fixed** |
| R16 | **P0** | `render.rs`, `tree.rs` | Leaving a nested layout left its last page's DOM in the document. A mounted subtree was detached with `node.dom.remove()`, and `MountedNode::dom` is `None` for a fragment and for an `Empty` — so removing a fragment removed nothing, while the bookkeeping that pointed at its children was replaced. Reported from an application; one copy left behind per navigation, for as long as the window was open. | **Fixed** |
| R17 | P2 | `render.rs` | The same `dom`-or-nothing assumption in `place`: a following sibling with no node of its own was skipped, so a child inserted before a fragment landed after everything the fragment held. | **Fixed** |
| R18 | P1 | `renderer/mod.rs` | `__rustx_frame` was registered through the public `register_native`, so it sat in the application's own namespace (replaceable) and on the JavaScript global as an ordinary writable property (callable, shadowable, deletable by the page). `dette-connue.md` §1. | **Fixed** |
| R19 | P1 | `frame.rs` | `Poster::post` from the browser process was accepted and never drained — `tick` runs in the render process — so the queue grew without bound and without a word. `dette-connue.md` §3. | **Fixed** |
| R20 | P1 | `lib.rs` | Nothing refused `panic = "abort"`, which turns every `catch_unwind` the engine's tolerance is built on into an abort of the process. `dette-connue.md` §5. | **Fixed** |
| R21 | P2 | `app.rs` | `App::run` called `cef::shutdown()` as the statement after `run_message_loop`, so a panic coming up through the loop skipped it. `dette-connue.md` §8. | **Fixed** |
| R22 | P2 | `rustx-os`, `rustx-engine`, `rustx` | 80 user-visible strings — error messages, refusals, test assertions — were in French. | **Fixed** |
| R13 | P2 | `watchdog.rs` | The watchdog thread loops for the life of the process and is never stopped. Cheap (one wake every 500 ms) but it is an uncontrolled background task by the letter of §3. | Open |
| R14 | P2 | `frame.rs` | `schedule_check` reschedules itself for ever: a closed window's renderer keeps a 50 ms timer alive, and `WATCHING` is never cleared. | Open |
| R15 | P3 | `value.rs` | `into_v8` recurses without a depth bound. Everything that reaches it from JavaScript is bounded at 32 by `from_v8`, but a `Value` built in Rust is not, so a deeply nested prop could overflow the stack. | Open |

### The eight open engine findings of `docs/dette-connue.md`, reconciled

That document's first section lists sixteen findings from a branch review
of 2026-10-28; the first eight were fixed, and eight were left. They are
engine findings, so they are this subsystem's. Each one was checked
against the code as it stands.

| # | The finding | Verdict | Where it stands |
|---|-------------|---------|-----------------|
| 1 | `__rustx_frame` is a public global registered through `register_native` | **still live** | Fixed (R18). The `__rustx_` prefix is reserved — `register_native` refuses it by name and says why — and the engine's own functions go on the global last, `READONLY | DONTENUM | DONTDELETE`. |
| 2 | `shutdown()` drops the jobs already queued | **still live, deliberately left** | Still true, and now less consequential: `shutdown` releases every mounted root first, so what a queued job would have *held* is released whether or not the job runs. Running them would mean running application code against a document that is going. The decision the document defers to a real capture application has not arrived, so the behaviour is unchanged and now documented at the call. |
| 3 | `Poster::post` from the browser process succeeds into the void | **still live** | Fixed (R19). A queue that has passed 1 024 jobs in a process where no frame has *ever* run is refused with `PostError::NoFrameLoop` and one sentence naming the cause. Not a flat "refuse outside the renderer": a post before the first frame is ordinary, and the engine's own headless tests make them. Pinned by `a_queue_is_only_refused_where_nothing_will_ever_drain_it`. |
| 4 | Hooks registered after the first context never run | **still live, documentation** | Fixed as the document asks: `on_context_created` now states both halves — once per process and not once per page, and therefore a hook registered after that first context never runs at all. |
| 5 | No compile-time guard against `panic = "abort"` | **still live** | Fixed (R20), and earlier than the document expected: `#[cfg(panic = "abort")] compile_error!` needs no build tooling. |
| 6 | `Document::current()` does not assert its thread | **still live, documentation** | Fixed as documentation. A `cef::currently_on(RENDERER)` would add a libcef call to a function the frame loop calls every frame to buy a diagnostic; the `None` is already safe, and what was missing was the sentence saying which thread it is about. |
| 7 | No test covers a DOM call that throws | **still live** | Open. It needs a window, and the scenario would have to make the DOM throw on purpose (a method called on a detached node). Listed in §5. |
| 8 | `App::run` does not `cef::shutdown()` on the error path | **still live** | Fixed (R21) with a `Drop` guard, which also covers the panic path the document names. |

Three more of that document's findings name files in these paths and were
checked too: **15** (`mount_in` has no rollback) and **16** (every
attribute goes through `setAttribute`) are both **already fixed** —
`build_into` builds a subtree whole before attaching it, and
`set_attribute` sets the live property as well for `value`, `checked`
and `selected` — and both have coverage, in `tests/mount.rs` and in the
`router`/`lifecycle` scenarios.

### `docs/manques-du-framework.md`, reconciled

The routing and async items are this subsystem's. **13–19** (query
string, catch-all, route groups, `replace`/`forward`, params in a
layout, `loading`/`error` per segment, a readable history) are all
**already fixed**, and all are covered: the first five by
`rustx-dom/tests/route.rs` and the router's own unit tests, the last two
by `wrap` in `router.rs`. One of them was *half* fixed and is now whole:
**18** gave a per-segment `error.rsx` but no default, so a page with
none still had no boundary at all — that is R3.

**20** (a component's `.await` asked once per cell) is fixed and covered
by `tests/awaited.rs`. **29** and **30** are compiler findings.

The "costs too much" column holds one item that is this subsystem's and
is now closed: **#19**, a history nothing could read, which `can_go_back`
/ `can_go_forward` / `history()` answer — and which R5 has now made
bounded as well, because a readable history that grows for ever is a
readable leak.

### `docs/chasse-aux-bugs.md`, reconciled

**#21** (the mailbox of a thread that ended without closing) and **#25**
(reading a page value from a future) are engine-adjacent and both fixed,
in `rustx-dom`'s `state.rs`; `Owned`'s `Drop` and `Remote::is_open` are
still there and `tests/a_new_context_reopens_the_mailbox.rs` covers the
reopening. **#16** (awaiting before the `return` blocked the render) is
fixed and covered by `tests/awaited.rs`. **#39**, **#40** (the window's
frame and the maximise toggle) are `rustx-os` and covered by
`crates/rustx-os/tests/window.rs`, which needs a real window.

### Not ours — reported, not touched

- **`rustx-dom` (compiler agent).** The reactivity core (`state.rs`,
  `store.rs`) lives there while "reactivity and component lifecycle" is
  this subsystem's remit, so every fix above works *around* it rather
  than in it. Two concrete items for that owner:
  - `state::mark_dirty` is `pub(crate)`, so the engine cannot put a
    dirty mark back. R2 was fixed by reordering instead of re-marking,
    which is the better fix, but the asymmetry is worth knowing:
    `take_dirty` is public and nothing can undo it.
  - `state::forget_on_drop` pushes a hook onto a list that is never
    pruned. Every registration is permanent and `drop_instance` walks
    the whole list. Today only `store` registers, once, so it is a
    latent cost rather than a leak.

---

## 3. Fixes, with the test that pins each

### R1 — the registry was borrowed across React's own code

`crates/rustx-engine/src/render.rs`. A root is now
`Rc<Entry>` where `Entry` splits the part that never changes (the
instance, the host id) from the part a render writes (`Body`: the
component and what is on screen). `each_entry` copies the list of
handles under a borrow it lets go *before* calling anything, so a visit
may add to the registry, remove from it or reach another entry.
`unmount` takes its entry out under the same short borrow and releases
the instance whatever the DOM says. A render reached from inside a
render of the same root is skipped with a sentence rather than left to
`RefCell`, whose panic would end the frame.

Tests (`render.rs`, `mod tests`):
`a_visit_may_add_to_the_registry_and_take_from_it`,
`a_visit_may_reach_the_other_entries`. Both panic with
`already mutably borrowed` against the previous implementation.

### R2 — writes lost on a frame with no context

`render_dirty` asks for the document first and takes the dirty marks
only once there is somewhere to draw them. No headless test: the branch
needs `Document::current()` to answer `None` *and* a mounted root, and a
root cannot be mounted without a document. Pinned by reading, and by
`without_a_window.rs` which establishes that the `None` answer is a
refusal and not a crash.

### R3 — a default screen for a route that is not found, and for an error

`router.rs` gained `Screens { not_found, failed }` with a `Default`, the
two screens themselves (`no_such_screen`, `fell_over`), three public ids
(`NO_SUCH_SCREEN_ID`, `FAILED_SCREEN_ID`, `FAILED_REASON_ID`) so a test
or a scenario can tell them from a page that happens to say the same
words, and `start_with`. `start` is unchanged and wraps it, so nothing
that called it breaks. `wrap` now always draws the page inside the
boundary; which screen goes there is the `error.rsx` nearest the page,
then the application's `Config::on_error`, then the engine's own. The
error screen displays the panic's message verbatim. The panic is
untouched: Rust's hook has already printed it and its backtrace.

`crates/rustx/src/lib.rs` gained `Config::on_error` and now takes both
defaults from the engine.

Tests (`router.rs`, `mod tests`):
`a_path_nobody_declared_is_a_screen_and_not_a_blank_window`,
`a_page_that_falls_over_is_replaced_by_a_screen_that_says_why`,
`an_error_screen_the_page_wrote_wins_over_the_engines_own`,
`an_error_screen_the_application_named_wins_over_the_engines_own`,
`what_is_around_a_page_that_fell_over_is_still_on_the_screen`,
`a_route_that_matches_is_drawn_and_not_replaced`.

### R4 — nothing released the roots at teardown

`frame::shutdown()` and `cef_app::on_context_released` (main frame only)
now call `render::unmount_all()` first, while the runtime the tasks are
on is still standing. The DOM is deliberately not touched there: by then
the context is going, and a handle from a dead context answers
`ContextDestroyed`.

### R5 — a history that only ever grew

`History::forget_the_oldest`, bounded by `MOST = 256`, with the finger
moved by as many as were dropped. Tests:
`a_history_that_only_ever_grew_is_a_leak`,
`going_back_still_works_after_the_oldest_screens_are_forgotten`.

### R6 — an early navigation from another thread

The held ask is one `Mutex<Option<String>>` for the process instead of a
thread-local. Test:
`an_early_navigation_from_another_thread_is_not_lost`.

### R7, R8 — two panics reading and writing a clipboard image

`windows/clipboard.rs` refuses a zero dimension by name on both sides,
and uses checked arithmetic for the buffer size. Tests (same file):
`an_image_on_the_clipboard_with_no_pixels_is_refused_and_not_a_panic`,
`writing_an_image_with_no_pixels_is_refused_and_not_a_panic`,
`an_image_written_as_a_dib_reads_back_the_same_way_up`,
`an_image_shorter_than_it_claims_is_refused`.

### R9 — a call that waits, made on the thread that draws

New module `crates/rustx-os/src/blocking.rs`. The host says which
thread draws (`draws_here`, called from `renderer::install`;
`draws_nowhere` on context release), and every call in the crate that
waits for something outside this process says what it is waiting for
(`waiting("the clipboard")`). A call made on the drawing thread prints
one line per capability naming it and naming `task(async { … })`.

It warns, it does not refuse: a `main` with no window asking for the
clipboard is a correct program, and this crate has no business deciding
otherwise. Nothing is paid when no window has registered — one relaxed
atomic load.

Annotated: `clipboard` (all six), `dialog::{open,save,folder}`,
`capture::{desktop,area,window}`, `print::{dialog,silently,printers}`,
`secret::{store,fetch,forget}`, `devices::devices_of`, `net::network`,
`bluetooth::paired`, `audio::{endpoints,volume,set_volume,set_muted}`,
`color::pick`, `notify::ask`, `instance::hand_over`, `about::show`,
`terminal::run`.

Test: `blocking.rs`,
`a_waiting_call_is_only_warned_about_on_the_thread_that_draws` — one
test rather than several because the answer to "which thread draws" is
process-wide and parallel tests would argue over it.

### R10 — a V8 context entered for ever

`in_context` enters, and leaves through a `Drop` guard.

### R11 — a second frame callback

A thread-local flag replaces the root count.

---

## 4. What this machine cannot verify

Said plainly, because `STABILITY.md` §6 asks for it.

- **Anything that needs a window.** `render::apply` and everything under
  it (patching, handlers on nodes, the React boundary) needs a live V8
  context. The engine's own integration tests get one by starting
  `src/bin/hello.rs` with `--scenario=…` (`tests/common/mod.rs`), which
  spawns a real CEF process. Those scenarios are the only coverage of
  the DOM half, and they are slow, serialised by a lock file, and need
  `RUSTX_SOFTWARE=1` on this machine.
- **R2, R10, R11** are reasoned corrections with no headless test, for
  the reason given beside each: all three branches require a mounted
  root, and a root cannot be mounted without a document.
- **R4** is verified by reading. A regression test would have to close a
  real window and then observe that a thread stopped, from a process
  that is on its way out.
- **macOS, Linux and `other`** in `rustx-os` are compile-checked only,
  through the `mod *_also_compiles` trick in `rustx-os/src/lib.rs`.
  **They have never been executed, here or anywhere.** Every claim in
  this document about a platform concerns Windows unless it says
  otherwise; R7 and R8 are Windows-only files, and the corresponding
  macOS and Linux clipboards go through `pbcopy`/`xclip` and have no
  DIB code to get wrong.
- **`unsafe`.** `rustx-os` holds 255 `unsafe` blocks. 107 of them have
  no `SAFETY` comment within eight lines; the raw-pointer and
  `from_raw_parts` sites in `windows/clipboard.rs` now do. The rest is
  open work, listed in §5.

---

## 5. What remains

In the order it should be picked up.

1. The `unsafe` audit of `rustx-os`, which is the largest piece left.
   107 sites without a stated invariant. The ones to read first, because
   they hold raw pointers or claim thread safety rather than simply
   calling an API:
   - `windows/instance.rs`: `unsafe impl Send for Handle`,
     `unsafe impl Send/Sync for Window`, `data.as_ref()` and
     `slice::from_raw_parts` in the window procedure.
   - `windows/tray.rs`: `unsafe impl Send/Sync for Window`, the window
     procedure, the menu and icon handles.
   - `windows/capture.rs`, `windows/window.rs`: GDI handles and bitmap
     buffers.
   - `macos/*`, `linux/*`: `unsafe extern "C"` declarations whose
     signatures nothing on this machine can check.
2. R13, R14, R15 above.
3. A benchmark for state propagation and for a heavy page's second
   render, tracked against a number rather than a feeling.
   `render::last_render()` already reports the cost of the last render
   and the number of patches it applied; nothing records it over time.
4. A scenario in `src/bin/hello.rs` that closes its window and reports
   whether a task its page started has stopped, which is the only way to
   turn R4 into a test.
