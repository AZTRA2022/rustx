//! What the machine can refuse, said in words a developer can act on.
//!
//! Every variant names the thing it is about — the device, the
//! platform, the file — because the whole difference between a usable
//! error and a useless one is whether the reader has to go looking for
//! which of their three webcams it meant.

/// What went wrong between the program and the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OsError {
    /// Nothing of that id is plugged in — or it was, a moment ago.
    NoSuchDevice(String),
    /// Something else is holding it. A meeting, usually.
    Busy(String),
    /// This platform does not do this, and says which platform it is.
    Unsupported(&'static str),
    /// The platform refused, in its own words.
    Platform(String),
    /// A file that could not be written, and why.
    File(String),
}

impl std::fmt::Display for OsError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchDevice(which) => write!(
                out,
                "there is no device called `{which}` — it may have been unplugged. \
                 `devices()` lists what is there now."
            ),
            Self::Busy(which) => write!(
                out,
                "`{which}` is being used by another application. Close whatever has \
                 it open, or choose another device."
            ),
            Self::Unsupported(what) => write!(
                out,
                "{what} is not available on this platform ({}). The capture layer is \
                 Windows for now, and says so rather than pretending.",
                std::env::consts::OS
            ),
            Self::Platform(said) => write!(out, "the system refused: {said}"),
            Self::File(said) => write!(out, "{said}"),
        }
    }
}

impl std::error::Error for OsError {}
