# CLI, the JavaScript ecosystem, and distribution

The stability mission (`STABILITY.md`) §3, for three of its subsystems:
*CLI and developer experience*, *JavaScript ecosystem*, *Production and
distribution*.

Scope of this document, and of the work it records:

- `crates/rustx-cli/**`
- `crates/*/README.md`
- `PUBLISHING.md`
- `Cargo.toml` at the workspace root, metadata only

Everything else belongs to another subsystem. Defects found outside
these paths are recorded under **Found elsewhere** and not touched.

## How to run what is here

```powershell
$env:CARGO_TARGET_DIR = "C:\Users\aztra\AppData\Local\Temp\rustx-target-cli"
cargo test --offline -p rustx-cli
cargo package --offline --no-verify -p rustx-cli
```

Cargo must go through PowerShell on this machine: Git Bash's
`/usr/bin/link` shadows MSVC's `link.exe` and every link fails. There is
no network, so everything is `--offline`, and no test may depend on
`npm install` or on crates.io.

---

## The audit

Every command the CLI has, every place it reaches for something on the
machine, and every file `rustx new` writes.

### What the CLI is

Five commands in `crates/rustx-cli/src/`: `new`, `dev`, `build`, `run`,
and the help. `build` calls cargo and then stamps the `.exe` with the
application's name, version and icon (`resources.rs`, Windows resource
API; a `.desktop` file on Linux, a minimal `.app` on macOS). `dev`
polls the project's files (`watch.rs`), rebuilds, and relaunches a
*copy* of the binary so that the file cargo must rewrite is never held.
`new` writes ten — now eleven — files.

### Everything the CLI reaches for outside the project

| Reached for | Where | Verdict |
| --- | --- | --- |
| `cargo`, by name, on the `PATH` | `build::run` | **Was a defect**: see D4. Now `$CARGO` first, and a refusal that names what is missing. |
| `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` | nowhere | **Was a defect**: see D1. |
| `.cargo/config.toml`, `[build] target-dir` | `build::target_directory` | Read, but only beside the project, and only by `held`. See D1. |
| `RUSTX_CARRIED` | `dev::restart` sets it | Correct: set by `rustx dev` and read by nothing else. |
| `std::env::current_dir` | `main.rs` | **Was a defect**: see D5. |
| `std::env::temp_dir` | tests only | Fine. |
| `node`, `npm`, `npx`, a shell, Python | **nowhere** | Verified by grep: the CLI depends on cargo and on nothing else. The JavaScript tools are reached for by `rustx-compiler::bundle`, from the application's own `node_modules`, never from the `PATH`. |
| An absolute path baked in anywhere | **nowhere** | Verified by grep. |

### Commands against documentation (§9)

| Accepted by the CLI | In `rustx --help` before | Now |
| --- | --- | --- |
| `rustx build --release` | **no** | yes |
| `rustx run --release` | silently ignored (**D2**) | read, and documented |
| `rustx --version` | **not a command at all** | a command, and documented |
| `rustx dev --release` | silently ignored | refused, with the reason |
| anything else after a command | silently ignored (**D2**) | refused, named, with a suggestion |

---

## Defects, root causes, fixes

### D1 — `rustx build` could not find the binary cargo had just built (P0)

**Symptom.** `cargo said it was done and there is no mon-app.exe under
any target/` — the command line calling cargo a liar about a file cargo
had written seconds earlier. `rustx run` the same; `rustx dev` rebuilt
for ever and never launched a window.

**Who hits it.** Anyone with `CARGO_TARGET_DIR` set — a shared build
directory between projects, a CI cache, every agent in this very
checkout — and anyone whose project is a member of a workspace, which is
every application under `apps/`.

**Root cause.** Four functions answered the question "where does this
project build?" and gave three different answers. `build::held` read
`[build] target-dir` from `.cargo/config.toml` beside the project.
`build::produced` read neither that nor the environment: it searched
`target/<profile>/` beside the manifest and in every ancestor directory.
`dev::copies` and `dev::carried_file` hard-coded `project/target`. None
of the four read `CARGO_TARGET_DIR`, which is the *first* thing cargo
itself reads.

**Evidence before the fix** (`tests/proof.rs`, removed after):

