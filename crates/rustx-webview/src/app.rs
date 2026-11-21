//! An application: its window, and the folder its files are served from.

use std::path::PathBuf;

use crate::error::EngineError;
use crate::host::{Config, Hooks};

/// Everything needed to open a window.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// The folder the window's files are served from: the application's
    /// `assets`.
    pub root: PathBuf,
    /// The page loaded first, as a path under `root`: `/index.html`.
    pub entry: String,
    /// The document to serve at `entry` when `root` has none of its own.
    /// Served from memory: nothing is written beside the assets.
    pub document: Option<String>,
    /// The frame the system draws around the window, or not: many
    /// applications draw their own.
    pub dressing: rustx_os::window::Dressing,
}

/// Which half of the relaunch this process is. There is no relaunch on a
/// system webview: the process the user started is the only one that runs
/// the application, so the answer is always the same. Kept so the facade's
/// first statement does not change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bootstrap {
    Browser,
    Child { exit_code: i32 },
}

pub fn bootstrap() -> Bootstrap {
    crate::host::since_start();
    Bootstrap::Browser
}

/// An application whose configuration has been validated.
#[derive(Debug)]
pub struct App {
    config: AppConfig,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self, EngineError> {
        // An application with no assets folder (no JavaScript, no files of its
        // own) is written a document of its own; the folder is only needed
        // when the document is meant to be read from it.
        if config.document.is_none() && !config.root.is_dir() {
            return Err(EngineError::IndexNotFound(config.root.clone()));
        }
        let relative = config.entry.trim_start_matches('/');
        if config.document.is_none() && !config.root.join(relative).is_file() {
            return Err(EngineError::IndexNotFound(config.root.join(relative)));
        }
        Ok(Self { config })
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Open the window and run until it closes.
    pub fn run(self) -> Result<(), EngineError> {
        // The application's hooks first: what they inject goes in the page,
        // and what they register is what the frame loop will call.
        crate::renderer::run_hooks();

        let mut config = Config::new(&self.config.title, self.config.root.clone());
        config.size = (self.config.width, self.config.height);
        config.entry = self.config.entry.clone();
        config.document = self.config.document.clone();
        config.decorations = self.config.dressing == rustx_os::window::Dressing::System;
        let mut hooks = Hooks::default();
        if let Some(spec) = crate::window::asked() {
            // A second window: what it was told to be, and gone with its parent.
            if let Some(title) = spec["title"].as_str() {
                config.title = title.to_string();
            }
            if let (Some(width), Some(height)) = (spec["width"].as_u64(), spec["height"].as_u64()) {
                config.size = (width as u32, height as u32);
            }
            config.decorations = spec["frameless"] != true;
            config.fragment = spec["at"].as_str().map(str::to_string);
            hooks.on_start = Box::new(|control| {
                let control = control.clone();
                std::thread::spawn(move || {
                    let _ = std::io::copy(&mut std::io::stdin(), &mut std::io::sink());
                    control.exit();
                });
            });
        }
        // Natives first: an injected script may call one at once.
        config.scripts = vec![crate::renderer::natives_script()];
        config.scripts.extend(crate::renderer::scripts());

        // `run` returns only an error: `Ok` holds an `Infallible`.
        let Err(error) = crate::host::run(config, hooks);
        Err(EngineError::EngineInit(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(document: Option<&str>) -> AppConfig {
        AppConfig {
            title: "t".into(),
            width: 1,
            height: 1,
            root: PathBuf::from("no-such-assets-folder"),
            entry: "/index.html".into(),
            document: document.map(str::to_string),
            dressing: rustx_os::window::Dressing::default(),
        }
    }

    #[test]
    fn an_app_with_no_assets_folder_runs_on_its_own_document() {
        assert!(App::new(config(Some("<html></html>"))).is_ok());
        assert!(App::new(config(None)).is_err());
    }
}
