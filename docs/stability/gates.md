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

### Gate results on this machine

Measured at `1db1f48`, when the gates were written:

```
SKIP  fmt      -- rustfmt is not usable on this toolchain; CI runs it
FAIL  package  -- no README.md in: rustx-dom, rustx-compiler, rustx-os,
                  rustx-macros, rustx-engine, rustx, rustx-cli, rustx-lsp
FAIL  editor   -- node --test failed
```

Exit code 1, "Not shippable."

Re-measured on 2026-11-14 after the six agents' work was committed
(`723a6d0`), with `-Only fmt,editor,deps`:

```
FAIL  fmt    -- 376 file(s) differ; run cargo fmt --all
PASS  deps   -- licences, bans, sources, advisories; 212 crates
FAIL  editor -- node --test failed
```

And what each of those turned out to be:

- `package` — the eight `README.md` files landed in `551a1ea`. The gate
  itself has still not been re-run, so it is not a gate that passed.
- `deps` — the ninth gate, and the dependency audit the mission asked
  for, landed with `deny.toml`. It passes over 212 crates.
- `fmt` — 376 files, up from 347 as the apps were rewritten. **Now
  done**: `cargo fmt --all` in `659b3e2`, and `cargo fmt --all --check`
  exits 0. It was held back while six agents were editing the tree;
  with the tree committed and nobody else in it, the whole workspace
  went through at once rather than in six pieces.
- `editor` — **not what it was recorded as.** It was written down here
  as the `indent.test.mjs` case failing because `apps/` was being
  rewritten underneath it. It was not. `node --test test` is read as a
  *module path* by Node 24 and fails before a single test runs, so the
  gate had never executed one line of the extension's suite. The gate
  now runs the pattern `package.json` already used. The suite then
  reported one real failure: the indenter took 5,692ms on 20,000
  unclosed braces against a 4,000ms budget, because two walks over the
  open-bracket stack ran per line. Fixed in `9b8233c` -- 59ms, and 79
  tests pass.

  Worth stating plainly, because §6 is about exactly this: a gate that
  cannot run is worse than no gate, since it reports a failure that
  gets explained away. This one was explained away in this document.

### What the first green workspace found

`cargo check --workspace --all-targets` had not passed at any point in
this session. Getting it to exit 0 turned up three defects, none of them
in a test:

| where | what |
|---|---|
| `rustx-compiler` | a handler that only **reads** a page value never got `let n = n.clone()`, so two reading handlers in one component were `use of moved value` in a generated file (`be00d15`) |
| `rustx-compiler` | `clips.get_mut(i)` was rewritten to `clips.get().get_mut(i)` -- a change made to the copy `get` returns and dropped at the end of the statement. It compiled, it ran, and the screen never moved. Now refused, with the shape that works (`be00d15`) |
| `rustx` facade | the twelve newest `rustx-os` capabilities had no short name, so `rustx::secret::fetch` did not exist while `rustx::clipboard::text` did. `apps/vault-storage` was written against the name that did not exist |

### clippy

First measured on the CEF-free crates only, because six agents were
building concurrently: **17 diagnostics**, all style-level. Measured
again over the whole workspace, CEF crates and applications included,
once the tree settled:

| | diagnostics |
|---|---|
| first full measurement | **143** |
| in `apps/*/target/rustx-pages/*.rs`, which the compiler writes | 115 |
| in code somebody wrote | 28 |

The 115 mattered more than the 28, because they are in files the
developer cannot edit, and they are what `cargo clippy` says about
*their* application. Two of them were the compiler's own text, and one
line each:

- **88 × `noop_method_call`.** A keyed element emitted
  `with_key(__key.clone(), …)` and `.key(__key.clone())`. When the key
  is already a `&str` — `key={"row"}`, `key={name.as_str()}` —
  `.clone()` on a reference is a call that does nothing, and `rustc`
  itself says so. Writing `&__key` instead merely traded it for 98
  `needless_borrow`; the emission now binds one owned string and hands
  a `&str` to both calls, which is neither.
- **~48 × `suspicious_else_formatting`**, which is a lint about where
  the `else` sits on the line. Machine-written text has no business
  being linted for its layout, so each generated module carries
  `#[allow(clippy::suspicious_else_formatting)]` — on the module the
  page is included into, so it covers that page and nothing else.

What is left in a generated file (21) comes from the `.rsx` the
developer wrote — a collapsible `if`, a `% 3 == 0` that wants
`is_multiple_of` — and those are theirs to act on. The position map
puts them on the line they wrote.

The 28 in written code were cleared: 10 collapsible `if`s and the other
mechanical ones by `cargo clippy --fix`, six "very complex type" by
giving the type a name (`Listeners`, `Hooks`, `Exports`, `Props`), two
`let…else` that wanted `?`, a `match` that wanted `if let`, a
`Default::default()` followed by a field assignment, and one
`should_implement_trait` on `rpc::Reader::next`, which is allowed with
its reason written down: the end of that stream is a `Frame::Over` the
server has to answer, not a `None` a `for` loop would swallow.

