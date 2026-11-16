//! What a frozen window says about itself.
//!
//! The thread that would notice is the thread that is stuck, so the
//! noticing is done from another one. These tests are that other one,
//! with the deadline handed in rather than waited out.

use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use rustx_engine::watchdog;

/// What is being drawn is one thing per process, so these tests are one
/// at a time. Run together they would read each other's render.
fn alone() -> MutexGuard<'static, ()> {
    static ONE: OnceLock<Mutex<()>> = OnceLock::new();
    ONE.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn a_render_that_came_back_is_not_complained_about() {
    let _alone = alone();
    {
        let _drawing = watchdog::drawing("#app");
    }
    assert_eq!(watchdog::check(Duration::ZERO), None);
}

#[test]
fn a_render_that_has_not_come_back_is_named() {
    let _alone = alone();
    let _drawing = watchdog::drawing("#app");
    let (what, _waited) = watchdog::check(Duration::ZERO).expect("the stuck render is named");
    assert_eq!(what, "#app");
}

#[test]
fn a_render_inside_the_deadline_is_left_alone() {
    let _alone = alone();
    let _drawing = watchdog::drawing("#app");
    assert_eq!(watchdog::check(Duration::from_secs(600)), None);
}

#[test]
fn a_stuck_render_is_complained_about_once() {
    let _alone = alone();
    // A loop that never ends would otherwise write a line every half
    // second for as long as the window is open, and the second line
    // says nothing the first did not.
    let _drawing = watchdog::drawing("#app");
    assert!(watchdog::check(Duration::ZERO).is_some());
    assert_eq!(watchdog::check(Duration::ZERO), None);
}

#[test]
fn the_outermost_render_is_the_one_named() {
    let _alone = alone();
    // A component drawn inside another is part of the same render: what
    // is worth naming is the one that was asked for.
    let _outer = watchdog::drawing("#app");
    let _inner = watchdog::drawing("Counter");
    let (what, _waited) = watchdog::check(Duration::ZERO).expect("named");
    assert_eq!(what, "#app");
}
