//! What `rustx build` leaves on disk.
//!
//! An application that ships is a file in an explorer window: it has a
//! logo, a name, and a version somebody can read in its properties.
//! Cargo does not put those there, and a `build.rs` in the developer's
//! project is exactly what this framework refuses — so the command line
//! stamps them into the `.exe` after the link, and these tests read them
//! back out of the file with the same call the explorer uses.

use std::fs;
use std::path::{Path, PathBuf};

use rustx_cli::resources::{self, About};

fn somewhere(what: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "rustx-build-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&path).expect("a directory to write into");
    path
}

/// A copy of this very test binary, which is a real `.exe` the linker
/// made — stamping a file the system will not load proves nothing.
fn an_exe(into: &Path) -> PathBuf {
    let exe = into.join("stamped.exe");
    fs::copy(std::env::current_exe().expect("this test is a file"), &exe).expect("a copy");
    exe
}

/// The smallest real `.ico`: one 16×16 image, a 32-bit BMP.
fn an_ico(into: &Path) -> PathBuf {
    let path = into.join("icon.ico");
    let pixels = 16 * 16 * 4;
    let mask = 16 * 4; // one bit per pixel, padded to 4 bytes a row
    let size = 40 + pixels + mask;

    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0u16.to_le_bytes()); // reserved
    bytes.extend_from_slice(&1u16.to_le_bytes()); // an icon, not a cursor
    bytes.extend_from_slice(&1u16.to_le_bytes()); // one image
    bytes.extend_from_slice(&[16, 16, 0, 0]); // width, height, colours, reserved
    bytes.extend_from_slice(&1u16.to_le_bytes()); // planes
    bytes.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    bytes.extend_from_slice(&(size as u32).to_le_bytes());
    bytes.extend_from_slice(&22u32.to_le_bytes()); // where the image starts

    // BITMAPINFOHEADER: an icon's height is doubled, colour over mask.
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&16i32.to_le_bytes());
    bytes.extend_from_slice(&32i32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&[0; 24]);
    bytes.extend(std::iter::repeat_n(0x7f_u8, pixels));
    bytes.extend(std::iter::repeat_n(0x00_u8, mask));

    fs::write(&path, bytes).expect("an icon");
    path
}

fn about() -> About {
    About {
        name: "Mon app".to_string(),
        version: "1.2.3".to_string(),
        description: "Une application".to_string(),
        publisher: "aztra".to_string(),
    }
}

