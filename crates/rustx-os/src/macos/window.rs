//! Window control, on macOS: not yet, and it says so.
//!
//! What this will be: `NSWindow`'s own `styleMask` without
//! `.titled` to remove the frame, `performWindowDragWithEvent:` to
//! start a drag from wherever a page's own handle calls for it, and
//! `toggleFullScreen:` for the full-screen state the two other
//! platforms have to track by hand (macOS already knows whether a
//! window is full screen, as a notification the window posts rather
//! than a size somebody chose). Three calls, on the day this file is
//! written for real — all three need the `NSWindow` handle the engine
//! holds and this crate does not reach yet, which is the actual work
//! here and not the three calls themselves.
//!
//! Refusing by naming the platform is deliberate: a program that
//! builds on macOS and whose window buttons do nothing in silence is
//! worse than one that says where it stands.

use crate::error::OsError;
use crate::window::{Dressing, Pane, Spread};

pub const AVAILABLE: bool = false;

const WHAT: &str = "window control";

pub fn dress(_pane: Pane, _dressing: Dressing) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}
pub fn spread(_pane: Pane, _how: Spread) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}
pub fn spread_of(_pane: Pane) -> Result<Spread, OsError> {
    Err(OsError::Unsupported(WHAT))
}
pub fn on_top(_pane: Pane, _above: bool) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}
pub fn drag(_pane: Pane) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}
pub fn close(_pane: Pane) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}
pub fn pointer() -> Result<(i32, i32), OsError> {
    Err(OsError::Unsupported(WHAT))
}
