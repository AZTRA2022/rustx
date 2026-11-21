//! The stub of a self-extracting installer: `rustx package` appends the
//! application to a copy of this program, and running the copy installs
//! it. No installer tool, no administrator: it goes under
//! `%LOCALAPPDATA%\Programs`, with a Start menu shortcut and an entry in
//! "Apps & features" that runs this same program with `--uninstall`.
//!
//! The screens are web pages shown in the system's web view, like the
//! application's own. The project writes them (`installer/install.html`,
//! `installer/uninstall.html`, with their styles and images); a page it did not
//! write is the default one compiled in here. A page talks to this program
//! through `window.setup`:
//!
//! ```text
//! setup.info        { name, version, publisher, destination, license, mode }
//! setup.install()   extract and register; events follow
//! setup.uninstall() unregister and remove; events follow
//! setup.launch()    start the installed application
//! setup.cancel()    close without doing anything
//! setup.close()     close
//! setup.on(fn)      fn({ event: "progress", done, total, file } | { event: "done" } | { event: "error", message })
//! ```
//!
//! `/S` installs (or, with `--uninstall <dir>`, removes) without a window: a
//! script, a test, a CI job. Windows only: macOS and Linux have their own
//! formats (`.pkg`, `.deb`).
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("rustx-setup installs on Windows. macOS and Linux get a .pkg or a .deb.");
    std::process::exit(1);
}

