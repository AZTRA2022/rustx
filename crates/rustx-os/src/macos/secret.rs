//! Secrets, on macOS: the login keychain, through the Security
//! framework's generic-password calls.
//!
//! # Never compiled for a Mac, never run
//!
//! Written on a Windows machine from the Security framework's
//! documentation (`SecKeychain.h`, `SecKeychainItem.h`). Never compiled
//! for an Apple target, never linked, and no secret has been stored in
//! a keychain from it. Only type-checked on Windows. Verify on a real
//! Mac before trusting it — this is code that handles passwords.
//!
//! # Why the `SecKeychain…GenericPassword` calls
//!
//! They are deprecated in favour of `SecItemAdd` and friends, which
//! take a `CFDictionary` of `kSec…` constants — and building that by
//! hand, without `core-foundation` bindings, is several times the
//! unverifiable code for the same keychain item. The older calls take
//! plain pointers and lengths, still work on current macOS, and store
//! the same item `SecItem` would find. Not `security add-generic-
//! password` either: its `-w` puts the secret on the command line, where
//! `ps` shows it to every process on the machine.

#[cfg(not(target_os = "macos"))]
use crate::error::OsError;

#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub use real::{fetch, forget, store};

#[cfg(target_os = "macos")]
mod real {
    use std::ffi::{c_char, c_void};

    use crate::error::OsError;

    type OsStatus = i32;
    type ItemRef = *mut c_void;

    #[cfg_attr(target_os = "macos", link(name = "Security", kind = "framework"))]
    unsafe extern "C" {
        fn SecKeychainAddGenericPassword(
            keychain: *mut c_void,
            service_length: u32,
            service: *const c_char,
            account_length: u32,
            account: *const c_char,
            password_length: u32,
            password: *const c_void,
            item: *mut ItemRef,
        ) -> OsStatus;
        fn SecKeychainFindGenericPassword(
            keychains: *const c_void,
            service_length: u32,
            service: *const c_char,
            account_length: u32,
            account: *const c_char,
            password_length: *mut u32,
            password: *mut *mut c_void,
            item: *mut ItemRef,
        ) -> OsStatus;
        fn SecKeychainItemModifyAttributesAndData(
            item: ItemRef,
            attributes: *const c_void,
            length: u32,
            data: *const c_void,
        ) -> OsStatus;
        fn SecKeychainItemDelete(item: ItemRef) -> OsStatus;
        fn SecKeychainItemFreeContent(attributes: *mut c_void, data: *mut c_void) -> OsStatus;
        fn CFRelease(object: *const c_void);
    }

    /// `errSecItemNotFound`.
    const NOT_FOUND: OsStatus = -25300;

    fn refused(what: &str, status: OsStatus) -> OsError {
        OsError::Platform(format!("the keychain refused {what} (OSStatus {status})"))
    }

    /// The item, if there is one, and its data when asked for.
    type Found = Option<(ItemRef, Option<Vec<u8>>)>;

    fn find(service: &str, account: &str, want_data: bool) -> Result<Found, OsError> {
        let mut item: ItemRef = std::ptr::null_mut();
        let mut length = 0u32;
        let mut data: *mut c_void = std::ptr::null_mut();
        // SAFETY: lengths and pointers of two Rust strings that outlive
        // the call; the data, when asked for, is freed below.
        let status = unsafe {
            SecKeychainFindGenericPassword(
                std::ptr::null(),
                service.len() as u32,
                service.as_ptr().cast(),
                account.len() as u32,
                account.as_ptr().cast(),
                if want_data { &mut length } else { std::ptr::null_mut() },
                if want_data { &mut data } else { std::ptr::null_mut() },
                &mut item,
            )
        };
        match status {
            0 => {}
            NOT_FOUND => return Ok(None),
            other => return Err(refused("the search", other)),
        }
        let bytes = (want_data && !data.is_null()).then(|| {
            // SAFETY: `length` bytes the keychain handed back, copied and
            // then freed the way the documentation says.
            unsafe {
                let copy = std::slice::from_raw_parts(data.cast::<u8>(), length as usize).to_vec();
                SecKeychainItemFreeContent(std::ptr::null_mut(), data);
                copy
            }
        });
        Ok(Some((item, bytes)))
    }

    fn release(item: ItemRef) {
        if !item.is_null() {
            // SAFETY: an item reference the keychain gave us, released once.
            unsafe { CFRelease(item) };
        }
    }

    pub fn store(service: &str, account: &str, secret: &str) -> Result<(), OsError> {
        if let Some((item, _)) = find(service, account, false)? {
            // SAFETY: the item just found, and the secret's bytes.
            let status = unsafe {
                SecKeychainItemModifyAttributesAndData(
                    item,
                    std::ptr::null(),
                    secret.len() as u32,
                    secret.as_ptr().cast(),
                )
            };
            release(item);
            return if status == 0 { Ok(()) } else { Err(refused("the new secret", status)) };
        }
        let mut item: ItemRef = std::ptr::null_mut();
        // SAFETY: lengths and pointers of Rust strings that outlive the
        // call; the default keychain.
        let status = unsafe {
            SecKeychainAddGenericPassword(
                std::ptr::null_mut(),
                service.len() as u32,
                service.as_ptr().cast(),
                account.len() as u32,
                account.as_ptr().cast(),
                secret.len() as u32,
                secret.as_ptr().cast(),
                &mut item,
            )
        };
        release(item);
        if status == 0 { Ok(()) } else { Err(refused("the secret", status)) }
    }

    pub fn fetch(service: &str, account: &str) -> Result<Option<String>, OsError> {
        let Some((item, bytes)) = find(service, account, true)? else {
            return Ok(None);
        };
        release(item);
        String::from_utf8(bytes.unwrap_or_default())
            .map(Some)
            .map_err(|_| OsError::Platform("the stored secret is not UTF-8".to_string()))
    }

    pub fn forget(service: &str, account: &str) -> Result<(), OsError> {
        let Some((item, _)) = find(service, account, false)? else {
            return Ok(());
        };
        // SAFETY: the item just found.
        let status = unsafe { SecKeychainItemDelete(item) };
        release(item);
        if status == 0 { Ok(()) } else { Err(refused("removing the secret", status)) }
    }
}

/// Never the platform module outside macOS — see `lib.rs`.
#[cfg(not(target_os = "macos"))]
pub fn store(_service: &str, _account: &str, _secret: &str) -> Result<(), OsError> {
    unreachable!("macos/secret.rs is never the platform module outside macOS")
}
#[cfg(not(target_os = "macos"))]
pub fn fetch(_service: &str, _account: &str) -> Result<Option<String>, OsError> {
    unreachable!("macos/secret.rs is never the platform module outside macOS")
}
#[cfg(not(target_os = "macos"))]
pub fn forget(_service: &str, _account: &str) -> Result<(), OsError> {
    unreachable!("macos/secret.rs is never the platform module outside macOS")
}
