//! `rustx package`: the folder that ships, and the archive of it.
//!
//! What this answers, said plainly, because the documentation used to
//! say "not written" for all of it: a rustx application is not one
//! file in spirit, but on the system webview it carries no engine: the
//! `.exe` the linker makes draws its window with the webview the system
//! already has, so the folder holds the binary and the application's own
//! `assets/`, which the runtime looks for beside the executable before
//! it looks anywhere else.
//!
//! This command builds, stamps, gathers what the binary needs, writes a
//! manifest of what it gathered, and optionally puts the whole thing in
//! one `.zip`.
//!
//! What it deliberately does not do is invent a format. The folder is
//! the artefact; the archive is the same folder, zipped, because that
//! is what a release page holds and what the updater in `rustx-os`
//! knows how to read.

use std::path::{Path, PathBuf};

use crate::{build, installer, zip};

/// What was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// A debug package, for somebody who wants to hand a colleague the
    /// build they are looking at. `rustx package` means `--release`,
    /// because a package is what ships.
    pub debug: bool,
    /// Where the folder goes. `dist/` beside the project by default.
    pub out: Option<PathBuf>,
    /// Also write the `.zip`.
    pub archive: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            debug: false,
            out: None,
            archive: true,
        }
    }
}

/// What came out.
#[derive(Debug, Clone)]
pub struct Packaged {
    pub directory: PathBuf,
    pub archive: Option<PathBuf>,
    pub files: usize,
    pub bytes: u64,
    /// Whether `signtool` signed the binary, and what it said if it was
    /// asked to and could not.
    pub signed: Option<Result<String, String>>,
    /// The installer, or the reason there is not one.
    pub installer: installer::Made,
}

/// The name of the manifest this writes, and the updater reads.
pub const MANIFEST: &str = "rustx-release.json";

/// Build, gather, and say what the artefact is.
pub fn run(project: &Path, options: &Options) -> Result<Packaged, String> {
    let release = !options.debug;
    let read = build::read(project)?;
    let exe = build::run(release, project)?;

    let folder = format!(
        "{}-{}-{}",
        slug(&read.about.name),
        read.about.version,
        std::env::consts::OS
    );
    let into = options
        .out
        .clone()
        .unwrap_or_else(|| project.join("dist"))
        .join(&folder);

    // A package is a statement about a moment, so the folder is the
    // moment and not an accumulation of them: a file left over from a
    // build two versions ago, copied into a release because it happened
    // to be in the directory, is the oldest packaging bug there is.
    if into.exists() {
        std::fs::remove_dir_all(&into).map_err(|error| {
            format!(
                "{} could not be emptied ({error}). Is a window of the last package \
                 still running, or a file manager open in it?",
                into.display()
            )
        })?;
    }
    std::fs::create_dir_all(&into)
        .map_err(|error| format!("{} could not be made ({error}).", into.display()))?;

    let mut gathered: Vec<(String, PathBuf)> = Vec::new();
    gather_beside(&exe, &mut gathered)?;
    gather_assets(project, &mut gathered)?;
    if cfg!(target_os = "macos") {
        // A bundle cannot carry a stamp, so the licence travels as an asset.
        if let Some(path) = licence_path(project) {
            gathered.push(("assets/LICENSE".to_string(), path));
        }
        in_a_bundle(&mut gathered, &read.about.name, &read.binary);
    }

    let mut bytes = 0_u64;
    for (name, path) in &gathered {
        let destination = into.join(name);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{} could not be made ({error}).", parent.display()))?;
        }
        std::fs::copy(path, &destination).map_err(|error| {
            format!(
                "{} could not be copied to {} ({error}).",
                path.display(),
                destination.display()
            )
        })?;
        bytes += std::fs::metadata(&destination)
            .map(|it| it.len())
            .unwrap_or(0);
    }

    if cfg!(target_os = "macos") {
        finish_the_bundle(&into, &read)?;
    }

    // The binary may install itself: stamped before it is signed, because a
    // signature covers what is in the file. A Mac bundle says it in its
    // Info.plist instead, which the resources step wrote.
    if !cfg!(target_os = "macos") {
        rustx_os::install::write_stamp(
            &into.join(exe.file_name().unwrap_or_default()),
            &rustx_os::install::Stamp {
                name: read.about.name.clone(),
                version: read.about.version.clone(),
                publisher: read.about.publisher.clone(),
                license: licence_of(project).unwrap_or_default(),
            },
        )
        .map_err(|error| format!("the install stamp could not be written ({error})."))?;
    }
    let signed = sign(&into.join(exe.file_name().unwrap_or_default()));

    let manifest = manifest_of(&read, &into, &gathered)?;
    std::fs::write(into.join(MANIFEST), manifest)
        .map_err(|error| format!("the release manifest could not be written ({error}).",))?;

    // Before the archive, so that whatever the installer step leaves
    // inside the folder -- on Linux, the desktop entry and the install
    // script -- is in the `.zip` a release page holds.
    let made = installer::make(&read, project, &into, &read.binary);

    let archive = if options.archive {
        let path = into.parent().unwrap_or(&into).join(format!("{folder}.zip"));
        zip::write(&into, &path)?;
        Some(path)
    } else {
        None
    };

    Ok(Packaged {
        installer: made,
        directory: into,
        archive,
        files: gathered.len() + 1,
        bytes,
        signed,
    })
}

