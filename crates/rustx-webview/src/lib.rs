//! The host of a rustx window drawn by the system's webview.
//!
//! The Rust tree stays in this process. What crosses to the page is a
//! batch of patches; what comes back is an event or a call, addressed by
//! id. [`protocol`] is that language and nothing else yet.

pub mod protocol;
