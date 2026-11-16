//! Keeping remote content out of an unsandboxed render process.
//!
//! The engine turns Chromium's sandbox off because the only code in the
//! render process is the developer's own (design document §7.1). That is a
//! precondition, not an observation: the moment a page reaches a remote
//! origin, somebody else's JavaScript is handed every native function the
//! application registered, unsandboxed, with the user's full token.
//!
//! So the refusal is enforced in two places, and both are needed:
//!
//! - [`is_local`] gates navigation, before the load starts.
//! - The render process checks the frame's URL before installing any native
//!   function, so a context that got in some other way is inert.

use std::sync::atomic::{AtomicU32, Ordering};

/// Schemes a `rustx` application may load.
const LOCAL_SCHEMES: [&str; 3] = ["file://", "about:blank", "data:"];

/// How many loads the engine has refused, for tests and diagnostics.
static REFUSALS: AtomicU32 = AtomicU32::new(0);

/// Whether this URL names content the application itself ships.
pub fn is_local(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() {
        // An empty URL is the initial about:blank of a fresh frame.
        return true;
    }
    let lowered = url.to_ascii_lowercase();
    LOCAL_SCHEMES
        .iter()
        .any(|scheme| lowered.starts_with(scheme))
}

/// Record that a load was refused, and say so where a developer will see it.
pub(crate) fn refuse(url: &str) {
    REFUSALS.fetch_add(1, Ordering::Relaxed);
    eprintln!(
        "rustx: refused to load {url}\n  \
         A rustx application only loads local content. The render process \
         runs your Rust unsandboxed, which is only safe while nothing else \
         can get in. See the design document, §5."
    );
}

/// How many loads have been refused in this process.
pub fn refusals() -> u32 {
    REFUSALS.load(Ordering::Relaxed)
}
