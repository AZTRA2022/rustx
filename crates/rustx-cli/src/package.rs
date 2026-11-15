//! `rustx package`: the folder that ships, and the archive of it.
//!
//! What this answers, said plainly, because the documentation used to
//! say "not written" for all of it: a rustx application is not one
//! file. The `.exe` the linker makes is a Chromium host, and Chromium
//! reads `libcef.dll`, the `.pak` files, `icudtl.dat`,
//! `v8_context_snapshot.bin` and `locales/` from beside that binary.
//! `rustx build` printed one path and then a paragraph explaining that
//! the path was not the application; a developer who copied it got a
//! program that does not start, and the dialog Windows shows for that
//! names a library rather than a cause.
//!
//! So this command does the copying. It builds, stamps, gathers
//! everything the binary needs — including the application's own
//! `assets/`, which the runtime looks for beside the executable before
//! it looks anywhere else — writes a manifest of what it gathered, and
//! optionally puts the whole thing in one `.zip`.
//!
//! What it deliberately does not do is invent a format. The folder is
//! the artefact; the archive is the same folder, zipped, because that
//! is what a release page holds and what [`crate::update`]'s client
//! knows how to read.

use std::path::{Path, PathBuf};

use crate::{build, zip};

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
}

/// The name of the manifest this writes, and the updater reads.
pub const MANIFEST: &str = "rustx-release.json";

/// Build, gather, and say what the artefact is.
pub fn run(project: &Path, options: &Options) -> Result<Packaged, String> {
    let release = !options.debug;
    let read = build::read(project)?;
    let exe = build::run(release, project)?;
    let from = exe
        .parent()
        .ok_or_else(|| format!("{} has no directory", exe.display()))?
        .to_path_buf();

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
    gather_beside(&from, &exe, &mut gathered)?;
    gather_assets(project, &mut gathered)?;

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

    let signed = signing(&into.join(exe.file_name().unwrap_or_default()));

    let manifest = manifest_of(&read, &into, &gathered)?;
    std::fs::write(into.join(MANIFEST), manifest)
        .map_err(|error| format!("the release manifest could not be written ({error}).",))?;

    let archive = if options.archive {
        let path = into.parent().unwrap_or(&into).join(format!("{folder}.zip"));
        zip::write(&into, &path)?;
        Some(path)
    } else {
        None
    };

    Ok(Packaged {
        directory: into,
        archive,
        files: gathered.len() + 1,
        bytes,
        signed,
    })
}

/// Everything in the build directory that the program needs at run
/// time, which is everything cargo did not leave there for cargo.
///
/// `build::ships_with` is the same answer `rustx build --release`
/// already prints, so the package holds exactly what that paragraph
/// named. Directories are walked — `locales/` is 51 MB of `.pak` files
/// and a package without it starts with no text in its menus.
fn gather_beside(from: &Path, exe: &Path, into: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let binary = exe
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let entries = std::fs::read_dir(from)
        .map_err(|error| format!("{} could not be read ({error}).", from.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !build::ships_with(&name) {
            continue;
        }
        if path.is_dir() {
            if build::runtime_directory(&name) {
                walk(&path, &name, into);
            }
            continue;
        }
        if path.is_file() && build::runtime_file(&name, &binary) {
            into.push((name, path));
        }
    }
    if !into.iter().any(|(name, _)| {
        name.eq_ignore_ascii_case(&exe.file_name().unwrap_or_default().to_string_lossy())
    }) {
        return Err(format!(
            "{} was built and is not in {}, so there is nothing to package.",
            exe.display(),
            from.display()
        ));
    }
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
fn signing(exe: &Path) -> Option<Result<String, String>> {
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
fn slug(name: &str) -> String {
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
    said.push_str(
        "rustx: the folder is the application. It runs from anywhere, including a \
         machine that has never built it.\n",
    );
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a package is allowed to hold. The first one made carried
    /// another application's `.exe`, CEF's `CMakeLists.txt` and its
    /// `bootstrap.exe`: a workspace builds every binary into one
    /// directory, and "everything that is not cargo's own" was the
    /// wrong question.
    #[test]
    fn a_package_holds_the_engine_and_this_applications_binary_and_nothing_else() {
        let ours = "mon-app.exe";
        for named in [
            "libcef.dll",
            "chrome_elf.dll",
            "icudtl.dat",
            "resources.pak",
            "chrome_100_percent.pak",
            "v8_context_snapshot.bin",
            "vk_swiftshader_icd.json",
            "vulkan-1.dll",
            "CREDITS.html",
            "mon-app.exe",
        ] {
            assert!(
                build::runtime_file(named, ours),
                "{named} is part of the application and was left behind"
            );
        }
        for named in [
            "rustx.exe",
            "another-app.exe",
            "bootstrap.exe",
            "bootstrapc.exe",
            "CMakeLists.txt",
            "archive.json",
            "mon-app.d",
        ] {
            assert!(
                !build::runtime_file(named, ours),
                "{named} was shipped and nothing reads it"
            );
        }
        assert!(build::runtime_directory("locales"));
        assert!(build::runtime_directory(
            "Chromium Embedded Framework.framework"
        ));
        assert!(!build::runtime_directory("deps"));
        assert!(!build::runtime_directory("build"));
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
        let _ = std::fs::create_dir_all(directory.join("locales"));
        std::fs::write(directory.join("mon-app.exe"), b"binary").expect("a binary");
        std::fs::write(directory.join("locales").join("fr.pak"), b"pak").expect("a pak");

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
                "locales/fr.pak".to_string(),
                directory.join("locales").join("fr.pak"),
            ),
        ];

        let manifest = manifest_of(&read, &directory, &gathered).expect("a manifest");

        assert!(manifest.contains("\"version\": \"1.2.3\""), "{manifest}");
        assert!(manifest.contains("\"name\": \"Mon App\""), "{manifest}");
        assert!(manifest.contains("locales/fr.pak"), "{manifest}");
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
            assert!(signing(Path::new("nothing.exe")).is_none());
        }
    }
}
