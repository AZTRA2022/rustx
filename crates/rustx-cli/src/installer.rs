//! The installer: the thing a user of this application double-clicks.
//!
//! Not a `.zip`. A `.zip` is what the updater downloads, and a release
//! page holds one for that reason, but it is not how software is
//! installed on any of the three platforms. So each one gets its own
//! native format, with the application's icon, its version, its author
//! and its licence inside it:
//!
//! | | |
//! |---|---|
//! | Windows | a setup `.exe` that extracts itself ([`crate::selfextract`]): no tool to install. `RUSTX_INSTALLER=inno` asks for Inno Setup instead, for its licence and wizard pages |
//! | Linux | a `.deb`, written by [`crate::deb`] without any tool at all, plus an `.rpm` and an AppImage when the tools are there |
//! | macOS | a `.pkg` through `pkgbuild`, which every Mac ships, and a `.dmg` beside it |
//!
//! # The pages a user sees
//!
//! An installer is a few screens, and which screens they are is not
//! this file's choice to make: it is what the project has to show. So
//! each one is a file, and a project that does not have it does not get
//! that page.
//!
//! | the file | the page |
//! |---|---|
//! | `LICENSE`, `LICENSE.txt`, `LICENSE.md` | the licence to accept |
//! | `assets/installer/before.txt` | shown before anything is chosen |
//! | `assets/installer/after.txt` | shown when it is done |
//! | `assets/installer/banner.bmp` | the image down the left of the wizard |
//! | `assets/installer/header.bmp` | the small image in the corner |
//! | `assets/icon.ico` / `icon.png` / `icon.icns` | the installer's own icon, and the application's |
//!
//! # Nothing here can fail the package
//!
//! An installer needs a third-party tool on Windows and a Mac on
//! macOS, and a developer with neither still wants the folder and the
//! archive [`crate::package`] just made. When a tool is missing, the
//! script this would have fed it is written down anyway, and the
//! sentence names the tool and the page it comes from.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{build, deb};

/// What came of it.
#[derive(Debug, Clone, Default)]
pub struct Made {
    /// What a user can install, newest format first. Empty when no
    /// tool on this machine could make one.
    pub artifacts: Vec<PathBuf>,
    /// The scripts and recipes written, run or not: an Inno script, an
    /// `install.sh`. A build machine runs these.
    pub recipes: Vec<PathBuf>,
    /// One line per thing that happened, for the developer.
    pub notes: Vec<String>,
}

impl Made {
    fn said(&mut self, line: impl Into<String>) {
        self.notes.push(line.into());
    }
}

/// Everything the installers of every platform read off the project.
#[derive(Debug, Clone, Default)]
pub struct Pages {
    pub license: Option<PathBuf>,
    pub before: Option<PathBuf>,
    pub after: Option<PathBuf>,
    pub banner: Option<PathBuf>,
    pub header: Option<PathBuf>,
    pub ico: Option<PathBuf>,
    pub png: Option<PathBuf>,
    pub icns: Option<PathBuf>,
}

/// Find them. A project with none of these files gets an installer with
/// the pages every installer has and no more.
pub fn pages(project: &Path) -> Pages {
    let found = |names: &[&str]| -> Option<PathBuf> {
        names
            .iter()
            .map(|name| project.join(name))
            .find(|path| path.is_file())
    };
    Pages {
        license: found(&[
            "LICENSE",
            "LICENSE.txt",
            "LICENSE.md",
            "LICENCE",
            "LICENCE.txt",
            "COPYING",
        ]),
        before: found(&["assets/installer/before.txt", "assets/installer/before.rtf"]),
        after: found(&["assets/installer/after.txt", "assets/installer/after.rtf"]),
        banner: found(&["assets/installer/banner.bmp"]),
        header: found(&["assets/installer/header.bmp"]),
        ico: found(&["assets/icon.ico"]),
        png: found(&["assets/icon.png", "assets/installer/icon.png"]),
        icns: found(&["assets/icon.icns", "assets/installer/icon.icns"]),
    }
}

