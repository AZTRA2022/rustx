//! `rustx dev`.
//!
//! Save a page, see it. What that means here is honest and said out
//! loud: the application is rebuilt and relaunched, quickly, and the
//! shared state and the page that was open come back with it. Live
//! reloading of native code, with the process never dying, is the
//! subsystem the design document names as its own thing — this is not
//! it, and does not pretend to be.
//!
//! The loop is kept apart from what it launches, so that a test can ask
//! whether a failed build leaves the window alone without compiling
//! anything.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use crate::build;
use crate::watch::{Coalescing, Watched};

/// What one turn of the loop did.
#[derive(Debug)]
pub enum Outcome {
    Ran(PathBuf),
    /// What the compiler said, which is already at the coordinates of
    /// the `.rsx`.
    Failed(String),
}

/// Build, and launch only if the build worked.
///
/// The whole of the rule in Review Focus 1: a typo must not take the
/// window off the screen. Building and launching are handed in so that
/// what is being tested is the rule, not cargo.
pub fn once(
    build: &mut impl FnMut() -> Result<PathBuf, String>,
    run: &mut impl FnMut(&Path),
) -> Outcome {
    match build() {
        Ok(exe) => {
            run(&exe);
            Outcome::Ran(exe)
        }
        Err(said) => Outcome::Failed(said),
    }
}

/// How long the typing has to stop before a rebuild starts.
const QUIET: Duration = Duration::from_millis(200);
/// How often the files are looked at.
const EVERY: Duration = Duration::from_millis(100);

/// Watch this project until its window is closed.
pub fn watch(project: &Path) -> Result<(), String> {
    // Read once, to fail early on a directory that is not a project.
    build::read(project)?;

    let mut watched = Watched::new(
        &[
            project.join("app"),
            project.join("src"),
            project.join("assets"),
        ],
        &["rsx", "rs", "css", "js", "jsx", "ts", "tsx", "json", "html"],
    );
    let mut coalescing = Coalescing::new(QUIET);

    let mut running: Option<Child> = None;
    restart(project, &mut running);

    loop {
        if coalescing.tick(watched.changed(), Instant::now()) {
            println!("rustx: something changed — rebuilding.");
            restart(project, &mut running);
        }

        // The developer closed the window: that is how `rustx dev` ends,
        // the same way `cargo run` ends. Not while a rebuild is waiting,
        // because then the window went away because we are about to put
        // a new one there.
        if let Some(child) = running.as_mut()
            && !coalescing.waiting()
            && matches!(child.try_wait(), Ok(Some(_)))
        {
            return Ok(());
        }

        std::thread::sleep(EVERY);
    }
}

/// Build, and put the new window where the old one was.
///
/// The order matters: the previous process is killed only once the new
/// binary exists. A failed build leaves what is on screen on screen,
/// showing the last page that compiled.
fn restart(project: &Path, running: &mut Option<Child>) {
    match build::run(false, project) {
        Ok(exe) => {
            if let Some(child) = running.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            *running = None;
            match std::process::Command::new(&exe)
                .current_dir(project)
                .spawn()
            {
                Ok(child) => *running = Some(child),
                Err(error) => eprintln!("rustx: {} could not be started: {error}", exe.display()),
            }
        }
        Err(said) => {
            // Cargo has already printed its complaint, at the
            // coordinates of the `.rsx`; an empty one means just that.
            if !said.is_empty() {
                eprintln!("rustx: {said}");
            }
            if running.is_some() {
                eprintln!("rustx: the window on screen is the last page that compiled.");
            }
        }
    }
}
