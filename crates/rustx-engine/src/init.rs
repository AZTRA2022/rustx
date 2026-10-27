//! Declaring the CEF API version.
//!
//! CEF's C API is versioned: libcef reads the version the client was built
//! against before it will accept any structure the client implements. A
//! process that skips this aborts with
//! `CefApp_0_CToCpp called with invalid version -1` the first time it hands
//! CEF a handler. It has to run in every process, before anything else.

use std::sync::Once;

static DECLARED: Once = Once::new();

/// Tell libcef which version of its API this binary speaks.
pub fn declare_api_version() {
    DECLARED.call_once(|| {
        let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);
    });
}
