//! Background work that stops when the component that started it goes.
//!
//! A capture thread is the shape the proof-of-concept software needs on its
//! first day: it opens a stream, it never stops on its own, and it is still
//! producing when the panel that started it leaves the screen. Nothing in
//! the application should have to remember to shut it down.
//!
//! So a task is a value, not a registration. [`spawn`] hands back a
//! [`Task`] token; dropping the token cancels the work. A component keeps
//! the token in one of its cells (`keep`), so the token goes when the
//! component goes, so the thread stops — the same `Drop` that releases
//! everything else a component holds, with no second mechanism beside it.
//!
//! Cancellation is cooperative, and deliberately so: dropping the token
//! does not join the thread. Joining would mean the UI thread waiting on a
//! capture thread that is blocked in a driver call, which is a frozen
//! window. The token flips a flag; the task reads it through
//! [`Alive::get`] and returns.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::frame::{PostError, poster};

/// What the token and its thread share: two flags and nothing else.
struct Shared {
    /// False as soon as the token is dropped.
    alive: AtomicBool,
    /// True until the body returns, panic or not.
    running: AtomicBool,
}

/// The task's end of the cancellation flag.
///
/// `Clone` and `Send`, so a task that fans out over several threads can
/// hand each of them the same answer to "should I still be working?".
#[derive(Clone)]
pub struct Alive {
    shared: Arc<Shared>,
}

impl Alive {
    /// Whether the work is still wanted. `false` once the token is gone.
    pub fn get(&self) -> bool {
        self.shared.alive.load(Ordering::Acquire)
    }

    /// Hand `job` to the UI thread, unless this task has been cancelled.
    ///
    /// A cancelled task's work is refused rather than queued: the
    /// component that would have read it is, by the time the task is
    /// cancelled, on its way out of the tree.
    pub fn post(&self, job: impl FnOnce() + Send + 'static) -> Result<(), PostError> {
        if !self.get() {
            return Err(PostError::Cancelled);
        }
        poster().post(job)
    }
}

/// A running task. Dropping it cancels the work.
///
/// Keep it in a component (the compiler's `task(…)` does exactly that) and
/// the work lasts precisely as long as the component does.
pub struct Task {
    shared: Arc<Shared>,
    /// Set when the body is a `Future` rather than a thread. A thread is
    /// asked to stop and notices; a future is abandoned where it waits,
    /// because an `await` has no flag to look at.
    abandon: Option<tokio::task::AbortHandle>,
    /// What the writes this task posted are bound to. Abandoning the
    /// future stops it making new ones; this is what refuses the ones
    /// already waiting for a frame.
    until: Option<rustx_dom::state::Cancel>,
}

impl Task {
    /// Whether the body is still on its thread.
    ///
    /// A cancelled task may answer `true` for a moment longer: the flag is
    /// read by the task itself, which notices when it next looks.
    pub fn is_running(&self) -> bool {
        self.shared.running.load(Ordering::Acquire)
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
        if let Some(until) = self.until.take() {
            until.cancel();
        }
        if let Some(abandon) = self.abandon.take() {
            abandon.abort();
        }
    }
}

/// Run `work` on a thread of its own, until it returns or the token goes.
///
/// The body runs inside a `catch_unwind`: a thread that touches hardware
/// will panic one day, and a framework whose first application drives
/// hardware has to stay on screen long enough to say so.
pub fn spawn(work: impl FnOnce(Alive) + Send + 'static) -> Task {
    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        running: AtomicBool::new(true),
    });
    let alive = Alive {
        shared: Arc::clone(&shared),
    };
    let thread_end = Arc::clone(&shared);

    std::thread::spawn(move || {
        let _finished = Finished(thread_end);
        if catch_unwind(AssertUnwindSafe(|| work(alive))).is_err() {
            report_panic();
        }
    });

    Task {
        shared,
        abandon: None,
        until: None,
    }
}

