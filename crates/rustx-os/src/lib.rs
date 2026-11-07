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
//! windows/          clipboard.rs  devices.rs  media.rs
//! linux/            clipboard.rs  devices.rs
//! other/            macOS and the rest, until somebody writes them
//! ```
//!
//! A capability is one file per platform, with the same function names
//! in each, and the module above it never asks which machine it is on.
//! So a capability arrives on a second platform by filling one file —
//! and the question "what is left to do on Linux?" is answered by
//! looking in `linux/`.
//!
//! Every capability is Windows today. The other platforms compile, and
//! refuse in a sentence that names both the platform and the capability:
//! a program that builds everywhere and silently does nothing on two of
//! the three is worse than one that says where it stands. The tests
//! compile `linux/` and `other/` as well, on whatever machine they run
//! on, so that a change to the API above breaks them here rather than
//! on somebody else's machine three weeks later.
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
#[cfg(not(any(windows, target_os = "linux")))]
#[path = "other/mod.rs"]
mod platform;

pub use error::OsError;

// The platforms that are not this one, compiled anyway when the tests
// run. They are ordinary Rust — no platform bindings — so a rename or a
// changed signature in the API above breaks them here, on whatever
// machine the tests are run on, instead of on somebody else's Linux
// three weeks later.
#[cfg(test)]
#[allow(dead_code)]
#[path = "linux/mod.rs"]
mod linux_also_compiles;
#[cfg(test)]
#[allow(dead_code)]
#[path = "other/mod.rs"]
mod elsewhere_also_compiles;

#[cfg(test)]
mod tests {
    #[test]
    fn a_platform_that_cannot_do_it_says_which_platform_it_is() {
        // What a developer on Linux reads today. The message names the
        // platform and the capability, because "unsupported" alone sends
        // the reader to the source.
        // macOS, aujourd'hui : le dossier `other/`. Linux n'est plus
        // ici — son presse-papiers passe par `wl-copy` / `xclip`, et sa
        // constante dit `true`.
        let refused = super::elsewhere_also_compiles::clipboard::text();
        let said = refused.expect_err("macOS refuses for now").to_string();
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
