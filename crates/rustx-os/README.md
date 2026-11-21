# rustx-os

**What the web cannot do: the clipboard, native dialogs, the tray,
global shortcuts, a terminal, one instance.** Part of
[rustx](https://github.com/AZTRA2022/rustx), and the one crate here that
is genuinely useful on its own.

> If you are writing a rustx application you want the `rustx` crate —
> the package `rustx-app` — which re-exports all of this under
> `rustx::clipboard`, `rustx::dialog`, `rustx::notify` and so on.
>
> But this crate knows nothing about rustx: no DOM, no web view, no
> components. It is hardware and the words for it. **Any Rust program
> can depend on it**, and nothing above it will be dragged in.

```toml
[dependencies]
rustx-os = "0.3"
```

```rust,ignore
use rustx_os::{clipboard, dialog, notify};

clipboard::set_text("copied")?;
let picked = dialog::open_file()?;
notify::show("Done", "The export finished.")?;
```

## The capabilities

| Module | What it is |
| --- | --- |
| `about` | the machine: its name, its version, its locale |
| `attention` | asking for the user's attention without stealing focus |
| `audio` | the output and input devices, and the volume |
| `autostart` | starting with the session |
| `blocking` | keeping the machine awake |
| `bluetooth` | the paired devices |
| `capture` | recording the screen |
| `clipboard` | text, images, and a custom format |
| `color` | picking a colour off the screen |
| `devices` | the cameras and the microphones |
| `dialog` | open, save, pick a directory, ask a question |
| `instance` | one instance of this application, and a second one's argv |
| `net` | which network this is, and whether there is one |
| `notify` | a notification the desktop draws |
| `patience` | a progress indicator on the taskbar or the dock |
| `print` | the printers, and printing |
| `screen` | the monitors, their size and their scale |
| `secret` | the operating system's own credential store |
| `shell` | opening a file or a URL with whatever owns it |
| `shortcut` | a shortcut that works when the window has no focus |
| `system` | idle time, battery, power |
| `terminal` | a real terminal, with a process in it |
| `tray` | an icon in the notification area, and its menu |
| `window` | the frame, the size, the rank, the move |

## One directory per operating system

```text
clipboard.rs      the API a page sees: the types, the words, no platform
devices.rs        idem
windows/          clipboard.rs  devices.rs  media.rs  …
linux/            clipboard.rs  devices.rs  …
macos/            clipboard.rs  devices.rs  …
other/            whatever is neither of the three, until written
```

A capability is one file per platform with the same function names in
each, and the module above never asks which machine it is on. So a
capability arrives on a second platform by filling one file — and the
question *"what is left to do on Linux?"* is answered by looking in
`linux/`.

## Where each platform actually stands

This is the part a readme usually fudges. It is written down instead.

**Windows** — every capability, implemented and tested.

**Linux** — the clipboard, notifications, autostart, a single instance,
file dialogs and the camera list are real, each through a program or a
file every desktop distribution already has (`xclip`/`wl-copy`,
`notify-send`, a `.desktop` file, a Unix socket, `zenity`/`kdialog`,
`/sys/class/video4linux`) rather than through a library this crate would
have to add. Window control is real on X11, through `x11rb` —
compile-checked and never run, which the file says at its top. Global
shortcuts and the tray **refuse**: each needs a desktop portal or a
`StatusNotifierItem` over D-Bus, which is a larger piece of work than a
file at a time.

**macOS** — the same six as Linux, by the same idea applied to what
macOS itself ships (`pbcopy`/`pbpaste`, `osascript`, a `LaunchAgent`
plist, a Unix socket, `system_profiler`). Global shortcuts (Carbon), the
tray and window control (AppKit) are written and have **never been
compiled for a Mac, let alone run**. Each file says so at its top, and
none of the three is to be trusted before it has been.

A platform that cannot do a capability refuses in a sentence that names
both the platform and the capability. A program that builds everywhere
and silently does nothing on two of the three is worse than one that
says where it stands.

**Compiled is not run, and the two are not the same claim.** Only the
Windows half has ever been *executed*. `linux/`, `macos/` and `other/`
are *compile-checked* on whatever machine the tests run on — `lib.rs`
includes each of them under `#[cfg(test)]` as `linux_also_compiles`,
`macos_also_compiles` and `elsewhere_also_compiles` — so a rename or a
changed signature in the API above breaks here rather than on somebody
else's machine three weeks later. That proves the code still
type-checks against the API it implements. It proves nothing about
whether `osascript` was handed the right script.

So: **portable by construction, verified on Windows.** Not
"cross-platform" without that sentence after it.

## Dependencies

One per platform, and nothing cross-platform:

- Windows: `windows`, Microsoft's own bindings.
- Linux: `x11rb`, whose default connection links neither `libX11` nor
  `libxcb`, so this crate asks the system for nothing but a `$DISPLAY`.
- macOS: `objc2` alone.

Nothing above this crate hears about any of them.

Rust 1.88 or newer, edition 2024. Licensed under MIT or Apache-2.0, at
your option.
