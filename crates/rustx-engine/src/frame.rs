//! The frame loop, and the one door worker threads may use.
//!
//! The design document's threading model in one file. State belongs to the
//! render process's UI thread and to nothing else: it is neither `Send` nor
//! `Sync`, there is no lock on the hot path, and a closure written by the
//! application carries no thread bound it did not ask for.
//!
//! A worker thread — capture, encoding, I/O — never mutates that state. It
//! posts a job through a [`Poster`], the only object in the engine that
//! crosses a thread boundary, and the UI thread drains the queue once per
//! frame before running the frame callbacks.
//!
//! That is what keeps a thread pushing ten thousand updates a second from
//! drowning the renderer: the updates all land, the screen is written once
//! per frame. The developer writes their loop as fast as they like and
//! cannot shoot themselves in the foot. No debounce to configure, no
//! throttling API to learn.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};

/// Work to run on the UI thread.
type Job = Box<dyn FnOnce() + Send>;

/// The shared queue. `None` once the window is gone.
fn queue() -> &'static Mutex<Option<Vec<Job>>> {
    static QUEUE: OnceLock<Mutex<Option<Vec<Job>>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(Some(Vec::new())))
}

thread_local! {
    /// Callbacks the application wants on every frame, on this thread.
    static ON_FRAME: RefCell<Vec<Box<dyn FnMut()>>> = const { RefCell::new(Vec::new()) };
    /// How many frames this thread has run.
    static FRAMES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Why a job could not be posted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PostError {
    /// The window is gone. Stop producing work and shut the thread down.
    WindowClosed,
    /// The task that posted this has been cancelled: the component that
    /// started it is on its way out of the tree, so what the task still
    /// has to say is refused rather than delivered to something that is
    /// no longer there.
    Cancelled,
}

impl std::fmt::Display for PostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowClosed => f.write_str("WindowClosed"),
            Self::Cancelled => f.write_str("Cancelled"),
        }
    }
}

impl std::error::Error for PostError {}

/// A handle a worker thread uses to hand work back to the UI thread.
///
/// It carries no DOM and no application state — only a function to run
/// later, where running it is safe.
#[derive(Clone, Debug)]
pub struct Poster {
    _private: (),
}

impl Poster {
    /// Queue `job` to run on the UI thread at the start of the next frame.
    pub fn post(&self, job: impl FnOnce() + Send + 'static) -> Result<(), PostError> {
        let mut guard = queue().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_mut() {
            Some(jobs) => {
                jobs.push(Box::new(job));
                Ok(())
            }
            None => Err(PostError::WindowClosed),
        }
    }
}

/// A poster for the current application. Hand it to any thread.
pub fn poster() -> Poster {
    Poster { _private: () }
}

/// Run `callback` once per frame, on the UI thread.
///
/// This is where rendering belongs. Mutating state does not draw anything;
/// a frame does.
pub fn on_frame(callback: impl FnMut() + 'static) {
    ON_FRAME.with(|callbacks| callbacks.borrow_mut().push(Box::new(callback)));
}

/// How many frames have run on this thread.
pub fn frame_count() -> u64 {
    FRAMES.with(std::cell::Cell::get)
}

/// One frame: drain everything the workers posted, then let the frame
/// callbacks draw the result of all of it at once.
///
/// Every job and every callback is isolated. A panic in one of them must
/// not end the frame loop: the window would freeze with nothing on screen
/// to explain it, the queue would grow without bound behind it, and the
/// application the framework is built for — one that drives hardware —
/// is precisely the kind where a job panics sooner or later.
pub(crate) fn tick() {
    let jobs = {
        let mut guard = queue().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_mut() {
            Some(jobs) => std::mem::take(jobs),
            None => Vec::new(),
        }
    };
    for job in jobs {
        report_panic("a job posted to the frame queue", catch_unwind(AssertUnwindSafe(job)));
    }

    FRAMES.with(|frames| frames.set(frames.get() + 1));

    // The callbacks are taken out of the cell for the duration of the call,
    // so that one of them may register another without deadlocking on the
    // borrow. `Restore` puts them back even if a callback unwinds — without
    // it, one panic would drop every registered callback for good.
    let callbacks = ON_FRAME.with(|callbacks| std::mem::take(&mut *callbacks.borrow_mut()));
    let mut taken = Restore(Some(callbacks));

    if let Some(callbacks) = taken.0.as_mut() {
        for callback in callbacks.iter_mut() {
            report_panic(
                "a frame callback",
                catch_unwind(AssertUnwindSafe(callback)),
            );
        }
    }
    drop(taken);
}

/// Puts the frame callbacks back on the way out, panic or not.
struct Restore(Option<Vec<Box<dyn FnMut()>>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let Some(mut taken) = self.0.take() else {
            return;
        };
        ON_FRAME.with(|existing| {
            let mut existing = existing.borrow_mut();
            taken.append(&mut existing);
            *existing = taken;
        });
    }
}

fn report_panic(what: &str, outcome: Result<(), Box<dyn std::any::Any + Send>>) {
    if outcome.is_err() {
        eprintln!("rustx: {what} panicked; the frame loop carried on");
    }
}

/// Stop accepting work. Anything posted afterwards is refused rather than
/// piling up for a frame that will never come, so a thread still producing
/// can see that it is time to stop.
///
/// The engine calls this when a context goes away; an application may call
/// it to wind down on its own terms.
pub fn shutdown() {
    let mut guard = queue().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = None;
}

/// Reopen the queue for a freshly created context.
pub(crate) fn open() {
    let mut guard = queue().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_none() {
        *guard = Some(Vec::new());
    }
}

/// The script that drives the loop.
///
/// `requestAnimationFrame` is the compositor's own vsync signal, and it
/// already runs on the thread that owns the DOM, so the frame loop rides on
/// it rather than inventing a timer that would drift against the screen.
/// The next frame is scheduled in a `finally`: if `__rustx_frame` ever
/// throws, the loop must limp on rather than stop for good.
pub(crate) const DRIVER_SCRIPT: &str = "\
(function () {
  function frame() {
    try {
      __rustx_frame();
    } finally {
      requestAnimationFrame(frame);
    }
  }
  requestAnimationFrame(frame);
})();
";
