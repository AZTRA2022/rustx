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
    let icon = match icon {
        Some(path) => Some(group_of(path)?),
        None => None,
    };
    let version = version_info(about);
    write_them(exe, &version, icon.as_ref())
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
fn write_them(exe: &Path, version: &[u8], icon: Option<&Group>) -> Result<(), String> {
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

#[cfg(not(windows))]
fn write_them(_exe: &Path, _version: &[u8], _icon: Option<&Group>) -> Result<(), String> {
    // Elsewhere the logo and the name live in a bundle or a desktop
    // entry, not inside the binary. The build still produced one, so
    // this is not a failure — it is a thing that does not apply.
    Ok(())
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