/// Run `future` on the async runtime, until it finishes or the token goes.
///
/// The same token as [`spawn`], so the same `Drop` ends it and the same
/// cell of a component holds it. What differs is what ending means: a
/// thread is asked to stop and looks at the flag when it next can, while
/// a future is abandoned at the `await` it is parked on. A library that
/// has nothing to poll cannot be asked politely.
///
/// The runtime starts here, on the first future of the process, and not
/// before — an application that writes no `async` pays for no thread.
pub fn spawn_future(future: impl Future<Output = ()> + Send + 'static) -> Task {
    spawn_future_for(future, None)
}

/// The same, with the token the writes of this future are bound to.
///
/// What the compiler emits for `task(async { … })`: the handles are
/// built before the block, on the render thread, and they hold the other
/// end of this token. Dropping the `Task` cancels it, so a write that
/// was already in the mailbox is refused rather than applied — which
/// for a value found by a name is the difference between landing on the
/// next page and not.
pub fn spawn_future_until(
    until: rustx_dom::state::Cancel,
    future: impl Future<Output = ()> + Send + 'static,
) -> Task {
    spawn_future_for(future, Some(until))
}

/// The value of a future, on the render that has it -- and nothing on
/// the ones before.
///
/// What `let x = f().await;` in a page becomes. A component is asked for
/// the screen as it is now, so it cannot stop and wait: the first render
/// asks for the future and is handed `None`, the future runs beside the
/// window on the runtime's own threads, and the frame that drains its
/// write draws the component again -- this time with the value.
///
/// `held` is where the answer lands, `slot` is where the asking is
/// remembered -- so the future is started once and not once per frame --
/// and `screen` is the address the render is of, which is what makes the
/// question be asked again when the address changes and not otherwise.
/// The compiler writes two different places for the first two because
/// one slot holds one type.
pub fn awaited<T, F>(
    held: rustx_dom::state::Slot,
    slot: rustx_dom::state::Slot,
    screen: &str,
    work: impl FnOnce() -> F,
) -> Option<T>
where
    T: Clone + Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    // Both places are keyed by the screen this render is of, which is
    // the whole answer to a question that cost a `with_key` written by
    // hand: `/depots/rust-lang/rust` and `/depots/tokio-rs/tokio` are
    // the same route, so the same component at the same place, so the
    // same cell -- and a question asked once per cell is a question
    // asked once for both repositories. The second screen showed the
    // first one's answer, and the only way to know was to have seen it.
    //
    // Keyed here rather than by the router, so that the component
    // itself is **not** re-keyed: it keeps its cells, an effect
    // watching the parameter still closes before it opens, and only
    // the asking and the answer start again.
    rustx_dom::state::with_key(format!("await\u{1f}{screen}"), || {
        let held = rustx_dom::state::cell(held, || None::<T>);
        // Kept, so this runs on the first render of this component and
        // not again; held, so the task is abandoned when the component
        // goes.
        let _asked = rustx_dom::state::keep_at(slot, || {
            let cancel = rustx_dom::state::Cancel::new();
            let answer = held.remote_until(&cancel);
            let work = work();
            spawn_future_until(cancel, async move {
                let got = work.await;
                answer.update(move |held| *held = Some(got));
            })
        });
        held.get()
    })
}

fn spawn_future_for(
    future: impl Future<Output = ()> + Send + 'static,
    until: Option<rustx_dom::state::Cancel>,
) -> Task {
    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        running: AtomicBool::new(true),
    });
    // Owned by the future, so it is dropped when the future finishes and
    // also when the future is abandoned without ever being polled.
    let finished = Finished(Arc::clone(&shared));

    let handle = crate::runtime::handle();
    let body = handle.spawn(async move {
        let _finished = finished;
        future.await;
    });
    let abandon = body.abort_handle();

    // A panic in a future arrives as a `JoinError`, and somebody has to
    // be waiting for it or nothing would ever be said. Abandoning the
    // body is not a panic, so a cancelled task stays quiet.
    handle.spawn(async move {
        if body.await.is_err_and(|ended| ended.is_panic()) {
            report_panic();
        }
    });

    Task {
        shared,
        abandon: Some(abandon),
        until,
    }
}

/// Says the body is over on the way out, whichever way it went.
struct Finished(Arc<Shared>);

impl Drop for Finished {
    fn drop(&mut self) {
        self.0.running.store(false, Ordering::Release);
    }
}

