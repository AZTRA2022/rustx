//! What an explorer window shows about a file.
//!
//! A logo, a name, a version in the properties dialog. Those live in the
//! `.exe` as resources, and the usual way to get them there is a
//! `build.rs` and a crate that writes one — which is exactly the
//! ceremony this framework took out of the developer's project. So they
//! are stamped in afterwards, into the file the linker already made, by
//! the three calls Windows has for it: `BeginUpdateResourceW`,
//! `UpdateResourceW`, `EndUpdateResource`.
//!
//! The icon is read with the same six bytes of header the window's icon
//! is read with, so a `.png` renamed to `.ico` is refused here in the
//! same words it is refused there.

use std::path::Path;

/// What an application says it is.
///
/// The same four fields the window carries, declared here rather than
/// borrowed from the engine: the command line builds projects, and
/// loading a browser engine to read a name would be absurd.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct About {
    pub name: String,
    pub version: String,
    pub description: String,
    pub publisher: String,
}

/// The resource types, by the numbers Windows gave them.
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
/// English (United States), Unicode — what every version resource on a
/// developer's machine already says.
const LANGUAGE: u16 = 0x0409;
const CODE_PAGE: u16 = 0x04B0;

/// Put `about`, and `icon` if there is one, into `exe`.
///
/// The icon is checked before the file is opened for writing: a half
/// stamped binary is worse than an unstamped one.
pub fn stamp(exe: &Path, about: &About, icon: Option<&Path>) -> Result<(), String> {
    let group = match icon {
        Some(path) => Some(group_of(path)?),
        None => None,
    };
    let version = version_info(about);
    write_them(exe, &version, group.as_ref(), about, icon)
}

/// An `.ico`, read into the images and the directory Windows wants.
///
/// A file holds one directory and its images at offsets; a binary holds
/// each image as its own resource and a directory that names them by
/// id. Same bytes, different shape — and the shape is the only reason
/// this function exists.
struct Group {
    directory: Vec<u8>,
    images: Vec<Vec<u8>>,
}

fn group_of(path: &Path) -> Result<Group, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("{} could not be read: {error}", path.display()))?;
    if bytes.len() < 6 {
        return Err(format!("{} is too short to be an icon", path.display()));
    }
    let word = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
    let long = |at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    };
    let count = word(4) as usize;
    if word(0) != 0 || word(2) != 1 || count == 0 {
        return Err(format!(
            "{} is not an icon file. An application's logo is an `.ico`; a `.png` \
             renamed to `.ico` is not one.",
            path.display()
        ));
    }
    if bytes.len() < 6 + count * 16 {
        return Err(format!(
            "{} says it holds {count} images and stops before naming them",
            path.display()
        ));
    }

    let mut directory = Vec::with_capacity(6 + count * 14);
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&1u16.to_le_bytes());
    directory.extend_from_slice(&(count as u16).to_le_bytes());
    let mut images = Vec::with_capacity(count);

    for which in 0..count {
        let entry = 6 + which * 16;
        let size = long(entry + 8);
        let at = long(entry + 12);
        if at + size > bytes.len() {
            return Err(format!(
                "{} names an image that is not inside the file — it is damaged, or it \
                 is not an `.ico`.",
                path.display()
            ));
        }
        // The file's entry, up to its size, is the binary's entry too;
        // only the tail differs — an offset becomes an id.
        directory.extend_from_slice(&bytes[entry..entry + 12]);
        directory.extend_from_slice(&((which + 1) as u16).to_le_bytes());
        images.push(bytes[at..at + size].to_vec());
    }
    Ok(Group { directory, images })
}