/// Make everything this platform's users would expect, from a folder
/// [`crate::package`] has already filled.
pub fn make(read: &build::Project, project: &Path, folder: &Path, binary: &str) -> Made {
    let pages = pages(project);
    if cfg!(target_os = "windows") {
        windows(read, project, folder, binary, &pages)
    } else if cfg!(target_os = "macos") {
        macos(read, project, folder, &pages)
    } else {
        linux(read, folder, binary, &pages)
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

/// `1.2.3` for a field that only takes numbers.
///
/// `VersionInfoVersion` is written into the setup binary's own version
/// resource, which Windows refuses to read unless it is up to four
/// numbers — so `1.0.0-rc.1` becomes `1.0.0` rather than failing the
/// whole script.
fn numeric(version: &str) -> String {
    let pieces: Vec<String> = version
        .split(['.', '-', '+'])
        .take_while(|piece| piece.chars().all(|letter| letter.is_ascii_digit()))
        .filter(|piece| !piece.is_empty())
        .take(4)
        .map(|piece| piece.to_string())
        .collect();
    if pieces.is_empty() {
        "0.0.0".to_string()
    } else {
        pieces.join(".")
    }
}

/// The Inno Setup script for this release.
pub fn inno_script(read: &build::Project, folder: &Path, binary: &str, pages: &Pages) -> String {
    let exe = format!("{binary}.exe");
    let out = folder.parent().unwrap_or(folder);
    let base = folder
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .unwrap_or_else(|| read.about.name.clone());

    let mut setup = String::new();
    setup.push_str(&format!("AppId={{{{{}}}\n", app_id(&read.about.name)));
    setup.push_str(&format!("AppName={}\n", inno(&read.about.name)));
    setup.push_str(&format!("AppVersion={}\n", inno(&read.about.version)));
    setup.push_str(&format!(
        "AppVerName={} {}\n",
        inno(&read.about.name),
        inno(&read.about.version)
    ));
    if !read.about.publisher.trim().is_empty() {
        setup.push_str(&format!(
            "AppPublisher={}\n",
            inno(read.about.publisher.trim())
        ));
    }
    if !read.about.description.trim().is_empty() {
        // What "Programs and Features" shows as the comment, and what
        // the setup binary's own file properties say.
        setup.push_str(&format!(
            "AppComments={}\n",
            inno(read.about.description.trim())
        ));
        setup.push_str(&format!(
            "VersionInfoDescription={}\n",
            inno(read.about.description.trim())
        ));
    }
    // The whole version resource, so that the file properties of the
    // setup binary say what it is before anybody runs it -- which is
    // also what a corporate software inventory reads, and what
    // SmartScreen shows when it has no signature to show instead.
    setup.push_str(&format!(
        "VersionInfoVersion={}\n",
        numeric(&read.about.version)
    ));
    setup.push_str(&format!(
        "VersionInfoProductVersion={}\n",
        numeric(&read.about.version)
    ));
    setup.push_str(&format!(
        "VersionInfoProductName={}\n",
        inno(&read.about.name)
    ));
    setup.push_str(&format!(
        "VersionInfoProductTextVersion={}\n",
        inno(&read.about.version)
    ));
    setup.push_str(&format!("VersionInfoOriginalFileName={base}-setup.exe\n"));
    if !read.about.publisher.trim().is_empty() {
        setup.push_str(&format!(
            "VersionInfoCopyright={}\n",
            inno(read.about.publisher.trim())
        ));
    }
    if !read.about.publisher.trim().is_empty() {
        setup.push_str(&format!(
            "VersionInfoCompany={}\n",
            inno(read.about.publisher.trim())
        ));
    }
    setup.push_str(&format!(
        "DefaultDirName={{autopf}}\\{}\n",
        inno(&read.about.name)
    ));
    setup.push_str(&format!("DefaultGroupName={}\n", inno(&read.about.name)));
    setup.push_str(&format!(
        "UninstallDisplayName={}\n",
        inno(&read.about.name)
    ));
    setup.push_str(&format!("UninstallDisplayIcon={{app}}\\{exe}\n"));
    setup.push_str(&format!("OutputDir={}\n", out.display()));
    setup.push_str(&format!("OutputBaseFilename={base}-setup\n"));
    setup.push_str("Compression=lzma2/max\nSolidCompression=yes\n");
    // Per user, so that installing asks for no administrator rights:
    // `{autopf}` becomes the user's own folder when it is not elevated.
    setup.push_str("PrivilegesRequired=lowest\nPrivilegesRequiredOverridesAllowed=dialog\n");
    setup.push_str("ArchitecturesInstallIn64BitMode=x64compatible\n");
    setup.push_str("ArchitecturesAllowed=x64compatible\nWizardStyle=modern\n");
    // 15 MB is enough of a margin for the engine; without it the
    // installer does not check and fails halfway through the copy.
    setup.push_str("ExtraDiskSpaceRequired=15728640\n");
    if let Some(icon) = &pages.ico {
        setup.push_str(&format!("SetupIconFile={}\n", icon.display()));
    }
    if let Some(license) = &pages.license {
        setup.push_str(&format!("LicenseFile={}\n", license.display()));
    }
    if let Some(before) = &pages.before {
        setup.push_str(&format!("InfoBeforeFile={}\n", before.display()));
    }
    if let Some(after) = &pages.after {
        setup.push_str(&format!("InfoAfterFile={}\n", after.display()));
    }
    if let Some(banner) = &pages.banner {
        setup.push_str(&format!("WizardImageFile={}\n", banner.display()));
    }
    if let Some(header) = &pages.header {
        setup.push_str(&format!("WizardSmallImageFile={}\n", header.display()));
    }

    format!(
        "; Written by `rustx package`. Edit it if you like: it is read by\n\
         ; ISCC.exe and by nothing in this framework.\n\
         [Setup]\n{setup}\n\
         [Languages]\n\
         Name: \"english\"; MessagesFile: \"compiler:Default.isl\"\n\
         \n\
         [Files]\n\
         ; The whole folder: the `.exe` and its `assets\\`. The window is\n\
         ; drawn by the WebView2 runtime the system already has.\n\
         Source: \"{source}\\*\"; DestDir: \"{{app}}\"; \
         Flags: recursesubdirs createallsubdirs ignoreversion\n\
         \n\
         [Icons]\n\
         Name: \"{{group}}\\{name}\"; Filename: \"{{app}}\\{exe}\"\n\
         Name: \"{{group}}\\Uninstall {name}\"; Filename: \"{{uninstallexe}}\"\n\
         Name: \"{{autodesktop}}\\{name}\"; Filename: \"{{app}}\\{exe}\"; \
         Tasks: desktopicon\n\
         \n\
         [Tasks]\n\
         Name: \"desktopicon\"; Description: \"Put {name} on the desktop\"\n\
         \n\
         [Run]\n\
         Filename: \"{{app}}\\{exe}\"; Description: \"Start {name}\"; \
         Flags: nowait postinstall skipifsilent\n\
         \n\
         [UninstallDelete]\n\
         ; The engine unpacks the application's own assets beside the\n\
         ; profile it keeps per title. Uninstalling leaves the user's\n\
         ; data alone and takes that copy.\n\
         Type: filesandordirs; Name: \"{{localappdata}}\\rustx\\{slug}\\assets\"\n",
        name = inno(&read.about.name),
        slug = crate::package::slug(&read.about.name),
        source = folder.display(),
    )
}

/// `ISCC.exe`: on the PATH, or where the installer of Inno Setup puts
/// it.
fn iscc() -> Option<PathBuf> {
    if looks_runnable("ISCC", &["/?"]) {
        return Some(PathBuf::from("ISCC"));
    }
    // `ProgramFiles` for a machine-wide install, and
    // `%LOCALAPPDATA%\Programs` for a per-user one -- which is where
    // `winget install JRSoftware.InnoSetup` puts it when it is run
    // without administrator rights, and so the common case on a
    // developer's own machine.
    let mut roots: Vec<PathBuf> = Vec::new();
    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = std::env::var_os(variable) {
            roots.push(PathBuf::from(root));
        }
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Programs"));
    }
    for root in roots {
        for version in ["Inno Setup 6", "Inno Setup 7", "Inno Setup 5"] {
            let guess = root.join(version).join("ISCC.exe");
            if guess.is_file() {
                return Some(guess);
            }
        }
    }
    None
}

/// Whether a program is there at all. Not whether it worked: a tool
/// asked for its own help text exits non-zero often enough that the
/// only question worth asking is whether the process started.
fn looks_runnable(program: &str, arguments: &[&str]) -> bool {
    Command::new(program)
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

/// The setup that extracts itself: `rustx-setup.exe` (built beside `rustx`)
/// with the folder appended. Needs nothing installed on the machine.
fn self_extracting(
    read: &build::Project,
    project: &Path,
    folder: &Path,
    binary: &str,
    pages: &Pages,
) -> Made {
    let mut made = Made::default();
    let stub = std::env::current_exe()
        .ok()
        .and_then(|me| me.parent().map(|dir| dir.join("rustx-setup.exe")))
        .filter(|path| path.is_file());
    let Some(stub) = stub else {
        made.said("no setup.exe: rustx-setup.exe is not beside the rustx command (`cargo install rustx-cli` installs both).");
        return made;
    };
    let beside = folder.parent().unwrap_or(folder);
    let base = folder
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .unwrap_or_default();
    let setup = beside.join(format!("{base}-setup.exe"));
    let meta = crate::selfextract::Meta {
        name: read.about.name.clone(),
        version: read.about.version.clone(),
        publisher: read.about.publisher.clone(),
        binary: binary_file(binary),
        files: 0,
    };
    match crate::selfextract::pack(&stub, folder, &setup_pages(project, pages), &meta, &setup) {
        Ok(()) => {
            match crate::package::sign(&setup) {
                Some(Ok(how)) => made.said(format!("{} ({how})", setup.display())),
                Some(Err(why)) => made.said(format!("{} ({why})", setup.display())),
                None => made.said(setup.display().to_string()),
            }
            made.artifacts.push(setup);
        }
        Err(why) => made.said(format!("the setup could not be made: {why}")),
    }
    made.said("The setup shows installer/install.html and installer/uninstall.html when the project has them, and its default pages otherwise.");
    made
}

/// What the project wrote for its own install and uninstall screens: every file
/// of `installer/` (pages, styles, images), and the licence as `LICENSE.txt`.
/// The setup shows its own pages for whatever is missing.
fn setup_pages(project: &Path, pages: &Pages) -> Vec<(String, PathBuf)> {
    fn walk(root: &Path, here: &Path, into: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(here) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if let Ok(relative) = path.strip_prefix(root) {
                into.push((relative.to_string_lossy().replace('\\', "/"), path.clone()));
            }
        }
    }
    let mut found = Vec::new();
    let root = project.join("installer");
    walk(&root, &root, &mut found);
    if let Some(license) = &pages.license {
        found.push(("LICENSE.txt".to_string(), license.clone()));
    }
    found.sort();
    found
}

/// The file name of the binary as it sits in the folder.
fn binary_file(binary: &str) -> String {
    if binary.to_ascii_lowercase().ends_with(".exe") {
        binary.to_string()
    } else {
        format!("{binary}.exe")
    }
}

fn windows(
    read: &build::Project,
    project: &Path,
    folder: &Path,
    binary: &str,
    pages: &Pages,
) -> Made {
    if std::env::var("RUSTX_INSTALLER")
        .map(|it| it != "inno")
        .unwrap_or(true)
    {
        return self_extracting(read, project, folder, binary, pages);
    }
    let mut made = Made::default();
    // ISCC reads every relative path in the script as relative to the
    // script itself, while `--out` is relative to where the command was
    // run. Absolute from here on, or the compile finds no files.
    let absolute = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
    let folder = absolute.as_path();
    let beside = folder.parent().unwrap_or(folder);
    let script = beside.join(format!("{}.iss", crate::package::slug(&read.about.name)));
    let text = inno_script(read, folder, binary, pages);
    if let Err(error) = std::fs::write(&script, &text) {
        made.said(format!(
            "{} could not be written ({error}).",
            script.display()
        ));
        return made;
    }
    made.recipes.push(script.clone());

    if let Some(iscc) = iscc() {
        let base = folder
            .file_name()
            .map(|it| it.to_string_lossy().into_owned())
            .unwrap_or_default();
        let setup = beside.join(format!("{base}-setup.exe"));
        match Command::new(&iscc).arg(&script).output() {
            Ok(output) if output.status.success() && setup.is_file() => {
                match crate::package::sign(&setup) {
                    Some(Ok(how)) => made.said(format!("{} ({how})", setup.display())),
                    Some(Err(why)) => made.said(format!("{} ({why})", setup.display())),
                    None => made.said(setup.display().to_string()),
                }
                made.artifacts.push(setup);
            }
            Ok(output) => made.said(format!(
                "Inno Setup refused the script: {}",
                last_words(&output.stdout, &output.stderr)
            )),
            Err(error) => made.said(format!("`{}` could not be run ({error}).", iscc.display())),
        }
    } else {
        made.said(format!(
            "no setup.exe: Inno Setup is not installed. The script is written at {}. \
             Install it with `winget install JRSoftware.InnoSetup`, or from \
             https://jrsoftware.org/isdl.php, and run `ISCC {}` -- or hand the script \
             to your build machine, which is what a CI job does.",
            script.display(),
            script.display()
        ));
    }

    // Said whatever happened above: a project that wants an MSI or a
    // store package knows what it wants, and this says what it has.
    if pages.license.is_none() {
        made.said(
            "no licence page: add a LICENSE file beside Cargo.toml and the installer \
             will ask the user to accept it."
                .to_string(),
        );
    }
    made
}

/// The end of what a tool said, which is where tools put the reason.
fn last_words(out: &[u8], err: &[u8]) -> String {
    let said = String::from_utf8_lossy(if err.is_empty() { out } else { err });
    let lines: Vec<&str> = said
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    // Two lines, not one: ISCC names the reason on one line and writes
    // `Compile aborted.` on the next, so the last line alone is the
    // only part of the message that says nothing.
    let tail: Vec<&str> = lines.iter().rev().take(2).rev().copied().collect();
    if tail.is_empty() {
        "nothing".to_string()
    } else {
        tail.join(" -- ")
    }
}

// ------------------------------------------------------------------ macOS

/// `pkgbuild` first, which every Mac has and which is what a user
/// expects to double-click; the `.dmg` beside it, because that is what
/// a download page links to.
fn macos(read: &build::Project, project: &Path, folder: &Path, pages: &Pages) -> Made {
    let mut made = Made::default();
    let name = &read.about.name;
    let bundle = folder.join(format!("{name}.app"));
    if !bundle.is_dir() {
        made.said(format!(
            "no installer: {} is not there, so there is no bundle to put in one.",
            bundle.display()
        ));
        return made;
    }

    if let Some(identity) = std::env::var_os("RUSTX_SIGN_IDENTITY") {
        // Deep, because macOS refuses a bundle whose insides are unsigned.
        match Command::new("codesign")
            .arg("--force")
            .arg("--deep")
            .arg("--options")
            .arg("runtime")
            .arg("--sign")
            .arg(&identity)
            .arg(&bundle)
            .output()
        {
            Ok(output) if output.status.success() => made.said("the bundle is signed"),
            Ok(output) => made.said(format!(
                "codesign refused: {}",
                last_words(&output.stdout, &output.stderr)
            )),
            Err(error) => made.said(format!(
                "signing was asked for and `codesign` could not be run ({error}). It \
                 comes with the Xcode command line tools."
            )),
        }
    }

    let beside = folder.parent().unwrap_or(folder);
    let base = folder
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.clone());

    // The `.pkg`. `--install-location /Applications` is what makes it
    // an installer rather than an archive: the user double-clicks, the
    // system asks for their password, and the application is where
    // every other application is.
    let pkg = beside.join(format!("{base}.pkg"));
    let slug = crate::package::slug(name);
    // With screens of its own the `.pkg` is made in two steps: the component,
    // then `productbuild` around it with the project's pages.
    let screens = mac_screens(project, pages);
    let component = beside.join(format!("{base}-component.pkg"));
    let produced = if screens.is_some() {
        component.clone()
    } else {
        pkg.clone()
    };
    // A bundle a package installs is "relocatable" by default: the installer then
    // updates a copy of it it finds elsewhere on the disk (the one in `dist/`)
    // instead of writing /Applications. So the package is made from a root that
    // holds only the bundle, with a component plist that says it is not.
    let staging = beside.join(format!("{base}-pkgroot"));
    let _ = std::fs::remove_dir_all(&staging);
    let staged = std::fs::create_dir_all(&staging).is_ok()
        && Command::new("cp")
            .arg("-R")
            .arg(&bundle)
            .arg(&staging)
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
    let plist = beside.join(format!("{base}-component.plist"));
    let bundle_name = bundle
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let _ = std::fs::write(
        &plist,
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\"><array><dict><key>RootRelativeBundlePath</key><string>{}</string><key>BundleIsRelocatable</key><false/><key>BundleIsVersionChecked</key><false/><key>BundleOverwriteAction</key><string>upgrade</string></dict></array></plist>
",
            xml(&bundle_name)
        ),
    );
    let mut build_pkg = Command::new("pkgbuild");
    build_pkg.arg("--install-location").arg("/Applications");
    if staged {
        build_pkg
            .arg("--root")
            .arg(&staging)
            .arg("--component-plist")
            .arg(&plist);
    } else {
        build_pkg.arg("--component").arg(&bundle);
    }
    build_pkg
        .arg("--identifier")
        .arg(format!("dev.rustx.{slug}"))
        .arg("--version")
        .arg(&read.about.version);
    if let Some(identity) = std::env::var_os("RUSTX_SIGN_INSTALLER_IDENTITY") {
        // A different certificate from the application's: Apple issues
        // "Developer ID Installer" for a `.pkg` and "Developer ID
        // Application" for a bundle, and each refuses the other's job.
        build_pkg.arg("--sign").arg(identity);
    }
    build_pkg.arg(&produced);
    match build_pkg.output() {
        Ok(output) if output.status.success() && produced.is_file() => {
            if let Some(screens) = &screens {
                match wrap_with_screens(read, &slug, beside, &component, &pkg, screens) {
                    Ok(()) => {
                        made.said(format!(
                            "{} (with the project's installer screens)",
                            pkg.display()
                        ));
                        made.artifacts.push(pkg);
                    }
                    Err(why) => {
                        made.said(format!(
                            "productbuild refused: {why}; the plain component is kept."
                        ));
                        made.artifacts.push(component);
                    }
                }
            } else {
                made.said(pkg.display().to_string());
                made.artifacts.push(pkg);
            }
        }
        Ok(output) => made.said(format!(
            "pkgbuild refused: {}",
            last_words(&output.stdout, &output.stderr)
        )),
        Err(error) => made.said(format!(
            "`pkgbuild` could not be run ({error}). It ships with macOS; on another \
             platform there is nothing to run it on."
        )),
    }

    // And the disk image, which is what a download link points at.
    let dmg = beside.join(format!("{base}.dmg"));
    let _ = std::fs::remove_file(&dmg);
    match Command::new("hdiutil")
        .arg("create")
        .arg("-volname")
        .arg(name)
        .arg("-srcfolder")
        .arg(&bundle)
        .arg("-ov")
        .arg("-format")
        .arg("UDZO")
        .arg(&dmg)
        .output()
    {
        Ok(output) if output.status.success() => {
            made.said(dmg.display().to_string());
            made.artifacts.push(dmg);
        }
        Ok(output) => made.said(format!(
            "hdiutil refused: {}",
            last_words(&output.stdout, &output.stderr)
        )),
        Err(error) => made.said(format!("`hdiutil` could not be run ({error}).")),
    }

    if pages.icns.is_none() {
        made.said(
            "no .icns: add assets/icon.icns and the bundle, the Dock and the installer \
             will all show it. An .ico is a Windows format and macOS does not read it."
                .to_string(),
        );
    }
    made
}