/// What the engine says when a task panics. The application carries on:
/// one broken capture thread is not a reason to take the window down.
const NOTICE: &str = "rustx: a background task panicked; the application carried on";

static PANICS: AtomicU32 = AtomicU32::new(0);

fn notice() -> &'static Mutex<Option<String>> {
    static NOTICE_SEEN: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    NOTICE_SEEN.get_or_init(|| Mutex::new(None))
}

fn report_panic() {
    PANICS.fetch_add(1, Ordering::Relaxed);
    eprintln!("{NOTICE}");
    let mut seen = notice()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *seen = Some(NOTICE.to_string());
}

/// How many tasks have panicked in this process.
pub fn panics() -> u32 {
    PANICS.load(Ordering::Relaxed)
}

/// What the engine last said about a panicking task.
///
/// The same sentence that went to stderr, kept so that the application —
/// and the test suite, which cannot rely on a CEF sub-process inheriting
/// the parent's stderr — can see that it was said.
pub fn last_panic() -> Option<String> {
    notice()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[cfg(test)]
mod tests {
    //! The same four claims the `task` scenario makes about a real
    //! window, made here in one process: a window is not needed to prove
    //! that a flag flips and a thread notices.

    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// Waits for `condition`, rather than sleeping a guessed amount.
    /// Cancellation is cooperative, so what is being waited for is the
    /// task's next look at the flag.
    fn within(patience: Duration, condition: impl Fn() -> bool) -> bool {
        let deadline = Instant::now() + patience;
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        condition()
    }

    const PATIENCE: Duration = Duration::from_secs(5);

    #[test]
    fn a_task_stops_when_its_token_is_dropped() {
        let stopped = Arc::new(AtomicBool::new(false));
        let witness = Arc::clone(&stopped);

        let task = spawn(move |alive| {
            while alive.get() {
                std::thread::sleep(Duration::from_millis(1));
            }
            witness.store(true, Ordering::Release);
        });

        assert!(task.is_running(), "the body should be on its thread");
        drop(task);

        assert!(
            within(PATIENCE, || stopped.load(Ordering::Acquire)),
            "the task never noticed that its token was gone"
        );
    }

    #[test]
    fn what_a_cancelled_task_posts_afterwards_is_refused() {
        let (sender, receiver) = mpsc::channel();

        let task = spawn(move |alive| {
            while alive.get() {
                std::thread::sleep(Duration::from_millis(1));
            }
            let _ = sender.send(alive.post(|| {}));
        });
        drop(task);

        assert_eq!(
            receiver.recv_timeout(PATIENCE),
            Ok(Err(PostError::Cancelled)),
            "work from a cancelled task was queued instead of refused"
        );
    }

    #[test]
    fn a_task_that_panics_is_reported_and_leaves_the_engine_standing() {
        let before = panics();
        let task = spawn(|_alive| panic!("a task panicked on purpose"));

        assert!(
            within(PATIENCE, || panics() > before),
            "a panicking task was never reported"
        );
        assert!(
            last_panic().is_some_and(|said| said.contains("a background task panicked")),
            "the engine said {:?} about a panicking task",
            last_panic()
        );
        assert!(
            within(PATIENCE, || !task.is_running()),
            "a panicking task never finished"
        );
    }

    #[test]
    fn a_task_kept_by_a_component_stops_when_the_component_goes() {
        use rustx_dom::state::{InstanceId, Scope, Slot, drop_instance, keep_at};

        let stopped = Arc::new(AtomicBool::new(false));
        let witness = Arc::clone(&stopped);
        let instance = InstanceId(9_001);

        {
            let _scope = Scope::enter(instance);
            keep_at(
                Slot {
                    file: file!(),
                    line: line!(),
                    column: column!(),
                },
                move || {
                    spawn(move |alive| {
                        while alive.get() {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        witness.store(true, Ordering::Release);
                    })
                },
            );
        }

        assert!(
            !stopped.load(Ordering::Acquire),
            "the task stopped while its component was still mounted"
        );

        // Nothing here cancels the task by name: the component leaves, so
        // what it kept is dropped, so the token goes.
        drop_instance(instance);

        assert!(
            within(PATIENCE, || stopped.load(Ordering::Acquire)),
            "the component left but its thread stayed"
        );
    }
}
