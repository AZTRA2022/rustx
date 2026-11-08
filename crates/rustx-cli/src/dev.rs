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
use std::sync::atomic::{AtomicU64, Ordering};
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
        // The bundle is written into `assets/` by every build. Watching
        // it would mean one saved page becomes a rebuild for ever.
        &[rustx_compiler::bundle::SERVED],
    );
    let mut coalescing = Coalescing::new(QUIET);

    // Where the application writes what is to cross the restart: the
    // values with a name, and the page that is open. In `target/`,
    // because it is a build artefact and `target/` is the one directory
    // a project already ignores.
    let carried = carried_file(project);
    let _ = std::fs::remove_file(&carried);

    let mut running: Option<Child> = None;
    restart(project, &carried, &mut running);

    loop {
        if coalescing.tick(watched.changed(), Instant::now()) {
            println!("rustx: something changed — rebuilding.");
            restart(project, &carried, &mut running);
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

/// The file the application and the loop pass state through.
///
/// One per project, beside the binary it belongs to.
pub fn carried_file(project: &Path) -> PathBuf {
    project.join("target").join("rustx-dev-state.txt")
}

/// Where the copies that are actually launched live.
///
/// Under `target/`, because they are build artefacts and `target/` is
/// the one directory a project already ignores.
pub fn copies(project: &Path) -> PathBuf {
    project.join("target").join("rustx-dev")
}

/// Copy what cargo built, and give back the copy to launch.
///
/// A running `.exe` is held by Windows: cargo cannot replace it and
/// `resources::stamp` cannot reopen it to put the logo in, so a loop
/// that launches `target/debug/<bin>.exe` directly refuses its own next
/// build with an access error on every save. The rule above -- the old
/// process is killed only once the new binary exists -- is what makes
/// the file busy at exactly the wrong moment, and it is a rule worth
/// keeping: a typo must not take the window off the screen. So the
/// process holds a copy instead, and the file cargo writes is never
/// held by anybody.
///
/// Each turn gets its own directory, because the copy of the turn
/// before is still running while this one is made. The turn is in the
/// directory's name and not the file's, so that the application keeps
/// the name the developer gave it -- in a task manager, and anywhere
/// else a running program is called by its file.
pub fn launchable(project: &Path, exe: &Path) -> Result<PathBuf, String> {
    let turn = TURN.fetch_add(1, Ordering::Relaxed);
    let directory = copies(project).join(format!("{}-{turn}", std::process::id()));
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("{} could not be made ({error}).", directory.display()))?;

    let named = exe.file_name().unwrap_or(std::ffi::OsStr::new("app.exe"));
    let copy = directory.join(named);
    std::fs::copy(exe, &copy).map_err(|error| {
        format!(
            "{} could not be copied to {} ({error}).",
            exe.display(),
            copy.display()
        )
    })?;

    // Tidying up is a courtesy and not the job: a copy from a turn
    // whose process has not quite finished dying is still held, and
    // that must not turn a build that worked into a failure.
    if let Ok(entries) = std::fs::read_dir(copies(project)) {
        for entry in entries.flatten() {
            if entry.path() == directory {
                continue;
            }
            let _ = std::fs::remove_dir_all(entry.path());
            let _ = std::fs::remove_file(entry.path());
        }
    }

    Ok(copy)
}

/// Which turn of the loop this is, so two copies never share a name.
static TURN: AtomicU64 = AtomicU64::new(0);

/// Wait for a killed process to let go of the runtime files.
///
/// `kill` and `wait` return before Windows has unmapped the libraries
/// the process had loaded — a moment, no more, and a build started
/// inside it fails on an access error that has no cause left to find.
/// Bounded, because waiting for ever for something that is not coming
/// would be worse than the error.
fn wait_until_free(project: &Path) {
    for _ in 0..40 {
        if build::held(project, false).is_none() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Build, and put the new window where the old one was.
///
/// The order matters: the previous process is killed only once the new
/// binary exists. A failed build leaves what is on screen on screen,
/// showing the last page that compiled.
fn restart(project: &Path, carried: &Path, running: &mut Option<Child>) {
    // The one case where the window on screen has to go first: CEF's
    // build replaces `libcef.dll` beside the binary, and the window
    // that is open has it loaded. The build would fail on an access
    // error for as long as that window lives, so a loop that keeps it
    // is a loop that never builds again. Said out loud, because the
    // rule it bends — a failed build leaves the last page that
    // compiled on screen — is one a developer counts on.
    if let Some(file) = build::held(project, false)
        && let Some(child) = running.as_mut()
    {
        println!(
            "rustx: the window has to close first — it holds {}.",
            file.display()
        );
        let _ = child.kill();
        let _ = child.wait();
        *running = None;
        wait_until_free(project);
    }

    match build::run(false, project) {
        Ok(exe) => {
            // The copy is made before anything is killed, so that a
            // copy that cannot be made leaves the window on screen for
            // the same reason a build that fails does.
            let exe = match launchable(project, &exe) {
                Ok(copy) => copy,
                Err(said) => {
                    eprintln!("rustx: {said}");
                    if running.is_some() {
                        eprintln!("rustx: the window on screen is the last page that compiled.");
                    }
                    return;
                }
            };
            if let Some(child) = running.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            *running = None;
            match std::process::Command::new(&exe)
                .current_dir(project)
                // The only thing in the world that sets this: an
                // application a developer built and ran keeps nothing
                // and reads nothing.
                .env("RUSTX_CARRIED", carried)
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
