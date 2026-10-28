//! Turning this executable into CEF's own sub-processes.
//!
//! This is the hinge the whole framework turns on. CEF is multi-process: it
//! relaunches the host executable once per sub-process, and the render
//! process is the one that holds Blink and V8. By answering that relaunch
//! ourselves, the application's Rust code ends up living inside the render
//! process — so an event handler is a function call, not a message.
//!
//! [`bootstrap`] must be the first statement of `main`.

use crate::cef_app::child_app;
use crate::init::declare_api_version;

/// Which half of the relaunch this process is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bootstrap {
    /// The process the user started. Carry on and open a window.
    Browser,
    /// A sub-process CEF asked for; it has already run to completion.
    /// Exit immediately with this code, without touching anything else.
    Child { exit_code: i32 },
}

/// The job CEF gave this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The main process: owns the windows and the application's state.
    Browser,
    /// Holds Blink and V8, and the application code that talks to them.
    Renderer,
    /// GPU, utility, zygote — processes the framework does not run code in.
    Other,
}

/// Run the sub-process entry point, and say what this process is for.
///
/// Call this before anything else in `main`:
///
/// ```no_run
/// # use rustx_engine::{bootstrap, Bootstrap};
/// if let Bootstrap::Child { exit_code } = bootstrap() {
///     std::process::exit(exit_code);
/// }
/// ```
pub fn bootstrap() -> Bootstrap {
    // Must precede every other CEF call, in every process.
    declare_api_version();

    // CEF returns >= 0 when this process was one of its sub-processes and the
    // work is done; -1 when this is the process the user launched.
    let code = cef::execute_process(
        Some(cef::args::Args::new().as_main_args()),
        Some(&mut child_app()),
        std::ptr::null_mut(),
    );

    if code >= 0 {
        Bootstrap::Child { exit_code: code }
    } else {
        Bootstrap::Browser
    }
}

/// Reading the process role off the command line.
pub mod process {
    use super::Role;

    /// CEF names the sub-process kind with this switch. Its absence means the
    /// browser process, which is the one the user started.
    const TYPE_SWITCH: &str = "--type=";

    /// What this process was launched to do.
    pub fn role() -> Role {
        role_from(std::env::args())
    }

    pub(crate) fn role_from(args: impl Iterator<Item = String>) -> Role {
        for arg in args {
            let Some(kind) = arg.strip_prefix(TYPE_SWITCH) else {
                continue;
            };
            return match kind {
                "renderer" => Role::Renderer,
                _ => Role::Other,
            };
        }
        Role::Browser
    }
}

#[cfg(test)]
mod tests {
    use super::process::role_from;
    use super::Role;

    fn args(list: &[&str]) -> impl Iterator<Item = String> {
        list.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_iter()
    }

    #[test]
    fn no_type_switch_means_the_browser_process() {
        assert_eq!(role_from(args(&["hello.exe", "--my-option=42"])), Role::Browser);
    }

    #[test]
    fn the_renderer_is_told_apart_from_the_other_sub_processes() {
        assert_eq!(role_from(args(&["hello.exe", "--type=renderer"])), Role::Renderer);
        assert_eq!(role_from(args(&["hello.exe", "--type=gpu-process"])), Role::Other);
        assert_eq!(role_from(args(&["hello.exe", "--type=utility"])), Role::Other);
    }
}
