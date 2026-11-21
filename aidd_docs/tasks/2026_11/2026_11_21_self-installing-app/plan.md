---
objective: "A packaged app is its own installer on Windows, macOS and Linux: one stamped binary shows the OS-facing screens (install, uninstall, update, licence, permissions, about, settings) written in RSX under system/, with no separate setup program."
status: pending
---

# Plan: The app is its own installer, on every OS

Source: the user's requests of 2026-11-21 (system screens coded in RSX in locked names or a folder; no separate setup.exe; the system screens are the OS-facing ones: install, update, config, metadata, permissions; every OS is in scope, none kept in parallel) and the advisor's contract.

## Phases

| #   | Phase        | File                         | Depends on |
| --- | ------------ | ---------------------------- | ---------- |
| 1   | Install, uninstall and update in rustx-os for each OS, and the setup API | [`phase-1.md`](./phase-1.md) | none |
| 2   | Package stamp and mode detection | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | system/*.rsx compiled, never routed, with default pages | [`phase-3.md`](./phase-3.md) | 1 |
| 4   | The other OS screens: licence, permissions, about, settings | [`phase-4.md`](./phase-4.md) | 3 |
| 5   | Package and prove it on Windows, macOS and Linux in CI | [`phase-5.md`](./phase-5.md) | 2, 4 |
| 6   | Remove the stub and its payload, update docs | [`phase-6.md`](./phase-6.md) | 5 |

## External sources

| URL   | What it settled   |
| ----- | ----------------- |
| none | none |

## Impact

| Category | Verdict and evidence | Handled in |
| -------- | -------------------- | ---------- |
| runtime | A release exe runs alone: counter.exe copied into an empty folder started and printed "page is ready after 936 ms" (observed 2026-11-21); macOS and Linux unverified | 5 |
| regression | rustx-setup, selfextract, the setup_ui test and the Windows CI steps are replaced; compiler page discovery changes | 3, 5, 6 |
| architecture | the macro compiles only app/ (crates/rustx-macros/src/lib.rs:65 joins "app"); system/ must be compiled as pages that are never routed | 3 |
| documentation | packaging.mdx, whats-left, result.md describe the stub | 6 |

## Rules

| Rule | Why it applies |
| ---- | -------------- |
| Build cargo from PowerShell | Git Bash link shadows the MSVC linker |
| Push once, watch the CI run to the end | a push cancels the run in flight |
| Read before editing | the harness refuses otherwise |

## Decisions

| Decision | Why |
| -------- | --- |
| system/ beside app/ holds locked names: install, uninstall, update, license, permissions, about, settings (.rsx); each optional with a framework default | the OS-facing screens are the framework contract with the system, not routes of the app |
| One install implementation per OS in rustx-os: Windows copies to LOCALAPPDATA with a shortcut and an Apps and features entry; macOS copies the bundle to the user Applications folder and relaunches it; Linux copies to the user local share with a .desktop entry and an icon | no administrator anywhere, and the same screens on all three |
| The stamp is a trailer appended to the binary before signing, read by the binary itself | works the same on PE, Mach-O and ELF, and needs no resource editing |
| Install mode only for a stamped binary outside its install location, never in debug, cargo run or rustx dev; Config::portable() opts out | a developer binary must never copy itself |
| The stub and the payload format are deleted once CI proves the 2-in-1 path on all three OS | two ways to install would drift |
