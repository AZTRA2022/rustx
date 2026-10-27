use std::path::{Path, PathBuf};

use cef::args::Args;
use cef::{CefString, Settings};

use crate::cef_app::{WindowSpec, engine_app};
use crate::init::declare_api_version;
use crate::error::EngineError;

/// URL schemes that name a resource outside this machine.
const REMOTE_SCHEMES: [&str; 4] = ["http://", "https://", "ftp://", "ws://"];

/// Prefix `std::fs::canonicalize` puts on Windows paths.
const VERBATIM_PREFIX: &str = r"\\?\";

/// Everything needed to open a window.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Local path of the document loaded at startup.
    pub index: PathBuf,
}

/// An application whose configuration has been validated.
#[derive(Debug)]
pub struct App {
    config: AppConfig,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self, EngineError> {
        if is_remote(&config.index) {
            return Err(EngineError::RemoteContentForbidden);
        }
        if !config.index.is_file() {
            return Err(EngineError::IndexNotFound(config.index.clone()));
        }
        Ok(Self { config })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// The `file://` URL the engine loads at startup.
    pub fn index_url(&self) -> Result<String, EngineError> {
        let absolute = self
            .config
            .index
            .canonicalize()
            .map_err(|_| EngineError::IndexNotFound(self.config.index.clone()))?;
        let text = absolute.to_string_lossy().into_owned();
        // Windows canonicalization yields a verbatim path; URLs do not take one.
        let text = text.strip_prefix(VERBATIM_PREFIX).unwrap_or(&text);
        let slashed = text.replace('\\', "/");
        Ok(format!("file:///{}", slashed.trim_start_matches('/')))
    }

    /// Where this application keeps its Chromium profile.
    ///
    /// Chromium enforces a process singleton per cache directory: two
    /// applications sharing one would hand their windows to whichever started
    /// first instead of opening their own. So the directory is derived from
    /// the application's own name.
    pub fn cache_dir(&self) -> PathBuf {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join("rustx").join(slug(&self.config.title))
    }

    /// Open the window and run until it closes.
    pub fn run(self) -> Result<(), EngineError> {
        declare_api_version();

        let url = self.index_url()?;
        let args = Args::new();
        let cache_dir = self.cache_dir();
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| EngineError::EngineInit(format!("cache directory: {e}")))?;

        let mut settings = Settings::default();
        settings.root_cache_path = CefString::from(cache_dir.to_string_lossy().as_ref());
        settings.log_file = CefString::from(cache_dir.join("engine.log").to_string_lossy().as_ref());
        // The render process hosts the application's own Rust code, so it is
        // not sandboxed. This is only defensible because the engine refuses
        // remote content — see `App::new`.
        settings.no_sandbox = 1;

        let mut application = engine_app(WindowSpec {
            title: self.config.title.clone(),
            width: self.config.width,
            height: self.config.height,
            url,
        });

        let initialized = cef::initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut application),
            std::ptr::null_mut(),
        );
        if initialized != 1 {
            return Err(EngineError::EngineInit(format!(
                "cef::initialize returned {initialized}"
            )));
        }

        cef::run_message_loop();
        cef::shutdown();
        Ok(())
    }
}

/// A filesystem-safe lowercase form of an application name.
fn slug(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut pending_dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() { "app".into() } else { out }
}

fn is_remote(path: &Path) -> bool {
    let Some(text) = path.to_str() else {
        return false;
    };
    let lowered = text.to_ascii_lowercase();
    REMOTE_SCHEMES
        .iter()
        .any(|scheme| lowered.starts_with(scheme))
}
