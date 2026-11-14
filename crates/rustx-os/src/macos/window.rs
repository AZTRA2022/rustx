//! Window control, on macOS: `NSWindow`, reached from CEF's `NSView`.
//!
//! # Never compiled for a Mac, never run
//!
//! This file was written on a Windows machine from Apple's `AppKit`
//! documentation. It has never been compiled for an Apple target, never
//! linked, and never seen a window. Its one check is a type-check on
//! Windows against `objc2` built for GNUstep, which says the Rust and
//! the `objc2` calls are well-formed — and nothing about whether a
//! window loses its title bar, fills the screen, or follows the
//! pointer. `cargo check --target aarch64-apple-darwin` first, then
//! every order below tried on a real Mac, before any of it is trusted.
//! The places most likely to be wrong are named where they are.
//!
//! # The handle
//!
//! On macOS, `BrowserHost::window_handle()` is the browser's `NSView`,
//! not a window: the window is `[view window]`. And unlike `IsWindow`
//! on Windows, nothing in `AppKit` can tell a live `NSView` pointer
//! from a dangling one without dereferencing it — this file trusts the
//! engine, which takes the handle fresh from a live browser on every
//! order, to pass a view that still exists.
//!
//! # The main thread
//!
//! Every `NSWindow` call is main-thread only. The engine gives its
//! orders on CEF's UI thread, which on macOS *is* the main thread, so
//! these run directly; from any other thread they are refused by name
//! rather than carried over (see `appkit.rs`) — an order to a window is
//! the engine's to give, from where the engine lives.

use crate::window::{Dressing, Spread};

pub const AVAILABLE: bool = true;

#[allow(unused_imports)]
pub use real::{close, drag, drag_out, dress, frame_of, on_top, pointer, spread, spread_of};

// ─────────────── What is computed without a window ───────────────

/// `NSWindowStyleMaskFullSizeContentView`: the content runs under the
/// title bar.
pub(crate) const FULL_SIZE_CONTENT: usize = 1 << 15;
/// `NSWindowStyleMaskFullScreen`: set by `AppKit` while the window is
/// full screen — the state macOS tracks on its own, unlike Windows.
pub(crate) const FULL_SCREEN: usize = 1 << 14;
/// `NSWindowCollectionBehaviorFullScreenPrimary`: the window may take
/// a space of its own when full screen.
pub(crate) const FULL_SCREEN_PRIMARY: usize = 1 << 7;
/// `NSNormalWindowLevel` and `NSFloatingWindowLevel`. Written as
/// numbers because, in `AppKit`'s headers, they are `CGWindowLevel`
/// constants computed by `CGWindowLevelForKey` — 0 and 3 on every
/// macOS shipped so far.
pub(crate) const NORMAL_LEVEL: isize = 0;
pub(crate) const FLOATING_LEVEL: isize = 3;

/// The style mask for this dressing, from the one the window has.
///
/// # Why the title bar is *hidden* and not *removed*
///
/// The obvious answer — clear `NSWindowStyleMaskTitled`, leaving a
/// borderless window — is the macOS twin of the trap Windows'
/// `WS_THICKFRAME` comment describes, and worse: a borderless
/// `NSWindow` answers `NO` to `canBecomeKeyWindow`, so the page inside
/// stops receiving the keyboard, and it loses its shadow, its rounded
/// corners and full screen besides. Every frameless macOS app that
/// keeps working (Electron's `titleBarStyle: 'hidden'`, Safari's own
/// tab bar) keeps the title bar and makes it invisible instead: the
/// content runs under it (`FullSizeContentView`, this mask), the bar
/// is transparent and untitled, and the three buttons are hidden —
/// those three in `real::dress`.
pub(crate) fn style_for(dressing: Dressing, current: usize) -> usize {
    match dressing {
        Dressing::None => current | FULL_SIZE_CONTENT,
        Dressing::System => current & !FULL_SIZE_CONTENT,
    }
}

/// What the window occupies, from what `AppKit` says of it.
///
/// Full screen first, as on Windows and Linux: a full-screen window
/// may still answer `isZoomed`, and it is full screen that shows.
pub(crate) fn spread_from(style: usize, miniaturized: bool, zoomed: bool) -> Spread {
    if style & FULL_SCREEN != 0 {
        Spread::Fullscreen
    } else if miniaturized {
        Spread::Minimized
    } else if zoomed {
        Spread::Maximized
    } else {
        Spread::Normal
    }
}

