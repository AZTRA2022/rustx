//! What crosses between Rust and the page, and what a native says when it
//! cannot do what was asked.

pub use rustx_dom::Value;

/// What a native function says when it cannot do what was asked.
///
/// The message reaches the page as the reason a promise was rejected, so
/// it is written for whoever is reading the console.
#[derive(Debug, Clone)]
pub struct NativeError {
    pub message: String,
}

impl NativeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for NativeError {}
