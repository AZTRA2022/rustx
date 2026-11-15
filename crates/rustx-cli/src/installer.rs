//! The installer, and the code signing that goes with it.
//!
//! The documentation said "there is no installer, no code signing and
//! no updater", and the updater is written now. This is the other two.
//!
//! What an installer is, per platform, is not a choice this makes: it
//! is what the platform's users already double-click. Windows has a
//! setup `.exe`; macOS has a `.dmg` you drag into Applications; Linux
//! has a `.desktop` file and a script that puts it where the desktop
//! looks. So that is what each one gets, and no format is invented.
//!
//! Nothing here fails the package. An installer needs a third-party
//! tool on Windows and a Mac on macOS, and a developer who has neither
//! still wants the folder and the archive that [`crate::package`] just
//! made. When the tool is missing, the script this would have fed it is
//! written down beside the folder, and the sentence says which tool
//! reads it.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::build;

/// What came of it.
#[derive(Debug, Clone, Default)]
pub struct Made {
    /// The installer, when one was produced.
    pub path: Option<PathBuf>,
    /// The script or recipe, when it was written but not run.
    pub recipe: Option<PathBuf>,
    /// One line for the developer: what happened, and what to install
    /// if nothing did.
    pub note: String,
}

/// Make the installer this platform's users would expect, from a folder
/// [`crate::package`] has already filled.
pub fn make(read: &build::Project, folder: &Path, binary: &str) -> Made {
    if cfg!(target_os = "windows") {
        windows(read, folder, binary)
    } else if cfg!(target_os = "macos") {
        macos(read, folder)
    } else {
        unix(read, folder, binary)
    }
}

/// A stable `AppId` for Inno Setup, derived from the application's name.
///
/// Inno uses it to recognise an upgrade of the same application rather
/// than a second one beside it, so it has to be the same GUID at every
/// release and different from every other application's. A digest of
/// the name is both, and it needs nothing kept in a file.
fn app_id(name: &str) -> String {
    let digest = rustx_os::digest::sha256_hex(name.as_bytes());
    format!(
        "{}-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[12..16],
        &digest[16..20],
        &digest[20..32]
    )
    .to_uppercase()
}

/// Inno Setup's own escape: a quote doubles, and nothing else is
/// special in the value of a directive.
fn inno(text: &str) -> String {
    text.replace('"', "\"\"")
}

/// The Inno Setup script for this release.
pub fn inno_script(read: &build::Project, folder: &Path, binary: &str) -> String {
    let exe = format!("{binary}.exe");
    let out = folder.parent().unwrap_or(folder);
    let base = folder
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .unwrap_or_else(|| read.about.name.clone());
    format!(
        "; Written by `rustx package`. Edit it if you like: it is read by\n\
         ; ISCC.exe and by nothing in this framework.\n\
         [Setup]\n\
         AppId={{{{{id}}}\n\
         AppName={name}\n\
         AppVersion={version}\n\
         AppPublisher={publisher}\n\
         AppVerName={name} {version}\n\
         DefaultDirName={{autopf}}\\{name}\n\
         DefaultGroupName={name}\n\
         UninstallDisplayIcon={{app}}\\{exe}\n\
         OutputDir={out}\n\
         OutputBaseFilename={base}-setup\n\
         Compression=lzma2/max\n\
         SolidCompression=yes\n\
         ; Per user, so that installing does not ask for administrator\n\
         ; rights: `{{autopf}}` becomes the user's own folder when the\n\
         ; install is not elevated.\n\
         PrivilegesRequired=lowest\n\
         PrivilegesRequiredOverridesAllowed=dialog\n\
         ArchitecturesInstallIn64BitMode=x64compatible\n\
         ArchitecturesAllowed=x64compatible\n\
         WizardStyle=modern\n\
         \n\
         [Files]\n\
         ; The whole folder: the `.exe` starts itself as Chromium's\n\
         ; helper processes and reads `libcef.dll`, the `.pak` files and\n\
         ; `locales\\` from beside itself, so a file left out is a\n\
         ; program that does not start.\n\
         Source: \"{source}\\*\"; DestDir: \"{{app}}\"; \
         Flags: recursesubdirs createallsubdirs ignoreversion\n\
         \n\
         [Icons]\n\
         Name: \"{{group}}\\{name}\"; Filename: \"{{app}}\\{exe}\"\n\
         Name: \"{{autodesktop}}\\{name}\"; Filename: \"{{app}}\\{exe}\"; \
         Tasks: desktopicon\n\
         \n\
         [Tasks]\n\
         Name: \"desktopicon\"; Description: \"Put {name} on the desktop\"\n\
         \n\
         [Run]\n\
         Filename: \"{{app}}\\{exe}\"; Description: \"Start {name}\"; \
         Flags: nowait postinstall skipifsilent\n",
        id = app_id(&read.about.name),
        name = inno(&read.about.name),
        version = inno(&read.about.version),
        publisher = inno(&read.about.publisher),
        out = out.display(),
        source = folder.display(),
    )
}