/// The pages a macOS installer shows around the files, from the project:
/// `installer/{welcome,readme,conclusion}.html` (static HTML: the system's
/// installer runs no script), `installer/background.png`, and the licence.
struct MacScreens {
    welcome: Option<PathBuf>,
    readme: Option<PathBuf>,
    conclusion: Option<PathBuf>,
    background: Option<PathBuf>,
    license: Option<PathBuf>,
}

fn mac_screens(project: &Path, pages: &Pages) -> Option<MacScreens> {
    let own = |name: &str| Some(project.join("installer").join(name)).filter(|path| path.is_file());
    let screens = MacScreens {
        welcome: own("welcome.html"),
        readme: own("readme.html"),
        conclusion: own("conclusion.html"),
        background: own("background.png"),
        license: pages.license.clone(),
    };
    let any = screens.welcome.is_some()
        || screens.readme.is_some()
        || screens.conclusion.is_some()
        || screens.background.is_some();
    any.then_some(screens)
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The `distribution.xml` that puts the screens around the component.
fn distribution(
    title: &str,
    id: &str,
    version: &str,
    component: &str,
    screens: &MacScreens,
) -> String {
    let file = |path: &Option<PathBuf>, fallback: &str| {
        path.as_ref().map(|path| {
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| fallback.to_string())
        })
    };
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>
<installer-gui-script minSpecVersion=\"1\">
",
    );
    out.push_str(&format!(
        "  <title>{}</title>
",
        xml(title)
    ));
    if let Some(name) = file(&screens.welcome, "welcome.html") {
        out.push_str(&format!(
            "  <welcome file=\"{}\"/>
",
            xml(&name)
        ));
    }
    if let Some(name) = file(&screens.readme, "readme.html") {
        out.push_str(&format!(
            "  <readme file=\"{}\"/>
",
            xml(&name)
        ));
    }
    if screens.license.is_some() {
        out.push_str(
            "  <license file=\"license.txt\"/>
",
        );
    }
    if let Some(name) = file(&screens.conclusion, "conclusion.html") {
        out.push_str(&format!(
            "  <conclusion file=\"{}\"/>
",
            xml(&name)
        ));
    }
    if let Some(name) = file(&screens.background, "background.png") {
        out.push_str(&format!(
            "  <background file=\"{}\" alignment=\"left\" scaling=\"proportional\"/>
",
            xml(&name)
        ));
    }
    out.push_str(
        "  <options customize=\"never\" require-scripts=\"false\"/>
",
    );
    out.push_str(&format!(
        "  <choices-outline><line choice=\"default\"><line choice=\"{id}\"/></line></choices-outline>
  <choice id=\"default\"/>
  <choice id=\"{id}\" visible=\"false\"><pkg-ref id=\"{id}\"/></choice>
  <pkg-ref id=\"{id}\" version=\"{}\" onConclusion=\"none\">{}</pkg-ref>
</installer-gui-script>
",
        xml(version),
        xml(component)
    ));
    out
}

