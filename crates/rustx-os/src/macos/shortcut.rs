//! Global shortcuts, on macOS: Carbon's `RegisterEventHotKey`.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the Carbon Event Manager's
//! documentation and headers (`CarbonEvents.h`, `Events.h`). It has
//! never been compiled for an Apple target, never linked against
//! Carbon, and no key has ever been pressed at it. Its one check is a
//! type-check on Windows; the key table below is tested on every
//! machine, and is the only part that is. `cargo check --target
//! aarch64-apple-darwin`, then `cargo run -p rustx-os --example
//! a_global_shortcut` on a real Mac, before any of it is trusted.
//!
//! # Why Carbon, in this decade
//!
//! Carbon is deprecated, and this one corner of it is still how
//! almost every macOS utility takes a system-wide hotkey: `AppKit` has
//! no replacement. The alternative — an `NSEvent` global monitor —
//! *watches* keys rather than taking them (the other application
//! still gets the key), and it needs the user to grant Input
//! Monitoring in System Settings first. A registered hotkey needs no
//! permission at all, and it is exclusive, which is what `listen`
//! promises on Windows: the combination is ours, or `Busy`.
//!
//! # The main thread
//!
//! The Carbon Event Manager is not thread-safe, and hotkey presses are
//! delivered as Carbon events on the main thread's event loop. So
//! registering and releasing are carried to the main thread
//! (`appkit::on_main`), and presses are only heard while that thread
//! runs its loop — which a CEF application's browser process always
//! does, and which the `a_global_shortcut` example, whose main thread
//! sleeps, does not: on macOS that example will register and then hear
//! nothing. That is a difference from Windows this file cannot paper
//! over — Windows gives the hotkey a thread of its own, macOS does not.

use crate::shortcut::{ALT, CONTROL, SHIFT, WIN};

pub const AVAILABLE: bool = true;

#[allow(unused_imports)]
pub use real::Shortcut;

// ─────────────── What is translated without a Mac ───────────────

/// Carbon's modifier bits (`Events.h`): `cmdKey`, `shiftKey`,
/// `optionKey`, `controlKey`.
pub(crate) const CMD_KEY: u32 = 1 << 8;
pub(crate) const SHIFT_KEY: u32 = 1 << 9;
pub(crate) const OPTION_KEY: u32 = 1 << 11;
pub(crate) const CONTROL_KEY: u32 = 1 << 12;

/// This crate's modifiers, as Carbon counts them.
///
/// `Win` is ⌘: `shortcut::parse` already reads `cmd`, `super` and
/// `meta` as that same bit, and ⌘ is the key a Mac user means by all
/// three.
pub(crate) fn carbon_modifiers(modifiers: u32) -> u32 {
    let mut carbon = 0;
    if modifiers & CONTROL != 0 {
        carbon |= CONTROL_KEY;
    }
    if modifiers & ALT != 0 {
        carbon |= OPTION_KEY;
    }
    if modifiers & SHIFT != 0 {
        carbon |= SHIFT_KEY;
    }
    if modifiers & WIN != 0 {
        carbon |= CMD_KEY;
    }
    carbon
}

/// The Carbon virtual key code (`kVK_*`, `Events.h`) for a Windows
/// virtual-key code, which is what `shortcut::parse` reads into.
///
/// Carbon's codes are positions on an ANSI keyboard, not letters —
/// `kVK_ANSI_A` is 0 and `kVK_ANSI_S` is 1 — so this is a table and not
/// arithmetic. Nothing for F21 to F24: no Mac keyboard has them.
pub(crate) fn carbon_key(virtual_key: u32) -> Option<u32> {
    const LETTERS: [u32; 26] = [
        0x00, 0x0B, 0x08, 0x02, 0x0E, 0x03, 0x05, 0x04, 0x22, 0x26, 0x28, 0x25, 0x2E, // A..M
        0x2D, 0x1F, 0x23, 0x0C, 0x0F, 0x01, 0x11, 0x20, 0x09, 0x0D, 0x07, 0x10, 0x06, // N..Z
    ];
    const DIGITS: [u32; 10] = [0x1D, 0x12, 0x13, 0x14, 0x15, 0x17, 0x16, 0x1A, 0x1C, 0x19];
    const FUNCTIONS: [u32; 20] = [
        0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, // F1..F10
        0x67, 0x6F, 0x69, 0x6B, 0x71, 0x6A, 0x40, 0x4F, 0x50, 0x5A, // F11..F20
    ];
    Some(match virtual_key {
        0x41..=0x5A => LETTERS[(virtual_key - 0x41) as usize],
        0x30..=0x39 => DIGITS[(virtual_key - 0x30) as usize],
        0x70..=0x83 => FUNCTIONS[(virtual_key - 0x70) as usize],
        0x20 => 0x31, // Space
        0x09 => 0x30, // Tab
        0x0D => 0x24, // Return
        0x1B => 0x35, // Escape
        0x08 => 0x33, // Backspace: Carbon's `kVK_Delete`
        0x2E => 0x75, // Delete: `kVK_ForwardDelete`
        // Insert: a PC keyboard's Insert key reaches a Mac as Help.
        0x2D => 0x72,
        0x24 => 0x73, // Home
        0x23 => 0x77, // End
        0x21 => 0x74, // PageUp
        0x22 => 0x79, // PageDown
        0x25 => 0x7B, // Left
        0x26 => 0x7E, // Up
        0x27 => 0x7C, // Right
        0x28 => 0x7D, // Down
        _ => return None,
    })
}