**And nine error messages that were not sentences.** Found while
reading the clippy output: a `\` at the end of a line inside a Rust
string literal continues the string *and* strips the next line's
indentation — but a literal newline does neither. Seven strings in
`rustx-os` and two panic messages in `rustx-dom` had been written as if
the continuation were there, so what a developer actually read was
`"...--action and --wait came                  with libnotify 0.8"`.
They are sentences now. Nothing tested them, because what a message
says is not what a test asserts, which is its own finding.

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

**Two of the three limitations this document first recorded are gone.**
The user authorized network access on 2026-11-14, after the gates were
written. What follows is the state after that.

### Resolved: rustfmt

It was genuinely broken. `rustup component list --installed` reported
`rustfmt-x86_64-pc-windows-msvc` while both `cargo fmt` and `rustfmt`
failed with *"the 'rustfmt.exe' binary, normally provided by the
'rustfmt' component, is not applicable to the
'stable-x86_64-pc-windows-msvc' toolchain"* — and the binary really was
absent from `~/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin/`.
rustup's metadata and its filesystem disagreed, and `rustup component
add rustfmt` answered *"component rustfmt is up to date"* and did
nothing. A `remove` followed by an `add` fixed it: `rustfmt 1.10.0-stable`.

And the measurement it was blocking:

```
cargo fmt --all --check  ->  347 files differ
```

**The workspace has never been formatted.** Not partly — essentially
every Rust file in it deviates.

That was not a reformat to do while six agents were editing the tree:
a whitespace commit of that size landing underneath them would conflict
with every one of them and bury their real changes. It was delegated
per owner, and then the session that was running those agents ended.
With their work committed (`723a6d0`) and nobody else in the tree, the
whole workspace went through at once instead: **`659b3e2`, 376 files,
whitespace only, and `cargo fmt --all --check` now exits 0.**

`cargo check --workspace --all-targets` passes at that commit, which is
the only claim a formatting commit can honestly make for itself.

Still no `rustfmt.toml`. Default rustfmt is what the whole workspace is
now formatted to; a configuration would move that target and cost
another whole-tree commit. Decide on one deliberately, if at all.

### Resolved: the network

cargo fetches, `npm install` works, `cargo install` works. The
consequences were dispatched to the agents whose work they unblock:

- **The dependency and security audit is now possible**, and it was the
  largest single unmet gate in the mission. Assigned to the CLI and
  distribution agent: `cargo-deny` and `cargo-audit` (installing),
  a real `deny.toml` for licenses, duplicate versions and advisories,
  and a ninth gate named `deps` added to `scripts/gates.ps1`.
- **`rustx new` can now be verified end to end for real** — scaffold,
  `npm install`, `npx shadcn@latest add button card`, build, run. That
  had only ever been done once, by hand. Same agent.
- **`cargo-fuzz` is available** to the compiler agent, alongside the
  seeded-generator `#[test]` harness its brief asked for. The harness
  is still worth having: it runs in CI on every push, where a fuzzer
  does not.
- **`npm install` works in `apps/`**, so the redesign agent is no
  longer restricted to libraries already in the cache.

### Still true: no GPU

`RUSTX_SOFTWARE=1` is set by both the script and the workflow.

## Deliberately not done yet

**No `[workspace.lints]`, no deny-level lints.** This was deferred
because six agents were editing the tree and a lint table would have
broken their builds on code they did not write. That reason has
expired; the remaining one is the order of work: the diagnostics get
cleared first, and the table goes in once there is nothing for it to
fail on. The gate script already enforces `-D warnings` at the point of
the gate, so the enforcement exists either way.

## Next

1. ~~Confirm every agent's `cargo fmt` commit landed~~ — done
   differently: the agents' session ended, their work was committed as
   `723a6d0`, and the reformat went in as one commit, `659b3e2`.
   `cargo fmt --all --check` exits 0.
2. Clear the clippy diagnostics. The count of 17 was measured before
   the apps and four crates were rewritten; re-measure before fixing.
3. Measure `rustx-app` and `rustx-cli` with clippy; both link CEF and
   were skipped to avoid build contention. The contention is over.
4. Add `[workspace.lints]` and `clippy.toml` after 2–3.
5. Re-run the `package` gate. The eight READMEs are on disk; the gate
   has not confirmed it.
6. Run the workflow for real and fix what it finds — particularly
   whether the `portable` jobs survive on `ubuntu-latest` and
   `macos-latest`. `rustx-os`'s macOS and Linux halves have never been
   compiled by their own compilers.
7. The subsystem work the agents left open, each recorded in its own
   document in this directory: the `unsafe` audit of `rustx-os` (107
   sites without a stated invariant), `rustx-macros` having no tests at
   all, the three French audit documents still unreconciled, and
   `rustx new` verified end to end against the network.