fn wrap_with_screens(
    read: &build::Project,
    slug: &str,
    beside: &Path,
    component: &Path,
    out: &Path,
    screens: &MacScreens,
) -> Result<(), String> {
    let resources = beside.join(format!("{slug}-installer-resources"));
    let _ = std::fs::remove_dir_all(&resources);
    std::fs::create_dir_all(&resources).map_err(|e| e.to_string())?;
    for path in [
        &screens.welcome,
        &screens.readme,
        &screens.conclusion,
        &screens.background,
    ]
    .into_iter()
    .flatten()
    {
        if let Some(name) = path.file_name() {
            std::fs::copy(path, resources.join(name)).map_err(|e| e.to_string())?;
        }
    }
    if let Some(license) = &screens.license {
        std::fs::copy(license, resources.join("license.txt")).map_err(|e| e.to_string())?;
    }
    let id = format!("dev.rustx.{slug}");
    let component_name = component
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let script = beside.join(format!("{slug}-distribution.xml"));
    std::fs::write(
        &script,
        distribution(
            &read.about.name,
            &id,
            &read.about.version,
            &component_name,
            screens,
        ),
    )
    .map_err(|e| e.to_string())?;
    let mut command = Command::new("productbuild");
    command
        .arg("--distribution")
        .arg(&script)
        .arg("--resources")
        .arg(&resources)
        .arg("--package-path")
        .arg(beside);
    if let Some(identity) = std::env::var_os("RUSTX_SIGN_INSTALLER_IDENTITY") {
        command.arg("--sign").arg(identity);
    }
    let output = command
        .arg(out)
        .output()
        .map_err(|e| format!("`productbuild` could not be run ({e})"))?;
    if output.status.success() && out.is_file() {
        let _ = std::fs::remove_file(component);
        Ok(())
    } else {
        Err(last_words(&output.stdout, &output.stderr))
    }
}

