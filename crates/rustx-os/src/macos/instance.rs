//! A single instance, on macOS: a Unix domain socket, as on Linux.
//!
//! macOS is a Unix underneath, and `std::os::unix::net` is the same
//! API there as on Linux — a socket answering "is one running?" by
//! whether connecting to it succeeds, and "what do I hand it?" by
//! what is written down that same connection before it closes. The
//! only platform difference is where the file goes: Linux has
//! `$XDG_RUNTIME_DIR`, a directory the session owns and clears at
//! logout; macOS's equivalent is `$TMPDIR`, which launchd sets to a
//! per-user directory it cleans on its own schedule, and which every
//! process already has set.
//!
//! See `linux/instance.rs` for the full reasoning — the stale-socket
//! handling, the wake-on-drop, the race on first start — none of
//! which changes here.
//!
//! # `#[cfg(unix)]`, in a file this crate also compiles on Windows
//!
//! Same reason as the Linux file: `lib.rs` compiles this one too
//! under `cargo test`, on whatever machine the tests run on, and
//! `std::os::unix` does not exist on Windows. The real socket code
//! sits behind `#[cfg(unix)]`; the stand-in beside it is never the
//! platform actually chosen outside macOS.
//!
//! # Written, not run
//!
//! This file was written on a Windows machine and has never run on
//! macOS, and there is no Apple target here to even type-check it
//! against — unlike the Linux file, this one has only been read over
//! by hand, changed from it by the directory it reads alone.

pub const AVAILABLE: bool = true;

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

    fn path(name: &str) -> PathBuf {
        let base = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join(format!("rustx-instance-{name}.sock"))
    }

    fn handed() -> &'static Mutex<Vec<String>> {
        static HANDED: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
        HANDED.get_or_init(|| Mutex::new(Vec::new()))
    }

    pub fn handed_over() -> Vec<String> {
        let mut held = handed().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *held)
    }

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
            let _ = UnixStream::connect(&self.path);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
            let _ = std::fs::remove_file(&self.path);
        }
    }

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
        let _ = stream.shutdown(std::net::Shutdown::Write);
        Ok(())
    }

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

#[cfg(not(unix))]
mod real {
    use crate::error::OsError;

    pub struct Only;

    impl Only {
        pub fn one(_name: &str) -> Result<Self, OsError> {
            unreachable!(
                "macos/instance.rs is compiled for its own sake on a non-macOS machine, \
                 and is never the platform module chosen there"
            )
        }
    }

    pub fn hand_over(_name: &str, _arguments: &[String]) -> Result<(), OsError> {
        unreachable!(
            "macos/instance.rs is compiled for its own sake on a non-macOS machine, \
             and is never the platform module chosen there"
        )
    }

    pub fn handed_over() -> Vec<String> {
        unreachable!(
            "macos/instance.rs is compiled for its own sake on a non-macOS machine, \
             and is never the platform module chosen there"
        )
    }
}