/// `'rsxk'`, the signature every hotkey of this crate carries, so that
/// a hotkey event is recognised as ours.
pub(crate) const SIGNATURE: u32 = u32::from_be_bytes(*b"rsxk");

// ─────────────────────── What talks to Carbon ───────────────────────

#[cfg(target_os = "macos")]
mod real {
    use std::ffi::{c_long, c_ulong, c_void};
    use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use super::super::appkit::{application, on_main};
    use super::{SIGNATURE, carbon_key, carbon_modifiers};
    use crate::error::OsError;

    type OsStatus = i32;
    type EventTargetRef = *mut c_void;
    type EventRef = *mut c_void;
    type EventHandlerCallRef = *mut c_void;
    type EventHandlerRef = *mut c_void;
    type EventHotKeyRef = *mut c_void;
    type EventHandlerProc =
        extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OsStatus;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct EventHotKeyId {
        signature: u32,
        id: u32,
    }

    #[repr(C)]
    struct EventTypeSpec {
        event_class: u32,
        event_kind: u32,
    }

    /// `kEventClassKeyboard` (`'keyb'`) and `kEventHotKeyPressed`.
    const KEYBOARD: u32 = u32::from_be_bytes(*b"keyb");
    const HOT_KEY_PRESSED: u32 = 5;
    /// `kEventParamDirectObject` (`'----'`) and `typeEventHotKeyID`
    /// (`'hkid'`).
    const DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");
    const TYPE_HOT_KEY_ID: u32 = u32::from_be_bytes(*b"hkid");
    /// `kEventHotKeyExclusive`: refuse rather than share a combination
    /// another application already holds — `Busy`, as on Windows.
    const EXCLUSIVE: u32 = 1;
    /// `eventHotKeyExistsErr`.
    const ALREADY_TAKEN: OsStatus = -9878;
    /// `noErr`, and `eventNotHandledErr` for an event that is not ours.
    const NO_ERR: OsStatus = 0;
    const NOT_HANDLED: OsStatus = -9874;

    unsafe extern "C" {
        fn GetApplicationEventTarget() -> EventTargetRef;
        fn InstallEventHandler(
            target: EventTargetRef,
            handler: EventHandlerProc,
            count: c_ulong,
            types: *const EventTypeSpec,
            user_data: *mut c_void,
            out: *mut EventHandlerRef,
        ) -> OsStatus;
        fn RegisterEventHotKey(
            key_code: u32,
            modifiers: u32,
            id: EventHotKeyId,
            target: EventTargetRef,
            options: u32,
            out: *mut EventHotKeyRef,
        ) -> OsStatus;
        fn UnregisterEventHotKey(hot_key: EventHotKeyRef) -> OsStatus;
        fn GetEventParameter(
            event: EventRef,
            name: u32,
            desired_type: u32,
            actual_type: *mut u32,
            buffer_size: c_ulong,
            actual_size: *mut c_ulong,
            data: *mut c_void,
        ) -> OsStatus;
    }

    /// Each held combination's counter, by the id Carbon hands back.
    type Held = Mutex<Vec<(u32, Arc<AtomicU64>)>>;

