//! The application as its own installer.
//!
//! A binary that `rustx package` stamped installable shows the pages of
//! `system/` (install, update, uninstall) instead of the application when it
//! is run outside its install location. The pages call what is here:
//!
//! ```ignore
//! let info = rustx::setup::info();          // name, version, destination, mode…
//! rustx::setup::install(|percent| progress = percent as u32)?;   // in a task
//! rustx::setup::launch()?;                  // start the installed copy
//! rustx::setup::quit();
//! ```
//!
//! Never in a debug build, under `cargo run` or under `rustx dev`: a
//! binary being developed does not copy itself. `RUSTX_FORCE_INSTALL=1` asks
//! for it anyway, for tests. `Config::portable()` opts an application out.

use std::path::PathBuf;
use std::sync::OnceLock;

use rustx_os::install::{self, App, Stamp};

/// What this start is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The application itself.
    Normal,
    /// Not installed: the install wizard.
    Install,
    /// A different version is installed: the update wizard.
    Update,
    /// Started to remove the installed copy.
    Uninstall,
}

impl Mode {
    /// The page of `system/` this mode shows.
    pub fn page(self) -> Option<&'static str> {
        match self {
            Mode::Normal => None,
            Mode::Install => Some("/__system/install"),
            Mode::Update => Some("/__system/update"),
            Mode::Uninstall => Some("/__system/uninstall"),
        }
    }
}

/// What a page may know.
#[derive(Debug, Clone)]
pub struct Info {
    pub mode: Mode,
    pub name: String,
    pub version: String,
    pub publisher: String,
    /// Where it installs.
    pub destination: String,
    /// The version already installed, when there is one.
    pub installed_version: Option<String>,
    /// The licence text, when the project has a `LICENSE`.
    pub license: String,
}

struct Detected {
    mode: Mode,
    stamp: Option<Stamp>,
}

static DETECTED: OnceLock<Detected> = OnceLock::new();

fn arguments() -> Vec<String> {
    std::env::args().skip(1).collect()
}

/// Decide the mode once, at the start. `portable` is the application's own
/// opt-out.
pub fn detect(portable: bool) -> Mode {
    DETECTED
        .get_or_init(|| {
            let stamp = install::read_stamp();
            let asked = arguments();
            let mode = match &stamp {
                None => Mode::Normal,
                Some(_) if asked.iter().any(|a| a == "--uninstall") => Mode::Uninstall,
                Some(_) if portable => Mode::Normal,
                Some(_)
                    if cfg!(debug_assertions)
                        && std::env::var_os("RUSTX_FORCE_INSTALL").is_none() =>
                {
                    Mode::Normal
                }
                Some(stamp) => {
                    let app = app_of(stamp);
                    if install::is_installed_here(&app) {
                        Mode::Normal
                    } else {
                        match install::installed_version(&stamp.name) {
                            Some(was) if was != stamp.version => Mode::Update,
                            _ => Mode::Install,
                        }
                    }
                }
            };
            Detected { mode, stamp }
        })
        .mode
}

fn app_of(stamp: &Stamp) -> App {
    App {
        name: stamp.name.clone(),
        version: stamp.version.clone(),
        publisher: stamp.publisher.clone(),
        binary: std::env::current_exe().unwrap_or_default(),
        icon: Some(crate::assets_dir().join("icon.png")),
    }
}

fn app() -> Result<App, String> {
    DETECTED
        .get()
        .and_then(|d| d.stamp.as_ref())
        .map(app_of)
        .ok_or_else(|| "this binary was not stamped installable by `rustx package`".to_string())
}

/// The mode of this start.
pub fn mode() -> Mode {
    DETECTED.get().map(|d| d.mode).unwrap_or(Mode::Normal)
}

/// What a page may know.
pub fn info() -> Info {
    let (mode, stamp) = DETECTED
        .get()
        .map(|d| (d.mode, d.stamp.clone()))
        .unwrap_or((Mode::Normal, None));
    // Not stamped (a debug run, a portable app): what the application says it is.
    let about = crate::rustx_engine::window::about();
    let stamp = stamp.unwrap_or(Stamp {
        name: about.name,
        version: about.version,
        publisher: about.publisher,
        license: String::new(),
    });
    Info {
        mode,
        destination: install::destination(&stamp.name)
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        installed_version: install::installed_version(&stamp.name),
        license: if stamp.license.is_empty() {
            license_text()
        } else {
            stamp.license.clone()
        },
        name: stamp.name,
        version: stamp.version,
        publisher: stamp.publisher,
    }
}

fn license_text() -> String {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from));
    for name in ["LICENSE", "LICENSE.txt", "LICENSE.md"] {
        let in_assets = std::fs::read_to_string(crate::assets_dir().join(name)).ok();
        let near = beside
            .as_ref()
            .and_then(|dir| std::fs::read_to_string(dir.join(name)).ok());
        if let Some(text) = in_assets.or(near) {
            return text;
        }
    }
    String::new()
}

