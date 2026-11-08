//! Looking at files.
//!
//! No dependency, no platform API: a census of the files that matter and
//! their modification dates, taken again every tenth of a second and
//! compared. A project is a few dozen files, and a few dozen `stat` calls
//! cost less than deciding which of the three incompatible watching APIs
//! this operating system is in the mood for.
//!
//! What it does not do is tell *which* file changed. Nothing downstream
//! wants to know: cargo decides what to recompile, and it does it better
//! than a file name would.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// The files of a project, as they were the last time anyone looked.
pub struct Watched {
    roots: Vec<PathBuf>,
    extensions: Vec<String>,
    /// File names the build itself writes, which are therefore not news.
    generated: Vec<String>,
    seen: HashMap<PathBuf, SystemTime>,
}

impl Watched {
    /// Start looking. The first census is now, so that what was on disk
    /// before `rustx dev` started is not a change.
    ///
    /// `generated` is the file names the build produces inside the
    /// directories being watched. They have to be named, because
    /// otherwise the loop feeds itself: the bundle is written into
    /// `assets/` by every build, and the next look would read it as a
    /// reason for the next build. `target/` is excluded for the same
    /// reason, one function down.
    pub fn new(roots: &[PathBuf], extensions: &[&str], generated: &[&str]) -> Self {
        let mut watched = Self {
            roots: roots.to_vec(),
            extensions: extensions.iter().map(|kind| kind.to_string()).collect(),
            generated: generated.iter().map(|name| name.to_string()).collect(),
            seen: HashMap::new(),
        };
        watched.seen = watched.census();
        watched
    }

    /// Has anything changed since the last time this was asked?
    ///
    /// A file saved, a file added, a file deleted — all three, because a
    /// page that is deleted is a route that is gone, and a window still
    /// showing it is showing something that no longer exists.
    pub fn changed(&mut self) -> bool {
        let now = self.census();
        let changed = now != self.seen;
        self.seen = now;
        changed
    }

    fn census(&self) -> HashMap<PathBuf, SystemTime> {
        let mut found = HashMap::new();
        let mut left: Vec<PathBuf> = self.roots.clone();
        while let Some(directory) = left.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    if worth_walking(&path) {
                        left.push(path);
                    }
                } else if self.interesting(&path) {
                    let when = entry
                        .metadata()
                        .and_then(|about| about.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    found.insert(path, when);
                }
            }
        }
        found
    }

    fn interesting(&self, path: &Path) -> bool {
        let named = path.file_name().and_then(|name| name.to_str());
        if named.is_some_and(|name| self.generated.iter().any(|written| written == name)) {
            return false;
        }
        path.extension()
            .and_then(|kind| kind.to_str())
            .is_some_and(|kind| self.extensions.iter().any(|wanted| wanted == kind))
    }
}

/// Directories a project has that are not a project.
///
/// `target` above all: it changes during every build this loop starts,
/// and watching it would mean a build that triggers the next one.
fn worth_walking(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !matches!(name, "target" | "node_modules" | ".git" | ".superpowers") && !name.starts_with('.')
}

/// Waiting for the typing to stop.
///
/// An editor that formats on save writes a file three times in a
/// hundred milliseconds, and a developer saves in bursts. Each change
/// pushes the rebuild back; it starts once nothing has changed for a
/// breath. Ten rebuilds would be nine windows opened and closed in front
/// of somebody who is still typing.
pub struct Coalescing {
    quiet: Duration,
    /// When the last change was seen, if one is waiting.
    last: Option<Instant>,
}

impl Coalescing {
    pub fn new(quiet: Duration) -> Self {
        Self { quiet, last: None }
    }

    /// Say what the last look found; get back whether it is time to
    /// rebuild.
    ///
    /// `now` is given rather than taken, so a test can make a breath pass
    /// without spending one.
    pub fn tick(&mut self, changed: bool, now: Instant) -> bool {
        if changed {
            self.last = Some(now);
            return false;
        }
        match self.last {
            Some(at) if now.duration_since(at) >= self.quiet => {
                self.last = None;
                true
            }
            _ => false,
        }
    }

    /// Is a rebuild waiting for the typing to stop?
    pub fn waiting(&self) -> bool {
        self.last.is_some()
    }
}
