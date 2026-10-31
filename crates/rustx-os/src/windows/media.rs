//! Starting Media Foundation, once, and putting it down after.
//!
//! `MFStartup` is counted: every start needs a shutdown, and a program
//! that forgets one leaves the subsystem loaded for as long as it runs.
//! So nobody calls it directly — they take a guard, and the last guard
//! to go puts the platform down.

#![cfg(windows)]

use std::sync::Mutex;

use windows::Win32::Media::MediaFoundation::{MF_VERSION, MFShutdown, MFStartup};

/// How many guards are out.
static HELD: Mutex<usize> = Mutex::new(0);

/// A guard on Media Foundation being up.
pub struct Started;

impl Drop for Started {
    fn drop(&mut self) {
        let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        *held = held.saturating_sub(1);
        if *held == 0 {
            unsafe {
                let _ = MFShutdown();
            }
        }
    }
}

/// Media Foundation, up for as long as the guard lives.
pub fn started() -> Started {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if *held == 0 {
        unsafe {
            // A failure here is not fatal to the caller: what follows
            // will refuse in its own words, naming the device, which is
            // more use than a start-up complaint nobody can place.
            let _ = MFStartup(MF_VERSION, 0);
        }
    }
    *held += 1;
    Started
}