/// Where the window's origin goes when the pointer has moved from
/// `grabbed_at` to `now`.
///
/// The pointer is read in Quartz's global coordinates (origin at the
/// top left, y downwards); the window's frame is in Cocoa's (origin at
/// the bottom left, y upwards). A difference does not care where the
/// origin is, only which way y runs — hence the one minus sign.
pub(crate) fn dragged_to(
    origin: (f64, f64),
    grabbed_at: (f64, f64),
    now: (f64, f64),
) -> (f64, f64) {
    (
        origin.0 + (now.0 - grabbed_at.0),
        origin.1 - (now.1 - grabbed_at.1),
    )
}

// ─────────────────────── What talks to AppKit ───────────────────────

#[cfg(target_os = "macos")]
mod real {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};

    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;

    use super::super::appkit::{
        MainOnly, Point, Rect, dispatch_after_f, dispatch_time, main_queue, on_main_thread,
    };
    use super::{
        FLOATING_LEVEL, FULL_SCREEN, FULL_SCREEN_PRIMARY, NORMAL_LEVEL, dragged_to, spread_from,
        style_for,
    };
    use crate::error::OsError;
    use crate::window::{Dressing, Pane, Spread};

    /// `NSWindowCloseButton`, `NSWindowMiniaturizeButton`,
    /// `NSWindowZoomButton`.
    const BUTTONS: [usize; 3] = [0, 1, 2];
    /// `NSWindowTitleVisible` and `NSWindowTitleHidden`.
    const TITLE_VISIBLE: isize = 0;
    const TITLE_HIDDEN: isize = 1;

    unsafe extern "C" {
        fn CGEventCreate(source: *const c_void) -> *mut c_void;
        fn CGEventGetLocation(event: *const c_void) -> Point;
        fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
        fn CFRelease(object: *const c_void);
    }

    /// `kCGEventSourceStateCombinedSessionState`, `kCGMouseButtonLeft`.
    const COMBINED_SESSION: i32 = 0;
    const LEFT_BUTTON: u32 = 0;

    /// The window the engine's view is in.
    fn window(pane: Pane) -> Result<Retained<AnyObject>, OsError> {
        if !pane.exists() {
            return Err(OsError::Platform(
                "there is no window to command yet: the handle is still nothing".to_string(),
            ));
        }
        if !on_main_thread() {
            return Err(OsError::Platform(
                "a macOS window is commanded from the main thread only, which is where the \
                 engine gives its orders — this one came from another thread"
                    .to_string(),
            ));
        }
        let view = pane.0 as *mut AnyObject;
        // SAFETY: the `NSView` CEF handed the engine (see the module
        // doc for why its liveness is trusted), on the main thread.
        let window: Option<Retained<AnyObject>> = unsafe { msg_send![view, window] };
        window.ok_or_else(|| {
            OsError::Platform(format!(
                "the view {:#x} is not in a window — it was closed, or not shown yet",
                pane.0
            ))
        })
    }

    fn style(window: &AnyObject) -> usize {
        // SAFETY: a property read on a live window, on the main thread.
        unsafe { msg_send![window, styleMask] }
    }

    pub fn dress(pane: Pane, dressing: Dressing) -> Result<(), OsError> {
        let window = window(pane)?;
        let bare = dressing == Dressing::None;
        // SAFETY: documented `NSWindow` setters, on a live window, on
        // the main thread. Uncertain, and to be watched on a real Mac:
        // with the bar transparent and the content under it, the top
        // ~28 points may still be taken by the (invisible) title bar for
        // its own drag and double-click, before the page sees them.
        unsafe {
            let _: () = msg_send![&*window, setStyleMask: style_for(dressing, style(&window))];
            let _: () = msg_send![&*window, setTitlebarAppearsTransparent: bare];
            let visibility = if bare { TITLE_HIDDEN } else { TITLE_VISIBLE };
            let _: () = msg_send![&*window, setTitleVisibility: visibility];
            for which in BUTTONS {
                let button: Option<Retained<AnyObject>> =
                    msg_send![&*window, standardWindowButton: which];
                if let Some(button) = button {
                    let _: () = msg_send![&*button, setHidden: bare];
                }
            }
        }
        Ok(())
    }

    fn miniaturized(window: &AnyObject) -> bool {
        // SAFETY: a property read on a live window, on the main thread.
        unsafe { msg_send![window, isMiniaturized] }
    }

    fn zoomed(window: &AnyObject) -> bool {
        // SAFETY: idem.
        unsafe { msg_send![window, isZoomed] }
    }

    /// `toggleFullScreen:`, with the window first allowed a space of
    /// its own — a window without `FullScreenPrimary` in its collection
    /// behaviour ignores the toggle, and CEF does not set it.
    fn toggle_full_screen(window: &AnyObject) {
        // SAFETY: documented `NSWindow` calls, on the main thread.
        unsafe {
            let behaviour: usize = msg_send![window, collectionBehavior];
            let _: () = msg_send![window, setCollectionBehavior: behaviour | FULL_SCREEN_PRIMARY];
            let _: () = msg_send![window, toggleFullScreen: None::<&AnyObject>];
        }
    }

    pub fn spread(pane: Pane, how: Spread) -> Result<(), OsError> {
        let window = window(pane)?;
        let full = style(&window) & FULL_SCREEN != 0;
        // Nothing to remember, unlike Windows: full screen is a state
        // `AppKit` keeps, and it knows which frame to come back to.
        if how == Spread::Fullscreen {
            if !full {
                toggle_full_screen(&window);
            }
            return Ok(());
        }
        if full {
            // Leaving full screen is an animation, and `AppKit` ignores
            // `miniaturize:` and `zoom:` while it runs — a full-screen
            // window has neither button. Coming back is all an order
            // can do here; doing the rest is a second order, given once
            // the window is back, and the refusal says so rather than
            // pretending the first one did both.
            toggle_full_screen(&window);
            if how == Spread::Normal {
                return Ok(());
            }
            return Err(OsError::Platform(
                "the window is leaving full screen first; macOS does not minimize or zoom a \
                 window during that animation — give the order again once it is back"
                    .to_string(),
            ));
        }
        // SAFETY: documented `NSWindow` actions, on the main thread; a
        // `nil` sender is what a programmatic call passes.
        unsafe {
            match how {
                Spread::Minimized => {
                    let _: () = msg_send![&*window, miniaturize: None::<&AnyObject>];
                }
                Spread::Maximized => {
                    if miniaturized(&window) {
                        let _: () = msg_send![&*window, deminiaturize: None::<&AnyObject>];
                    }
                    // `zoom:` toggles, so it is only sent to a window that
                    // is not zoomed yet: "maximize" twice must not undo
                    // itself.
                    if !zoomed(&window) {
                        let _: () = msg_send![&*window, zoom: None::<&AnyObject>];
                    }
                }
                Spread::Normal => {
                    if miniaturized(&window) {
                        let _: () = msg_send![&*window, deminiaturize: None::<&AnyObject>];
                    } else if zoomed(&window) {
                        // Zooming a zoomed window gives it back the frame
                        // the user had — `AppKit` remembers it, as
                        // `ShowWindow` does on Windows.
                        let _: () = msg_send![&*window, zoom: None::<&AnyObject>];
                    }
                }
                Spread::Fullscreen => {}
            }
        }
        Ok(())
    }

    pub fn spread_of(pane: Pane) -> Result<Spread, OsError> {
        let window = window(pane)?;
        Ok(spread_from(
            style(&window),
            miniaturized(&window),
            zoomed(&window),
        ))
    }

    pub fn on_top(pane: Pane, above: bool) -> Result<(), OsError> {
        let window = window(pane)?;
        let level = if above { FLOATING_LEVEL } else { NORMAL_LEVEL };
        // SAFETY: a documented setter, on the main thread.
        unsafe {
            let _: () = msg_send![&*window, setLevel: level];
        }
        Ok(())
    }

    /// The pointer, in Quartz's global coordinates, and whether the
    /// left button is down.
    fn pointer_state() -> ((f64, f64), bool) {
        // SAFETY: Quartz Event Services calls documented to work from
        // any thread; the event made here is released here.
        unsafe {
            let event = CGEventCreate(std::ptr::null());
            let at = if event.is_null() {
                Point { x: 0.0, y: 0.0 }
            } else {
                let at = CGEventGetLocation(event);
                CFRelease(event);
                at
            };
            let down = CGEventSourceButtonState(COMBINED_SESSION, LEFT_BUTTON);
            ((at.x, at.y), down)
        }
    }

    /// One drag at a time: two loops moving the same window would
    /// fight over it.
    static DRAGGING: AtomicBool = AtomicBool::new(false);

    /// What a drag in progress carries from one tick to the next.
    struct Dragging {
        window: MainOnly<Retained<AnyObject>>,
        origin: (f64, f64),
        grabbed_at: (f64, f64),
    }

    /// Start moving the window with the pointer, until the button is
    /// released.
    ///
    /// # Why not `performWindowDragWithEvent:`
    ///
    /// It is `AppKit`'s own way, and it needs *the mouse-down event that
    /// started the drag*. That event went to Chromium, which sent it to
    /// the page, which sent this order back over IPC: by the time it
    /// arrives, `[NSApp currentEvent]` is whatever `AppKit` is handling
    /// now — often not a mouse-down at all. Handing it a wrong event is
    /// undocumented behaviour, and this file would be guessing.
    ///
    /// What does not depend on that event: reading where the pointer
    /// is and whether the button is held (Quartz answers both, at any
    /// time) and moving the window by the difference, once a frame on
    /// the main queue, until the button comes up. What it costs: none
    /// of the edge-snapping a real title-bar drag gets.
    pub fn drag(pane: Pane) -> Result<(), OsError> {
        let window = window(pane)?;
        let (grabbed_at, down) = pointer_state();
        if !down {
            // Already released: it was a click, not a drag.
            return Ok(());
        }
        if DRAGGING.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        // SAFETY: a property read on a live window, on the main thread;
        // `frame` returns a struct, which `msg_send!` knows how to ask
        // for on both Mac architectures.
        let frame: Rect = unsafe { msg_send![&*window, frame] };
        let state = Box::new(Dragging {
            window: MainOnly(window),
            origin: (frame.origin.x, frame.origin.y),
            grabbed_at,
        });
        schedule(Box::into_raw(state).cast());
        Ok(())
    }

    /// The next tick, in a sixtieth of a second, on the main queue.
    fn schedule(state: *mut c_void) {
        // SAFETY: `DISPATCH_TIME_NOW` plus 16 ms, and the main queue;
        // the state is owned by `tick` from here on.
        unsafe {
            let when = dispatch_time(0, 16_000_000);
            dispatch_after_f(when, main_queue(), state, tick);
        }
    }

    extern "C" fn tick(context: *mut c_void) {
        // SAFETY: the `Box<Dragging>` `drag` leaked, handed back once per
        // tick; it is either rescheduled or dropped below, never both.
        let state = unsafe { Box::from_raw(context.cast::<Dragging>()) };
        let (now, down) = pointer_state();
        if !down {
            DRAGGING.store(false, Ordering::SeqCst);
            return;
        }
        let (x, y) = dragged_to(state.origin, state.grabbed_at, now);
        // SAFETY: a documented setter on a window this state retains, on
        // the main thread (the main queue runs there).
        unsafe {
            let _: () = msg_send![&*state.window.0, setFrameOrigin: Point { x, y }];
        }
        schedule(Box::into_raw(state).cast());
    }

    /// Where the window is on screen, in this crate's coordinates
    /// (origin at the top left, y downwards) — what `capture.rs` needs
    /// to aim `screencapture` at it.
    ///
    /// `NSWindow`'s own frame is in Cocoa's bottom-left-origin space, so
    /// it is flipped against the primary screen the same way
    /// `screen.rs` flips every rectangle it reports, and by the same
    /// function, so the two cannot drift apart.
    pub fn frame_of(pane: Pane) -> Result<crate::screen::Area, OsError> {
        let window = window(pane)?;
        // SAFETY: a property read on a live window, on the main thread.
        let frame: Rect = unsafe { msg_send![&*window, frame] };
        let primary = crate::screen::primary()?;
        Ok(super::screen::flipped(
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
            f64::from(primary.bounds.height),
        ))
    }

    /// Refused: `beginDraggingSessionWithItems:event:source:` takes the
    /// `NSEvent` of the mouse-down that began the drag, and a dragging
    /// source object that answers for the session's whole life. The
    /// event went to Chromium and is gone by the time this order
    /// arrives — the same wall `drag` describes, and here there is
    /// nothing to work around it with. See `window::drag_out`.
    pub fn drag_out(_pane: Pane, _paths: &[std::path::PathBuf]) -> Result<(), OsError> {
        Err(OsError::Unsupported(
            "dragging files out of the window on macOS              (beginDraggingSessionWithItems:event:source: needs the original NSEvent, which              belongs to Chromium)",
        ))
    }

    pub fn close(pane: Pane) -> Result<(), OsError> {
        let window = window(pane)?;
        // `performClose:` and not `close`: the first is the close
        // button, asked through the window's delegate — which is CEF's,
        // and which is how Chromium learns to tear the browser down
        // properly. `close` would pull the window from under it. The
        // button is hidden on a frameless window, not removed, so the
        // window still has one to "press".
        // SAFETY: a documented action, on the main thread.
        unsafe {
            let _: () = msg_send![&*window, performClose: None::<&AnyObject>];
        }
        Ok(())
    }

    /// Where the pointer is, in Quartz's global coordinates: origin at
    /// the top left of the main display, y downwards — the same
    /// convention as Windows' screen coordinates, which is why Quartz
    /// is asked and not `[NSEvent mouseLocation]` (bottom-left origin).
    pub fn pointer() -> Result<(i32, i32), OsError> {
        let ((x, y), _) = pointer_state();
        Ok((x.round() as i32, y.round() as i32))
    }
}

