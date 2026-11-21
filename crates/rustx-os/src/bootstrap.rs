//! The runtime an application needs before it can draw anything.
//!
//! A self-installing program shows its own install screen in a web view, so it
//! meets a machine with no web view runtime before it has drawn a pixel. On
//! Windows that runtime is Microsoft's WebView2: this asks (a native box, since
//! there is no page yet), downloads Microsoft's Evergreen bootstrapper and runs
//! it. On Linux there is no installing it from here: the missing package is
//! named. macOS always has WKWebView.

use crate::error::OsError;

/// What `start` returned: the runtime is there now, and the program should start again.
pub const MICROSOFT_BOOTSTRAPPER: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

/// Does this start-up failure say the web view runtime is missing?
pub fn is_missing_runtime(failure: &str) -> bool {
    let lowered = failure.to_ascii_lowercase();
    lowered.contains("webview2 runtime") || lowered.contains("webkit")
}

/// The package a Linux user installs, named for the common distributions.
pub fn linux_package_hint() -> &'static str {
    "the WebKitGTK web view is missing: install `libwebkit2gtk-4.1-0` (Debian, Ubuntu), \
     `webkit2gtk4.1` (Fedora) or `webkit2gtk-4.1` (Arch), then start the program again"
}

/// Download the Evergreen bootstrapper from `url` and run it. `url` is
/// [`MICROSOFT_BOOTSTRAPPER`] unless `RUSTX_BOOTSTRAP_URL` says otherwise (a
/// test over loopback). Returns the installer's exit code.
pub fn run_installer(url: &str, app: &str) -> Result<i32, OsError> {
    let folder = crate::data::cache(app)?.join("runtime");
    std::fs::create_dir_all(&folder)
        .map_err(|e| OsError::Platform(format!("{} could not be made ({e})", folder.display())))?;
    let exe = folder.join("MicrosoftEdgeWebview2Setup.exe");
    crate::platform::http::download(
        url,
        &format!("{app} (rustx bootstrapper)"),
        &exe,
        &mut |_| {},
    )?;
    let mut command = std::process::Command::new(&exe);
    command.args(["/silent", "/install"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let status = command
        .status()
        .map_err(|e| OsError::Platform(format!("the runtime installer did not start ({e})")))?;
    let _ = std::fs::remove_file(&exe);
    Ok(status.code().unwrap_or(-1))
}

/// Ask, then install the runtime. `Ok(true)` when it was installed and the
/// program should start again; `Ok(false)` when the person said no.
#[cfg(windows)]
pub fn offer_runtime(app: &str) -> Result<bool, OsError> {
    use windows::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONQUESTION, MB_YESNO, MessageBoxW};
    use windows::core::PCWSTR;
    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().chain(Some(0)).collect() };
    let silent = std::env::var_os("RUSTX_BOOTSTRAP").is_some_and(|v| v == "auto");
    if !silent {
        let text = wide(&format!(
            "{app} needs the Microsoft Edge WebView2 Runtime to show its windows, and this \
             computer does not have it.\n\nDownload and install it now?"
        ));
        let title = wide(app);
        // SAFETY: both buffers are nul-terminated and live through the call.
        let answer = unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MB_YESNO | MB_ICONQUESTION,
            )
        };
        if answer != IDYES {
            return Ok(false);
        }
    }
    let url =
        std::env::var("RUSTX_BOOTSTRAP_URL").unwrap_or_else(|_| MICROSOFT_BOOTSTRAPPER.into());
    match run_installer(&url, app)? {
        0 | 3010 => Ok(true),
        code => Err(OsError::Platform(format!(
            "the WebView2 installer ended with code {code}"
        ))),
    }
}

/// No runtime to install from here: say what is missing.
#[cfg(not(windows))]
pub fn offer_runtime(_app: &str) -> Result<bool, OsError> {
    Err(OsError::Platform(linux_package_hint().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_up_failure_of_a_missing_runtime_is_recognised() {
        assert!(is_missing_runtime(
            "could not start the webview: x. The Microsoft Edge WebView2 Runtime may be missing"
        ));
        assert!(!is_missing_runtime("the port is busy"));
    }
}
