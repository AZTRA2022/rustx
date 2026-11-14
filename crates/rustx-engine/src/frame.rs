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
    /// Nothing in this process has ever run a frame, and the queue has
    /// grown past what a queue nobody drains may hold.
    ///
    /// `tick` runs in the render process. A `post` made from the
    /// browser process — where the OS layer gives an application real
    /// reasons to have code — was accepted and never drained: the queue
    /// grew without bound and without a word. It is still accepted,
    /// because a post made before the first frame is ordinary and
    /// common; what is refused is the thousandth one in a process that
    /// has never drawn.
    NoFrameLoop,
}

impl std::fmt::Display for PostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowClosed => f.write_str("WindowClosed"),
            Self::Cancelled => f.write_str("Cancelled"),
            Self::NoFrameLoop => f.write_str("NoFrameLoop"),
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
        let mut guard = queue()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_mut() {
            Some(jobs) => {
                if nobody_will_drain(jobs.len(), TICKS.load(Ordering::Relaxed)) {
                    said_nobody_is_draining();
                    return Err(PostError::NoFrameLoop);
                }
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

/// How many jobs a queue nobody has ever drained may hold.
///
/// Generous, because a post before the first frame is ordinary: a
/// window takes a moment to come up and a worker thread may already be
/// producing. Small enough that a process which will never drain —
/// the browser process, where `tick` does not run — stops instead of
/// growing for as long as it is open.
const UNDRAINED: usize = 1_024;

/// How many frames this **process** has run, on any thread.
///
/// Process-wide rather than per-thread, because the question a post
/// asks is "is there a frame loop anywhere here?" and a post is made
/// from a thread that is by definition not the one drawing.
static TICKS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

use std::sync::atomic::Ordering;

/// Said once, where a developer will see it.
fn said_nobody_is_draining() {
    static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if SAID.swap(true, Ordering::Relaxed) {
        return;
    }
    eprintln!(
        "rustx: {UNDRAINED} jobs have been posted to the frame queue and no \
         frame has ever run in this process, so nothing will ever run them. \
         A `Poster` hands work to the thread that draws, and that thread is \
         in the render process — code in the browser process has no frame \
         to post to."
    );
}

/// How many frames have run in this process, on any thread.
pub fn frames_in_this_process() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Will anything ever run what is already queued?
///
/// `false` while a frame loop exists, however long the queue is: a
/// worker pushing ten thousand updates a second is the case the whole
/// design is for, and they all land. `true` only for a queue that has
/// grown past what waiting could explain in a process where no frame
/// has *ever* run — which is the browser process, where `tick` does
/// not.
fn nobody_will_drain(queued: usize, frames: u64) -> bool {
    queued >= UNDRAINED && frames == 0
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
pub fn tick() {
    // The writes a future posted since the last frame, first: a write
    // that landed has to be in the render it asked for and not in the
    // one after. State is this thread's, so this is the only thread
    // that could apply them.
    rustx_dom::state::apply_remote();

    let jobs = {
        let mut guard = queue()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.as_mut() {
            Some(jobs) => std::mem::take(jobs),
            None => Vec::new(),
        }
    };
    for job in jobs {
        report_panic(
            "a job posted to the frame queue",
            catch_unwind(AssertUnwindSafe(job)),
        );
    }

    FRAMES.with(|frames| frames.set(frames.get() + 1));
    TICKS.fetch_add(1, Ordering::Relaxed);
    LAST.with(|last| last.set(Some(std::time::Instant::now())));

    // The callbacks are taken out of the cell for the duration of the call,
    // so that one of them may register another without deadlocking on the
    // borrow. `Restore` puts them back even if a callback unwinds — without
    // it, one panic would drop every registered callback for good.
    let callbacks = ON_FRAME.with(|callbacks| std::mem::take(&mut *callbacks.borrow_mut()));
    let mut taken = Restore(Some(callbacks));

    if let Some(callbacks) = taken.0.as_mut() {
        for callback in callbacks.iter_mut() {
            report_panic("a frame callback", catch_unwind(AssertUnwindSafe(callback)));
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
    // Everything the page had mounted, first: releasing a root runs its
    // components' `Drop`, which is where a file is closed, a capture
    // session is ended and a `task(…)` token is dropped — so it has to
    // happen while the runtime those tasks are on is still there.
    // Before this call, nothing released them: a window that closed
    // while the process stayed left every cell alive, and a thread
    // started by `task(…)` went on looking at a flag nobody would ever
    // flip. In the browser process there is nothing mounted and this is
    // a look at an empty list.
    crate::render::unmount_all();
    // And the thread that watches for a render that never comes back.
    // There is no render to be slow any more, and a thread waking twice
    // a second for the life of a process that has stopped drawing is an
    // uncontrolled background task however cheap it is. The next window
    // that mounts something starts another.
    crate::watchdog::stop();

    let mut guard = queue()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = None;
    drop(guard);
    // And the mailbox a future writes state through, for whichever
    // thread calls this. In the render process that is the thread that
    // owns the cells; in the browser process there is no mailbox worth
    // closing and this makes an empty one. The call that matters for a
    // real window is the one in `cef_app::on_context_released`, which
    // runs where the runtime and the cells are — this one is for the
    // single-process paths and for an application winding down on its
    // own terms.
    rustx_dom::state::close_remote();
    // And the async runtime, in the process that has one. A future left
    // polling after its window has gone has nobody to post to and
    // nothing to draw; in the browser process there is no runtime and
    // this is a lock and a look.
    crate::runtime::stop();
}

/// How long the loop waits for a frame of its own before running one.
///
/// A window that is drawing calls `__rustx_frame` sixty times a second
/// and this never fires. A window that is not calls it never.
const FLOOR: i64 = 100;

thread_local! {
    /// When a frame last ran on this thread, by the engine's own clock.
    static LAST: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
    /// Whether the floor is already waiting to check.
    static WATCHING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Whether a frame has already been asked for by a change.
    static WAKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Put a floor under the frame loop.
///
/// `requestAnimationFrame` is the compositor's own signal, and when the
/// compositor produces no frames — a GPU process that crashes on every
/// start, a window the system has hidden — it is never called back. The
/// application then draws nothing, with nothing in it wrong.
///
/// A timer in the page is not the answer either: Chromium throttles the
/// timers of a window it believes is hidden down to one a second, which
/// is exactly the window this is about. So the floor is a delayed task
/// on the renderer's own thread, which belongs to the engine and not to
/// the page.
pub(crate) fn watch() {
    if WATCHING.with(std::cell::Cell::get) {
        return;
    }
    WATCHING.with(|watching| watching.set(true));
    schedule_check();
}

/// Ask for a frame now, because something changed.
///
/// The floor is for a window that is drawing nothing and has nothing to
/// draw; a click is the opposite, and waiting for the floor to come
/// round would put a tenth of a second between a button and what it
/// does. One task per burst: a handler that writes ten cells asks for
/// one frame, which is the whole point of coalescing them.
pub(crate) fn wake() {
    if WAKING.with(std::cell::Cell::get) {
        return;
    }
    WAKING.with(|waking| waking.set(true));
    crate::renderer::after(0, || {
        WAKING.with(|waking| waking.set(false));
        crate::renderer::in_context(tick);
    });
}

fn schedule_check() {
    crate::renderer::after(FLOOR as u64 / 2, || {
        // The window is gone: there is no frame to put a floor under,
        // and a task that reschedules itself for ever is a task nobody
        // can stop. `watch` is called again for the next context, and
        // the flag being false is what lets it arm a new chain rather
        // than believe one is already running.
        if !accepting() {
            WATCHING.with(|watching| watching.set(false));
            return;
        }
        let stale = LAST
            .with(std::cell::Cell::get)
            .is_none_or(|last| last.elapsed().as_millis() as i64 >= FLOOR);
        if stale {
            // Inside the page's context: everything a frame does
            // touches the DOM, and V8 has no current context in a
            // delayed task of the engine's own.
            crate::renderer::in_context(tick);
        }
        schedule_check();
    });
}

/// Is the frame queue still taking work?
///
/// `false` from [`shutdown`] until the next [`open`], which is the
/// engine's own answer to "is there still a window here?".
pub fn accepting() -> bool {
    queue()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_some()
}

/// Reopen the queue for a freshly created context.
pub fn open() {
    let mut guard = queue()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.is_none() {
        *guard = Some(Vec::new());
    }
    drop(guard);
    // And the mailbox a future writes state through. Without this, a
    // page that follows a closed context ran its futures and had every
    // write refused in silence. The queue's lock is let go first, the
    // way `shutdown` lets it go: no path takes these two in the other
    // order today, and keeping them apart is how it stays true.
    rustx_dom::state::open_remote();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule a post is refused by, at its two edges. Written as a
    /// predicate rather than inline so that it can be said here without
    /// a process that has never drawn and a queue of a thousand jobs.
    #[test]
    fn a_queue_is_only_refused_where_nothing_will_ever_drain_it() {
        // A frame loop exists: the queue may be as long as it likes.
        assert!(!nobody_will_drain(UNDRAINED * 100, 1));
        assert!(!nobody_will_drain(0, 1));

        // Nothing has ever drawn, but waiting for the first frame
        // explains a short queue.
        assert!(!nobody_will_drain(0, 0));
        assert!(!nobody_will_drain(UNDRAINED - 1, 0));

        // And past that it does not.
        assert!(nobody_will_drain(UNDRAINED, 0));
        assert!(nobody_will_drain(UNDRAINED + 1, 0));
    }

    #[test]
    fn every_reason_a_post_can_fail_says_which_it_was() {
        // `PostError` reaches a developer through `Display`, and a
        // variant that printed nothing of use would be a worse answer
        // than no answer.
        for refusal in [
            PostError::WindowClosed,
            PostError::Cancelled,
            PostError::NoFrameLoop,
        ] {
            assert!(!refusal.to_string().is_empty(), "{refusal:?} says nothing");
        }
    }
}

/// The script that drives the loop.
///
/// `requestAnimationFrame` is the compositor's own vsync signal, and it
/// already runs on the thread that owns the DOM, so the frame loop rides on
/// it rather than inventing a timer that would drift against the screen.
///
/// When the compositor produces no frames at all, it is never called
/// back — and [`watch`] is the floor under that: the engine's own timer,
/// on the renderer's thread, which no page and no throttling can reach.
/// The next frame is scheduled in a `finally`: if `__rustx_frame` ever
/// throws, the loop must limp on rather than stop for good.
pub(crate) const DRIVER_SCRIPT: &str = "\n(function () {
  window.__rustx_driving = (window.__rustx_driving || 0) + 1;
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