/// The same names, so that this file is checked on a machine that is
/// not a Mac — see `lib.rs`. Never the platform chosen anywhere else.
#[cfg(not(target_os = "macos"))]
mod real {
    use crate::error::OsError;
    use crate::window::{Dressing, Pane, Spread};

    const ELSEWHERE: &str = "macos/window.rs is compiled for its own sake on a machine that is \
                             not a Mac, and is never the platform module chosen there";

    pub fn dress(_pane: Pane, _dressing: Dressing) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn spread(_pane: Pane, _how: Spread) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn spread_of(_pane: Pane) -> Result<Spread, OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn on_top(_pane: Pane, _above: bool) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn drag(_pane: Pane) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn drag_out(_pane: Pane, _paths: &[std::path::PathBuf]) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn close(_pane: Pane) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn frame_of(_pane: Pane) -> Result<crate::screen::Area, OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn pointer() -> Result<(i32, i32), OsError> {
        unreachable!("{ELSEWHERE}")
    }
}

#[cfg(test)]
mod tests {
    // Run on every machine: this is the part of the file that is plain
    // integers and floats, and the only part a machine without a Mac
    // can run at all.
    use super::*;

    const TITLED_AND_RESIZABLE: usize = 0b1111;

    #[test]
    fn frameless_keeps_the_title_bar_and_runs_the_content_under_it() {
        let bare = style_for(Dressing::None, TITLED_AND_RESIZABLE);
        assert_eq!(
            bare & 1,
            1,
            "NSWindowStyleMaskTitled is kept: the keyboard needs it"
        );
        assert_eq!(bare & 0b1000, 0b1000, "resizable is kept");
        assert_ne!(bare & FULL_SIZE_CONTENT, 0);
    }