/// Copy this application into place and register it. Blocks: call it in a
/// `task`. `progress` gets 0 to 100.
pub fn install(mut progress: impl FnMut(u8)) -> Result<(), String> {
    let app = app()?;
    install::install(&app, &mut progress)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Start the installed copy, then [`quit`].
pub fn launch() -> Result<(), String> {
    let app = app()?;
    let binary = install::installed_binary(&app).map_err(|e| e.to_string())?;
    let started = if cfg!(target_os = "macos") {
        let bundle = install::destination(&app.name).map_err(|e| e.to_string())?;
        std::process::Command::new("open").arg(bundle).spawn()
    } else {
        std::process::Command::new(&binary)
            .current_dir(binary.parent().unwrap_or(&binary))
            .spawn()
    };
    started
        .map(|_| ())
        .map_err(|e| format!("could not start {}: {e}", binary.display()))
}

/// Unregister and remove the installed copy.
pub fn uninstall() -> Result<(), String> {
    let app = app()?;
    install::uninstall(&app.name).map_err(|e| e.to_string())
}

/// Close the window.
pub fn quit() {
    crate::window::shut();
}

/// `--install --silent` and `--uninstall --silent`: no window, for scripts and
/// CI. Returns the exit code when it handled the start.
pub fn silent() -> Option<i32> {
    let asked = arguments();
    if !asked
        .iter()
        .any(|a| a == "--silent" || a.eq_ignore_ascii_case("/s"))
    {
        return None;
    }
    let result = match mode() {
        Mode::Uninstall => uninstall(),
        Mode::Install | Mode::Update if asked.iter().any(|a| a == "--install") => install(|_| {}),
        _ => return None,
    };
    Some(match result {
        Ok(()) => 0,
        Err(why) => {
            eprintln!("rustx: {why}");
            1
        }
    })
}

// ------------------------------------------------- the other OS-facing screens

/// The permissions the application declared (`Config::permissions`), as the
/// permissions page lists them.
pub fn permissions() -> Vec<String> {
    PERMISSIONS.with(|held| held.borrow().clone())
}

thread_local! {
    static PERMISSIONS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub(crate) fn declare_permissions(names: Vec<String>) {
    PERMISSIONS.with(|held| *held.borrow_mut() = names);
}

thread_local! {
    static UPDATES: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn declare_updates(repository: Option<String>) {
    UPDATES.with(|held| *held.borrow_mut() = repository);
}

/// Whether the application said where its releases are (`Config::updates`).
pub fn updates_known() -> bool {
    UPDATES.with(|held| held.borrow().is_some())
}

fn repository() -> Result<String, String> {
    UPDATES
        .with(|held| held.borrow().clone())
        .ok_or_else(|| "this application did not say where its releases are".to_string())
}

/// Ask the release page: `Ok(Some(version))` when a newer one exists. Blocks:
/// call it in a `task`.
pub fn check_for_update() -> Result<Option<String>, String> {
    crate::update::newest(&repository()?)
        .map(|found| found.map(|release| release.version))
        .map_err(|e| e.to_string())
}

/// Download the newer release, check it against the published digest, and put
/// it in place. Blocks: call it in a `task`. `progress` gets 0 to 100.
pub fn install_update(mut progress: impl FnMut(u8)) -> Result<(), String> {
    crate::update::from_github_watching(&repository()?, &mut |seen, of| {
        if let Some(percent) = (seen * 100).checked_div(of) {
            progress(percent.min(100) as u8);
        }
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Start the updated program and leave.
pub fn restart() -> Result<(), String> {
    crate::update::restart().map_err(|e| e.to_string())
}

/// Show one of the OS-facing screens of `system/`: `license`, `permissions`,
/// `about` or `settings`. [`close`] goes back to where the application was.
pub fn open(screen: &str) {
    crate::router::navigate(format!("/__system/{screen}"));
}

/// Leave an OS-facing screen.
pub fn close() {
    if !crate::router::back() {
        crate::router::navigate("/");
    }
}

/// What the system says about a permission: `Allowed`, `Blocked`, or
/// `Asked on first use` where the system exposes nothing (macOS and Linux
/// answer at the moment of use; Windows keeps a consent store).
pub fn permission_state(name: &str) -> String {
    let asked = "Asked on first use".to_string();
    if !cfg!(windows) {
        return asked;
    }
    let capability = match name {
        "camera" => "webcam",
        "microphone" => "microphone",
        "location" => "location",
        _ => return asked,
    };
    let key = format!(
        r"HKCU\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\{capability}"
    );
    let mut command = std::process::Command::new("reg");
    command.args(["query", &key, "/v", "Value"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let text = command
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
        .unwrap_or_default();
    state_from_registry(&text).unwrap_or(asked)
}

fn state_from_registry(text: &str) -> Option<String> {
    if text.contains("Allow") {
        Some("Allowed".into())
    } else if text.contains("Deny") {
        Some("Blocked".into())
    } else {
        None
    }
}

/// Open the operating system's own page for a permission (`camera`,
/// `microphone`, `location`, `notifications`, `files`).
pub fn open_permission_settings(name: &str) -> Result<(), String> {
    let target = if cfg!(windows) {
        match name {
            "camera" => "ms-settings:privacy-webcam",
            "microphone" => "ms-settings:privacy-microphone",
            "location" => "ms-settings:privacy-location",
            "notifications" => "ms-settings:notifications",
            _ => "ms-settings:privacy",
        }
    } else if cfg!(target_os = "macos") {
        match name {
            "camera" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera",
            "microphone" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
            }
            "location" => {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices"
            }
            "notifications" => "x-apple.systempreferences:com.apple.preference.notifications",
            _ => "x-apple.systempreferences:com.apple.preference.security",
        }
    } else {
        "privacy"
    };
    let started = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(target).spawn()
    } else {
        std::process::Command::new("gnome-control-center")
            .arg(target)
            .spawn()
    };
    started
        .map(|_| ())
        .map_err(|e| format!("could not open the system settings: {e}"))
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn the_consent_store_is_read() {
        assert_eq!(
            state_from_registry("    Value    REG_SZ    Allow").as_deref(),
            Some("Allowed")
        );
        assert_eq!(
            state_from_registry("    Value    REG_SZ    Deny").as_deref(),
            Some("Blocked")
        );
        assert_eq!(state_from_registry("ERROR: not found"), None);
    }
}
