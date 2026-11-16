//! `NSMenu`: the context menu, and the menu bar macOS needs before a
//! keyboard shortcut works at all.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from Apple's documentation. The class
//! this registers, the selectors it sends and the coordinate flip have
//! had only the type-check `cargo check --target aarch64-apple-darwin`
//! gives. Verify on a real Mac before trusting them.

// Reachable from `crate::platform` on a Mac and from the type-check
// copy of this directory anywhere else -- where nothing calls it, which
// is what the allow says.
#[allow(unused_imports)]
pub use real::{application, popup};

pub fn supported() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(target_os = "macos")]
mod real {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicIsize, Ordering};

    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, NSObject, Sel};
    use objc2::{ClassType, class, msg_send, sel};

    use super::super::appkit::{MainOnly, Point, application as the_app, ns_string, on_main};
    use crate::error::OsError;
    use crate::tray::Item;
    use crate::window::Pane;

    /// The tag of the line that was chosen, or zero.
    ///
    /// One static and not a queue: a popup menu is modal, so one is
    /// open at a time and nobody is waiting on an earlier answer.
    static PICKED: AtomicIsize = AtomicIsize::new(0);

    extern "C" fn picked(_this: *mut AnyObject, _command: Sel, sender: *mut AnyObject) {
        if sender.is_null() {
            return;
        }
        // SAFETY: `sender` is the `NSMenuItem` that was chosen, which
        // answers `tag`.
        let tag: isize = unsafe { msg_send![sender, tag] };
        PICKED.store(tag, Ordering::Release);
    }

    /// The object every line is aimed at, made once and kept.
    ///
    /// `NSMenuItem` does not retain its target, and a target freed
    /// while its menu is open is a crash rather than a missing click.
    fn target() -> Result<&'static MainOnly<Retained<AnyObject>>, OsError> {
        static TARGET: OnceLock<Result<MainOnly<Retained<AnyObject>>, OsError>> = OnceLock::new();
        TARGET
            .get_or_init(|| {
                let class = target_class()?;
                // SAFETY: a class registered just above, whose
                // superclass is `NSObject`.
                let made: Option<Retained<AnyObject>> = unsafe { msg_send![class, new] };
                made.map(MainOnly).ok_or_else(|| {
                    OsError::Platform("the menu's click target could not be made".to_string())
                })
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn target_class() -> Result<&'static AnyClass, OsError> {
        let mut builder =
            ClassBuilder::new(c"RustxMenuTarget", NSObject::class()).ok_or_else(|| {
                OsError::Platform(
                    "an Objective-C class named RustxMenuTarget already exists".to_string(),
                )
            })?;
        // SAFETY: `picked:` takes one object and returns nothing, which
        // is what this function's signature says.
        unsafe {
            builder.add_method(
                sel!(picked:),
                picked as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
        }
        Ok(builder.register())
    }

    pub fn popup(_pane: Pane, items: &[Item], at: (i32, i32)) -> Result<Option<String>, OsError> {
        // Quartz counts from the top of the primary screen and Cocoa
        // from its bottom, and `atLocation:` is Cocoa's. The same flip
        // `screen.rs` does, by the same height.
        let height = f64::from(crate::screen::primary()?.bounds.height);
        let where_ = Point {
            x: f64::from(at.0),
            y: height - f64::from(at.1),
        };
        let lines: Vec<Item> = items.to_vec();

        PICKED.store(0, Ordering::Release);
        let shown = on_main("the menu", move || {
            let target = target().ok()?;
            // SAFETY: documented `NSMenu` and `NSMenuItem` calls, on the
            // main thread, with strings that live across each call.
            unsafe {
                let menu: Option<Retained<AnyObject>> = msg_send![class!(NSMenu), new];
                let menu = menu?;
                // AppKit disables a line whose target does not answer
                // its action, and decides that by asking the target --
                // which here is an object with one method and no
                // `validateMenuItem:`. Off, so every line stays live.
                let _: () = msg_send![&*menu, setAutoenablesItems: false];
                for (index, item) in lines.iter().enumerate() {
                    if item.label.is_empty() {
                        let line: Option<Retained<AnyObject>> =
                            msg_send![class!(NSMenuItem), separatorItem];
                        if let Some(line) = line {
                            let _: () = msg_send![&*menu, addItem: &*line];
                        }
                        continue;
                    }
                    let label = ns_string(&item.label)?;
                    let empty = ns_string("")?;
                    let line: Option<Retained<AnyObject>> = msg_send![class!(NSMenuItem), new];
                    let line = line?;
                    let _: () = msg_send![&*line, setTitle: &*label];
                    let _: () = msg_send![&*line, setKeyEquivalent: &*empty];
                    let _: () = msg_send![&*line, setAction: sel!(picked:)];
                    let _: () = msg_send![&*line, setTarget: &*target.0];
                    let _: () = msg_send![&*line, setTag: (index as isize) + 1];
                    if let Some(ticked) = item.checked {
                        // 1 is `NSControlStateValueOn`, 0 is off.
                        let state: isize = if ticked { 1 } else { 0 };
                        let _: () = msg_send![&*line, setState: state];
                    }
                    let _: () = msg_send![&*menu, addItem: &*line];
                }
                // Nil for both: positioning on no item and in no view
                // is what makes the location a screen location.
                let nothing: *mut AnyObject = std::ptr::null_mut();
                let _: bool = msg_send![
                    &*menu,
                    popUpMenuPositioningItem: nothing,
                    atLocation: where_,
                    inView: nothing,
                ];
            }
            Some(())
        })?;
        if shown.is_none() {
            return Err(OsError::Platform(
                "AppKit did not open the menu".to_string(),
            ));
        }

        let tag = PICKED.swap(0, Ordering::AcqRel);
        if tag <= 0 {
            return Ok(None);
        }
        Ok(items.get((tag - 1) as usize).map(|item| item.id.clone()))
    }

    /// One line of the bar: a title, and the lines under it.
    ///
    /// Each line names an AppKit selector rather than an action of
    /// ours. That is the whole point of this function: `copy:` is
    /// handled by whatever has the keyboard, and what the bar adds is
    /// the shortcut that reaches it.
    type Line = (&'static str, Sel, &'static str);

    pub fn application() -> Result<(), OsError> {
        let made = on_main("the application menu", || {
            let app = the_app()?;
            // SAFETY: documented `NSMenu`, `NSMenuItem` and
            // `NSApplication` calls, on the main thread.
            unsafe {
                let bar: Option<Retained<AnyObject>> = msg_send![class!(NSMenu), new];
                let bar = bar?;
                let menus: [(&str, Vec<Line>); 3] = [
                    (
                        "Application",
                        vec![
                            ("Hide", sel!(hide:), "h"),
                            ("Hide Others", sel!(hideOtherApplications:), ""),
                            ("Show All", sel!(unhideAllApplications:), ""),
                            ("", sel!(hide:), ""),
                            ("Quit", sel!(terminate:), "q"),
                        ],
                    ),
                    (
                        "Edit",
                        vec![
                            ("Undo", sel!(undo:), "z"),
                            ("Redo", sel!(redo:), "Z"),
                            ("", sel!(undo:), ""),
                            ("Cut", sel!(cut:), "x"),
                            ("Copy", sel!(copy:), "c"),
                            ("Paste", sel!(paste:), "v"),
                            ("Select All", sel!(selectAll:), "a"),
                        ],
                    ),
                    (
                        "Window",
                        vec![
                            ("Minimise", sel!(performMiniaturize:), "m"),
                            ("Close", sel!(performClose:), "w"),
                        ],
                    ),
                ];
                for (title, lines) in menus {
                    let name = ns_string(title)?;
                    let empty = ns_string("")?;
                    let holder: Option<Retained<AnyObject>> = msg_send![class!(NSMenuItem), new];
                    let holder = holder?;
                    let submenu: Option<Retained<AnyObject>> = msg_send![class!(NSMenu), new];
                    let submenu = submenu?;
                    let _: () = msg_send![&*submenu, setTitle: &*name];
                    for (label, action, key) in lines {
                        let line: Option<Retained<AnyObject>> = if label.is_empty() {
                            msg_send![class!(NSMenuItem), separatorItem]
                        } else {
                            msg_send![class!(NSMenuItem), new]
                        };
                        let line = line?;
                        if !label.is_empty() {
                            let shown = ns_string(label)?;
                            let shortcut = ns_string(key)?;
                            let _: () = msg_send![&*line, setTitle: &*shown];
                            let _: () = msg_send![&*line, setAction: action];
                            let _: () = msg_send![&*line, setKeyEquivalent: &*shortcut];
                            // No target: AppKit walks the responder
                            // chain, which is how `copy:` reaches the
                            // text field that has the keyboard rather
                            // than something of ours.
                        }
                        let _: () = msg_send![&*submenu, addItem: &*line];
                    }
                    let _: () = msg_send![&*holder, setTitle: &*empty];
                    let _: () = msg_send![&*holder, setSubmenu: &*submenu];
                    let _: () = msg_send![&*bar, addItem: &*holder];
                }
                let _: () = msg_send![&*app, setMainMenu: &*bar];
            }
            Some(())
        })?;
        made.ok_or_else(|| {
            OsError::Platform("AppKit did not take the application menu".to_string())
        })
    }
}

/// The same names, so that this file is type-checked on a machine that
/// is not a Mac. Never the platform module chosen there.
#[cfg(not(target_os = "macos"))]
mod real {
    use crate::error::OsError;
    use crate::tray::Item;
    use crate::window::Pane;

    const ELSEWHERE: &str = "macos/menu.rs is compiled for its own sake on a machine that is \
                             not a Mac, and is never the platform module chosen there";

    pub fn popup(_pane: Pane, _items: &[Item], _at: (i32, i32)) -> Result<Option<String>, OsError> {
        unreachable!("{ELSEWHERE}")
    }

    pub fn application() -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
}