    #[test]
    fn framed_gives_the_title_bar_its_own_room_back() {
        let bare = style_for(Dressing::None, TITLED_AND_RESIZABLE);
        assert_eq!(style_for(Dressing::System, bare), TITLED_AND_RESIZABLE);
        // Twice is once.
        assert_eq!(style_for(Dressing::None, bare), bare);
    }

    #[test]
    fn full_screen_is_read_before_anything_else() {
        assert_eq!(spread_from(FULL_SCREEN, false, true), Spread::Fullscreen);
        assert_eq!(spread_from(0, true, true), Spread::Minimized);
        assert_eq!(spread_from(0, false, true), Spread::Maximized);
        assert_eq!(spread_from(0, false, false), Spread::Normal);
    }

    #[test]
    fn a_dragged_window_moves_down_when_the_pointer_does() {
        // Quartz y grows downwards, Cocoa y upwards: a pointer 30 points
        // lower puts the origin 30 points lower, which is a *smaller* y.
        assert_eq!(
            dragged_to((100.0, 500.0), (150.0, 40.0), (170.0, 70.0)),
            (120.0, 470.0)
        );
        assert_eq!(
            dragged_to((100.0, 500.0), (150.0, 40.0), (150.0, 40.0)),
            (100.0, 500.0)
        );
    }

    #[test]
    fn the_levels_are_the_ones_appkit_documents() {
        assert_eq!(NORMAL_LEVEL, 0);
        assert_eq!(FLOATING_LEVEL, 3);
    }
}
