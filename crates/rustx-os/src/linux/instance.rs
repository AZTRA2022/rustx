//! A single instance, on Linux: a Unix domain socket.
//!
//! One socket answers both questions a second launch asks. "Is one
//! already running?" — connecting as a client and succeeding means
//! yes. "What do I hand it?" — writing to that same connection and
//! closing it is the handoff, read on the other end by whichever
//! thread is `accept()`ing. One file does the job the Windows side
//! needs a mutex *and* a window for.
//!
//! The socket lives in `$XDG_RUNTIME_DIR` — a directory the session
//! owns and that is cleared at logout, so an instance left behind by a
//! crash does not survive to the next login even if nothing here
//! cleaned it up. A machine with no `XDG_RUNTIME_DIR` (some minimal
//! setups) falls back to the system temp directory.
//!
//! # A stale file left by a crash
//!
//! A socket file survives a process that dies without closing it, and
//! binding to a path that already exists fails. Connecting first
//! tells the two cases apart: success means another instance is
//! really listening, and failure — refused or simply not found — means
//! the file, if there is one, is a corpse to remove before binding.
//!
//! # `#[cfg(unix)]`, in a file named for Linux
//!
//! `lib.rs` compiles this file unconditionally under `cargo test`, on
//! whatever machine the tests run on — including this one, which is
//! Windows — so that a change to the API above breaks here rather
//! than on somebody else's Linux three weeks later. `std::os::unix`
//! does not exist on Windows at all, so the real socket code sits
//! behind `#[cfg(unix)]`; the fallback beside it exists only to give
//! the same names something to type-check against, and is never the
//! platform actually chosen outside Linux (`lib.rs` picks this
//! directory's `mod.rs` only under `cfg(target_os = "linux")`, which
//! implies `cfg(unix)`).
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! Linux. `cargo check --target x86_64-unknown-linux-gnu` is what
//! verified it compiles; nothing here has been seen to actually hand
//! arguments from one process to another.

pub const AVAILABLE: bool = true;

// Unused on a machine where this is only compiled to be checked, not
// chosen as the platform: `lib.rs`'s `platform` module picks this
// directory under `cfg(target_os = "linux")` alone.
#[allow(unused_imports)]
pub use real::{Only, hand_over, handed_over};

#[cfg(unix)]
mod real {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use crate::error::OsError;
    use crate::instance::BETWEEN;

    /// Where the socket for `name` lives.
    fn path(name: &str) -> PathBuf {
        let base = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join(format!("rustx-instance-{name}.sock"))
    }

