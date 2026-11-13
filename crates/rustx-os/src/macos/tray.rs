//! The system tray icon, on macOS: an `NSStatusItem` in the menu bar.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from Apple's `AppKit` documentation.
//! It has never been compiled for an Apple target, never linked, and
//! no icon has ever appeared in a menu bar from it. Its one check is a
//! type-check on Windows against `objc2` built for GNUstep — the Rust
//! and the `objc2` calls are well-formed, and that is all that is
//! known. `cargo check --target aarch64-apple-darwin`, then `cargo run
//! -p rustx-os --example tray_icon` on a real Mac, before any of it is
//! trusted.
//!
//! # What differs from Windows, on purpose
//!
//! * **A click opens the menu.** On a Mac, every menu bar item with a
//!   menu opens it on any click; there is no "left click does one thing,
//!   right click shows the menu". So [`crate::tray::CLICK`] is reported
//!   only by an icon that has no menu — once `menu` is called, clicks
//!   are the menu's.
//! * **No thread of its own.** Windows gives the icon a thread and a
//!   hidden window; `AppKit` only lets the main thread touch a status
//!   item, and clicks arrive through the main run loop. Everything here
//!   is carried to the main thread (`appkit::on_main`), and clicks are
//!   only heard while that thread runs its loop — which a CEF
//!   application's browser process does, and which the `tray_icon`
//!   example (whose main thread sleeps between reads) does not.
//! * **Which process.** A status item needs the process's connection to
//!   the window server. Chromium is documented to cut that connection
//!   in its renderer processes on macOS, so an icon asked for from page
//!   code that runs in a renderer is likely to fail there, where on
//!   Windows it works from anywhere. Unverified either way; the browser
//!   process is where an icon belongs on a Mac.
//! * **Notifications** do not go through the icon: macOS has no
//!   balloon. `notify` is `osascript`'s `display notification`, the
//!   same as `notify::show`.

pub const AVAILABLE: bool = true;

#[allow(unused_imports)]
pub use real::{Icon, shown};

/// `NSVariableStatusItemLength`: as wide as the icon.
pub(crate) const VARIABLE_LENGTH: f64 = -1.0;
/// The menu bar is 22 points high and its icons are drawn at 18: an
/// image left at its own size (a 256-pixel `.ico` frame) would fill the
/// bar or overflow it.
pub(crate) const ICON_POINTS: f64 = 18.0;
/// `NSControlStateValueOn` and `NSControlStateValueOff`.
pub(crate) fn check_state(checked: Option<bool>) -> isize {
    match checked {
        Some(true) => 1,
        _ => 0,
    }
}

// ─────────────────────── What talks to AppKit ───────────────────────