// ------------------------------------------------------------------ Linux

/// Everything in the packaged folder, at the path it installs to under
/// `/opt`.
fn under_opt(folder: &Path, slug: &str) -> Result<Vec<deb::Entry>, String> {
    let mut found = Vec::new();
    walk(folder, folder, slug, &mut found)?;
    found.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(found)
}

fn walk(root: &Path, here: &Path, slug: &str, into: &mut Vec<deb::Entry>) -> Result<(), String> {
    let entries = std::fs::read_dir(here)
        .map_err(|error| format!("{} could not be read ({error}).", here.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, slug, into)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| format!("{} is not under {}", path.display(), root.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        // Anything with no extension is the binary or a helper: on Linux
        // that is how an executable looks, and a `.so` has to be
        // executable too for the loader to map it.
        let executable = path.extension().is_none()
            || path
                .extension()
                .is_some_and(|kind| kind == "so" || kind == "sh");
        into.push(deb::Entry {
            path: format!("/opt/{slug}/{relative}"),
            from: deb::Source::File(path),
            mode: if executable { 0o755 } else { 0o644 },
        });
    }
    Ok(())
}

/// The desktop entry a Linux desktop reads. Installed, so the paths are
/// the installed ones.
pub fn desktop_entry(read: &build::Project, binary: &str, scheme: Option<&str>) -> String {
    let slug = crate::package::slug(&read.about.name);
    let mut text = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName={name}\n",
        name = read.about.name
    );
    if !read.about.description.trim().is_empty() {
        text.push_str(&format!("Comment={}\n", read.about.description.trim()));
    }
    text.push_str(&format!("Exec=/opt/{slug}/{binary} %U\n"));
    text.push_str(&format!("Icon={slug}\n"));
    text.push_str("Terminal=false\nCategories=Utility;\n");
    text.push_str(&format!("StartupWMClass={}\n", read.about.name));
    if let Some(scheme) = scheme {
        text.push_str(&format!("MimeType=x-scheme-handler/{scheme};\n"));
    }
    text
}