/// The binary: the system's webview is the engine, so nothing else of the
/// build directory travels.
fn gather_beside(exe: &Path, into: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let name = exe
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    if !exe.is_file() {
        return Err(format!(
            "{} was built and is not there, so there is nothing to package.",
            exe.display()
        ));
    }
    into.push((name, exe.to_path_buf()));
    Ok(())
}

fn walk(directory: &Path, prefix: &str, into: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = format!("{prefix}/{}", entry.file_name().to_string_lossy());
        if path.is_dir() {
            walk(&path, &name, into);
        } else if path.is_file() {
            into.push((name, path));
        }
    }
}

/// The application's own `assets/`, beside the binary.
///
/// They are also carried *inside* the binary by `#[rustx::main]`, and
/// the runtime prefers what is on disk beside the executable — so
/// copying them here is what lets somebody swap a logo or patch the
/// document in a shipped package without a compiler. An application
/// with no `assets/` has nothing to copy and that is not an error.
fn licence_path(project: &Path) -> Option<PathBuf> {
    ["LICENSE", "LICENSE.txt", "LICENSE.md"]
        .iter()
        .map(|name| project.join(name))
        .find(|path| path.is_file())
}

fn licence_of(project: &Path) -> Option<String> {
    std::fs::read_to_string(licence_path(project)?).ok()
}

fn gather_assets(project: &Path, into: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let root = project.join("assets");
    if !root.is_dir() {
        return Ok(());
    }
    let mut found = Vec::new();
    walk(&root, "assets", &mut found);
    // The bundle the attribute built is already in `assets/`, and the
    // document too; nothing is filtered out here, because what the
    // runtime reads beside the binary has to be the whole directory or
    // a page will name a file that is not there.
    into.extend(found);
    Ok(())
}

/// On macOS the folder is not the application: a bundle is.
///
/// A loose binary on a Mac has no name in the Dock, no icon, no
/// `Info.plist` and no way to be dragged into Applications. So the
/// gathered names are rewritten into the layout Apple documents, and
/// nothing is copied twice.
///
/// The binary stays beside its `assets/`, which is `Contents/MacOS`:
/// the runtime reads them from the executable's own directory, and
/// putting them in `Contents/Resources` where Apple would want them
/// would mean a bundle whose pages do not load.
fn in_a_bundle(gathered: &mut [(String, PathBuf)], name: &str, binary: &str) {
    let app = format!("{name}.app");
    for (held, _) in gathered.iter_mut() {
        *held = if held.as_str() == binary {
            format!("{app}/Contents/MacOS/{binary}")
        } else {
            format!("{app}/Contents/MacOS/{held}")
        };
    }
}