/// The `VS_VERSIONINFO` the properties dialog reads.
///
/// A tree of nodes, each one its own length, its value's length, whether
/// the value is text, a key, and its children — every piece aligned on
/// four bytes, which is the whole difficulty of the format.
fn version_info(about: &About) -> Vec<u8> {
    let (major, minor, patch, build) = numbers_of(&about.version);
    let mut fixed = Vec::with_capacity(52);
    fixed.extend_from_slice(&0xFEEF_04BD_u32.to_le_bytes()); // the signature
    fixed.extend_from_slice(&0x0001_0000_u32.to_le_bytes()); // of this struct
    for _ in 0..2 {
        // The file's version, then the product's: the same here.
        fixed.extend_from_slice(&(((major as u32) << 16) | minor as u32).to_le_bytes());
        fixed.extend_from_slice(&(((patch as u32) << 16) | build as u32).to_le_bytes());
    }
    fixed.extend_from_slice(&0x0000_003F_u32.to_le_bytes()); // which flags are valid
    fixed.extend_from_slice(&0u32.to_le_bytes()); // and none of them set
    fixed.extend_from_slice(&0x0004_0004_u32.to_le_bytes()); // Windows NT, 32-bit
    fixed.extend_from_slice(&1u32.to_le_bytes()); // an application
    fixed.extend_from_slice(&0u32.to_le_bytes()); // of no subtype
    fixed.extend_from_slice(&0u64.to_le_bytes()); // dated never

    let name = if about.name.is_empty() {
        "Application"
    } else {
        &about.name
    };
    let mut strings = Vec::new();
    for (key, value) in [
        ("CompanyName", about.publisher.as_str()),
        ("FileDescription", description_of(about)),
        ("FileVersion", about.version.as_str()),
        ("InternalName", name),
        ("OriginalFilename", name),
        ("ProductName", name),
        ("ProductVersion", about.version.as_str()),
    ] {
        if value.is_empty() {
            continue;
        }
        strings.extend(node(key, Value::Text(value), &[]));
    }
    let table = node(
        &format!("{LANGUAGE:04x}{CODE_PAGE:04x}"),
        Value::Nothing,
        &strings,
    );
    let string_info = node("StringFileInfo", Value::Nothing, &table);

    let mut pair = Vec::with_capacity(4);
    pair.extend_from_slice(&LANGUAGE.to_le_bytes());
    pair.extend_from_slice(&CODE_PAGE.to_le_bytes());
    let translation = node("Translation", Value::Binary(&pair), &[]);
    let var_info = node("VarFileInfo", Value::Nothing, &translation);

    let mut children = string_info;
    children.extend(var_info);
    node("VS_VERSION_INFO", Value::Binary(&fixed), &children)
}

/// Something to show when the project said nothing.
fn description_of(about: &About) -> &str {
    if about.description.is_empty() {
        &about.name
    } else {
        &about.description
    }
}

