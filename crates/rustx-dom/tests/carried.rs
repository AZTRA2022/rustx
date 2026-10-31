//! What survives `rustx dev` relaunching the application.
//!
//! Not the process: this framework does not pretend to reload native
//! code with the process still alive, and the design document names that
//! as its own subsystem. What comes back is what has a name — the
//! `let mut` written at the top of a page — and the page that was open,
//! so the screen returns where it was instead of to the beginning.

use std::fs;
use std::path::PathBuf;

use rustx_dom::carried;
use rustx_dom::state::{InstanceId, Scope, Slot, cell};
use rustx_dom::store::{self, shared};

fn somewhere(what: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "rustx-carried-{what}-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
    ))
}

/// What `rustx dev` does between two builds: the same file, a process
/// that knows nothing.
fn a_restart(file: &std::path::Path) {
    store::reset();
    carried::forget();
    carried::carrying(file);
}

#[test]
fn a_shared_value_comes_back_after_a_restart() {
    let file = somewhere("value");
    carried::carrying(&file);

    let clicks = shared("clicks", || 0_i32);
    let name = shared("name", || "aztra".to_string());
    clicks.set(7);
    name.set("quelqu'un".to_string());

    // Written as it happens, not at the end: `rustx dev` kills the
    // window it is replacing, and a process that is killed says goodbye
    // to nobody.
    assert!(file.is_file(), "nothing was written");

    a_restart(&file);
    assert_eq!(shared("clicks", || 0_i32).get(), 7);
    assert_eq!(shared("name", || "aztra".to_string()).get(), "quelqu'un");

    let _ = fs::remove_file(&file);
}

#[test]
fn a_value_whose_shape_changed_is_thrown_away() {
    // `let mut n = 0;` became `let mut n = "0";` between two builds. The
    // name is the same and the value is not: the kept one is dropped and
    // the page starts from its own initialiser, because forcing it in is
    // how a reload corrupts an application instead of restoring it.
    let file = somewhere("shape");
    carried::carrying(&file);
    shared("n", || 0_i32).set(7);

    a_restart(&file);
    assert_eq!(shared("n", || "zero".to_string()).get(), "zero");

    let _ = fs::remove_file(&file);
}

#[test]
fn a_component_cell_starts_fresh() {
    // The limit, written black on white. A cell's identity is a place in
    // a file that has just been recompiled; a name is a name.
    let file = somewhere("cell");
    carried::carrying(&file);
    shared("kept", || 1_i32).set(2);
    {
        let _scope = Scope::enter(InstanceId(1));
        let private = cell(
            Slot {
                file: "page.rsx",
                line: 4,
                column: 9,
            },
            || 0_i32,
        );
        private.set(99);
    }
    carried::keep();

    let written = fs::read_to_string(&file).expect("a file");
    assert!(written.contains("kept"), "got {written}");
    assert!(
        !written.contains("99"),
        "a component's own cell was carried: {written}"
    );

    let _ = fs::remove_file(&file);
}

#[test]
fn the_page_that_was_open_is_the_page_that_opens() {
    let file = somewhere("page");
    carried::carrying(&file);
    assert_eq!(carried::page(), None, "nothing was carried into this one");

    carried::be_at("/settings/devices");
    a_restart(&file);
    assert_eq!(carried::page().as_deref(), Some("/settings/devices"));

    let _ = fs::remove_file(&file);
}

#[test]
fn a_file_that_is_nonsense_is_not_a_crash() {
    // Half a line, because the window was killed mid-write; or a file
    // somebody edited. An application that refuses to start over its own
    // cache is worse than one that starts over from its initialisers.
    let file = somewhere("nonsense");
    fs::write(&file, "clicks\ti32\t\x07not a number\nhalf a li").expect("a file");
    carried::carrying(&file);
    assert_eq!(shared("clicks", || 3_i32).get(), 3);
    let _ = fs::remove_file(&file);
}

#[test]
fn nothing_is_written_when_nothing_asked_for_it() {
    // Outside `rustx dev` there is no file to carry anything in, and an
    // application must not start writing one on its own.
    let file = somewhere("quiet");
    carried::forget();
    shared("clicks", || 0_i32).set(4);
    carried::keep();
    assert!(!file.exists());
}
