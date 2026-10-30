//! What this application is: devices, a selection, and a stream that
//! closes itself.
//!
//! The screens are `app/`. This is the software under them.

use std::sync::atomic::{AtomicU32, Ordering};

use rustx::router;
use rustx::store::store;

/// How long the application has been up, in ticks of the layout's
/// thread. A store, so the bar and anything else see the same one.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ticks(pub u32);

/// Which device the user picked. Read by both screens, handed down by
/// neither.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct Selected(pub String);

/// How many streams have been closed, so a test can see that leaving a
/// preview closed the one it had opened.
pub static CLOSED: AtomicU32 = AtomicU32::new(0);

/// How many times the layout has started its thread.
///
/// This is what says the layout survived the navigation, and it says it
/// without a stopwatch: a layout that had been released and built again
/// would have started a second one.
pub static LAYOUT_STARTS: AtomicU32 = AtomicU32::new(0);

/// Called by the layout's background task, once per start.
pub fn started() {
    LAYOUT_STARTS.fetch_add(1, Ordering::Release);
}

/// A capture stream, standing in for the hardware the OS layer will
/// bring. What matters is that it is opened by a component and closed by
/// nobody.
pub struct Stream {
    pub device: String,
}

impl Stream {
    pub fn open(device: &str) -> Self {
        Self {
            device: device.to_string(),
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        CLOSED.fetch_add(1, Ordering::Release);
    }
}

/// Picking a device from the list: remember it, and go to its screen.
pub fn open(device: &str) {
    store::<Selected>().set(Selected(device.to_string()));
    router::navigate(format!("/device/{device}"));
}