enum Value<'a> {
    Text(&'a str),
    Binary(&'a [u8]),
    /// A node that only holds other nodes.
    Nothing,
}

fn node(key: &str, value: Value<'_>, children: &[u8]) -> Vec<u8> {
    let (bytes, length, kind) = match value {
        Value::Text(text) => {
            let wide = wide(text);
            // Counted in characters, including the final nothing — the
            // one place in the format that does not count bytes.
            let length = (text.encode_utf16().count() + 1) as u16;
            (wide, length, 1_u16)
        }
        Value::Binary(bytes) => (bytes.to_vec(), bytes.len() as u16, 0_u16),
        Value::Nothing => (Vec::new(), 0, 1_u16),
    };

    let mut out = Vec::with_capacity(8 + key.len() * 2 + bytes.len() + children.len());
    out.extend_from_slice(&0u16.to_le_bytes()); // its length, once it is known
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend(wide(key));
    align(&mut out);
    out.extend_from_slice(&bytes);
    align(&mut out);
    out.extend_from_slice(children);
    let length = out.len() as u16;
    out[..2].copy_from_slice(&length.to_le_bytes());
    out
}

fn wide(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn align(bytes: &mut Vec<u8>) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
}

/// `"1.2.3"` as the four numbers Windows sorts by.
///
/// A version with a `-beta.1` on it keeps its three numbers: the text
/// field says the rest, and refusing to stamp a file over a suffix would
/// be absurd.
fn numbers_of(version: &str) -> (u16, u16, u16, u16) {
    let mut numbers = version
        .split(['.', '-', '+'])
        .filter_map(|piece| piece.parse::<u16>().ok());
    (
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
        numbers.next().unwrap_or(0),
    )
}

#[cfg(windows)]
fn write_them(
    exe: &Path,
    version: &[u8],
    icon: Option<&Group>,
    _about: &About,
    _icon_path: Option<&Path>,
) -> Result<(), String> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn BeginUpdateResourceW(file: *const u16, delete_existing: i32) -> *mut c_void;
        fn UpdateResourceW(
            update: *mut c_void,
            kind: *const u16,
            name: *const u16,
            language: u16,
            data: *const c_void,
            size: u32,
        ) -> i32;
        fn EndUpdateResourceW(update: *mut c_void, discard: i32) -> i32;
        fn GetLastError() -> u32;
    }

    let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    // The existing resources stay: a Rust `.exe` carries nothing we are
    // replacing, and throwing away what a linker put there would be a
    // command line deciding something it was not asked about.
    let update = unsafe { BeginUpdateResourceW(path.as_ptr(), 0) };
    if update.is_null() {
        return Err(format!(
            "{} could not be opened to be stamped ({}). Is it running?",
            exe.display(),
            unsafe { GetLastError() }
        ));
    }

    let put = |kind: u16, name: u16, bytes: &[u8]| -> Result<(), String> {
        let done = unsafe {
            UpdateResourceW(
                update,
                kind as usize as *const u16,
                name as usize as *const u16,
                LANGUAGE,
                bytes.as_ptr() as *const c_void,
                bytes.len() as u32,
            )
        };
        if done == 0 {
            return Err(format!(
                "a resource of {} could not be written ({})",
                exe.display(),
                unsafe { GetLastError() }
            ));
        }
        Ok(())
    };

    let mut trouble = put(RT_VERSION, 1, version);
    if let (Ok(()), Some(group)) = (&trouble, icon) {
        trouble = put(RT_GROUP_ICON, 1, &group.directory);
        for (which, image) in group.images.iter().enumerate() {
            if trouble.is_err() {
                break;
            }
            trouble = put(RT_ICON, (which + 1) as u16, image);
        }
    }

    // Whatever happened, the handle is closed — discarded on trouble, so
    // that a failure leaves the file exactly as the linker made it.
    let kept = unsafe { EndUpdateResourceW(update, i32::from(trouble.is_err())) };
    trouble?;
    if kept == 0 {
        return Err(format!(
            "{} could not be written back ({})",
            exe.display(),
            unsafe { GetLastError() }
        ));
    }
    Ok(())
}

/// Linux: a `.desktop` entry beside the binary. Written here rather
/// than inside it, the same reason the version resource above is
/// written into the `.exe` rather than its own file next to it: it is
/// where the desktop a developer builds for already looks.
#[cfg(target_os = "linux")]
fn write_them(
    exe: &Path,
    _version: &[u8],
    _icon: Option<&Group>,
    about: &About,
    icon_path: Option<&Path>,
) -> Result<(), String> {
    let path = exe.with_extension("desktop");
    std::fs::write(&path, desktop_entry(exe, about, icon_path))
        .map_err(|error| format!("{} could not be written: {error}", path.display()))
}