    /// What a second instance has handed over, by application: the
    /// page reads from one thread, the socket's accept loop writes
    /// from another.
    fn handed() -> &'static Mutex<Vec<String>> {
        static HANDED: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
        HANDED.get_or_init(|| Mutex::new(Vec::new()))
    }

    pub fn handed_over() -> Vec<String> {
        let mut held = handed().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *held)
    }

    /// The name held: the listening thread, and the file to remove
    /// when it stops.
    pub struct Only {
        path: PathBuf,
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Only {
        pub fn one(name: &str) -> Result<Self, OsError> {
            let path = path(name);
            if UnixStream::connect(&path).is_ok() {
                return Err(OsError::Busy(format!(
                    "{name} — another instance is already running. \
                     `instance::hand_over` passes this one's arguments to it."
                )));
            }
            // What is at `path`, if anything, answered nobody: a
            // corpse from a crash, not a running instance.
            let _ = std::fs::remove_file(&path);
            let listener = bind(&path)?;

            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = stop.clone();
            let thread = std::thread::Builder::new()
                .name("rustx-instance".to_string())
                .spawn(move || live(listener, thread_stop))
                .map_err(|error| {
                    OsError::Platform(format!("the instance thread did not start: {error}"))
                })?;

            Ok(Self {
                path,
                stop,
                thread: Some(thread),
            })
        }
    }

    /// Bind `path`, with one retry for the race where two instances
    /// start at once: both find the socket absent, both remove it,
    /// and only one wins the bind. The loser asks once more whether
    /// the winner is listening yet, rather than reporting a platform
    /// error for a race that is not one.
    fn bind(path: &std::path::Path) -> Result<UnixListener, OsError> {
        match UnixListener::bind(path) {
            Ok(listener) => Ok(listener),
            Err(first) => {
                if UnixStream::connect(path).is_ok() {
                    return Err(OsError::Busy(
                        "another instance won the race to start first".to_string(),
                    ));
                }
                Err(OsError::Platform(format!(
                    "{} could not be bound: {first}",
                    path.display()
                )))
            }
        }
    }

    impl Drop for Only {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            // `accept()` blocks, and the only way to wake it from
            // outside is to give it a connection to accept — which is
            // also why the flag above is set *before* this, so the
            // accept loop checks it and leaves without trying to read
            // a handoff out of our own wake-up call.
            let _ = UnixStream::connect(&self.path);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

    /// Pass arguments to the instance that is running.
    pub fn hand_over(name: &str, arguments: &[String]) -> Result<(), OsError> {
        let path = path(name);
        let mut stream = UnixStream::connect(&path).map_err(|_| {
            OsError::NoSuchDevice(format!(
                "{name} — no instance is running, there is nobody to hand this to"
            ))
        })?;
        let joined: String = arguments.join(&BETWEEN.to_string());
        stream.write_all(joined.as_bytes()).map_err(|error| {
            OsError::Platform(format!("could not write to the running instance: {error}"))
        })?;
        // The listener reads until the peer closes its writing half;
        // a stream that is simply dropped does that too, but naming
        // the shutdown says plainly that this is the end of the
        // message and not a connection that happened to end.
        let _ = stream.shutdown(std::net::Shutdown::Write);
        Ok(())
    }

    /// The life of the listening thread: accept, read one handoff to
    /// the end, repeat, until told to stop.
    fn live(listener: UnixListener, stop: Arc<AtomicBool>) {
        loop {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let mut text = String::new();
            if stream.read_to_string(&mut text).is_ok() && !text.is_empty() {
                let arguments: Vec<String> = text
                    .split(BETWEEN)
                    .filter(|piece| !piece.is_empty())
                    .map(str::to_string)
                    .collect();
                let mut held =
                    handed().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                held.extend(arguments);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_name_nobody_holds_is_free() {
            // Two different names never collide on the same machine; a
            // random one is as good as a fresh one.
            let name = format!("rustx-test-free-{}", std::process::id());
            assert!(UnixStream::connect(path(&name)).is_err());
        }

        #[test]
        fn handing_over_with_nobody_listening_names_the_application() {
            let name = format!("rustx-test-nobody-{}", std::process::id());
            let error = hand_over(&name, &["one".to_string()]).expect_err("nobody is listening");
            assert!(matches!(error, OsError::NoSuchDevice(_)), "got {error:?}");
        }
    }
}

/// The names this module exports, given something to type-check
/// against on a machine where `std::os::unix` does not exist. Never
/// the platform actually in effect outside Linux — see the module
/// doc.
#[cfg(not(unix))]
mod real {
    use crate::error::OsError;

    pub struct Only;

    impl Only {
        pub fn one(_name: &str) -> Result<Self, OsError> {
            unreachable!(
                "linux/instance.rs is compiled for its own sake on a non-Linux machine, \
                 and is never the platform module chosen there"
            )
        }
    }

    pub fn hand_over(_name: &str, _arguments: &[String]) -> Result<(), OsError> {
        unreachable!(
            "linux/instance.rs is compiled for its own sake on a non-Linux machine, \
             and is never the platform module chosen there"
        )
    }

    pub fn handed_over() -> Vec<String> {
        unreachable!(
            "linux/instance.rs is compiled for its own sake on a non-Linux machine, \
             and is never the platform module chosen there"
        )
    }
}