/// `Info.plist` for the bundle.
///
/// `"true"` and `"false"` become `<true/>` and `<false/>`: a boolean
/// written as a string is read as true whatever it says, which would
/// make `LSUIElement` impossible to turn off.
fn info_plist(pairs: &[(&str, &str)]) -> String {
    let mut text = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \
         \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n",
    );
    for (key, value) in pairs {
        let held = if *value == "true" || *value == "false" {
            format!("  <{value}/>\n")
        } else {
            format!("  <string>{}</string>\n", escaped_xml(value))
        };
        text.push_str(&format!("  <key>{}</key>\n{held}", escaped_xml(key)));
    }
    text.push_str("</dict>\n</plist>\n");
    text
}

fn escaped_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// What the copy loop could not copy: the property list and the icon.
fn finish_the_bundle(into: &Path, read: &build::Project) -> Result<(), String> {
    let name = &read.about.name;
    let slug = slug(name);
    let contents = into.join(format!("{name}.app")).join("Contents");
    let binary = contents.join("MacOS").join(&read.binary);
    if !binary.is_file() {
        return Err(format!(
            "{} is not there, so the bundle has no executable.",
            binary.display()
        ));
    }

    let icon = read
        .icon
        .as_deref()
        .filter(|path| path.extension().is_some_and(|kind| kind == "icns"));
    let icon_name = format!("{slug}.icns");
    if let Some(icon) = icon {
        let resources = contents.join("Resources");
        std::fs::create_dir_all(&resources)
            .map_err(|error| format!("{} could not be made ({error}).", resources.display()))?;
        std::fs::copy(icon, resources.join(&icon_name))
            .map_err(|error| format!("{} could not be copied ({error}).", icon.display()))?;
    }

    let version = read.about.version.clone();
    let publisher = read.about.publisher.clone();
    let identifier = format!("dev.rustx.{slug}");
    let mut keys: Vec<(&str, &str)> = vec![
        ("CFBundleDevelopmentRegion", "en"),
        ("CFBundleDisplayName", name),
        ("CFBundleExecutable", &read.binary),
        ("CFBundleIdentifier", &identifier),
        ("CFBundleInfoDictionaryVersion", "6.0"),
        ("CFBundleName", name),
        ("CFBundlePackageType", "APPL"),
        ("CFBundleShortVersionString", &version),
        ("CFBundleVersion", &version),
        ("LSMinimumSystemVersion", "11.0"),
        ("NSHighResolutionCapable", "true"),
        ("NSSupportsAutomaticGraphicsSwitching", "true"),
        // The binary may install itself: see `rustx_os::install::read_stamp`.
        ("RustxInstallable", "true"),
        ("RustxPublisher", &publisher),
    ];
    if icon.is_some() {
        keys.push(("CFBundleIconFile", &icon_name));
    }
    write_text(&contents.join("Info.plist"), &info_plist(&keys))?;

    Ok(())
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} could not be made ({error}).", parent.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("{} could not be written ({error}).", path.display()))
}