/// What the script does, written out so a reader can do it by hand.
pub fn install_script(read: &build::Project, binary: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by `rustx package`. Installs {name} for the user who\n\
         # runs it: no root, no package manager, nothing outside $HOME\n\
         # unless you pass --system. The `.deb` beside this folder is\n\
         # the better choice on anything Debian-based.\n\
         set -eu\n\
         \n\
         here=$(cd \"$(dirname \"$0\")\" && pwd)\n\
         if [ \"${{1:-}}\" = \"--system\" ]; then\n\
         \tprefix=/opt/{slug}\n\
         \tapplications=/usr/share/applications\n\
         \ticons=/usr/share/icons/hicolor/256x256/apps\n\
         else\n\
         \tprefix=\"$HOME/.local/opt/{slug}\"\n\
         \tapplications=\"${{XDG_DATA_HOME:-$HOME/.local/share}}/applications\"\n\
         \ticons=\"${{XDG_DATA_HOME:-$HOME/.local/share}}/icons/hicolor/256x256/apps\"\n\
         fi\n\
         \n\
         # The whole folder: the binary and its assets/. The window is\n\
         # drawn by WebKitGTK, which the system provides.\n\
         mkdir -p \"$prefix\" \"$applications\" \"$icons\"\n\
         cp -R \"$here/.\" \"$prefix/\"\n\
         chmod +x \"$prefix/{binary}\"\n\
         [ -f \"$here/assets/icon.png\" ] && cp \"$here/assets/icon.png\" \
         \"$icons/{slug}.png\"\n\
         \n\
         sed \"s|^Exec=.*|Exec=$prefix/{binary} %U|\" \"$here/{slug}.desktop\" \\\n\
         \t> \"$applications/{slug}.desktop\"\n\
         update-desktop-database \"$applications\" 2>/dev/null || true\n\
         gtk-update-icon-cache -f -t \"$(dirname \"$(dirname \"$(dirname \"$icons\")\")\")\" \
         2>/dev/null || true\n\
         \n\
         echo \"{name} is in $prefix. Start it from the menu, or run \
         $prefix/{binary}.\"\n",
        name = read.about.name,
        slug = crate::package::slug(&read.about.name),
    )
}