#[cfg(target_os = "macos")]
mod real {
    use std::path::Path;
    use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, NSObject, Sel};
    use objc2::{ClassType, class, msg_send, sel};

    use super::super::appkit::{MainOnly, Size, application, ns_string, on_main};
    use super::{ICON_POINTS, VARIABLE_LENGTH, check_state};
    use crate::error::OsError;
    use crate::notify::What;
    use crate::tray::{CLICK, Chosen, Item};

    /// How many icons this process has in the bar and has not removed.
    static SHOWN: AtomicUsize = AtomicUsize::new(0);

    pub fn shown() -> usize {
        SHOWN.load(Ordering::Acquire)
    }

    /// What each tag means: the queue a click goes to, and what it says.
    ///
    /// A tag rather than an object per line: `NSMenuItem` carries an
    /// integer tag for exactly this, and one shared target reading it
    /// is one Objective-C class instead of one per icon.
    fn tags() -> &'static Mutex<Vec<(isize, Chosen, String)>> {
        static TAGS: OnceLock<Mutex<Vec<(isize, Chosen, String)>>> = OnceLock::new();
        TAGS.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn lock<T>(lock: &'static Mutex<Vec<T>>) -> std::sync::MutexGuard<'static, Vec<T>> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn next_tag() -> isize {
        static NEXT: AtomicIsize = AtomicIsize::new(1);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    /// What every menu line and every icon button calls: `chosen:`.
    extern "C" fn chosen(_this: *mut AnyObject, _command: Sel, sender: *mut AnyObject) {
        if sender.is_null() {
            return;
        }
        // SAFETY: `sender` is the `NSMenuItem` or `NSStatusBarButton`
        // that was clicked; both answer `tag`.
        let tag: isize = unsafe { msg_send![sender, tag] };
        if let Some((_, chosen, id)) = lock(tags()).iter().find(|(each, _, _)| *each == tag) {
            chosen.push(id.clone());
        }
    }

    /// The one object every click is sent to, made once on the main
    /// thread and kept for the life of the program — `NSMenuItem` and
    /// `NSControl` do not retain their target.
    fn target() -> Result<&'static MainOnly<Retained<AnyObject>>, OsError> {
        static TARGET: OnceLock<Result<MainOnly<Retained<AnyObject>>, OsError>> = OnceLock::new();
        TARGET
            .get_or_init(|| {
                let class = target_class()?;
                // SAFETY: a class registered just above, whose superclass
                // is `NSObject`, made the way `NSObject` documents.
                let made: Option<Retained<AnyObject>> = unsafe { msg_send![class, new] };
                made.map(MainOnly).ok_or_else(|| {
                    OsError::Platform("the tray's click target could not be made".to_string())
                })
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn target_class() -> Result<&'static AnyClass, OsError> {
        let mut builder = ClassBuilder::new(c"RustxStatusTarget", NSObject::class())
            .ok_or_else(|| {
                OsError::Platform(
                    "an Objective-C class named RustxStatusTarget already exists".to_string(),
                )
            })?;
        // SAFETY: `chosen:` takes one object and returns nothing, which is
        // what this function's signature says.
        unsafe {
            builder.add_method(
                sel!(chosen:),
                chosen as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
        }
        Ok(builder.register())
    }

    /// The status item, held on the main thread's behalf.
    struct Held {
        item: MainOnly<Retained<AnyObject>>,
    }

    pub struct Icon {
        held: Option<Held>,
        chosen: Chosen,
        /// The tags this icon has registered, to forget them when the
        /// menu is replaced or the icon goes.
        owned: Mutex<Vec<isize>>,
    }

    /// Load the icon file as an `NSImage` sized for the bar.
    ///
    /// `NSImage` reads `.ico` through ImageIO, which is what lets this
    /// file take the same icon file Windows does — the facade already
    /// refuses anything that is not one.
    fn image(path: &Path) -> Result<Retained<AnyObject>, OsError> {
        let text = path.to_string_lossy();
        let Some(name) = ns_string(&text) else {
            return Err(OsError::File(format!("{} is not a path AppKit can read", path.display())));
        };
        // SAFETY: `alloc` then `initWithContentsOfFile:`, the documented
        // pair; `nil` back means the file was not an image.
        let image: Option<Retained<AnyObject>> = unsafe {
            let allocated: *mut AnyObject = msg_send![class!(NSImage), alloc];
            let image: *mut AnyObject = msg_send![allocated, initWithContentsOfFile: &*name];
            // `init…` hands back a +1 reference; taking it without an
            // extra retain is what owning it means.
            Retained::from_raw(image)
        };
        let image = image.ok_or_else(|| {
            OsError::File(format!("{} could not be read as an image by AppKit", path.display()))
        })?;
        // SAFETY: a documented setter, on the main thread.
        unsafe {
            let _: () = msg_send![
                &*image,
                setSize: Size {
                    width: ICON_POINTS,
                    height: ICON_POINTS
                }
            ];
        }
        Ok(image)
    }

    /// The button the status item draws itself with.
    fn button(item: &AnyObject) -> Option<Retained<AnyObject>> {
        // SAFETY: `button`, documented since 10.10, on the main thread.
        unsafe { msg_send![item, button] }
    }

    impl Icon {
        pub fn new(icon: &Path, tooltip: &str) -> Result<Self, OsError> {
            let chosen = Chosen::new();
            let click = next_tag();
            lock(tags()).push((click, chosen.clone(), CLICK.to_string()));

            let path = icon.to_path_buf();
            let tooltip = tooltip.to_string();
            let made = on_main("the system tray icon", move || {
                let _ = application();
                let target = target()?;
                let image = image(&path)?;
                // SAFETY: documented `NSStatusBar` calls, on the main
                // thread. `statusItemWithLength:` returns an item the
                // bar keeps only while somebody else does — `Retained`
                // is that somebody.
                unsafe {
                    let bar: *mut AnyObject = msg_send![class!(NSStatusBar), systemStatusBar];
                    let item: Option<Retained<AnyObject>> =
                        msg_send![bar, statusItemWithLength: VARIABLE_LENGTH];
                    let item = item.ok_or_else(|| {
                        OsError::Platform("the menu bar refused a status item".to_string())
                    })?;
                    if let Some(button) = button(&item) {
                        let _: () = msg_send![&*button, setImage: &*image];
                        if let Some(tip) = ns_string(&tooltip) {
                            let _: () = msg_send![&*button, setToolTip: &*tip];
                        }
                        // Until a menu exists, a click on the icon is a
                        // click — `CLICK`, as on Windows.
                        let _: () = msg_send![&*button, setTarget: &*target.0];
                        let _: () = msg_send![&*button, setAction: sel!(chosen:)];
                        let _: () = msg_send![&*button, setTag: click];
                    }
                    Ok(MainOnly(item))
                }
            })
            .and_then(|inner| inner);

            match made {
                Ok(item) => {
                    SHOWN.fetch_add(1, Ordering::AcqRel);
                    Ok(Self {
                        held: Some(Held { item }),
                        chosen,
                        owned: Mutex::new(vec![click]),
                    })
                }
                Err(refused) => {
                    lock(tags()).retain(|(each, _, _)| *each != click);
                    Err(refused)
                }
            }
        }

        /// Not on macOS: notifications do not go through an icon here
        /// (see the module doc), so an invisible one has nothing to do.
        pub fn hidden() -> Result<Self, OsError> {
            Err(OsError::Unsupported(
                "an invisible tray icon on macOS — notifications go through `osascript`, \
                 not through an icon",
            ))
        }

        pub fn menu(&self, items: &[Item]) -> Result<(), OsError> {
            let Some(held) = &self.held else {
                return Err(OsError::Platform("the icon is already gone".to_string()));
            };
            // The old lines' tags go; the new ones come.
            let mut owned = self
                .owned
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let click = owned.first().copied();
            let old: Vec<isize> = owned.iter().skip(1).copied().collect();
            lock(tags()).retain(|(each, _, _)| !old.contains(each));
            owned.truncate(1);

            let mut lines = Vec::new();
            for item in items {
                if item.label.is_empty() {
                    lines.push((None, String::new(), None));
                    continue;
                }
                let tag = next_tag();
                lock(tags()).push((tag, self.chosen.clone(), item.id.clone()));
                owned.push(tag);
                lines.push((Some(tag), item.label.clone(), item.checked));
            }
            drop(owned);

            let item = MainOnly(held.item.0.clone());
            on_main("the system tray menu", move || {
                let item = item;
                let target = target()?;
                // SAFETY: documented `NSMenu` and `NSMenuItem` calls, on
                // the main thread. The menu is retained by the status
                // item once set, so dropping ours after is fine.
                unsafe {
                    let menu: Option<Retained<AnyObject>> = msg_send![class!(NSMenu), new];
                    let menu = menu.ok_or_else(|| {
                        OsError::Platform("AppKit refused to make a menu".to_string())
                    })?;
                    // Enabled because this file says so, not because AppKit
                    // looked for a responder: the target answers `chosen:`,
                    // and there is nothing else to ask.
                    let _: () = msg_send![&*menu, setAutoenablesItems: false];
                    for (tag, label, checked) in &lines {
                        let line: Option<Retained<AnyObject>> = match tag {
                            None => msg_send![class!(NSMenuItem), separatorItem],
                            Some(_) => msg_send![class!(NSMenuItem), new],
                        };
                        let Some(line) = line else { continue };
                        if let Some(tag) = tag {
                            if let Some(title) = ns_string(label) {
                                let _: () = msg_send![&*line, setTitle: &*title];
                            }
                            let _: () = msg_send![&*line, setTarget: &*target.0];
                            let _: () = msg_send![&*line, setAction: sel!(chosen:)];
                            let _: () = msg_send![&*line, setTag: *tag];
                            let _: () = msg_send![&*line, setEnabled: true];
                            let _: () = msg_send![&*line, setState: check_state(*checked)];
                        }
                        let _: () = msg_send![&*menu, addItem: &*line];
                    }
                    let _: () = msg_send![&*item.0, setMenu: &*menu];
                }
                Ok(())
            })
            .and_then(|inner| inner)
            .inspect(|()| {
                // With a menu, a click opens it: the icon's own click tag
                // will not be sent again, and is forgotten.
                if let Some(click) = click {
                    lock(tags()).retain(|(each, _, _)| *each != click);
                }
            })
        }

        pub fn clicked(&self) -> Option<String> {
            self.chosen.take()
        }

        pub fn notify(&self, what: &What) -> Result<(), OsError> {
            super::super::notify::show(what)
        }
    }

    impl Drop for Icon {
        fn drop(&mut self) {
            let owned = std::mem::take(
                &mut *self
                    .owned
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            );
            lock(tags()).retain(|(each, _, _)| !owned.contains(each));
            let Some(held) = self.held.take() else {
                return;
            };
            // Waited for, as on Windows: the icon leaves the bar before
            // `Drop` returns, or `shown()` would count one that is going.
            let removed = on_main("removing the system tray icon", move || {
                let held = held;
                // SAFETY: the item this icon added, removed once, on the
                // main thread; `held` drops the last reference after.
                unsafe {
                    let bar: *mut AnyObject = msg_send![class!(NSStatusBar), systemStatusBar];
                    let _: () = msg_send![bar, removeStatusItem: &*held.item.0];
                }
                drop(held);
            });
            if removed.is_ok() {
                SHOWN.fetch_sub(1, Ordering::AcqRel);
            }
        }
    }
}

/// The same names, so that this file is checked on a machine that is
/// not a Mac — see `lib.rs`. Never the platform chosen anywhere else.
#[cfg(not(target_os = "macos"))]
mod real {
    use std::path::Path;

    use crate::error::OsError;
    use crate::notify::What;
    use crate::tray::Item;

    const ELSEWHERE: &str = "macos/tray.rs is compiled for its own sake on a machine that is \
                             not a Mac, and is never the platform module chosen there";

    pub fn shown() -> usize {
        unreachable!("{ELSEWHERE}")
    }

    pub struct Icon;

    impl Icon {
        pub fn new(_icon: &Path, _tooltip: &str) -> Result<Self, OsError> {
            unreachable!("{ELSEWHERE}")
        }
        pub fn hidden() -> Result<Self, OsError> {
            unreachable!("{ELSEWHERE}")
        }
        pub fn menu(&self, _items: &[Item]) -> Result<(), OsError> {
            unreachable!("{ELSEWHERE}")
        }
        pub fn clicked(&self) -> Option<String> {
            unreachable!("{ELSEWHERE}")
        }
        pub fn notify(&self, _what: &What) -> Result<(), OsError> {
            unreachable!("{ELSEWHERE}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_checked_line_is_on_and_everything_else_is_off() {
        assert_eq!(check_state(Some(true)), 1);
        assert_eq!(check_state(Some(false)), 0);
        assert_eq!(check_state(None), 0);
    }
}
