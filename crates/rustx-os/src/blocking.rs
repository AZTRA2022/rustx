//! What may not be waited for on the thread that draws.
//!
//! Everything in this crate is synchronous, and most of it is fast. A
//! few calls are not, and they are not slow by accident: a file dialog
//! waits for a person, `terminal::run` waits for a process, the
//! clipboard waits for whichever application is holding it, and a
//! device enumeration waits for COM. Called from a page's `onClick` —
//! which runs on the thread that owns the screen — any of them stops
//! the window for as long as they take. The framework's answer is
//! `task(async { … })`, which runs beside the window on the runtime's
//! own threads, and [`crate::terminal::run`] documents it.
//!
//! Documenting it is not enough. A developer who has not read that
//! paragraph sees a window that stopped responding and has no reason to
//! suspect the call they just wrote: the symptom is "the framework
//! hung". So the calls say so themselves. The host tells this module
//! which thread draws ([`draws_here`]), each waiting call says what it
//! is about to wait for ([`waiting`]), and a call made on that thread
//! prints one line naming the capability and what to do about it.
//!
//! It is a warning and not a refusal. The call is a correct call — a
//! `main` that asks for the clipboard before it opens a window, a tool
//! with no window at all — and this crate has no business deciding that
//! an application may not make it. What it can do is make sure the
//! developer is never left guessing.
//!
//! Nothing here costs anything when no window has registered: one
//! relaxed read of an atomic, which is the state of every test of this
//! crate and of every program that uses it as a library.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::ThreadId;

/// Whether any thread has claimed to be the drawing one.
///
/// Read before the mutex, so a process with no window pays one relaxed
/// load per call and never takes a lock.
static CLAIMED: AtomicBool = AtomicBool::new(false);

/// How many warnings have been printed, for tests and diagnostics.
static WARNINGS: AtomicU64 = AtomicU64::new(0);

fn drawing_thread() -> &'static Mutex<Option<ThreadId>> {
    static DRAWS: OnceLock<Mutex<Option<ThreadId>>> = OnceLock::new();
    DRAWS.get_or_init(|| Mutex::new(None))
}

/// What has already been warned about, so a call in a render loop
/// prints one line and not sixty a second.
fn already_said() -> &'static Mutex<Vec<String>> {
    static SAID: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    SAID.get_or_init(|| Mutex::new(Vec::new()))
}

/// The last thing this module said, kept so that a test can see that it
/// was said — a sub-process's stderr is not the test's.
fn last_said() -> &'static Mutex<Option<String>> {
    static LAST: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    LAST.get_or_init(|| Mutex::new(None))
}

fn held<T>(lock: &'static Mutex<T>) -> std::sync::MutexGuard<'static, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Say that this thread is the one the window is drawn on.
///
/// Called by the host when the page's context is created, which is the
/// moment the render process's UI thread becomes the thread that owns
/// the screen. Calling it again from another thread replaces the answer
/// — there is one screen per render process, and a second call means
/// the first one was about a process that has gone.
pub fn draws_here() {
    *held(drawing_thread()) = Some(std::thread::current().id());
    CLAIMED.store(true, Ordering::Release);
}

/// Forget which thread draws.
///
/// For the host's teardown, and for a test that has to put the process
/// back as it found it.
pub fn draws_nowhere() {
    *held(drawing_thread()) = None;
    CLAIMED.store(false, Ordering::Release);
    held(already_said()).clear();
}

/// Is this the thread the window is drawn on?
///
/// `false` when nothing has claimed one, which is every program that
/// uses this crate without a window.
pub fn on_the_drawing_thread() -> bool {
    if !CLAIMED.load(Ordering::Acquire) {
        return false;
    }
    *held(drawing_thread()) == Some(std::thread::current().id())
}

/// Said by every call in this crate that waits for something outside
/// this process.
///
/// `what` names the capability as a page would say it: "a file dialog",
/// "the clipboard", "a script in the shell". It goes into the sentence,
/// so it reads as part of one.
pub fn waiting(what: &str) {
    if !on_the_drawing_thread() {
        return;
    }
    {
        let mut said = held(already_said());
        if said.iter().any(|each| each == what) {
            return;
        }
        said.push(what.to_string());
    }
    let notice = format!(
        "rustx: {what} was asked for on the thread that draws, and it waits. \
         The window cannot draw until it comes back. Move the call into \
         `task(async {{ … }})`, which runs beside the window and writes its \
         answer into a value the next frame draws."
    );
    WARNINGS.fetch_add(1, Ordering::Relaxed);
    eprintln!("{notice}");
    *held(last_said()) = Some(notice);
}

/// How many times this module has warned, in this process.
pub fn warnings() -> u64 {
    WARNINGS.load(Ordering::Relaxed)
}

/// The last warning, word for word.
pub fn last_warning() -> Option<String> {
    held(last_said()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests share one process-wide answer to "which thread
    /// draws?", so they are driven from one test rather than racing
    /// each other over it.
    #[test]
    fn a_waiting_call_is_only_warned_about_on_the_thread_that_draws() {
        draws_nowhere();

        // No window: nothing is said, whatever is called.
        let before = warnings();
        waiting("the clipboard");
        assert_eq!(
            warnings(),
            before,
            "a program with no window was warned about its own clipboard"
        );

        // This thread draws now.
        draws_here();
        assert!(on_the_drawing_thread());
        waiting("the clipboard");
        assert_eq!(warnings(), before + 1, "the window was not warned");
        let said = last_warning().expect("something was said");
        assert!(said.contains("the clipboard"), "got {said}");
        assert!(said.contains("task("), "it must say what to do: {said}");

        // Once per capability, not once per call: a call in a render
        // loop would otherwise write sixty lines a second.
        waiting("the clipboard");
        assert_eq!(
            warnings(),
            before + 1,
            "the same capability was warned about twice"
        );

        // A different capability is a different sentence.
        waiting("a file dialog");
        assert_eq!(warnings(), before + 2);

        // And another thread is not the drawing one, even now.
        let elsewhere = std::thread::spawn(|| {
            let before = warnings();
            waiting("a script in the shell");
            (on_the_drawing_thread(), warnings() == before)
        })
        .join()
        .expect("the thread ran");
        assert_eq!(
            elsewhere,
            (false, true),
            "a worker thread was warned about waiting, which is its job"
        );

        draws_nowhere();
        assert!(!on_the_drawing_thread());
    }
}
