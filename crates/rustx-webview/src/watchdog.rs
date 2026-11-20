//! Saying so when a component does not come back.
//!
//! A component is asked for the screen as it is now, so it runs on the
//! thread that owns the screen. A `loop` before the `return`, a blocking
//! read, a `while` whose condition never turns false: the window freezes,
//! with nothing on screen and nothing on the console to say why — and
//! what the developer concludes is that the framework hung.
//!
//! The compiler catches what it can see (a write to state during a
//! render, which asks for a render that writes it again). This is for
//! what it cannot: a loop that simply never ends. One thread, asleep
//! almost always, that says which component has been drawing and for
//! how long. It cannot stop it — nothing can interrupt a Rust loop from
//! outside — but a frozen window with an explanation is a bug the
//! developer can fix in a minute instead of an evening.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// What is being drawn, and since when.
fn drawing_now() -> &'static Mutex<Option<(String, Instant)>> {
    static DRAWING: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();
    DRAWING.get_or_init(|| Mutex::new(None))
}

/// Whether the current render has already been complained about.
static TOLD: AtomicBool = AtomicBool::new(false);

/// How long a render may take before the watchdog says something.
///
/// Long enough that a heavy first mount never trips it — a page whose
/// boundaries are waiting for a JavaScript library to load takes over
/// two seconds, measured, and is not stuck — and short enough that a
/// developer who froze their window reads the reason before they start
/// bisecting it.
const DEADLINE: Duration = Duration::from_secs(5);

/// A render in progress. Dropping it says the component came back.
pub struct Drawing {
    _private: (),
}

impl Drop for Drawing {
    fn drop(&mut self) {
        *lock() = None;
        TOLD.store(false, Ordering::Release);
    }
}

fn lock() -> std::sync::MutexGuard<'static, Option<(String, Instant)>> {
    drawing_now()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Say that `what` is being drawn now.
///
/// Nested renders are not tracked: a component drawn inside another is
/// part of the same render, and the outermost one is the one worth
/// naming. The guard of an inner render replaces nothing.
pub fn drawing(what: &str) -> Drawing {
    let mut held = lock();
    if held.is_none() {
        *held = Some((what.to_string(), Instant::now()));
    }
    Drawing { _private: () }
}

/// What has been drawing for longer than `deadline`, once per render.
///
/// Once, because a loop that never ends would otherwise write a line a
/// second for as long as the window is open, and the second line says
/// nothing the first did not.
pub fn check(deadline: Duration) -> Option<(String, Duration)> {
    let held = lock();
    let (what, started) = held.as_ref()?;
    let waited = started.elapsed();
    if waited < deadline || TOLD.swap(true, Ordering::AcqRel) {
        return None;
    }
    Some((what.clone(), waited))
}

/// How often the watchdog looks.
const LOOKS_EVERY: Duration = Duration::from_millis(500);

/// Whether the watchdog is wanted, and whether it is there.
///
/// Two flags rather than one, because `watch` has to know that a thread
/// is already on the job without waiting for it to say so, and
/// [`watching`] has to answer about the thread and not about the wish.
static WANTED: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Start watching. A second call while one is watching does nothing.
///
/// A thread rather than a timer on the renderer: the thread the timer
/// would run on is the thread that is stuck.
pub fn watch() {
    WANTED.store(true, Ordering::Release);
    if RUNNING.swap(true, Ordering::AcqRel) {
        return;
    }
    let started = std::thread::Builder::new()
        .name("rustx-watchdog".to_string())
        .spawn(|| {
            // Says the thread is gone whichever way it leaves, so a
            // later `watch` starts another rather than believing one is
            // still there.
            struct Gone;
            impl Drop for Gone {
                fn drop(&mut self) {
                    RUNNING.store(false, Ordering::Release);
                }
            }
            let _gone = Gone;

            while WANTED.load(Ordering::Acquire) {
                std::thread::sleep(LOOKS_EVERY);
                if let Some((what, waited)) = check(DEADLINE) {
                    eprintln!(
                        "rustx: `{what}` has been drawing for {:.1}s and has not come back. \
                         A component is asked for the screen as it is now, so it cannot \
                         wait or loop for ever — move that work into `task(…)`, which runs \
                         beside the component and stops when the component goes.",
                        waited.as_secs_f32()
                    );
                }
            }
        });
    if started.is_err() {
        RUNNING.store(false, Ordering::Release);
    }
}

/// Stop watching.
///
/// Called when the frame queue shuts down: there is no render to be
/// slow any more, and a thread that wakes twice a second for the life
/// of a process that has stopped drawing is an uncontrolled background
/// task however cheap it is. [`watch`] starts another one when the next
/// window mounts something.
pub fn stop() {
    WANTED.store(false, Ordering::Release);
}

/// Is there a watchdog thread right now?
///
/// For tests and diagnostics. It lags [`stop`] by up to one look, which
/// is what cooperative stopping means.
pub fn watching() -> bool {
    RUNNING.load(Ordering::Acquire)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both claims in one test, because the flags are process-wide and
    /// two tests would be two threads arguing over them.
    #[test]
    fn the_watchdog_starts_once_stops_when_asked_and_can_start_again() {
        let patience = LOOKS_EVERY * 8;
        let settled = |wanted: bool| {
            let deadline = Instant::now() + patience;
            while Instant::now() < deadline {
                if watching() == wanted {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            watching() == wanted
        };

        watch();
        assert!(watching(), "nothing is watching");
        // A second call is not a second thread. There is no count to
        // read, so what is asserted is that it neither panics nor
        // leaves the first one's flags in a state `stop` cannot undo.
        watch();
        assert!(watching());

        stop();
        assert!(settled(false), "the watchdog would not stop");

        watch();
        assert!(settled(true), "the watchdog would not start again");
        stop();
        assert!(settled(false));
    }
}
