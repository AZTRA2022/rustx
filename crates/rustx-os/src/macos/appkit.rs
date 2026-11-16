//! What the three `AppKit` files share: the main thread, a string, a
//! point, and the frameworks to link.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from Apple's documentation and
//! `objc2`'s own source. It has never been compiled for an Apple
//! target, never linked against `AppKit`, and never run. The only
//! check it has had is a type-check on Windows, against `objc2` built
//! for GNUstep — which proves the Rust is well-formed and the `objc2`
//! calls have the right shapes, and proves nothing about whether macOS
//! answers the way this file expects. Before any of it is trusted:
//! `cargo check --target aarch64-apple-darwin` at the very least, and
//! then running `tray_icon` and `a_global_shortcut` from `examples/`
//! on a real Mac.
//!
//! # The main thread
//!
//! `AppKit` and the Carbon Event Manager both belong to the main
//! thread: windows, status items and hotkeys are created there, and
//! what they report arrives there, through the main run loop. Windows
//! lets each of this crate's capabilities run a thread of its own with
//! its own loop; macOS does not, so every call here is carried to the
//! main thread instead — and only works while that thread is running
//! its event loop, which a CEF application's browser process always
//! is (`CefRunMessageLoop` is `[NSApp run]`). A program whose main
//! thread is asleep gets a refusal after a few seconds instead of a
//! hang.

#[cfg(target_os = "macos")]
pub(crate) use real::*;

#[cfg(target_os = "macos")]
mod real {
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::sync::mpsc;
    use std::time::Duration;

    use objc2::encode::{Encode, Encoding};
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};

    use crate::error::OsError;

    // The frameworks, said once. `AppKit` is already loaded in a CEF
    // application; a plain program (the examples) needs it linked.
    #[cfg_attr(target_os = "macos", link(name = "AppKit", kind = "framework"))]
    unsafe extern "C" {}
    #[cfg_attr(target_os = "macos", link(name = "Carbon", kind = "framework"))]
    unsafe extern "C" {}
    #[cfg_attr(target_os = "macos", link(name = "CoreGraphics", kind = "framework"))]
    unsafe extern "C" {}

    /// What `dispatch_get_main_queue()` returns: it is a macro in C,
    /// over this symbol.
    #[repr(C)]
    pub(crate) struct DispatchQueue {
        _opaque: [u8; 0],
    }

    unsafe extern "C" {
        static _dispatch_main_q: DispatchQueue;
        fn dispatch_async_f(
            queue: *const DispatchQueue,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
        pub(crate) fn dispatch_after_f(
            when: u64,
            queue: *const DispatchQueue,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
        pub(crate) fn dispatch_time(when: u64, delta: i64) -> u64;
        fn pthread_main_np() -> c_int;
    }

    pub(crate) fn main_queue() -> *const DispatchQueue {
        // Taking an extern static's address reads nothing, and is safe.
        &raw const _dispatch_main_q
    }

    /// Whether this is the main thread.
    pub(crate) fn on_main_thread() -> bool {
        // SAFETY: no arguments, no state.
        unsafe { pthread_main_np() != 0 }
    }

    /// How long a caller waits for the main thread before saying it is
    /// not listening.
    const PATIENCE: Duration = Duration::from_secs(5);

    /// Run `work` on the main thread and hand back what it returned.
    ///
    /// Directly when this already is the main thread. Otherwise queued
    /// on the main dispatch queue, and waited for — with a limit,
    /// because a main thread that is not running its loop never takes
    /// the work, and a call that hangs forever is worse than one that
    /// says why. `async` and a channel rather than `dispatch_sync`, for
    /// exactly that limit: what was queued owns everything it touches,
    /// so it is still sound if it runs after the caller gave up.
    pub(crate) fn on_main<R, F>(what: &'static str, work: F) -> Result<R, OsError>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        if on_main_thread() {
            return Ok(objc2::rc::autoreleasepool(|_| work()));
        }
        let (say, heard) = mpsc::channel::<R>();
        let job: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(move || {
            let answer = objc2::rc::autoreleasepool(|_| work());
            let _ = say.send(answer);
        }));
        extern "C" fn run(context: *mut c_void) {
            // SAFETY: the pointer `on_main` made from a `Box` just below,
            // run exactly once by libdispatch.
            let job = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce() + Send>>()) };
            // A panic must not cross into libdispatch.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(*job));
        }
        // SAFETY: the main queue, a context owned by the callee from here
        // on, and a function that frees it.
        unsafe { dispatch_async_f(main_queue(), Box::into_raw(job).cast(), run) };
        heard.recv_timeout(PATIENCE).map_err(|_| {
            OsError::Platform(format!(
                "{what} needs the main thread, and the main thread did not take the work \
                 within {} seconds — on macOS it has to be running its event loop \
                 (`[NSApp run]`, which a CEF application's own loop is)",
                PATIENCE.as_secs()
            ))
        })
    }

    /// An `NSString`, from Rust text.
    ///
    /// A NUL inside the text would end the C string early; it becomes a
    /// space instead, since no label or path means one.
    pub(crate) fn ns_string(text: &str) -> Option<Retained<AnyObject>> {
        let text = CString::new(text.replace('\0', " ")).ok()?;
        let pointer: *const c_char = text.as_ptr();
        // SAFETY: a NUL-terminated UTF-8 string that outlives the call;
        // the result is autoreleased and `Retained` retains it.
        unsafe { msg_send![class!(NSString), stringWithUTF8String: pointer] }
    }

    /// `NSApplication`'s shared instance, created if this program never
    /// asked for it: a status item or a hotkey in a plain program needs
    /// the application object to exist, and asking for it is how it is
    /// made.
    pub(crate) fn application() -> Option<Retained<AnyObject>> {
        // SAFETY: a class method with no argument, on the main thread
        // (every caller is inside `on_main`).
        unsafe { msg_send![class!(NSApplication), sharedApplication] }
    }

    /// `NSPoint` / `CGPoint`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) struct Point {
        pub x: f64,
        pub y: f64,
    }

    // SAFETY: two `CGFloat`s, which are `f64` on every 64-bit Mac, in
    // the order `CGPoint` declares them.
    unsafe impl Encode for Point {
        const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
    }

    /// `NSSize` / `CGSize`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) struct Size {
        pub width: f64,
        pub height: f64,
    }

    // SAFETY: as `Point`.
    unsafe impl Encode for Size {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
    }

    /// `NSRect` / `CGRect`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(crate) struct Rect {
        pub origin: Point,
        pub size: Size,
    }

    // SAFETY: a `CGPoint` then a `CGSize`, as `CGRect` declares them.
    unsafe impl Encode for Rect {
        const ENCODING: Encoding = Encoding::Struct("CGRect", &[Point::ENCODING, Size::ENCODING]);
    }

    /// A value only ever touched on the main thread, carried across
    /// threads to get there.
    ///
    /// `AppKit` objects are not `Send`, and rightly. They still have to be
    /// *held* by a Rust value that lives on whichever thread made the
    /// call; this is that holder, and every use of what it holds is
    /// inside `on_main`.
    pub(crate) struct MainOnly<T>(pub T);

    // SAFETY: see above — the inside is only used on the main thread.
    unsafe impl<T> Send for MainOnly<T> {}
    // SAFETY: idem.
    unsafe impl<T> Sync for MainOnly<T> {}
}