/// `ISCC.exe`: on the PATH, or where the installer of Inno Setup 6
/// puts it.
fn iscc() -> Option<PathBuf> {
    if Command::new("ISCC")
        .arg("/?")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return Some(PathBuf::from("ISCC"));
    }
    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = std::env::var_os(variable) {
            let guess = PathBuf::from(root).join("Inno Setup 6/ISCC.exe");
            if guess.is_file() {
                return Some(guess);
            }
        }
    }
    None
}

fn windows(read: &build::Project, folder: &Path, binary: &str) -> Made {
    let script = folder
        .parent()
        .unwrap_or(folder)
        .join(format!("{}.iss", crate::package::slug(&read.about.name)));
    let text = inno_script(read, folder, binary);
    if let Err(error) = std::fs::write(&script, &text) {
        return Made {
            note: format!("{} could not be written ({error}).", script.display()),
            ..Made::default()
        };
    }

    let Some(iscc) = iscc() else {
        return Made {
            recipe: Some(script.clone()),
            note: format!(
                "no installer: Inno Setup is not installed. The script is written at {}. \
                 Install Inno Setup 6 (https://jrsoftware.org/isdl.php) and run \
                 `ISCC {}`, or hand the script to your build machine.",
                script.display(),
                script.display()
            ),
            path: None,
        };
    };

    let done = Command::new(&iscc).arg(&script).output();
    let base = folder
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .unwrap_or_default();
    let made = folder
        .parent()
        .unwrap_or(folder)
        .join(format!("{base}-setup.exe"));
    match done {
        Ok(output) if output.status.success() && made.is_file() => {
            let how = crate::package::sign(&made);
            Made {
                note: match how {
                    Some(Ok(how)) => format!("{} ({how})", made.display()),
                    Some(Err(why)) => format!("{} ({why})", made.display()),
                    None => made.display().to_string(),
                },
                path: Some(made),
                recipe: Some(script),
            }
        }
        Ok(output) => Made {
            recipe: Some(script),
            note: format!(
                "Inno Setup refused the script: {}",
                last_words(&output.stdout, &output.stderr)
            ),
            path: None,
        },
        Err(error) => Made {
            recipe: Some(script),
            note: format!("`{}` could not be run ({error}).", iscc.display()),
            path: None,
        },
    }
}

/// The end of what a tool said, which is where tools put the reason.
fn last_words(out: &[u8], err: &[u8]) -> String {
    let said = String::from_utf8_lossy(if err.is_empty() { out } else { err });
    said.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("nothing")
        .trim()
        .to_string()
}

/// `hdiutil`, which every Mac has, on the `.app` bundle the package
/// step made.
///
/// `codesign` first when `RUSTX_SIGN_IDENTITY` names a certificate,
/// because a signature has to be inside the disk image: signing the
/// `.dmg` and not the bundle leaves Gatekeeper refusing the application
/// after the user has dragged it across.
fn macos(read: &build::Project, folder: &Path) -> Made {
    let name = &read.about.name;
    let bundle = folder.join(format!("{name}.app"));
    if !bundle.is_dir() {
        return Made {
            note: format!(
                "no .dmg: {} is not there, so there is no bundle to put in one.",
                bundle.display()
            ),
            ..Made::default()
        };
    }

    if let Some(identity) = std::env::var_os("RUSTX_SIGN_IDENTITY") {
        // Deep, because the bundle holds the framework and the helper
        // apps, and macOS refuses a bundle whose insides are unsigned.
        let done = Command::new("codesign")
            .arg("--force")
            .arg("--deep")
            .arg("--options")
            .arg("runtime")
            .arg("--sign")
            .arg(&identity)
            .arg(&bundle)
            .output();
        match done {
            Ok(output) if output.status.success() => {}
            Ok(output) => {
                return Made {
                    note: format!(
                        "codesign refused: {}",
                        last_words(&output.stdout, &output.stderr)
                    ),
                    ..Made::default()
                };
            }
            Err(error) => {
                return Made {
                    note: format!(
                        "signing was asked for and `codesign` could not be run ({error}). \
                         It comes with the Xcode command line tools."
                    ),
                    ..Made::default()
                };
            }
        }
    }

    let base = folder
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.clone());
    let dmg = folder
        .parent()
        .unwrap_or(folder)
        .join(format!("{base}.dmg"));
    let _ = std::fs::remove_file(&dmg);
    let done = Command::new("hdiutil")
        .arg("create")
        .arg("-volname")
        .arg(name)
        .arg("-srcfolder")
        .arg(&bundle)
        .arg("-ov")
        .arg("-format")
        .arg("UDZO")
        .arg(&dmg)
        .output();
    match done {
        Ok(output) if output.status.success() => Made {
            note: dmg.display().to_string(),
            path: Some(dmg),
            recipe: None,
        },
        Ok(output) => Made {
            note: format!(
                "hdiutil refused: {}",
                last_words(&output.stdout, &output.stderr)
            ),
            ..Made::default()
        },
        Err(error) => Made {
            note: format!("`hdiutil` could not be run ({error})."),
            ..Made::default()
        },
    }
}

