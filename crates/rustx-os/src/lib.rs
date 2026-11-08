//! What a program needs from the machine.
//!
//! This is the layer that separates `rustx` from a faster Tauri: the
//! devices, the camera, the screen, recording. It knows nothing about
//! the DOM, nothing about CEF and nothing about components — it is
//! hardware and the words for it, so that the engine above can stay
//! about drawing.
//!
//! # One directory per operating system
//!
//! ```text
//! clipboard.rs      the API a page sees: the types, the words, no platform
//! devices.rs        idem
//! windows/          clipboard.rs  devices.rs  media.rs  …
//! linux/            clipboard.rs  devices.rs  …
//! macos/            clipboard.rs  devices.rs  …
//! other/            whatever is neither of the three, until written
//! ```
//!
//! A capability is one file per platform, with the same function names
//! in each, and the module above it never asks which machine it is on.
//! So a capability arrives on a second platform by filling one file —
//! and the question "what is left to do on Linux?" is answered by
//! looking in `linux/`.
//!
//! Every capability is written for Windows. On Linux, the clipboard,
//! notifications, autostart, a single instance, file dialogs and the
//! camera list are real too — each by a program or a file every
//! desktop distribution already has (`xclip`/`wl-copy`,
//! `notify-send`, a `.desktop` file, a Unix socket, `zenity`/
//! `kdialog`, `/sys/class/video4linux`), not by a library this crate
//! would have to add and this machine cannot run. Global shortcuts,
//! the tray and window control still refuse on Linux: each needs
//! either a desktop portal over D-Bus or a connection to the display
//! server, which is a larger piece of work than a file at a time. On
//! macOS, the same five capabilities plus file dialogs are real, by
//! the same idea applied to what macOS itself ships
//! (`pbcopy`/`pbpaste`, `osascript`, a `LaunchAgent` plist, a Unix
//! socket again, `system_profiler`); global shortcuts, the tray and
//! window control need `AppKit`/Carbon there too and still refuse.
//! Every platform that cannot do a capability refuses in a sentence
//! that names both the platform and the capability: a program that
//! builds everywhere and silently does nothing on two of the three is
//! worse than one that says where it stands. The tests compile
//! `linux/`, `macos/` and `other/` as well, on whatever machine they
//! run on, so that a change to the API above breaks them here rather
//! than on somebody else's machine three weeks later — which is also
//! why the files that need a type `std::os::unix` does not have on
//! Windows (both platforms' `instance.rs`) keep that part behind
//! `#[cfg(unix)]`, with a stand-in beside it for this machine's own
//! check to compile against.
//!
//! # One dependency
//!
//! `windows`, Microsoft's own bindings, and only in this crate. The
//! design document asks for an existing crate wherever there is a good
//! one (§7.6), and hand-writing the COM vtables of Media Foundation is
//! precisely the work this project has no reason to redo. Nothing above
//! this crate hears about it.

pub mod autostart;
pub mod clipboard;
pub mod devices;
pub mod dialog;
pub mod error;
pub mod instance;
pub mod notify;
pub mod patience;
pub mod shortcut;
pub mod tray;
pub mod window;

// One directory per operating system, and one file per capability
// inside it. Which one is compiled is the only place in this crate
// where a platform is named: everything above `platform` asks for a
// capability and is handed the one the machine has.
#[cfg(windows)]
#[path = "windows/mod.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
#[path = "other/mod.rs"]
mod platform;

pub use error::OsError;

// The platforms that are not this one, compiled anyway when the tests
// run. They are ordinary Rust — no platform bindings — so a rename or a
// changed signature in the API above breaks them here, on whatever
// machine the tests are run on, instead of on somebody else's Linux or
// macOS three weeks later.
#[cfg(test)]
#[allow(dead_code)]
#[path = "linux/mod.rs"]
mod linux_also_compiles;
#[cfg(test)]
#[allow(dead_code)]
#[path = "macos/mod.rs"]
mod macos_also_compiles;
#[cfg(test)]
#[allow(dead_code)]
#[path = "other/mod.rs"]
mod elsewhere_also_compiles;

#[cfg(test)]
mod tests {
    #[test]
    fn a_platform_that_cannot_do_it_says_which_platform_it_is() {
        // What a developer on whatever this crate calls "other" reads
        // today — neither Windows, Linux nor macOS, none of which are
        // `other/` any more: Linux's clipboard goes through
        // `wl-copy`/`xclip` and macOS's through `pbcopy`/`pbpaste`,
        // and both constants say `true`. The message names the
        // platform and the capability, because "unsupported" alone
        // sends the reader to the source.
        let refused = super::elsewhere_also_compiles::clipboard::text();
        let said = refused.expect_err("this platform refuses for now").to_string();
        assert!(said.contains("the clipboard"), "got {said}");
        assert!(
            said.contains(std::env::consts::OS),
            "the platform is not named: {said}"
        );
        // `const`, donc l'assertion est constante : c'est voulu, elle
        // épingle la constante d'un fichier qu'on remplira un jour.
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(
                !super::elsewhere_also_compiles::clipboard::AVAILABLE,
                "a platform that refuses must not say it is available"
            );
        }
    }
}
