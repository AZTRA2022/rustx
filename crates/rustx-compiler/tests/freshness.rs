//! What a second build must not do.
//!
//! The pages the compiler writes are `include!`d by `#[rustx::main]`,
//! so cargo records them as sources of the crate. A build that rewrites
//! them with the same bytes still gives them a new modification date,
//! and cargo then finds the crate older than its own sources: it
//! rebuilds, which writes them again, for ever. `rustx dev` rebuilt a
//! project nobody had touched, and this is the reason.
//!
//! The rule, therefore: **writing is only writing when the bytes
//! differ.** These laws measure it where it is observable — the
//! modification date of the generated files.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rustx_compiler::build::{PAGES, compile_app};

/// A directory of this test's own, emptied first.
fn room(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rustx-freshness-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("a directory under the temporary one");
    path
}

/// An `app/` with one page in it.
fn an_app(at: &Path, body: &str) -> PathBuf {
    let app = at.join("app");
    std::fs::create_dir_all(&app).expect("app/");
    std::fs::write(app.join("page.rsx"), body).expect("app/page.rsx");
    app
}

/// When each generated file under `at` was last written.
fn dates(at: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut found = Vec::new();
    let mut left = vec![at.to_path_buf()];
    while let Some(directory) = left.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.is_dir() {
                left.push(path);
            } else {
                let when = path
                    .metadata()
                    .and_then(|about| about.modified())
                    .expect("a file this test just wrote has a date");
                found.push((path, when));
            }
        }
    }
    found.sort();
    found
}

/// Long enough that a second write would have a date of its own.
///
/// Windows keeps a file's modification time to about 100 ns, and the
/// clock a build reads moves in bigger steps than that; a tenth of a
/// second is above every one of them, and this test spends it once.
fn a_moment() {
    std::thread::sleep(std::time::Duration::from_millis(120));
}

#[test]
fn a_second_build_touches_nothing() {
    let room = room("twice");
    let app = an_app(&room, "fn Page() -> Element {\n    return <p>hello</p>;\n}\n");
    let out = room.join("out");

    compile_app(&app, &out).expect("the first build");
    let before = dates(&out);
    assert!(
        before.iter().any(|(path, _)| path.ends_with(PAGES)),
        "the first build writes the table"
    );

    a_moment();
    compile_app(&app, &out).expect("the second build");
    let after = dates(&out);

    assert_eq!(
        before, after,
        "a build that changes nothing must leave every generated file \
         exactly as it was — a new date is what makes cargo rebuild a \
         project nobody touched"
    );
}

#[test]
fn a_changed_page_is_written_again() {
    let room = room("changed");
    let app = an_app(&room, "fn Page() -> Element {\n    return <p>hello</p>;\n}\n");
    let out = room.join("out");

    compile_app(&app, &out).expect("the first build");
    let before = dates(&out);

    a_moment();
    std::fs::write(
        app.join("page.rsx"),
        "fn Page() -> Element {\n    return <p>goodbye</p>;\n}\n",
    )
    .expect("the page, edited");
    compile_app(&app, &out).expect("the second build");
    let after = dates(&out);

    assert_ne!(
        before, after,
        "skipping an identical write must not skip a real one"
    );
    let page = out.join("page.rs");
    let written = std::fs::read_to_string(&page).expect("the generated page");
    assert!(
        written.contains("goodbye"),
        "the page that changed is the page that was written: {written}"
    );
}
