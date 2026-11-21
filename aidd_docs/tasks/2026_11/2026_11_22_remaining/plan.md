---
objective: "Everything still open after the self-installing app: fix what just shipped, add the bootstrapper and auto-updater, prove the screens visually, then the remaining BUG.md items."
status: pending
---

# Plan: What remains

Source: the advisor's plan of 2026-11-21 after the self-installing app (CI run 44907756511 green, branch webview-migration, draft PR 7) and BUG.md.

## Phases

| #   | Phase        | File                         | Depends on |
| --- | ------------ | ---------------------------- | ---------- |
| 1   | Fix what just shipped | [`phase-1.md`](./phase-1.md) | none |
| 2   | Bootstrapper and auto-updater | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | Visual proof and editor support for system/ | [`phase-3.md`](./phase-3.md) | 1 |
| 4   | The rest of BUG.md | [`phase-4.md`](./phase-4.md) | 1 |

## External sources

| URL   | What it settled   |
| ----- | ----------------- |
| none | none |

## Impact

| Category | Verdict and evidence | Handled in |
| -------- | -------------------- | ---------- |
| runtime | a signed exe may no longer carry its stamp as the last 14 bytes (signtool appends a certificate table): unverified | 1 |
| regression | apps using main(pages) get no system routes; staging compiles from target/rustx-tree | 1 |
| architecture | a self-installing binary needs a runtime (WebView2) before it can draw its own installer | 2 |
| documentation | packaging.mdx, whats-left and result.md must say what each CI job tested | 1, 2, 3 |

## Rules

| Rule | Why it applies |
| ---- | -------------- |
| Build cargo from PowerShell | Git Bash link shadows the MSVC linker |
| Push once and wait for the CI run to finish (scratchpad run.sh) | a push cancels the run in flight |
| Every change ships with a check | the user asked for tests with each fix |
| Say tested in docs only where a test ran on that OS | CI counts per file prove it |
| Write scripts and files with the Write tool, set MSYS_NO_PATHCONV=1, use TEMP for Python paths | Git Bash mangles backslashes, apostrophes and slash paths |
| Give every test a unique folder label or counter | shared or time-stamped fixture folders collided |
| Do not touch BUG.md | it stays untracked |

## Decisions

| Decision | Why |
| -------- | --- |
| Phases 2 to 4 do not block shipping the self-installing app; phase 1 does | 1a and 1b can produce an installer that does nothing |
| Phase 4 items each get their own plan before any code | they are large and independent |