```
---- a_member_of_a_workspace_builds_in_the_workspaces_own_target ----
assertion `left == right` failed
  left: "…\rustx-proof-9052\mon-app\target"
 right: "…\rustx-proof-9052\target"
```

**Fix.** One function, `build::target_directory`, taking cargo's own four
steps in cargo's own order: `CARGO_TARGET_DIR`, then
`CARGO_BUILD_TARGET_DIR`, then `[build] target-dir` in the nearest
`.cargo/config.toml` **at or above** the project (cargo walks up from
where it was run; this did not), then `target/` at the **workspace
root** (`workspace_root`, new). `produced` now looks in exactly that one
place instead of searching — a search had found stale binaries in a
`target/` cargo had stopped writing to and handed one back as the build
that had just succeeded. `held` and `produced` share it, so the file a
window is holding and the file that is looked for are the same file.

The environment is a parameter of `target_directory_told`,
`produced_told` and `held_told`, with the convenience wrappers reading
it: tests of one crate share one process, so a test that sets
`CARGO_TARGET_DIR` sets it for every other test, and writing it is
`unsafe` since the 2024 edition.

`dev::copies` and `dev::carried_file` were deliberately **left** under
`project/target`: they are the command line's own scratch, not cargo's
output, and they must stay one-per-project — two projects sharing one
`CARGO_TARGET_DIR` would otherwise share one `rustx-dev-state.txt` and
one application's shared state would arrive in another's window.

**Tests.** `tests/target.rs`, 11 laws.

### D2 — three silent wrong answers at the command line (P1)

**Symptoms, all silent.**

- `rustx build --relese` built in debug and said nothing. The flag was
  read with `rest.contains(&"--release")`; any other word was simply not
  there.
- `rustx run --release` built and ran the **debug** binary and said
  nothing: `run` read no arguments at all.
- `rustx new foo bar` wrote `foo` and dropped `bar`.
- `rustx new --help` wrote a project into a directory called `--help`.
- `rustx --version` was "not a command".

**Root cause.** `arguments::read` consumed the first word and then
either looked for one flag by name or ignored the rest entirely. There
was no representation for *an argument this command does not take*, so
there was nothing to say.

**Fix.** `Command::Misused(String)` — a command that exists, given
something it does not take, carrying the sentence about it, because the
code that read the word is the code that knows what was wrong with it.
`run` reads `--release`. `--help` and `-h` anywhere are a question
(`rustx new --help` is a question about `new`). `--version`/`-V` and a
bare `version` answer with the version. `rustx dev --release` is refused
with the reason it does not exist. A mistyped flag gets the same
one-letter-apart suggestion an unknown command already got. Exit code
`2` for a mistake at the command line, `1` for work that failed — said
out loud in `main.rs`, because a script reads those.

**Tests.** `tests/typed.rs`, 17 laws.

### D3 — `rustx new` accepted eleven names that cannot work (P1)

**Evidence before the fix.** The committed `refuse_a_bad_name`, extracted
and run against the names a developer plausibly types:

```
          --help -> ACCEPTED        test -> ACCEPTED         nul -> ACCEPTED
              -h -> ACCEPTED         std -> ACCEPTED         con -> ACCEPTED
       --release -> ACCEPTED       crate -> ACCEPTED    rustx-os -> ACCEPTED
            _app -> ACCEPTED        self -> ACCEPTED   rustx-app -> ACCEPTED
                              rustx-compiler -> ACCEPTED  rustx-lsp -> ACCEPTED
```

Each of those writes eleven files and a `Cargo.toml` naming a crate
cargo refuses, or — `nul`, `con`, `com1` — a directory Windows will not
make, and the error a developer then reads is cargo's or Windows's and
says nothing about a project name. The function's own doc comment
promises the opposite: *"half a project on disk, named something cargo
will not build, is worse than a sentence."*

