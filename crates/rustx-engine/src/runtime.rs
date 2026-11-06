//! The reactor a `Future` needs, on threads that are not the render's.
//!
//! A component is asked for the screen as it is now, so it cannot wait.
//! What waits is a task, and a task that is a `Future` needs somebody to
//! poll it. That somebody is tokio, because tokio is the reactor the
//! ecosystem's libraries are written against — `reqwest`, `sqlx`, a
//! timer — and an executor written by hand here would poll futures
//! without ever being the thing those libraries register their I/O
//! with.
//!
//! Three rules hold it in place:
//!
//! - **Nothing starts until something asks.** An application with no
//!   `async` in it pays for no thread. The runtime is built on the first
//!   call to [`handle`], which is the first `task(async { … })`.
//! - **Never the render thread.** The runtime owns its own workers. No
//!   `block_on` is called anywhere in the engine: a window that waits is
//!   the defect this module exists to avoid.
//! - **State is not reachable from here.** A future writes through a
//!   handle that posts on the frame queue, and the frame applies it. The
//!   runtime never touches a cell.

use std::sync::{Mutex, OnceLock};

use tokio::runtime::{Builder, Handle, Runtime};

/// What the engine says when the runtime cannot be built.
///
/// Said rather than swallowed: an application whose futures silently
/// never run is worse than one that stops with a reason.
const CANNOT_BUILD: &str = "rustx: the async runtime could not be started";

fn held() -> &'static Mutex<Option<Runtime>> {
    static HELD: OnceLock<Mutex<Option<Runtime>>> = OnceLock::new();
    HELD.get_or_init(|| Mutex::new(None))
}

/// Is there a runtime in this process?
///
/// False until the first future asks for one, which is what makes an
/// application that writes no `async` pay nothing for it.
pub fn started() -> bool {
    locked().is_some()
}

/// The runtime's handle, starting it if this is the first ask.
///
/// Cheap after the first call: what is returned is a clone of a handle,
/// not a runtime.
pub fn handle() -> Handle {
    let mut held = locked();
    if held.is_none() {
        *held = Some(build());
    }
    held.as_ref()
        .map(|runtime| runtime.handle().clone())
        .unwrap_or_else(|| unreachable!("{CANNOT_BUILD}: just built"))
}

/// Let go of the runtime, abandoning whatever it still had to do.
///
/// Called when a window is going. `shutdown_background` rather than a
/// plain drop: dropping a multi-threaded runtime waits for its workers,
/// and the thread that calls this is the one drawing the last frame.
pub fn stop() {
    let taken = locked().take();
    if let Some(runtime) = taken {
        runtime.shutdown_background();
    }
}

fn locked() -> std::sync::MutexGuard<'static, Option<Runtime>> {
    held()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn build() -> Runtime {
    Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        // Named, because a stack in a debugger that says `rustx-async`
        // is the difference between reading a trace and guessing at one.
        .thread_name("rustx-async")
        .build()
        .unwrap_or_else(|error| panic!("{CANNOT_BUILD}: {error}"))
}
