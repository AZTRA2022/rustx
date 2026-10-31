//! What Windows can do, and how.
//!
//! One directory per platform, and one file per capability inside it:
//! the API a page sees lives one level up, in `clipboard.rs` and
//! `devices.rs`, and says nothing about any platform. So a capability
//! arrives on a second platform by writing one file whose functions
//! have the same names — `linux/clipboard.rs` is the same list as
//! this one — and nothing above changes.

pub mod clipboard;
pub mod devices;
pub mod media;