/// macOS: a minimal `.app` bundle around a copy of `exe`. Beside it,
/// not instead of it — `exe` is still the file `rustx dev`'s own
/// restart loop and `build::produced` look for, so a bundle gone wrong
/// cannot turn a successful build into a missing binary.
///
/// Written from Apple's documented bundle layout and never run on a
/// real Mac: compiled on this machine, not verified on that one. The
/// icon is not converted — `.ico` is not `.icns`, and a wrong guess at
/// that conversion would be worse than an app with the system's own
/// default icon.
#[cfg(target_os = "macos")]
fn write_them(
    exe: &Path,
    _version: &[u8],
    _icon: Option<&Group>,
    about: &About,
    _icon_path: Option<&Path>,
) -> Result<(), String> {
    write_app_bundle(exe, about)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn write_them(
    _exe: &Path,
    _version: &[u8],
    _icon: Option<&Group>,
    _about: &About,
    _icon_path: Option<&Path>,
) -> Result<(), String> {
    // Elsewhere the logo and the name live in a bundle or a desktop
    // entry, not inside the binary. The build still produced one, so
    // this is not a failure — it is a thing that does not apply.
    Ok(())
}

/// The text of a `.desktop` entry — not gated on Linux, so a test on
/// any machine can read exactly what one says.
#[cfg_attr(not(test), allow(dead_code))]
fn desktop_entry(exe: &Path, about: &About, icon: Option<&Path>) -> String {
    let name = if about.name.is_empty() {
        exe.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("App")
            .to_string()
    } else {
        single_line(&about.name)
    };

    let mut text = String::new();
    text.push_str("[Desktop Entry]\n");
    text.push_str("Type=Application\n");
    text.push_str(&format!("Name={name}\n"));
    if !about.description.is_empty() {
        text.push_str(&format!("Comment={}\n", single_line(&about.description)));
    }
    text.push_str(&format!("Exec={}\n", exe.display()));
    if let Some(icon) = icon {
        text.push_str(&format!("Icon={}\n", icon.display()));
    }
    text.push_str("Terminal=false\n");
    if !about.version.is_empty() {
        text.push_str(&format!("X-AppVersion={}\n", single_line(&about.version)));
    }
    if !about.publisher.is_empty() {
        text.push_str(&format!(
            "X-AppPublisher={}\n",
            single_line(&about.publisher)
        ));
    }
    text
}

/// A `.desktop` value may not hold a bare newline — one from a
/// multi-line description would end the file at the key that follows
/// it, silently dropping everything after.
#[cfg_attr(not(test), allow(dead_code))]
fn single_line(text: &str) -> String {
    text.replace('\n', " ")
}

/// Build the bundle folder and copy `exe` into it. Not gated on macOS,
/// so a test on any machine can make one and read it back; only
/// `write_them` decides whether this ever runs for real.
#[cfg_attr(not(test), allow(dead_code))]
fn write_app_bundle(exe: &Path, about: &About) -> Result<(), String> {
    let Some(parent) = exe.parent() else {
        return Ok(());
    };
    let name = if about.name.is_empty() {
        exe.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("App")
            .to_string()
    } else {
        about.name.clone()
    };
    let binary_name = exe
        .file_name()
        .and_then(|found| found.to_str())
        .unwrap_or("app")
        .to_string();

    let macos_dir = parent.join(format!("{name}.app/Contents/MacOS"));
    std::fs::create_dir_all(&macos_dir)
        .map_err(|error| format!("{} could not be made: {error}", macos_dir.display()))?;

    let copied = macos_dir.join(&binary_name);
    std::fs::copy(exe, &copied).map_err(|error| {
        format!(
            "{} could not be copied into the bundle: {error}",
            copied.display()
        )
    })?;

    let plist_path = macos_dir
        .parent()
        .expect("Contents/MacOS has a Contents parent")
        .join("Info.plist");
    std::fs::write(&plist_path, info_plist(about, &binary_name))
        .map_err(|error| format!("{} could not be written: {error}", plist_path.display()))
}

/// The text of `Info.plist` — the whole case for "install panels show
/// the software's own name, version and publisher" on macOS, the same
/// fields the Windows version resource above carries.
#[cfg_attr(not(test), allow(dead_code))]
fn info_plist(about: &About, binary_name: &str) -> String {
    let version = if about.version.is_empty() {
        "0.0.0"
    } else {
        about.version.as_str()
    };
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \x20   <key>CFBundleName</key>\n\
         \x20   <string>{}</string>\n\
         \x20   <key>CFBundleExecutable</key>\n\
         \x20   <string>{}</string>\n\
         \x20   <key>CFBundleShortVersionString</key>\n\
         \x20   <string>{}</string>\n\
         \x20   <key>CFBundleVersion</key>\n\
         \x20   <string>{}</string>\n\
         \x20   <key>CFBundlePackageType</key>\n\
         \x20   <string>APPL</string>\n\
         \x20   <key>NSHumanReadableCopyright</key>\n\
         \x20   <string>{}</string>\n\
         </dict>\n</plist>\n",
        xml_escape(&plist_name(about, binary_name)),
        xml_escape(binary_name),
        xml_escape(version),
        xml_escape(version),
        xml_escape(&about.publisher),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn plist_name(about: &About, binary_name: &str) -> String {
    if about.name.is_empty() {
        binary_name.to_string()
    } else {
        about.name.clone()
    }
}

/// The five characters a plist's XML may not hold bare — a publisher
/// or a name written with an `&` or a `"` would otherwise end the
/// string, or the file, at the character that broke it.
#[cfg_attr(not(test), allow(dead_code))]
fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            letter => out.push(letter),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_four_numbers() {
        assert_eq!(numbers_of("1.2.3"), (1, 2, 3, 0));
        assert_eq!(numbers_of("0.1.0-beta.2"), (0, 1, 0, 2));
        assert_eq!(numbers_of(""), (0, 0, 0, 0));
    }

    #[test]
    fn a_desktop_entry_names_everything_about_says() {
        let about = About {
            name: "Mon app".to_string(),
            version: "1.2.3".to_string(),
            description: "Une description".to_string(),
            publisher: "aztra".to_string(),
        };
        let text = desktop_entry(
            Path::new("/opt/mon-app/mon-app"),
            &about,
            Some(Path::new("/opt/mon-app/icon.png")),
        );
        assert!(text.starts_with("[Desktop Entry]\n"));
        assert!(text.contains("Name=Mon app\n"));
        assert!(text.contains("Comment=Une description\n"));
        assert!(text.contains("Exec=/opt/mon-app/mon-app\n"));
        assert!(text.contains("Icon=/opt/mon-app/icon.png\n"));
        assert!(text.contains("X-AppVersion=1.2.3\n"));
        assert!(text.contains("X-AppPublisher=aztra\n"));
    }

    #[test]
    fn a_desktop_entry_falls_back_to_the_binarys_own_name() {
        let about = About::default();
        let text = desktop_entry(Path::new("/opt/mon-app/mon-app"), &about, None);
        assert!(text.contains("Name=mon-app\n"), "got {text}");
        assert!(!text.contains("Icon="), "no icon was given: {text}");
    }

    #[test]
    fn a_newline_in_a_description_does_not_truncate_the_entry() {
        let about = About {
            description: "Une ligne\nUne autre".to_string(),
            ..About::default()
        };
        let text = desktop_entry(Path::new("/opt/mon-app/mon-app"), &about, None);
        assert!(
            text.contains("Comment=Une ligne Une autre\n"),
            "a bare newline would end the file at the next key: {text}"
        );
    }

    #[test]
    fn a_bundle_is_made_with_a_copy_of_the_binary_and_a_plist() {
        let room = std::env::temp_dir().join(format!(
            "rustx-resources-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&room).expect("a room to build in");
        let exe = room.join("mon-app");
        std::fs::write(&exe, b"not really a binary").expect("a fake binary");

        let about = About {
            name: "Mon app".to_string(),
            version: "1.2.3".to_string(),
            description: String::new(),
            publisher: "aztra".to_string(),
        };
        write_app_bundle(&exe, &about).expect("a bundle");

        let bundled = room.join("Mon app.app/Contents/MacOS/mon-app");
        assert!(
            bundled.is_file(),
            "the binary was not copied into the bundle"
        );
        assert_eq!(
            std::fs::read(&bundled).unwrap(),
            b"not really a binary",
            "the bundle must hold the real binary, not something else"
        );

        let plist = std::fs::read_to_string(room.join("Mon app.app/Contents/Info.plist"))
            .expect("an Info.plist");
        assert!(plist.contains("<string>Mon app</string>"), "got {plist}");
        assert!(plist.contains("<string>1.2.3</string>"), "got {plist}");
        assert!(plist.contains("<string>aztra</string>"), "got {plist}");

        let _ = std::fs::remove_dir_all(&room);
    }

    #[test]
    fn an_ampersand_or_a_quote_does_not_break_the_plists_xml() {
        let about = About {
            name: "A & B \"app\"".to_string(),
            version: String::new(),
            description: String::new(),
            publisher: String::new(),
        };
        let plist = info_plist(&about, "mon-app");
        assert!(
            plist.contains("<string>A &amp; B &quot;app&quot;</string>"),
            "got {plist}"
        );
    }

    #[test]
    fn every_node_is_aligned_and_says_its_own_length() {
        let bytes = version_info(&About {
            name: "Mon app".to_string(),
            version: "1.2.3".to_string(),
            description: String::new(),
            publisher: "aztra".to_string(),
        });
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]) as usize,
            bytes.len(),
            "the root node must measure the whole resource"
        );
        assert_eq!(bytes.len() % 4, 0, "it must end on a four-byte boundary");
    }
}