/// The uninstaller, written beside the installer because a package
/// manager has one and a folder does not.
pub fn uninstall_script(read: &build::Project) -> String {
    format!(
        "#!/bin/sh\n\
         # Removes what install.sh put there. The user's own data, in\n\
         # ${{XDG_DATA_HOME:-$HOME/.local/share}}/{slug}, is left alone:\n\
         # uninstalling an application is not the same as being asked to\n\
         # throw away what it saved.\n\
         set -eu\n\
         \n\
         if [ \"${{1:-}}\" = \"--system\" ]; then\n\
         \tprefix=/opt/{slug}\n\
         \tapplications=/usr/share/applications\n\
         \ticons=/usr/share/icons/hicolor/256x256/apps\n\
         else\n\
         \tprefix=\"$HOME/.local/opt/{slug}\"\n\
         \tapplications=\"${{XDG_DATA_HOME:-$HOME/.local/share}}/applications\"\n\
         \ticons=\"${{XDG_DATA_HOME:-$HOME/.local/share}}/icons/hicolor/256x256/apps\"\n\
         fi\n\
         \n\
         rm -rf \"$prefix\"\n\
         rm -f \"$applications/{slug}.desktop\" \"$icons/{slug}.png\"\n\
         update-desktop-database \"$applications\" 2>/dev/null || true\n\
         echo \"{name} is removed. Its data is still in \
         ${{XDG_DATA_HOME:-$HOME/.local/share}}/{slug}.\"\n",
        name = read.about.name,
        slug = crate::package::slug(&read.about.name),
    )
}

fn linux(read: &build::Project, folder: &Path, binary: &str, pages: &Pages) -> Made {
    let mut made = Made::default();
    let slug = crate::package::slug(&read.about.name);
    let beside = folder.parent().unwrap_or(folder);

    // The two scripts go in the folder, so the update archive carries
    // them and so a machine with no package manager has a way in.
    for (name, text) in [
        (format!("{slug}.desktop"), desktop_entry(read, binary, None)),
        ("install.sh".to_string(), install_script(read, binary)),
        ("uninstall.sh".to_string(), uninstall_script(read)),
    ] {
        let path = folder.join(&name);
        if let Err(error) = std::fs::write(&path, &text) {
            made.said(format!(
                "{} could not be written ({error}).",
                path.display()
            ));
            return made;
        }
        #[cfg(unix)]
        if name.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
        }
        made.recipes.push(path);
    }

    // The `.deb`, which needs no tool at all.
    let mut files = match under_opt(folder, &slug) {
        Ok(files) => files,
        Err(why) => {
            made.said(why);
            return made;
        }
    };
    files.push(deb::Entry {
        // `/usr/bin` is on every PATH, and a symbolic link is what
        // every distribution's own packages put there.
        path: format!("/usr/bin/{slug}"),
        from: deb::Source::Link(format!("/opt/{slug}/{binary}")),
        mode: 0o777,
    });
    files.push(deb::Entry {
        path: format!("/usr/share/applications/{slug}.desktop"),
        from: deb::Source::Text(desktop_entry(read, binary, None)),
        mode: 0o644,
    });
    if let Some(png) = &pages.png {
        // The theme's own tree, so the menu, the dock and the window's
        // own icon all find it under one name.
        files.push(deb::Entry {
            path: format!("/usr/share/icons/hicolor/256x256/apps/{slug}.png"),
            from: deb::Source::File(png.clone()),
            mode: 0o644,
        });
    } else {
        made.said(
            "no icon in the .deb: add assets/icon.png (256x256) and the menu will show \
             it. A .ico is a Windows format and no Linux desktop reads it."
                .to_string(),
        );
    }
    if let Some(license) = &pages.license {
        // Where a Debian package's licence belongs, and what `apt` and
        // the software centre read.
        files.push(deb::Entry {
            path: format!("/usr/share/doc/{slug}/copyright"),
            from: deb::Source::File(license.clone()),
            mode: 0o644,
        });
    }

    let package = deb::Package {
        name: slug.clone(),
        version: read.about.version.clone(),
        maintainer: maintainer(&read.about.publisher),
        description: described(read),
        architecture: deb::architecture().to_string(),
        depends: deb::WEBVIEW_NEEDS.to_string(),
        section: "utils".to_string(),
        homepage: String::new(),
        files,
    };
    match deb::write(&package, beside) {
        Ok(path) => {
            made.said(path.display().to_string());
            made.artifacts.push(path);
        }
        Err(why) => made.said(why),
    }

    // An AppImage, when the machine has the tool. No download is made
    // on a developer's behalf.
    if looks_runnable("appimagetool", &["--version"]) {
        made.said(
            "appimagetool is installed; an AppImage is not written yet. The .deb and \
             install.sh cover the same ground."
                .to_string(),
        );
    }
    if !looks_runnable("dpkg-deb", &["--version"]) {
        made.said(
            "the .deb was written without dpkg-deb, which this machine does not have: \
             the format is an ar archive of two tarballs, and rustx writes it directly."
                .to_string(),
        );
    }
    made
}

