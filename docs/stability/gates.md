# Quality gates — progress

Owner: the coordinating session. Scope: `scripts/gates.ps1`,
`.github/workflows/ci.yml`, and the measured baseline below.

This is the part of `STABILITY.md` §6 that was missing entirely: the
gates themselves. Six agents are fixing subsystems in parallel; none of
that is enforceable without something that runs the gates and says no.

---

## What existed before

Nothing. Measured on 2026-11-14, at commit `1db1f48`:

| | |
|---|---|
| `.github/` | absent — no CI of any kind, on any platform |
| `rustfmt.toml` | absent |
| `clippy.toml` | absent |
| `[workspace.lints]` | absent |
| `deny.toml` / dependency audit | absent |
| test files under `crates/*/tests/` | 94 |

So the workspace had a real test suite and no gate in front of it. Every
check was something a person remembered to run.

## What exists now

### `scripts/gates.ps1`

Eight gates — `fmt`, `clippy`, `build`, `test`, `docs`, `package`,
`editor`, `site`. `-List` prints them, `-Only <names>` runs a subset,
`-Offline` keeps everything off the network, `-TargetDir` keeps the run
from fighting an editor or an application for the lock on `target/`.

PowerShell and not a shell script, for a reason that is not taste: Git
Bash puts its own `/usr/bin/link` ahead of MSVC's `link.exe` on the
`PATH`, so every native build started from bash fails at the linker.

Two properties it was built to have:

- **A skipped gate is reported as a skip, never as a pass.** The summary
  counts skips separately and states that they remain unverified. §6
  says a passing suite is evidence and not proof, and that a limitation
  gets documented rather than quietly absorbed.
- **A selection that matches nothing exits 2**, rather than reporting
  that all zero gates passed. That was a real bug in the first draft,
  found by running it: `powershell -File gates.ps1 -Only fmt,package`
  hands `fmt,package` over as a single string, nothing matched, and the
  script cheerfully said everything was fine. The matcher splits on
  commas now and an empty result is an error.

### `.github/workflows/ci.yml`

Four jobs. `windows` runs the gate script in full — it is the platform
`rustx-os` is written and measured on, and the only one where an
application has ever actually run. `portable` is a matrix over
`ubuntu-latest` and `macos-latest`. `editor` and `site` cover the two
Node trees.

The `portable` job is the one that buys something this machine cannot
provide. `rustx-os/src/lib.rs` compile-checks the other platforms'
modules through a `mod *_also_compiles` trick, so a Windows build does
catch a syntax error in `macos/audio.rs`. But a trick is not a target:
it does not typecheck against the real platform's `libc`, its real
system crates, or its real conditional `cfg`. That job builds those
halves with the compiler that actually targets them, and it is the gate
that has to go green before `rustx-os` can honestly claim macOS or
Linux support.

**Unverified:** the workflow has never been executed. There is no
network in this environment and no way to run GitHub Actions from it.
The YAML parses (checked with `js-yaml`) and every job's steps are
shell commands that were each run by hand here, but the first real run
of this file will be the first time it is tested. Expect to iterate on
it once, particularly on whether CEF fetches and links on the macOS and
Linux runners — which is exactly why those jobs are scoped to the
crates that do not touch it.

---

## The baseline, as measured

### Gate results on this machine, at `1db1f48`

```
SKIP  fmt      -- rustfmt is not usable on this toolchain; CI runs it
FAIL  package  -- no README.md in: rustx-dom, rustx-compiler, rustx-os,
                  rustx-macros, rustx-engine, rustx, rustx-cli, rustx-lsp
FAIL  editor   -- node --test failed
```

Exit code 1, "Not shippable." Both failures are known and both are
owned by another agent right now:

- `package` — none of the eight crates has its own `README.md`.
  `readme.workspace` resolves per manifest, so every crates.io page
  would render empty, and `cargo package` only *warns*. The CLI and
  distribution agent is writing them.
- `editor` — the one pre-existing failure in
  `editors/vscode/test/indent.test.mjs`: the test that formats the
  repository's own `.rsx` pages, which fails because `apps/` is being
  rewritten underneath it. Verified earlier to fail identically with
  the unmodified `indent.mjs`, so it is not a regression from the
  hardening in `9826d97`. The LSP and editor agent is deciding what
  that test should actually be — a test that depends on files another
  part of the repository is free to rewrite is a test that will fail
  again.