#[cfg(windows)]
fn main() {
    if let Err(why) = windows::run() {
        windows::say(&format!("The setup failed: {why}"), "Setup");
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod windows {
    use rustx_cli::selfextract::{Located, Meta, locate, unpack};
    use std::borrow::Cow;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tao::dpi::{LogicalPosition, LogicalSize};
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
    use tao::platform::run_return::EventLoopExtRunReturn;
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    const NO_WINDOW: u32 = 0x0800_0000;
    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    const INSTALL_PAGE: &str = include_str!("setup-install.html");
    const UNINSTALL_PAGE: &str = include_str!("setup-uninstall.html");
    /// The folder, inside the install folder, that keeps the screens for the uninstaller.
    const KEPT: &str = "installer";

    fn hidden(program: &str) -> Command {
        let mut command = Command::new(program);
        command.creation_flags(NO_WINDOW);
        command
    }

    /// A message box, for what happens before there is a window to say it.
    /// Text travels in the environment, so no quote in a name can end the script.
    pub fn say(text: &str, title: &str) {
        let script = "Add-Type -AssemblyName PresentationFramework; \
             [void][System.Windows.MessageBox]::Show($env:RX_TEXT,$env:RX_TITLE)";
        let _ = hidden("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script])
            .env("RX_TEXT", text)
            .env("RX_TITLE", title)
            .status();
    }

    fn folder_name(name: &str) -> String {
        let cleaned: String = name
            .chars()
            .map(|c| if r#"<>:"/\|?*"#.contains(c) { '-' } else { c })
            .collect();
        let cleaned = cleaned.trim().trim_end_matches('.').to_string();
        if cleaned.is_empty() {
            "App".to_string()
        } else {
            cleaned
        }
    }

    fn base(variable: &str) -> Result<PathBuf, String> {
        std::env::var_os(variable)
            .map(PathBuf::from)
            .ok_or_else(|| format!("%{variable}% is not set"))
    }

    fn shortcut(name: &str) -> Result<PathBuf, String> {
        Ok(base("APPDATA")?
            .join(r"Microsoft\Windows\Start Menu\Programs")
            .join(format!("{}.lnk", folder_name(name))))
    }

    fn registry(name: &str, args: &[&str]) -> bool {
        let key = format!(r"HKCU\{UNINSTALL}\{}", folder_name(name));
        hidden("reg")
            .arg("add")
            .arg(&key)
            .args(args)
            .arg("/f")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Everything an install needs, owned, so a thread can carry it.
    #[derive(Clone)]
    struct Plan {
        meta: Meta,
        exe: PathBuf,
        located: Located,
        destination: PathBuf,
    }

    fn plan() -> Result<Plan, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("this program could not find itself ({e})"))?;
        let (meta, located) = locate(&exe).ok_or("there is no application inside this file")?;
        let destination = base("LOCALAPPDATA")?
            .join("Programs")
            .join(folder_name(&meta.name));
        Ok(Plan { meta, exe, located, destination })
    }

    pub fn run() -> Result<(), String> {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        let silent = arguments
            .iter()
            .any(|a| a.eq_ignore_ascii_case("/s") || a == "--silent");
        if arguments.first().map(String::as_str) == Some("--uninstall") {
            let folder = PathBuf::from(arguments.get(1).ok_or("--uninstall needs the install folder")?);
            return if silent { remove(&folder) } else { uninstall_window(&folder) };
        }
        let plan = plan()?;
        if silent {
            return install(&plan, &mut |_, _| {});
        }
        install_window(plan)
    }

    /// Extract, write the uninstaller, register. `progress(done, file)`.
    fn install(plan: &Plan, progress: &mut dyn FnMut(usize, &str)) -> Result<(), String> {
        let kept = plan.destination.join(KEPT);
        unpack(&plan.exe, plan.located, Some(&plan.destination), Some(&kept), progress)?;
        write_uninstaller(&plan.exe, plan.located.stub, &plan.destination)?;
        register(&plan.meta, &plan.destination)
    }

    /// This program without its payload, inside the install folder.
    fn write_uninstaller(exe: &Path, stub: u64, into: &Path) -> Result<(), String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(exe)
            .and_then(|file| file.take(stub).read_to_end(&mut bytes))
            .and_then(|_| std::fs::write(into.join("uninstall.exe"), bytes))
            .map_err(|e| format!("the uninstaller could not be written ({e})"))
    }

    fn register(meta: &Meta, destination: &Path) -> Result<(), String> {
        let target = destination.join(&meta.binary);
        let link = shortcut(&meta.name)?;
        let script = "$s=(New-Object -ComObject WScript.Shell).CreateShortcut($env:RX_LINK); \
                      $s.TargetPath=$env:RX_TARGET; $s.WorkingDirectory=$env:RX_DIR; $s.Save()";
        if let Some(parent) = link.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let made = hidden("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", script])
            .env("RX_LINK", &link)
            .env("RX_TARGET", &target)
            .env("RX_DIR", destination)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !made {
            return Err("the Start menu shortcut could not be made".to_string());
        }
        let uninstall = format!(
            "\"{}\" --uninstall \"{}\"",
            destination.join("uninstall.exe").display(),
            destination.display()
        );
        let location = destination.display().to_string();
        let icon = target.display().to_string();
        let entries: [(&str, &str); 5] = [
            ("DisplayName", &meta.name),
            ("DisplayVersion", &meta.version),
            ("Publisher", &meta.publisher),
            ("InstallLocation", &location),
            ("DisplayIcon", &icon),
        ];
        for (key, value) in entries {
            registry(&meta.name, &["/v", key, "/t", "REG_SZ", "/d", value]);
        }
        registry(&meta.name, &["/v", "UninstallString", "/t", "REG_SZ", "/d", &uninstall]);
        Ok(())
    }

    fn name_of(folder: &Path) -> String {
        folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// Unregister and remove. This program lives in the folder it removes, so a
    /// second process waits for it to end and then deletes the folder.
    fn remove(folder: &Path) -> Result<(), String> {
        let name = name_of(folder);
        if let Ok(link) = shortcut(&name) {
            let _ = std::fs::remove_file(link);
        }
        let key = format!(r"HKCU\{UNINSTALL}\{}", folder_name(&name));
        let _ = hidden("reg").args(["delete", &key, "/f"]).status();
        hidden("cmd")
            .raw_arg(r#"/C ping -n 3 127.0.0.1 >nul & rmdir /s /q "%RX_DIR%""#)
            .env("RX_DIR", folder)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("the folder could not be removed ({e})"))
    }

    // ------------------------------------------------------------ the window

    enum Wake {
        Script(String),
        Quit,
    }

    /// What a page may ask for.
    #[derive(Clone)]
    enum Job {
        Install(Plan),
        Uninstall(PathBuf),
    }

    fn mime(path: &str) -> &'static str {
        match path.rsplit('.').next().unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "css" => "text/css",
            "js" => "text/javascript",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "ico" => "image/x-icon",
            "woff2" => "font/woff2",
            "txt" => "text/plain; charset=utf-8",
            _ => "application/octet-stream",
        }
    }

    fn emit(proxy: &EventLoopProxy<Wake>, event: serde_json::Value) {
        let _ = proxy.send_event(Wake::Script(format!("window.setup._emit({event})")));
    }

    fn install_window(plan: Plan) -> Result<(), String> {
        // The screens come out first, to a folder of their own that the
        // window serves; the application comes out when the page asks.
        let pages = std::env::temp_dir().join(format!("rustx-setup-pages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pages);
        unpack(&plan.exe, plan.located, None, Some(&pages), &mut |_, _| {})?;
        let license = std::fs::read_to_string(pages.join("LICENSE.txt")).unwrap_or_default();
        let info = serde_json::json!({
            "mode": "install",
            "name": plan.meta.name,
            "version": plan.meta.version,
            "publisher": plan.meta.publisher,
            "destination": plan.destination.display().to_string(),
            "license": license,
        });
        let title = format!("{} setup", plan.meta.name);
        show(&title, &pages, "/install.html", INSTALL_PAGE, info, Job::Install(plan));
        let _ = std::fs::remove_dir_all(&pages);
        Ok(())
    }

    fn uninstall_window(folder: &Path) -> Result<(), String> {
        let name = name_of(folder);
        let info = serde_json::json!({
            "mode": "uninstall",
            "name": name,
            "destination": folder.display().to_string(),
        });
        show(
            &format!("{name} setup"),
            &folder.join(KEPT),
            "/uninstall.html",
            UNINSTALL_PAGE,
            info,
            Job::Uninstall(folder.to_path_buf()),
        );
        Ok(())
    }

    /// Open the window on `entry` (the project's page, or the default one) and
    /// run until it closes.
    fn show(
        title: &str,
        pages: &Path,
        entry: &'static str,
        default: &'static str,
        info: serde_json::Value,
        job: Job,
    ) {
        let mut event_loop = EventLoopBuilder::<Wake>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        let mut window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(LogicalSize::new(560.0, 440.0))
            .with_resizable(false)
            .with_visible(false);
        if let Some(monitor) = event_loop.primary_monitor() {
            let scale = monitor.scale_factor();
            let size = monitor.size();
            window = window.with_position(LogicalPosition::new(
                (size.width as f64 / scale - 560.0) / 2.0,
                (size.height as f64 / scale - 440.0) / 2.0,
            ));
        }
        let window = window.build(&event_loop).expect("a window");

        let served = pages.to_path_buf();
        let startup = format!(
            "window.setup = {{ info: {info}, _on: [], _emit(e) {{ this._on.forEach((f) => f(e)); }}, \
             on(f) {{ this._on.push(f); }}, \
             _send(cmd) {{ window.ipc.postMessage(JSON.stringify({{ cmd }})); }}, \
             install() {{ this._send('install'); }}, uninstall() {{ this._send('uninstall'); }}, \
             launch() {{ this._send('launch'); }}, cancel() {{ this._send('close'); }}, \
             close() {{ this._send('close'); }} }};"
        );
        let handler_proxy = proxy.clone();
        let mut context =
            wry::WebContext::new(Some(std::env::temp_dir().join("rustx-setup-webview")));
        let webview = WebViewBuilder::new_with_web_context(&mut context)
            .with_initialization_script(startup)
            .with_background_color((255, 255, 255, 255))
            .with_custom_protocol("setup".to_string(), move |_id, request| {
                let path = request.uri().path();
                let path = if path == "/" { entry } else { path };
                let relative = path.trim_start_matches('/');
                let safe = !relative
                    .split('/')
                    .any(|part| part == ".." || part.contains(':'));
                let found = safe
                    .then(|| std::fs::read(served.join(relative)).ok())
                    .flatten();
                let (status, body) = match found {
                    Some(bytes) => (200, bytes),
                    None if path == entry => (200, default.as_bytes().to_vec()),
                    None => (404, Vec::new()),
                };
                wry::http::Response::builder()
                    .status(status)
                    .header("Content-Type", mime(path))
                    .body(Cow::Owned(body))
                    .unwrap_or_else(|_| wry::http::Response::new(Cow::Borrowed(&b""[..])))
            })
            .with_ipc_handler(move |request| {
                let Ok(message) = serde_json::from_str::<serde_json::Value>(request.body()) else {
                    return;
                };
                match message["cmd"].as_str() {
                    Some("install") | Some("uninstall") => {
                        let (job, proxy) = (job.clone(), handler_proxy.clone());
                        std::thread::spawn(move || {
                            let result = match &job {
                                Job::Install(plan) => {
                                    let total = plan.meta.files;
                                    let mut tell = |done: usize, file: &str| {
                                        emit(
                                            &proxy,
                                            serde_json::json!({
                                                "event": "progress", "done": done,
                                                "total": total, "file": file
                                            }),
                                        );
                                    };
                                    install(plan, &mut tell)
                                }
                                Job::Uninstall(folder) => remove(folder),
                            };
                            match result {
                                Ok(()) => emit(&proxy, serde_json::json!({ "event": "done" })),
                                Err(message) => emit(
                                    &proxy,
                                    serde_json::json!({ "event": "error", "message": message }),
                                ),
                            }
                        });
                    }
                    Some("launch") => {
                        if let Job::Install(plan) = &job {
                            let _ = Command::new(plan.destination.join(&plan.meta.binary))
                                .current_dir(&plan.destination)
                                .spawn();
                        }
                        let _ = handler_proxy.send_event(Wake::Quit);
                    }
                    Some("close") => {
                        let _ = handler_proxy.send_event(Wake::Quit);
                    }
                    _ => {}
                }
            })
            .with_url("http://setup.localhost/")
            .build(&window)
            .expect("a web view");

        // Shown once there is a loop to show it from: no empty frame first.
        let shown = std::cell::Cell::new(false);
        event_loop.run_return(move |event, _, flow| {
            *flow = ControlFlow::Wait;
            if !shown.replace(true) {
                window.set_visible(true);
            }
            match event {
                Event::UserEvent(Wake::Script(code)) => {
                    let _ = webview.evaluate_script(&code);
                }
                Event::UserEvent(Wake::Quit)
                | Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *flow = ControlFlow::Exit;
                }
                _ => {}
            }
        });
    }
}