/// `Name <email>`, which is the one field `dpkg` is strict about.
///
/// `authors = ["Someone <someone@example.test>"]` already has that
/// shape, and a name with no address gets one that is obviously a
/// placeholder rather than a refusal to build.
fn maintainer(publisher: &str) -> String {
    let publisher = publisher.trim();
    if publisher.is_empty() {
        return "unknown <nobody@example.invalid>".to_string();
    }
    if publisher.contains('<') && publisher.contains('>') {
        return publisher.to_string();
    }
    format!("{publisher} <nobody@example.invalid>")
}

/// The summary line, then the long description.
fn described(read: &build::Project) -> String {
    let summary = read.about.description.trim();
    if summary.is_empty() {
        return read.about.name.clone();
    }
    summary.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::About;

    /// ISCC nomme la raison sur une ligne et ecrit `Compile
    /// aborted.` sur la suivante : la derniere ligne seule est la
    /// seule part du message qui ne dit rien.
    #[test]
    fn the_reason_iscc_gave_is_kept_and_not_just_its_last_line() {
        let said = b"Inno Setup 6\nError on line 7: no such file\nCompile aborted.\n";
        assert_eq!(
            last_words(said, b""),
            "Error on line 7: no such file -- Compile aborted."
        );
        assert_eq!(last_words(b"", b""), "nothing");
        assert_eq!(last_words(b"only one\n", b""), "only one");
    }

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
            &Pages::default(),
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
        // Metadata the user sees in Programs and Features.
        assert!(text.contains("AppPublisher=Someone"), "{text}");
        assert!(text.contains("VersionInfoVersion=1.2.3"), "{text}");
        assert!(
            text.contains(r#"UninstallDisplayName=My ""Counter"""#),
            "{text}"
        );
    }

    #[test]
    fn a_page_is_declared_only_when_the_project_has_its_file() {
        let bare = inno_script(
            &project(),
            Path::new("/tmp/dist/x"),
            "counter",
            &Pages::default(),
        );
        assert!(!bare.contains("LicenseFile="), "{bare}");

        let with = inno_script(
            &project(),
            Path::new("/tmp/dist/x"),
            "counter",
            &Pages {
                license: Some(PathBuf::from("/p/LICENSE")),
                before: Some(PathBuf::from("/p/before.txt")),
                ico: Some(PathBuf::from("/p/icon.ico")),
                ..Pages::default()
            },
        );
        assert!(with.contains("LicenseFile=/p/LICENSE"), "{with}");
        assert!(with.contains("InfoBeforeFile=/p/before.txt"), "{with}");
        assert!(with.contains("SetupIconFile=/p/icon.ico"), "{with}");
    }

    /// Windows reads a version resource as up to four numbers, so a
    /// pre-release tag has to be cut rather than passed through.
    #[test]
    fn a_version_for_a_numeric_field_keeps_only_its_numbers() {
        assert_eq!(numeric("1.2.3"), "1.2.3");
        assert_eq!(numeric("1.0.0-rc.1"), "1.0.0");
        assert_eq!(numeric("2.0"), "2.0");
        assert_eq!(numeric("v1.2"), "0.0.0");
        assert_eq!(numeric(""), "0.0.0");
    }

    #[test]
    fn the_desktop_entry_and_the_scripts_agree_on_where_the_binary_goes() {
        let entry = desktop_entry(&project(), "counter", None);
        assert!(entry.contains("Exec=/opt/my-counter/counter %U"), "{entry}");
        assert!(entry.contains("Icon=my-counter\n"), "{entry}");
        let script = install_script(&project(), "counter");
        assert!(script.contains("prefix=/opt/my-counter"), "{script}");
        assert!(script.contains("my-counter.desktop"), "{script}");
        let removed = uninstall_script(&project());
        assert!(removed.contains("rm -rf \"$prefix\""), "{removed}");
    }

    #[test]
    fn a_scheme_puts_its_handler_in_the_desktop_entry() {
        let entry = desktop_entry(&project(), "counter", Some("myapp"));
        assert!(
            entry.contains("MimeType=x-scheme-handler/myapp;"),
            "{entry}"
        );
    }

    /// `dpkg` refuses a maintainer without an address, and a build that
    /// fails because `authors` is a bare name is a build nobody can fix
    #[test]
    fn the_distribution_names_only_the_screens_the_project_has() {
        let screens = MacScreens {
            welcome: Some(PathBuf::from("/p/installer/welcome.html")),
            readme: None,
            conclusion: None,
            background: None,
            license: Some(PathBuf::from("/p/LICENSE")),
        };
        let text = distribution("My <App>", "dev.rustx.my-app", "1.2.3", "c.pkg", &screens);
        assert!(text.contains("<welcome file=\"welcome.html\"/>"), "{text}");
        assert!(text.contains("<license file=\"license.txt\"/>"), "{text}");
        assert!(
            !text.contains("<readme")
                && !text.contains("<conclusion")
                && !text.contains("<background"),
            "{text}"
        );
        assert!(text.contains("<title>My &lt;App&gt;</title>"), "{text}");
        assert!(
            text.contains("version=\"1.2.3\"") && text.contains(">c.pkg</pkg-ref>"),
            "{text}"
        );
    }

    /// from the message.
    #[test]
    fn a_maintainer_always_has_an_address() {
        assert_eq!(maintainer("A <a@b.test>"), "A <a@b.test>");
        assert!(maintainer("Someone").contains('<'));
        assert!(maintainer("").contains('<'));
    }
}