    fn held() -> &'static Held {
        static HELD: OnceLock<Held> = OnceLock::new();
        HELD.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn lock<T>(
        lock: &'static Mutex<Vec<T>>,
    ) -> std::sync::MutexGuard<'static, Vec<T>> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Called by Carbon, on the main thread, for every hotkey press of
    /// this application.
    extern "C" fn pressed(
        _next: EventHandlerCallRef,
        event: EventRef,
        _user_data: *mut c_void,
    ) -> OsStatus {
        let mut id = EventHotKeyId::default();
        // SAFETY: the event Carbon is handing us, and a buffer of the
        // exact size of the parameter asked for.
        let status = unsafe {
            GetEventParameter(
                event,
                DIRECT_OBJECT,
                TYPE_HOT_KEY_ID,
                std::ptr::null_mut(),
                std::mem::size_of::<EventHotKeyId>() as c_ulong,
                std::ptr::null_mut(),
                (&raw mut id).cast(),
            )
        };
        if status != NO_ERR || id.signature != SIGNATURE {
            return NOT_HANDLED;
        }
        if let Some((_, count)) = lock(held()).iter().find(|(each, _)| *each == id.id) {
            count.fetch_add(1, Ordering::AcqRel);
        }
        NO_ERR
    }

    /// One handler for the whole application, installed the first time
    /// a combination is asked for. Run on the main thread.
    fn handler_installed() -> Result<(), OsError> {
        static INSTALLED: OnceLock<Result<(), OsError>> = OnceLock::new();
        INSTALLED
            .get_or_init(|| {
                let _ = application();
                let wanted = EventTypeSpec {
                    event_class: KEYBOARD,
                    event_kind: HOT_KEY_PRESSED,
                };
                let mut handler: EventHandlerRef = std::ptr::null_mut();
                // SAFETY: the application's target, a handler that lives
                // as long as the program, and one event type that outlives
                // the call. Never removed: it serves every combination.
                let status = unsafe {
                    InstallEventHandler(
                        GetApplicationEventTarget(),
                        pressed,
                        1,
                        &wanted,
                        std::ptr::null_mut(),
                        &mut handler,
                    )
                };
                if status == NO_ERR {
                    Ok(())
                } else {
                    Err(OsError::Platform(format!(
                        "Carbon refused the hotkey handler (OSStatus {status})"
                    )))
                }
            })
            .clone()
    }

    /// A combination held, and the counter its presses go to.
    pub struct Shortcut {
        id: u32,
        hot_key: usize,
        pressed: Arc<AtomicU64>,
    }

    impl Shortcut {
        pub fn listen(modifiers: u32, key: u32, keys: &str) -> Result<Self, OsError> {
            let Some(code) = carbon_key(key) else {
                return Err(OsError::Platform(format!(
                    "`{keys}` names a key a Mac keyboard does not have (F21 to F24 have no \
                     code on macOS)"
                )));
            };
            let carbon = carbon_modifiers(modifiers);
            static NEXT: AtomicU32 = AtomicU32::new(1);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let pressed = Arc::new(AtomicU64::new(0));
            lock(held()).push((id, pressed.clone()));

            let written = keys.to_string();
            let registered = on_main("a global shortcut", move || -> Result<usize, OsError> {
                handler_installed()?;
                let mut hot_key: EventHotKeyRef = std::ptr::null_mut();
                // SAFETY: a key code and modifiers Carbon defines, our own
                // id, the application's target, on the main thread.
                let status = unsafe {
                    RegisterEventHotKey(
                        code,
                        carbon,
                        EventHotKeyId {
                            signature: SIGNATURE,
                            id,
                        },
                        GetApplicationEventTarget(),
                        EXCLUSIVE,
                        &mut hot_key,
                    )
                };
                match status {
                    NO_ERR => Ok(hot_key as usize),
                    ALREADY_TAKEN => Err(OsError::Busy(format!(
                        "{written} — another application already holds this combination"
                    ))),
                    other => Err(OsError::Platform(format!(
                        "Carbon refused `{written}` (OSStatus {other})"
                    ))),
                }
            })
            .and_then(|inner| inner);

            match registered {
                Ok(hot_key) => Ok(Self {
                    id,
                    hot_key,
                    pressed,
                }),
                Err(refused) => {
                    lock(held()).retain(|(each, _)| *each != id);
                    Err(refused)
                }
            }
        }

        pub fn pressed(&self) -> u64 {
            self.pressed.swap(0, Ordering::AcqRel)
        }
    }