/// The desktop entry a Linux desktop reads.
pub fn desktop_entry(read: &build::Project, binary: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={name}\nComment={comment}\n\
         Exec=/opt/{slug}/{binary} %U\nIcon={slug}\nTerminal=false\n\
         Categories=Utility;\nStartupWMClass={name}\n",
        name = read.about.name,
        comment = read.about.description,
        slug = crate::package::slug(&read.about.name),
    )
}

/// What the script does, written out so a reader can do it by hand.
pub fn install_script(read: &build::Project, binary: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by `rustx package`. Installs {name} for the user who\n\
         # runs it: no root, no package manager, nothing outside $HOME\n\
         # unless you pass --system.\n\
         set -eu\n\
         \n\
         here=$(cd \"$(dirname \"$0\")\" && pwd)\n\
         if [ \"${{1:-}}\" = \"--system\" ]; then\n\
         \tprefix=/opt/{slug}\n\
         \tapplications=/usr/share/applications\n\
         else\n\
         \tprefix=\"$HOME/.local/opt/{slug}\"\n\
         \tapplications=\"${{XDG_DATA_HOME:-$HOME/.local/share}}/applications\"\n\
         fi\n\
         \n\
         # The whole folder: the binary starts itself as Chromium's\n\
         # helper processes and reads libcef.so, the .pak files and\n\
         # locales/ from beside itself.\n\
         mkdir -p \"$prefix\" \"$applications\"\n\
         cp -R \"$here/.\" \"$prefix/\"\n\
         chmod +x \"$prefix/{binary}\"\n\
         \n\
         sed \"s|^Exec=.*|Exec=$prefix/{binary} %U|\" \"$here/{slug}.desktop\" \\\n\
         \t> \"$applications/{slug}.desktop\"\n\
         update-desktop-database \"$applications\" 2>/dev/null || true\n\
         \n\
         echo \"{name} is in $prefix. Start it from the menu, or run \
         $prefix/{binary}.\"\n",
        name = read.about.name,
        slug = crate::package::slug(&read.about.name),
    )
}

/// Linux, and anything else with a POSIX shell: the two files go in the
/// folder, so the archive already made carries them.
///
/// No AppImage and no `.deb`: `appimagetool` is a download this will
/// not make on a developer's behalf, and a `.deb` that declares no
/// dependencies is a `.deb` that installs and then does not start. The
/// folder plus this script is what both of those would wrap.
fn unix(read: &build::Project, folder: &Path, binary: &str) -> Made {
    let slug = crate::package::slug(&read.about.name);
    let entry = folder.join(format!("{slug}.desktop"));
    let script = folder.join("install.sh");
    if let Err(error) = std::fs::write(&entry, desktop_entry(read, binary)) {
        return Made {
            note: format!("{} could not be written ({error}).", entry.display()),
            ..Made::default()
        };
    }
    if let Err(error) = std::fs::write(&script, install_script(read, binary)) {
        return Made {
            note: format!("{} could not be written ({error}).", script.display()),
            ..Made::default()
        };
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755));
    }
    Made {
        note: format!(
            "{} and {} are in the folder: `./install.sh` installs for this user, \
             `./install.sh --system` into /opt.",
            script.display(),
            entry.display()
        ),
        recipe: Some(script),
        path: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::About;

    fn project() -> build::Project {
        build::Project {
            binary: "counter".to_string(),
            about: About {
                name: "My \"Counter\"".to_string(),
                version: "1.2.3".to_string(),
                description: "Counts".to_string(),
                publisher: "Someone".to_string(),
            },
            icon: None,
        }
    }

    #[test]
    fn the_identifier_is_the_same_every_time_and_differs_by_name() {
        assert_eq!(app_id("Counter"), app_id("Counter"));
        assert_ne!(app_id("Counter"), app_id("counter"));
        assert_eq!(app_id("Counter").len(), 36);
    }

    #[test]
    fn the_script_names_the_folder_the_version_and_the_binary() {
        let text = inno_script(
            &project(),
            Path::new("/tmp/dist/counter-1.2.3-windows"),
            "counter",
        );
        assert!(text.contains("AppVersion=1.2.3"), "{text}");
        assert!(
            text.contains("OutputBaseFilename=counter-1.2.3-windows-setup"),
            "{text}"
        );
        assert!(text.contains(r#"Filename: "{app}\counter.exe""#), "{text}");
        // A quote in the name doubles, which is Inno's escape: an
        // unescaped one ends the directive and the script will not
        // compile.
        assert!(text.contains(r#"AppName=My ""Counter"""#), "{text}");
        // One brace survives, because Inno reads `{{` as a literal one
        // and `{` as the start of a constant.
        assert!(text.contains("AppId={{"), "{text}");
    }

    #[test]
    fn the_desktop_entry_and_the_script_agree_on_where_the_binary_goes() {
        let entry = desktop_entry(&project(), "counter");
        assert!(entry.contains("Exec=/opt/my-counter/counter %U"), "{entry}");
        let script = install_script(&project(), "counter");
        assert!(script.contains("prefix=/opt/my-counter"), "{script}");
        assert!(script.contains("my-counter.desktop"), "{script}");
    }
}