/// `signtool`, if the machine has one and the project asked for it.
///
/// Three environment variables and no configuration file, because
/// signing credentials are not something to write into a repository:
///
/// - `RUSTX_SIGN_PFX` and `RUSTX_SIGN_PASSWORD`: a certificate file.
/// - `RUSTX_SIGN_SUBJECT`: a certificate already in the machine's
///   store, by its subject name.
///
/// With none of them set, nothing is signed and nothing is said: an
/// unsigned build is the normal case for anybody who has not bought a
/// certificate. With one of them set and no `signtool` on the PATH, it
/// is an error with the reason, because a developer who asked for a
/// signature and got silence would ship unsigned without knowing.
pub fn sign(exe: &Path) -> Option<Result<String, String>> {
    let pfx = std::env::var("RUSTX_SIGN_PFX").ok();
    let subject = std::env::var("RUSTX_SIGN_SUBJECT").ok();
    if pfx.is_none() && subject.is_none() {
        return None;
    }

    let mut command = std::process::Command::new("signtool");
    command.arg("sign").arg("/fd").arg("SHA256").arg("/tr");
    command
        .arg("http://timestamp.digicert.com")
        .arg("/td")
        .arg("SHA256");
    if let Some(pfx) = &pfx {
        command.arg("/f").arg(pfx);
        if let Ok(password) = std::env::var("RUSTX_SIGN_PASSWORD") {
            command.arg("/p").arg(password);
        }
    } else if let Some(subject) = &subject {
        command.arg("/n").arg(subject);
    }
    command.arg(exe);

    match command.output() {
        Ok(output) if output.status.success() => Some(Ok(format!(
            "signed with {}",
            pfx.or(subject).unwrap_or_default()
        ))),
        Ok(output) => Some(Err(format!(
            "signtool refused: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))),
        Err(error) => Some(Err(format!(
            "signing was asked for and `signtool` could not be run ({error}). It comes \
             with the Windows SDK; the Developer Command Prompt has it on the PATH."
        ))),
    }
}

/// The manifest an updater reads: what this release is, and the digest
/// of every file in it.
///
/// JSON by hand, for the reason the rest of this crate gives: a command
/// line is not where this project starts taking dependencies, and the
/// shape written here is four keys and a list.
fn manifest_of(
    read: &build::Project,
    into: &Path,
    gathered: &[(String, PathBuf)],
) -> Result<String, String> {
    let mut files: Vec<String> = Vec::new();
    for (name, _) in gathered {
        let path = into.join(name);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("{} could not be read back ({error}).", path.display()))?;
        files.push(format!(
            "    {{ \"name\": {}, \"size\": {}, \"sha256\": \"{}\" }}",
            quoted(name),
            bytes.len(),
            rustx_os::digest::sha256_hex(&bytes)
        ));
    }
    files.sort();
    Ok(format!(
        "{{\n  \"name\": {},\n  \"version\": {},\n  \"binary\": {},\n  \"os\": {},\n  \
         \"arch\": {},\n  \"files\": [\n{}\n  ]\n}}\n",
        quoted(&read.about.name),
        quoted(&read.about.version),
        quoted(&format!("{}{}", read.binary, std::env::consts::EXE_SUFFIX)),
        quoted(std::env::consts::OS),
        quoted(std::env::consts::ARCH),
        files.join(",\n")
    ))
}

fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if (other as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", other as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// A name as a folder name: lowercase, one dash where anything that is
/// not a letter or a digit was.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed
    }
}

/// What to print about a package, for a person rather than a script.
pub fn says_what_it_made(packaged: &Packaged) -> String {
    let mut said = format!(
        "rustx: {} files, {:.1} MB, in {}\n",
        packaged.files,
        packaged.bytes as f64 / 1_048_576.0,
        packaged.directory.display()
    );
    if let Some(archive) = &packaged.archive {
        let size = std::fs::metadata(archive).map(|it| it.len()).unwrap_or(0);
        said.push_str(&format!(
            "rustx: {} ({:.1} MB)\n",
            archive.display(),
            size as f64 / 1_048_576.0
        ));
    }
    match &packaged.signed {
        Some(Ok(how)) => said.push_str(&format!("rustx: {how}\n")),
        Some(Err(why)) => said.push_str(&format!("rustx: {why}\n")),
        None => {}
    }
    for artifact in &packaged.installer.artifacts {
        let size = std::fs::metadata(artifact).map(|it| it.len()).unwrap_or(0);
        said.push_str(&format!(
            "rustx: {} ({:.1} MB)\n",
            artifact.display(),
            size as f64 / 1_048_576.0
        ));
    }
    // Every note that is not just the path of something listed above.
    for note in &packaged.installer.notes {
        if packaged
            .installer
            .artifacts
            .iter()
            .any(|artifact| note.starts_with(&artifact.display().to_string()))
        {
            continue;
        }
        said.push_str(&format!("rustx: {note}\n"));
    }
    said.push_str(
        "rustx: the folder is the application, the installer beside it is what a user \
         double-clicks, and the .zip is what the updater downloads.\n",
    );
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a package is allowed to hold: this application's binary. A
    /// workspace builds every binary into one directory, and "everything
    /// that is not cargo's own" shipped another application's `.exe`.
    #[test]
    fn a_package_holds_this_applications_binary_and_nothing_else() {
        let ours = "mon-app.exe";
        assert!(build::runtime_file("mon-app.exe", ours));
        for named in [
            "rustx.exe",
            "another-app.exe",
            "libcef.dll",
            "resources.pak",
            "mon-app.d",
        ] {
            assert!(
                !build::runtime_file(named, ours),
                "{named} was shipped and nothing reads it"
            );
        }
    }

    /// The one piece of the macOS bundle that can be checked from a
    /// machine that is not a Mac: where each gathered name lands.
    #[test]
    fn a_bundle_puts_the_binary_and_the_assets_in_macos() {
        let mut gathered = vec![
            ("counter".to_string(), PathBuf::from("/b/counter")),
            ("assets/index.html".to_string(), PathBuf::from("/b/y")),
        ];
        in_a_bundle(&mut gathered, "My App", "counter");
        let names: Vec<&str> = gathered.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "My App.app/Contents/MacOS/counter",
                "My App.app/Contents/MacOS/assets/index.html"
            ]
        );
    }

    #[test]
    fn a_boolean_in_a_property_list_is_a_tag_and_not_a_string() {
        let text = info_plist(&[("LSUIElement", "true"), ("CFBundleName", "A & B")]);
        assert!(text.contains("<key>LSUIElement</key>\n  <true/>"), "{text}");
        assert!(text.contains("<string>A &amp; B</string>"), "{text}");
    }

    #[test]
    fn a_name_becomes_a_folder_name() {
        assert_eq!(slug("FORGE ORG"), "forge-org");
        assert_eq!(slug("Mon App 2.0"), "mon-app-2-0");
        assert_eq!(slug("---"), "app");
    }

    #[test]
    fn json_text_is_escaped() {
        assert_eq!(quoted("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(quoted("a\nb"), "\"a\\nb\"");
    }

    /// The manifest is read by the updater, so its shape is a contract
    /// and not a convenience.
    #[test]
    fn the_manifest_names_the_release_and_every_file_in_it() {
        let directory =
            std::env::temp_dir().join(format!("rustx-package-manifest-{}", std::process::id()));
        let _ = std::fs::create_dir_all(directory.join("assets"));
        std::fs::write(directory.join("mon-app.exe"), b"binary").expect("a binary");
        std::fs::write(directory.join("assets").join("a.txt"), b"text").expect("an asset");

        let read = build::Project {
            binary: "mon-app".to_string(),
            about: crate::resources::About {
                name: "Mon App".to_string(),
                version: "1.2.3".to_string(),
                description: String::new(),
                publisher: String::new(),
            },
            icon: None,
        };
        let gathered = vec![
            ("mon-app.exe".to_string(), directory.join("mon-app.exe")),
            (
                "assets/a.txt".to_string(),
                directory.join("assets").join("a.txt"),
            ),
        ];

        let manifest = manifest_of(&read, &directory, &gathered).expect("a manifest");

        assert!(manifest.contains("\"version\": \"1.2.3\""), "{manifest}");
        assert!(manifest.contains("\"name\": \"Mon App\""), "{manifest}");
        assert!(manifest.contains("assets/a.txt"), "{manifest}");
        assert!(
            manifest.contains(&rustx_os::digest::sha256_hex(b"binary")),
            "the digest of the binary is in it: {manifest}"
        );
        assert!(manifest.contains("\"size\": 6"), "{manifest}");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn nothing_is_signed_unless_the_environment_asks() {
        // The default case, and the one that must stay silent: an
        // unsigned build is normal.
        if std::env::var_os("RUSTX_SIGN_PFX").is_none()
            && std::env::var_os("RUSTX_SIGN_SUBJECT").is_none()
        {
            assert!(sign(Path::new("nothing.exe")).is_none());
        }
    }
}