#[test]
fn an_icon_becomes_a_resource_of_the_exe() {
    let into = somewhere("icon");
    let exe = an_exe(&into);
    let icon = an_ico(&into);
    resources::stamp(&exe, &about(), Some(&icon)).expect("the exe should be stamped");

    let group = read_back::resource(&exe, read_back::GROUP_ICON, 1)
        .expect("the explorer looks for icon group 1");
    assert_eq!(
        u16::from_le_bytes([group[4], group[5]]),
        1,
        "the group should name one image"
    );
    let image = read_back::resource(&exe, read_back::ICON, 1).expect("the image itself");
    assert_eq!(
        u32::from_le_bytes([image[0], image[1], image[2], image[3]]),
        40,
        "an icon's image begins with its own header, not with the file's"
    );
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn the_version_info_carries_the_name_and_the_version() {
    let into = somewhere("version");
    let exe = an_exe(&into);
    resources::stamp(&exe, &about(), None).expect("the exe should be stamped");

    let info = read_back::resource(&exe, read_back::VERSION, 1).expect("a version resource");
    let said = read_back::text(&info);
    assert!(said.contains("Mon app"), "got {said}");
    assert!(said.contains("1.2.3"), "got {said}");
    assert!(said.contains("aztra"), "got {said}");
    // The numbers the explorer sorts by, not only the text it shows.
    let fixed = info
        .windows(4)
        .position(|four| four == 0xFEEF_04BD_u32.to_le_bytes())
        .expect("a VS_FIXEDFILEINFO");
    assert_eq!(
        u32::from_le_bytes([
            info[fixed + 8],
            info[fixed + 9],
            info[fixed + 10],
            info[fixed + 11]
        ]),
        (1 << 16) | 2,
        "the major and minor of 1.2.3"
    );
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_project_without_an_icon_still_builds() {
    // Nothing frightening happens: the file is stamped with what there
    // is, and what is missing is missing.
    let into = somewhere("plain");
    let exe = an_exe(&into);
    resources::stamp(&exe, &about(), None).expect("no icon is not a failure");
    assert!(read_back::resource(&exe, read_back::VERSION, 1).is_some());
    assert!(read_back::resource(&exe, read_back::GROUP_ICON, 1).is_none());
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_file_that_is_not_an_ico_is_refused_in_words() {
    let into = somewhere("png");
    let exe = an_exe(&into);
    let lying = into.join("logo.ico");
    fs::write(&lying, b"\x89PNG\r\n\x1a\n and the rest of a png").expect("a png");
    let said = resources::stamp(&exe, &about(), Some(&lying)).expect_err("refused");
    assert!(said.contains("not an icon"), "got {said}");
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn the_name_the_version_and_the_icon_come_from_the_project() {
    // Where the developer already wrote them: the manifest, and the
    // `Config` in `main.rs`. A second file to keep in step would be a
    // second file to forget.
    let into = somewhere("read");
    rustx_cli::new::write("mon-app", &into).expect("a project");
    let root = into.join("mon-app");
    fs::write(root.join("assets/icon.ico"), b"\x00\x00\x01\x00\x01\x00").expect("an icon");
    let main = root.join("src/main.rs");
    let source = fs::read_to_string(&main).expect("a main");
    fs::write(
        &main,
        source.replace(
            ".size(900, 640)",
            ".size(900, 640).icon(\"assets/icon.ico\")",
        ),
    )
    .expect("a main with an icon");

    let read = rustx_cli::build::read(&root).expect("the project should be readable");
    assert_eq!(read.about.name, "Mon app", "the title of the Config");
    assert_eq!(read.about.version, "0.1.0", "the version of the manifest");
    assert_eq!(read.binary, "mon-app", "what cargo will call the exe");
    assert_eq!(read.icon, Some(root.join("assets/icon.ico")));
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_logo_in_the_obvious_place_is_found_without_being_declared() {
    // The same courtesy the window pays `assets/icon.ico`. If the two
    // disagreed, a developer would get a window wearing one logo and a
    // file wearing another.
    let into = somewhere("obvious");
    rustx_cli::new::write("mon-app", &into).expect("a project");
    let root = into.join("mon-app");
    let icon = an_ico(&root.join("assets"));
    let read = rustx_cli::build::read(&root).expect("readable");
    assert_eq!(read.icon, Some(root.join("assets/icon.ico")));
    assert!(icon.is_file());
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_version_the_workspace_declares_is_the_version_stamped() {
    // `version.workspace = true` is how this very repository writes it.
    // A `.exe` stamped `0.0.0` because of that is a bug whose cause a
    // developer cannot see.
    let into = somewhere("workspace");
    rustx_cli::new::write("mon-app", &into).expect("a project");
    let root = into.join("mon-app");
    fs::write(
        into.join("Cargo.toml"),
        "[workspace]\nmembers = [\"mon-app\"]\n\n[workspace.package]\n\
         version = \"3.4.5\"\nauthors = [\"aztra\"]\n",
    )
    .expect("a workspace");
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("a manifest");
    fs::write(
        root.join("Cargo.toml"),
        manifest.replace(
            "version = \"0.1.0\"",
            "version.workspace = true\nauthors.workspace = true",
        ),
    )
    .expect("a member");

    let read = rustx_cli::build::read(&root).expect("readable");
    assert_eq!(read.about.version, "3.4.5");
    assert_eq!(read.about.publisher, "aztra");
    let _ = fs::remove_dir_all(&into);
}

#[test]
fn a_project_that_is_not_a_project_is_refused_in_words() {
    let into = somewhere("empty");
    let said = rustx_cli::build::read(&into).expect_err("refused");
    assert!(said.contains("Cargo.toml"), "got {said}");
    let _ = fs::remove_dir_all(&into);
}

/// Reading a resource back out of a file, the way the explorer does.
#[cfg(windows)]
mod read_back {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    pub const ICON: u16 = 3;
    pub const GROUP_ICON: u16 = 14;
    pub const VERSION: u16 = 16;

    const AS_DATAFILE: u32 = 0x0000_0002;

    unsafe extern "system" {
        fn LoadLibraryExW(name: *const u16, unused: *mut c_void, flags: u32) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> i32;
        fn FindResourceW(module: *mut c_void, name: *const u16, kind: *const u16) -> *mut c_void;
        fn LoadResource(module: *mut c_void, found: *mut c_void) -> *mut c_void;
        fn LockResource(loaded: *mut c_void) -> *mut c_void;
        fn SizeofResource(module: *mut c_void, found: *mut c_void) -> u32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    pub fn resource(exe: &Path, kind: u16, name: u16) -> Option<Vec<u8>> {
        let wide = wide(exe);
        let module = unsafe { LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), AS_DATAFILE) };
        if module.is_null() {
            panic!("the stamped file could not be loaded at all");
        }
        let found = unsafe {
            FindResourceW(
                module,
                name as usize as *const u16,
                kind as usize as *const u16,
            )
        };
        let bytes = if found.is_null() {
            None
        } else {
            let size = unsafe { SizeofResource(module, found) } as usize;
            let loaded = unsafe { LoadResource(module, found) };
            let at = unsafe { LockResource(loaded) } as *const u8;
            if at.is_null() || size == 0 {
                None
            } else {
                Some(unsafe { std::slice::from_raw_parts(at, size) }.to_vec())
            }
        };
        unsafe { FreeLibrary(module) };
        bytes
    }

    /// Every piece of text in a resource, run together.
    pub fn text(bytes: &[u8]) -> String {
        let wide: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|two| u16::from_le_bytes([two[0], two[1]]))
            .collect();
        String::from_utf16_lossy(&wide).replace('\u{0}', " ")
    }
}

#[cfg(not(windows))]
mod read_back {
    use std::path::Path;
    pub const ICON: u16 = 3;
    pub const GROUP_ICON: u16 = 14;
    pub const VERSION: u16 = 16;
    pub fn resource(_: &Path, _: u16, _: u16) -> Option<Vec<u8>> {
        None
    }
    pub fn text(_: &[u8]) -> String {
        String::new()
    }
}