    impl Drop for Shortcut {
        fn drop(&mut self) {
            let hot_key = self.hot_key;
            // Waited for, as on Windows: a `Drop` that returns before the
            // combination is free is one after which a second `listen`
            // fails. If the main thread is not listening, there is
            // nothing more to do than to say nothing — a `Drop` cannot
            // refuse.
            let _: Result<c_long, OsError> = on_main("releasing a global shortcut", move || {
                // SAFETY: the reference `RegisterEventHotKey` gave back,
                // released once, on the main thread.
                c_long::from(unsafe { UnregisterEventHotKey(hot_key as EventHotKeyRef) })
            });
            lock(held()).retain(|(each, _)| *each != self.id);
        }
    }
}

/// The same names, so that this file is checked on a machine that is
/// not a Mac — see `lib.rs`. Never the platform chosen anywhere else.
#[cfg(not(target_os = "macos"))]
mod real {
    use crate::error::OsError;

    pub struct Shortcut;

    impl Shortcut {
        pub fn listen(_modifiers: u32, _key: u32, _keys: &str) -> Result<Self, OsError> {
            unreachable!(
                "macos/shortcut.rs is compiled for its own sake on a machine that is not a \
                 Mac, and is never the platform module chosen there"
            )
        }

        pub fn pressed(&self) -> u64 {
            unreachable!(
                "macos/shortcut.rs is compiled for its own sake on a machine that is not a \
                 Mac, and is never the platform module chosen there"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    // Run on every machine: the translation is a table, and a table is
    // what a machine without a Mac can check.
    use super::*;
    use crate::shortcut::parse;

    fn code(keys: &str) -> Option<u32> {
        carbon_key(parse(keys).expect("a combination").1)
    }

    #[test]
    fn letters_are_positions_on_the_keyboard_not_alphabet_order() {
        assert_eq!(code("A"), Some(0x00));
        assert_eq!(code("S"), Some(0x01));
        assert_eq!(code("R"), Some(0x0F));
        assert_eq!(code("Z"), Some(0x06));
        assert_eq!(code("Q"), Some(0x0C));
        assert_eq!(code("M"), Some(0x2E));
        assert_eq!(code("N"), Some(0x2D));
    }

    #[test]
    fn every_letter_and_digit_has_its_own_code() {
        let mut seen = Vec::new();
        for key in (b'A'..=b'Z').chain(b'0'..=b'9') {
            let found = carbon_key(u32::from(key)).expect("a Mac has it");
            assert!(!seen.contains(&found), "{} collides", key as char);
            seen.push(found);
        }
    }

    #[test]
    fn digits_follow_the_mac_keyboard_not_counting_order() {
        assert_eq!(code("1"), Some(0x12));
        assert_eq!(code("5"), Some(0x17));
        assert_eq!(code("6"), Some(0x16));
        assert_eq!(code("0"), Some(0x1D));
    }

    #[test]
    fn function_keys_up_to_f20_and_not_beyond() {
        assert_eq!(code("F1"), Some(0x7A));
        assert_eq!(code("F4"), Some(0x76));
        assert_eq!(code("F12"), Some(0x6F));
        assert_eq!(code("F20"), Some(0x5A));
        assert_eq!(code("F21"), None);
        assert_eq!(code("F24"), None);
    }

    #[test]
    fn named_keys_reach_their_carbon_codes() {
        assert_eq!(code("Space"), Some(0x31));
        assert_eq!(code("Esc"), Some(0x35));
        assert_eq!(code("Enter"), Some(0x24));
        assert_eq!(code("Backspace"), Some(0x33));
        assert_eq!(code("Delete"), Some(0x75));
        assert_eq!(code("Left"), Some(0x7B));
        assert_eq!(code("Up"), Some(0x7E));
    }

    #[test]
    fn modifiers_are_translated_and_win_is_command() {
        let (modifiers, _) = parse("Ctrl+Alt+Shift+Cmd+K").expect("a combination");
        assert_eq!(
            carbon_modifiers(modifiers),
            CONTROL_KEY | OPTION_KEY | SHIFT_KEY | CMD_KEY
        );
        let (modifiers, _) = parse("Super+Space").expect("a combination");
        assert_eq!(carbon_modifiers(modifiers), CMD_KEY);
        assert_eq!(carbon_modifiers(0), 0);
    }

    #[test]
    fn the_signature_spells_its_four_letters() {
        assert_eq!(SIGNATURE.to_be_bytes(), *b"rsxk");
    }
}