### clippy

17 unique diagnostics across the crates that do not pull CEF
(`--workspace --exclude rustx --exclude rustx-cli --all-targets`):

| crate | diagnostics |
|---|---|
| `rustx-lsp` | 7 |
| `rustx-os` | 6 |
| `rustx-compiler` | 2 (+1 in the `judge` example) |
| `rustx-dom` | 1 |
| `rustx-macros` | 0 |

All style-level, nothing that indicates a defect: nine collapsible
`if`s, three "very complex type", a `match` that wants `if let`, a
`chunks_exact` with a constant size, an unnecessary closure in
`bool::then`, a `let…else` that wants `?`, and items declared after a
test module.

**Not yet measured:** `rustx` and `rustx-cli`. Both link CEF, and with
six agents building concurrently the measurement was not worth the
contention. Do it when the tree settles.

### `unsafe`

Lines mentioning `unsafe` in each crate's `src/`:

| crate | lines |
|---|---|
| `rustx-os` | 277 |
| `rustx-engine` | 8 |
| `rustx-cli` | 7 |
| `rustx-dom` | 1 |
| `rustx`, `rustx-macros`, `rustx-compiler`, `rustx-lsp` | 0 |

`rustx-os` is platform FFI, so a large number there is expected rather
than alarming — but it is 97% of the `unsafe` in the repository and it
has had no systematic audit. Assigned to the runtime agent, whose brief
asks for a stated invariant and a reason it holds per block.

### Panic sites in non-test code

`.unwrap()`, `.expect(`, `panic!`, `unreachable!`, `todo!` under `src/`:

| crate | lines |
|---|---|
| `rustx-os` | 66 |
| `rustx-engine` | 18 |
| `rustx-dom` | 10 |
| `rustx-cli` | 6 |
| `rustx-compiler` | 5 |
| `rustx-lsp` | 4 |
| `rustx`, `rustx-macros` | 0 |

A count is not a verdict — plenty of these are provably fine. But §3
asks for no compiler panics and no LSP crash during normal editing, and
`rustx-lsp`'s four sit on a path reachable from a keystroke, which is
the worst place for them.

---

## Environment limitations

Stated here rather than worked around, per §10.

- **rustfmt cannot run.** `rustup component list --installed` reports
  `rustfmt-x86_64-pc-windows-msvc`, and both `cargo fmt` and `rustfmt`
  fail with *"the 'rustfmt.exe' binary, normally provided by the
  'rustfmt' component, is not applicable to the
  'stable-x86_64-pc-windows-msvc' toolchain"*. The binary is genuinely
  absent from
  `~/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/` — rustup's
  metadata and its filesystem disagree. Repairing it needs
  `rustup component remove/add rustfmt`, which needs the network.
  **Consequence: the whole workspace's formatting is unmeasured.** The
  `fmt` gate will be the first thing CI reports, and it may well report
  a great deal. No `rustfmt.toml` was added for the same reason — a
  config nobody can run against the tree is a config that encodes a
  guess.
- **No network.** cargo runs `--offline`; `npm install` fails with DNS
  errors. No dependency can be added, which rules out `cargo-deny`,
  `cargo-audit`, `cargo-fuzz` and `cargo-llvm-cov`. The dependency
  integrity and security audit §3 asks for is therefore **not done**,
  and `deny.toml` is not written. This is the largest single unmet gate
  and it is an environment limitation, not a decision.
- **No GPU.** `RUSTX_SOFTWARE=1` is set by both the script and the
  workflow.

## Deliberately not done yet

**No `[workspace.lints]`, no deny-level lints.** Turning a clippy
warning into an error, or adding a workspace lint table, changes what
compiles for every one of the six agents editing this tree right now —
their builds would start failing on code they did not write. The gate
script already enforces `-D warnings` at the point of the gate, which
gets the enforcement without the mid-flight breakage. Add the lint
table once the agents land and the 17 diagnostics are cleared.

## Next

1. Clear the 17 clippy diagnostics once the subsystem agents have
   stopped editing those files.
2. Measure `rustx` and `rustx-cli` with clippy.
3. Add `[workspace.lints]` and `clippy.toml` after that.
4. Get `rustfmt` working, or accept that CI is where formatting is
   first measured, and commit the resulting reformat as one isolated
   commit.
5. Run the workflow for real and fix what it finds.
6. Dependency and security audit — blocked on the network.