**Root cause.** The check tested four things (empty, leading digit,
illegal character, four of the framework's names) and the set of ways a
name can be wrong is larger than four. The framework list named four of
its own ten crate names — it had not been updated as crates were added.

**Fix.** The first character must be a letter, which covers `--help`,
`-h` and npm's refusal of a leading `_` in one rule. All ten of the
framework's own names, by package name and by directory name, compared
with `_` and `-` the same and case-insensitively. Rust's keywords,
including the reserved ones. The crates every Rust program already has
(`std`, `core`, `alloc`, `test`, `proc_macro`), which cargo refuses by
name. Windows's twenty-two device names. A length bound of 64, which is
crates.io's.

**Tests.** `tests/typed.rs`:
`a_name_that_cannot_work_is_refused_before_a_directory_is_made` (23
names, each asserting the *reason* appears in the refusal) and
`the_names_that_do_work_still_work` (10 names that must stay possible,
so the list cannot grow into the reasonable ones).

### D4 — `cargo could not be started: program not found` (P2)

**Root cause.** `Command::new("cargo")`, and a message that named
neither what was looked for nor where. A developer whose `PATH` is
missing `~/.cargo/bin` — a terminal opened before the install, a CI
image, a GUI-launched editor on macOS — reads that as a bug in `rustx`.

**Fix.** `$CARGO` first, which is the one thing in the environment that
names a cargo for certain: cargo sets it for everything it starts, so a
`rustx build` run from inside a cargo subcommand or a test uses the
toolchain already in play rather than whichever `cargo` that process's
`PATH` reaches first. Then the name on the `PATH`. `build::says_no_cargo`
names what was looked for, says that `rustx build` calls cargo rather
than replacing it, gives `cargo --version` as the way to check and
rustup.rs as the way to fix, and names the stale-`PATH` case because it
is the usual one.

**Test.** `tests/typed.rs::a_machine_without_cargo_is_told_what_is_missing_and_where_to_get_it`.

### D5 — a deleted working directory became a silent relative path (P2)

`std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))`, three
times in `main.rs`. A directory deleted or renamed under a running
shell, or one this process may not read, turned every path the command
line then built into a relative one: the project was written somewhere
nobody could name and the refusal said nothing about why. Now asked
once, in `here()`, and refused in words.

### D6 — six compiler warnings in a crate that is about to be published (P2)

`cargo build -p rustx-cli` printed six `dead_code` warnings on Windows:
`desktop_entry`, `single_line`, `write_app_bundle`, `info_plist`,
`plist_name`, `xml_escape` are compiled on every platform on purpose —
so a test on any machine can read exactly what a `.desktop` entry or an
`Info.plist` says — and used for real only by the `write_them` of their
own platform. They are now `#[cfg_attr(not(test), allow(dead_code))]`:
dead on purpose, and said to be. A crate whose first `cargo build` on a
user's machine prints six warnings about its own internals is not a
crate that looks maintained.

### D7 — every crate's crates.io page would have been empty (P1, distribution)

See **Distribution** below.

---

## Tests added

| File | Laws | What they are about |
| --- | --- | --- |
| `tests/target.rs` | 11 | Where cargo builds and where the command line looks (D1). |
| `tests/typed.rs` | 17 | Every word the command line is given, and every name it refuses (D2, D3, D4). |
| `tests/scaffold.rs` | 14 | `rustx new` is internally consistent with itself (below). |

`tests/new.rs` keeps its 15; `tests/help.rs`, `tests/build.rs`,
`tests/dev.rs`, `tests/held.rs` and `tests/carried.rs` are unchanged
except where D1 made two of `held`'s assertions depend on the
environment of the test run, which they now say out loud.

### The scaffold is consistent with itself

A `rustx new` test that asserts a file list stops being worth anything
the day the scaffold changes. `tests/scaffold.rs` asserts instead that
what it writes *agrees with itself*, which stays true as it grows:

- every file it reports writing is on disk, and every file on disk is
  reported — no file written and not named, none named and not written;
- every `.json` it writes parses, by a real parser written for the test
  (the one in `tests/new.rs` counted braces, so a trailing comma passed
  and `npm install` failed on it);
- every bare module specifier any file it writes imports is declared in
  the `package.json` it writes — `lib/utils.ts` imports `clsx` and
  `tailwind-merge`, `assets/tailwind.css` imports `tailwindcss` and
  `tw-animate-css`, and a file that imports what the manifest does not
  declare is a project that fails on its first `npm install`;
- every path any file it writes names is a path it created — this is
  what added `components/readme.md`: `tsconfig.json` compiles
  `components`, `components.json` aliases it as `@/components`, and the
  bundler walks it, and no file in the project had ever made it;
- the `@/*` alias resolves `@/lib/utils` to a file that exists;
- the page it writes compiles, through the real transpiler;
- the manifest it writes is the manifest `build::read` reads back, with
  the right name, version and window title;
- the entry point `rustx-compiler::bundle` generates for the scaffold's
  own dependencies imports every library it declares, exposes each one
  under the name a page would write, and has no two libraries whose
  names collide.

The last one is the JavaScript-interoperability law: it runs the real
`bundle::dependencies_in`, `bundle::entry_for` and
`bundle::a_collision_in` over the real `package.json` that `rustx new`
writes, so the scaffold and the bundler cannot drift apart without a
test failing. It needs no `npm install` and no network.

---

## Distribution

### D7 — eight crates, no README (P1)

`readme.workspace = true` resolves **per manifest**: every one of the
eight crates asked for `README.md` *beside its own `Cargo.toml`*, and
not one existed. `cargo package` only warns. The consequence is eight
empty crates.io pages for a framework whose whole argument is that
someone can discover it and evaluate it.

**Fix.** A real `README.md` for each crate, saying what the crate is,
where it sits in the workspace, and what somebody would use it for
directly — which for six of the eight is honestly *"you want the
`rustx` crate instead"*, and that is worth saying on the page rather
than leaving a reader to find out. Each manifest keeps
`readme.workspace = true`, which now resolves to a file that exists.

Status: written, and `cargo package --no-verify` run per crate — see
**Verification**.

### What a built application has to ship

`rustx build --release` printed one path: the `.exe`. A rustx
application is a Chromium host — the binary loads `libcef.dll` when it
starts and Chromium reads its resources (`*.pak`, `icudtl.dat`,
`v8_context_snapshot.bin`, `locales/`) from beside that binary — so a
developer who copies that one path onto another machine gets a program
that does not start, and the dialog Windows shows for that names a
library and not a cause. On this checkout that is 18 files and a
directory beside the `.exe`, 330 MB of them.

There is no `rustx package`, and inventing one is a feature, not a
stability fix. What the command line does now is say so: stdout is still
the one path, because a script reads it, and stderr names the artefact —
how many files, where, and that the directory is the thing to copy. The
classifier (`build::ships_with`) also draws the line that matters for
*"no development-only dependency reaching a release"*: `.pdb` (84 MB of
symbols here), `.lib` and `.exp` (import libraries the linker reads and
a program cannot use), `.d` (cargo's own dependency lists) and cargo's
directories are named as not part of it.

### Publish order

`PUBLISHING.md`'s order is unchanged by this work and still correct —
nothing here adds or removes an internal dependency. `cargo package
--no-verify` results per crate are in **Verification** below.

**Nothing has been published.** `cargo publish` has not been run, in any
form.

---

## Found elsewhere — not mine to fix

Reported rather than touched; each names the exact change.

1. **`rustx-compiler::bundle`: a library that is installed, declared and
   silently left out of the bundle.** `Installed::NothingToImport`
   (`bundle.rs`) drops a package from the entry point with no word said
   — deliberately, for the two shapes that justify it (a package with
   sub-paths and no root, like `@tiptap/pm`; a stylesheet-only package,
   like `tw-animate-css`). But a page that writes a tag from such a
   package then fails five seconds later with *"the page asked for 'X',
   and nothing has it"*, which is the exact failure shape `STABILITY.md`
   keeps naming: silent where it happens, surfacing far away as
   something else. **Change:** in `bundle::build`, collect the
   `NothingToImport` packages and `eprintln!` one line naming them and
   saying that a tag from one of them will not resolve. Owner:
   compiler.

2. **`rustx-compiler::bundle::dependencies_in` stops at the first `}`.**
   `manifest[open..].find('}')` ends the dependency object at the first
   closing brace, so a `package.json` whose `dependencies` contains a
   nested object — npm's `overrides` shape, or any hand-written nesting
   — silently loses every dependency after it, and the page is empty
   with no error. **Change:** count depth as `value_after` already does
   in the same file. Owner: compiler.

3. **`rustx dev`'s launched copy and the CEF runtime.** `dev::launchable`
   copies only the `.exe` into `target/rustx-dev/<pid>-<turn>/`, and
   `libcef.dll` is not in that directory. Probed: a copy of
   `target/debug/ops-monitor.exe` run alone in an empty directory
   started and stayed alive for eight seconds, so the runtime is found
   some other way and the mechanism works. It is not *verified* here
   that it finds the right one. Owner: runtime, to say how libcef is
   located.

4. **`bundle::content_globs` cannot be read by the flag it is written
   for.** It returns one comma-separated string, and two of the four
   globs in it contain commas of their own:

   ```
   ./app/**/*.rsx,./src/**/*.rs,./components/**/*.{ts,tsx,js,jsx},./lib/**/*.{ts,tsx,js,jsx}
   ```

   Tailwind 3's `--content` is a comma-separated list, so what it
   receives is `./components/**/*.{ts`, `tsx`, `js`, `jsx}` and three
   more — and `components/` and `lib/` are never scanned, so every
   shadcn component installed in a Tailwind 3 project has no utilities
   and no error is printed. Tailwind 4 is unaffected: `input_for` writes
   one `@source` line per glob, and a line is not split on commas. This
   test suite found it — `tests/scaffold.rs::at_top_level` exists
   because the naive split produced `tsx/` as a directory to look for.
   **Change:** in `arguments_for`, pass the brace groups expanded, or
   pass `--content` once per glob if the installed Tailwind 3 accepts a
   repeated flag (its own documentation says it does not, which is why
   the list exists). **Unverified here:** only Tailwind 4.3.3 and 4.3.5
   are installed in this checkout and there is no network to fetch a 3.
   Owner: compiler.

5. The `<base href>` fix for a generated document in a temporary
   directory is present and correct in `crates/rustx/src/lib.rs`
   (`"<base href=\"{base}\" />"`, with the comment explaining it).
   Confirmed, not re-tested: that crate is the runtime's.

---

## Unverified in this environment

- **`npm install` and `npx shadcn add`.** No network. Every JavaScript
  law here is asserted on what the CLI *writes* and on the consistency
  between the scaffold and `rustx-compiler::bundle`, never on a
  successful install. That a fresh scaffold installs cleanly was
  verified by hand once, in commit `6519687`; it is not verified by this
  suite and cannot be here.
- **`cargo package` for the five dependent crates.** Expected to fail
  dependency resolution against crates.io, because 0.3.0 is not
  published. That is the known-good signal.
- **Linux and macOS.** `resources::write_them` writes a `.desktop`
  entry and an `.app` bundle; both are covered by unit tests that read
  back the text they generate on any machine, and neither has been run
  on the system it is for.
- **A real `rustx build` end to end.** It needs `rustx-app` from
  crates.io, which 0.3.0 is not. The command line's own half — reading
  the manifest, finding the binary, stamping the resources — is covered
  without cargo.

---

## Verification

Run on `asynchrone`, Windows 10, cargo 1.99.1, rustc 1.99.1, offline.

### `cargo test --offline -p rustx-cli`

| Target | Before | After |
| --- | --- | --- |
| lib (unit) | 9 | 9 |
| `tests/build.rs` | 8 | 8 |
| `tests/carried.rs` | 1 (+1 ignored) | 1 (+1 ignored) |
| `tests/dev.rs` | 10 | 10 |
| `tests/held.rs` | 4 | 4 |
| `tests/help.rs` | 4 | 4 |
| `tests/new.rs` | 15 | 15 |
| `tests/target.rs` | — | 11 |
| `tests/typed.rs` | — | 17 |
| `tests/scaffold.rs` | — | 14 |
| **total** | **51** | **93** |

`0 failed` throughout. Zero warnings from `rustx-cli`.

---

## What remains

- [ ] `rustx package`, or whatever the framework decides shipping an
      application looks like. Named as a gap, not designed here.
- [ ] A CI workflow. There is none in the repository; §6 asks for one.
- [ ] `cargo clippy` over the crate. Not run here.
- [ ] The two `rustx-compiler::bundle` defects above, which belong to the
      compiler.
