use std::fmt;
use std::path::PathBuf;

/// Errors the engine reports back to the application.
#[derive(Debug)]
#[non_exhaustive]
pub enum EngineError {
    /// The entry document does not exist on disk.
    IndexNotFound(PathBuf),
    /// The entry point names a remote resource. Forbidden: the render process
    /// sandbox is disabled, so only local content may be loaded into it.
    RemoteContentForbidden,
    /// CEF failed to initialize.
    EngineInit(String),
    /// A window was asked for from somewhere that cannot ask: outside the
    /// render process, or before a page exists. Only a page can ask for a
    /// second window, because only a page has the frame the request
    /// travels on.
    NoPage,
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexNotFound(path) => {
                write!(f, "entry document not found: {}", path.display())
            }
            Self::RemoteContentForbidden => {
                write!(
                    f,
                    "remote content forbidden: rustx only loads local resources"
                )
            }
            Self::EngineInit(detail) => {
                write!(f, "could not initialize the engine: {detail}")
            }
            Self::NoPage => write!(
                f,
                "a window can only be opened from a page: there is no frame here \
                 to carry the request to the process that owns the windows"
            ),
        }
    }
}

impl std::error::Error for EngineError {}
