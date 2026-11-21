//! The stub of a self-extracting installer: `rustx package` appends the
//! application to a copy of this program, and running the copy installs
//! it. No installer tool, no administrator: it goes under
//! `%LOCALAPPDATA%\Programs`, with a Start menu shortcut and an entry in
//! "Apps & features" that runs this same program with `--uninstall`.
//!
//! `/S` installs without asking (a script, a test). Windows only: macOS and
//! Linux have their own formats (`.pkg`, `.deb`).
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("rustx-setup installs on Windows. macOS and Linux get a .pkg or a .deb.");
    std::process::exit(1);
}

#[cfg(windows)]
fn main() {
    if let Err(why) = windows::run() {
        windows::say(&format!("The installation failed: {why}"), "Setup", false);
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod windows {
    use rustx_cli::selfextract::{Meta, locate, unpack};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    const NO_WINDOW: u32 = 0x0800_0000;
    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

    fn hidden(program: &str) -> Command {
        let mut command = Command::new(program);
        command.creation_flags(NO_WINDOW);
        command
    }

    /// A message box, through PowerShell: no GUI dependency in this stub.
    /// Text travels in the environment, so no quote in a name can end the script.
    pub fn say(text: &str, title: &str, ask: bool) -> bool {
        let buttons = if ask { "YesNo" } else { "OK" };
        let script = format!(
            "Add-Type -AssemblyName PresentationFramework; \
             $r=[System.Windows.MessageBox]::Show($env:RX_TEXT,$env:RX_TITLE,'{buttons}','Information'); \
             if($r -eq 'No'){{exit 1}}"
        );
        hidden("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &script,
            ])
            .env("RX_TEXT", text)
            .env("RX_TITLE", title)
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
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

    pub fn run() -> Result<(), String> {
        let arguments: Vec<String> = std::env::args().skip(1).collect();
        if arguments.first().map(String::as_str) == Some("--uninstall") {
            let folder = arguments
                .get(1)
                .ok_or("--uninstall needs the install folder")?;
            return uninstall(Path::new(folder), arguments.iter().any(|a| a == "/S"));
        }
        let silent = arguments
            .iter()
            .any(|a| a.eq_ignore_ascii_case("/s") || a == "--silent");
        let exe = std::env::current_exe()
            .map_err(|e| format!("this program could not find itself ({e})"))?;
        let (meta, located) = locate(&exe).ok_or("there is no application inside this file")?;
        let destination = base("LOCALAPPDATA")?
            .join("Programs")
            .join(folder_name(&meta.name));

        if !silent
            && !say(
                &format!(
                    "Install {} {} for you, in\n{}?",
                    meta.name,
                    meta.version,
                    destination.display()
                ),
                &format!("{} setup", meta.name),
                true,
            )
        {
            return Ok(());
        }

        unpack(&exe, located, &destination)?;
        write_uninstaller(&exe, located.stub, &destination)?;
        register(&meta, &destination)?;

        if !silent
            && say(
                &format!("{} is installed. Start it now?", meta.name),
                &format!("{} setup", meta.name),
                true,
            )
        {
            let _ = Command::new(destination.join(&meta.binary))
                .current_dir(&destination)
                .spawn();
        }
        Ok(())
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
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                script,
            ])
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
        registry(
            &meta.name,
            &["/v", "UninstallString", "/t", "REG_SZ", "/d", &uninstall],
        );
        Ok(())
    }

    fn uninstall(folder: &Path, silent: bool) -> Result<(), String> {
        let meta_name = folder
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !silent
            && !say(
                &format!("Remove {meta_name} from this computer?"),
                &format!("{meta_name} setup"),
                true,
            )
        {
            return Ok(());
        }
        if let Ok(link) = shortcut(&meta_name) {
            let _ = std::fs::remove_file(link);
        }
        let key = format!(r"HKCU\{UNINSTALL}\{}", folder_name(&meta_name));
        let _ = hidden("reg").args(["delete", &key, "/f"]).status();
        // This program lives in the folder it removes, so a second process
        // waits for it to end and then deletes the folder.
        hidden("cmd")
            .raw_arg(r#"/C ping -n 3 127.0.0.1 >nul & rmdir /s /q "%RX_DIR%""#)
            .env("RX_DIR", folder)
            .spawn()
            .map_err(|e| format!("the folder could not be removed ({e})"))?;
        Ok(())
    }
}
