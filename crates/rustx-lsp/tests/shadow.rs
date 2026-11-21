//! The `.rs` that stands in for the `.rsx` an editor is holding.
//!
//! `rust-analyzer` reads Rust files, and the Rust of a page already
//! exists: the attribute macro includes it by path, so it is part of the
//! application's crate and has the whole vocabulary of that crate. The
//! server's job is to keep it in step with what is being typed, and to
//! know where it is.

use std::fs;

use rustx_lsp::shadow;

/// A file just written, read back. A virus scanner on a CI runner may hold a
/// file for a moment after it is written, and a missing one then is not a
/// shadow that was not written: try again a few times.
fn read_back(path: &std::path::Path) -> std::io::Result<String> {
    let mut last = fs::read_to_string(path);
    for _ in 0..10 {
        if last.is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        last = fs::read_to_string(path);
    }
    last
}

/// A throwaway application: a manifest, an `app/`, one page.
fn an_application(page: &str) -> (std::path::PathBuf, String) {
    let root = std::env::temp_dir().join(format!(
        "rustx-shadow-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(root.join("app")).expect("an app directory");
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"shadowed\"\n").expect("a manifest");
    let page_path = root.join("app").join("page.rsx");
    fs::write(&page_path, page).expect("a page");
    let uri = shadow::uri_of(&page_path);
    (root, uri)
}

const GOOD: &str = "fn Page() -> Element {\n    return <p>bonjour</p>;\n}\n";
const BETTER: &str = "fn Page() -> Element {\n    return <p>bonsoir</p>;\n}\n";
const BAD: &str = "fn Page() -> Element {\n    return <p>bonjour</dvi>;\n}\n";

#[test]
fn a_page_becomes_a_rust_file_where_the_build_puts_one() {
    let (root, uri) = an_application(GOOD);
    let shadow = shadow::of(&uri, GOOD).expect("a shadow");
    let written = read_back(&shadow.path).expect("the shadow is on disk");
    assert!(written.contains("\"p\""), "got {written}");
    // Where the build puts it, so `rust-analyzer` is reading the file
    // the application actually compiles.
    let expected = rustx_compiler::build::generated_directory(&root).join("page.rs");
    assert_eq!(shadow.path, expected);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_change_rewrites_the_shadow() {
    let (root, uri) = an_application(GOOD);
    shadow::of(&uri, GOOD).expect("a shadow");
    let shadow = shadow::of(&uri, BETTER).expect("a second shadow");
    let written = read_back(&shadow.path).expect("on disk");
    assert!(written.contains("bonsoir"), "got {written}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_page_that_does_not_compile_leaves_the_last_shadow_alone() {
    // Half a tag is what a `.rsx` is most of the time. Overwriting the
    // shadow with it would turn every keystroke into a file
    // `rust-analyzer` cannot parse, and completion would go out exactly
    // when it is wanted.
    let (root, uri) = an_application(GOOD);
    let shadow = shadow::of(&uri, GOOD).expect("a shadow");
    assert!(shadow::of(&uri, BAD).is_none());
    let written = read_back(&shadow.path).expect("on disk");
    assert!(written.contains("bonjour"), "got {written}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_shadow_carries_the_map_of_the_page_it_came_from() {
    let (root, uri) = an_application(GOOD);
    let shadow = shadow::of(&uri, GOOD).expect("a shadow");
    // Line 1 is `fn Page()`, untouched, so a position on it crosses.
    assert_eq!(shadow.map.to_generated_position(1, 4), Some((1, 4)));
    // The tag on line 2 does not.
    assert_eq!(shadow.map.to_generated_position(2, 16), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_file_outside_an_application_has_no_shadow() {
    let loose = std::env::temp_dir().join("rustx-loose-page.rsx");
    fs::write(&loose, GOOD).expect("a page");
    assert!(shadow::of(&shadow::uri_of(&loose), GOOD).is_none());
    let _ = fs::remove_file(&loose);
}

/// La meme chose la ou le chemin n'a pas de lettre de lecteur : la
/// barre de tete est la racine, et c'est elle que la version Windows de
/// cette fonction mangeait -- le serveur cherchait alors `tmp/...` sous
/// le dossier courant, ne le trouvait pas, et aucune page n'avait
/// d'ombre.
#[test]
fn a_uri_without_a_drive_keeps_the_root_it_came_with() {
    if cfg!(windows) {
        return;
    }
    let path = std::path::Path::new("/tmp/rustx dev/app/page.rsx");
    let uri = shadow::uri_of(path);
    assert_eq!(uri, "file:///tmp/rustx%20dev/app/page.rsx", "got {uri}");
    assert_eq!(shadow::path_of(&uri).as_deref(), Some(path));
}

#[test]
fn a_uri_and_a_path_are_the_same_thing_twice() {
    let path = std::path::Path::new("C:\\Users\\dev\\app\\page.rsx");
    let uri = shadow::uri_of(path);
    assert!(uri.starts_with("file:///"), "got {uri}");
    assert_eq!(shadow::path_of(&uri).as_deref(), Some(path));
}

#[test]
fn each_change_tells_the_downstream_a_new_version() {
    // A document version that does not move is a document
    // `rust-analyzer` is entitled to ignore, and the file it then
    // answers about is the one before the change. Every change this
    // server sends is a new version of the shadow.
    let first = rustx_lsp::shadow::changed("file:///a.rs", "fn main() {}");
    let second = rustx_lsp::shadow::changed("file:///a.rs", "fn main() {} // more");
    let version = |body: &str| {
        let at = body.find("\"version\":").expect("a version") + "\"version\":".len();
        body[at..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<usize>()
            .expect("a number")
    };
    assert!(
        version(&second) > version(&first),
        "the version did not move: {first} then {second}"
    );
}

#[test]
fn an_answer_in_a_generated_file_is_told_in_the_file_the_developer_wrote() {
    let root = std::env::temp_dir().join(format!("rustx-back-{}", std::process::id()));
    std::fs::create_dir_all(root.join("app")).expect("app");
    std::fs::create_dir_all(root.join("src")).expect("src");
    std::fs::write(root.join("Cargo.toml"), "[package]\nname=\"a\"\n").expect("manifest");
    std::fs::write(root.join("app/page.rsx"), "let mut n: u8 = 0;\n").expect("page");
    let page_uri = rustx_lsp::shadow::uri_of(&root.join("app/page.rsx"));
    let shadow = rustx_lsp::shadow::of(&page_uri, "let mut n: u8 = 0;\n").expect("a shadow");
    // The way rust-analyzer writes a Windows file: another case, an
    // escaped colon. Textual replacement missed it and the definition
    // went to `target/`.
    let theirs = shadow.uri.replacen("file:///C:", "file:///c%3A", 1);
    let src = rustx_lsp::shadow::uri_of(&shadow.directory.join("src/ffmpeg.rs"));
    let answer = format!("[{{\"uri\":\"{theirs}\"}},{{\"uri\":\"{src}\"}}]");
    let back = rustx_lsp::shadow::retargeted(&answer, &page_uri, &shadow);
    assert!(!back.contains("rustx-pages"), "still generated: {back}");
    assert!(back.contains(&page_uri), "{back}");
    assert!(back.contains("/src/ffmpeg.rs"), "{back}");
    let _ = std::fs::remove_dir_all(&root);
}
