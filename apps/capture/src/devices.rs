//! What this application is: a stream that closes itself.
//!
//! The screens are `app/`. This is the software under them.

/// A capture stream, standing in for the hardware the OS layer will
/// bring. What matters is that it is opened by a component and closed by
/// nobody.
pub struct Stream {
    pub device: String,
}

impl Stream {
    pub fn open(device: &str) -> Self {
        Self {
            device: device.to_string(),
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // Nobody closes a stream: the preview held it, and leaving the
        // preview let it go. What the application counts is a value its
        // pages share, so the interface can say that it happened.
        rustx::store::shared("closed_streams", || 0).update(|closed| *closed += 1);
    }
}
