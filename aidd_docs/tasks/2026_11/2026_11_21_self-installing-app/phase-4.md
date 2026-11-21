---
status: pending
---

# Instruction: The other OS screens: licence, permissions, about, settings

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
crates/rustx/src/lib.rs ✏️
crates/rustx/assets/system/license.rsx ✅
crates/rustx/assets/system/permissions.rsx ✅
crates/rustx/assets/system/about.rsx ✅
crates/rustx/assets/system/settings.rsx ✅
```

## Test Scope

| Scenario     | Action                  | Observable outcome   | Actor                           |
| ------------ | ----------------------- | -------------------- | ------------------------------- |
| Setup | an app declaring camera | page lists it | system |
| Happy path | open about | name, version, publisher, licence shown | system |
| Edge case | no LICENSE | the licence step is skipped | system |
| Teardown | none | none | system |

## Tasks to do

### `1)` rustx::system

> The app opens its OS-facing screens.

1. Add open for licence, permissions, about and settings; the licence gate runs before the first install or first run when a LICENSE exists.

### `2)` Declared permissions

> Asked in words before the system asks.

1. Config declares the permissions the app uses (camera, microphone, location, notifications, files); the permissions page lists them with their state and asks through the OS on use.
2. The settings page covers start with the session and update checks, using the existing autostart and update modules.

## Test acceptance criteria

| Task | Acceptance criteria              |
| ---- | -------------------------------- |
| 1 | each screen opens from Rust and closes back to the app |
| 2 | the permissions page shows the declared list and the settings toggles change autostart |
