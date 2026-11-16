//! The JavaScript half of the boundary, which the framework brings.
//!
//! An application says which libraries it uses; it does not write the
//! bridge. What is tested here is the part a developer meets when they
//! make a mistake: a component that is not there has to name what was
//! asked for and where it was looked for.

use std::path::Path;
use std::process::Command;

const RUNTIME: &str = include_str!("../assets/runtime.js");

#[test]
fn the_runtime_is_in_the_binary_not_on_disk() {
    // Compiled in, so an application has no file to copy and no script
    // tag to write.
    assert!(
        RUNTIME.contains("__rustx_react"),
        "the runtime is missing its half of the contract"
    );
    assert!(
        RUNTIME.contains("window.rustx"),
        "nothing installs `rustx.use`"
    );
}

/// The runtime, run under Node with a question asked of it.
fn ask(script: &str) -> Option<String> {
    let directory = std::env::temp_dir().join(format!("rustx-runtime-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a directory");
    // A pair of files of its own per question: the tests run at once,
    // and all of them writing one `runtime.js` and one `asked.js` ran
    // node on a file another question was halfway through writing.
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let at = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let runtime = directory.join(format!("runtime-{at}.js"));
    std::fs::write(&runtime, RUNTIME).expect("a file");
    let asked = directory.join(format!("asked-{at}.js"));
    std::fs::write(
        &asked,
        format!(
            "const rustx = require({});\n{script}\n",
            serde_path(&runtime)
        ),
    )
    .expect("a file");

    let output = Command::new("node").arg(&asked).output().ok()?;
    if !output.status.success() {
        panic!(
            "node refused the runtime:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// A path as a JavaScript string literal.
fn serde_path(path: &Path) -> String {
    let written = path.to_string_lossy().replace('\\', "/");
    format!("\"{written}\"")
}

#[test]
fn a_missing_module_is_named() {
    let Some(said) = ask("rustx.use({ recharts: { LineChart: 1 } });\n\
         console.log(rustx.resolve('recarts::LineChart').error ?? 'no complaint');")
    else {
        // Node is how this file is read outside a browser. Without it
        // the runtime is still compiled in and still tested by the
        // hybrid application's own scenario.
        eprintln!("node is not on this machine: the runtime's own test did not run");
        return;
    };
    assert!(said.contains("recarts"), "got {said}");
    assert!(
        said.contains("does not have `recarts` installed"),
        "got {said}"
    );
}

#[test]
fn a_missing_export_is_named() {
    let Some(said) = ask("rustx.use({ recharts: { LineChart: 1, XAxis: 2 } });\n\
         console.log(rustx.resolve('recharts::Lne').error ?? 'no complaint');")
    else {
        eprintln!("node is not on this machine: the runtime's own test did not run");
        return;
    };
    assert!(said.contains("Lne"), "got {said}");
    assert!(said.contains("does not export it"), "got {said}");
    // And what it does export, so the developer can see the name they
    // meant.
    assert!(said.contains("LineChart"), "got {said}");
}

#[test]
fn a_component_that_is_there_resolves_to_it() {
    let Some(said) = ask("rustx.use({ recharts: { LineChart: 'the chart' } });\n\
         console.log(rustx.resolve('recharts::LineChart').component);")
    else {
        return;
    };
    assert_eq!(said, "the chart");
}

#[test]
fn a_bare_name_is_found_in_a_declared_library() {
    // No prefix in the page: the name is looked for among the libraries
    // the application installed.
    let Some(said) = ask("rustx.use({ recharts: { LineChart: 'the chart' } });\n\
         console.log(rustx.resolve('LineChart').component);")
    else {
        return;
    };
    assert_eq!(said, "the chart");
}

#[test]
fn a_dotted_name_walks_into_an_export() {
    let Some(said) = ask(
        "rustx.use({ radixDialog: { Dialog: { Trigger: 'the trigger' } } });\n\
         console.log(rustx.resolve('Dialog.Trigger').component);",
    ) else {
        return;
    };
    assert_eq!(said, "the trigger");
}

#[test]
fn a_name_two_libraries_export_warns_and_names_both() {
    // The first wins, because something has to, and the page is told —
    // with the explicit form to write if it wanted the other one.
    let Some(said) = ask(
        "const warnings = [];\nconsole.warn = (text) => warnings.push(text);\n\
         rustx.use({ one: { Button: 1 }, two: { Button: 2 } });\n\
         const found = rustx.resolve('Button');\n\
         console.log(JSON.stringify([found.component, warnings.length, warnings[0] ?? '']));",
    ) else {
        return;
    };
    assert!(
        said.contains("[1,1,"),
        "the first library should win: {said}"
    );
    assert!(said.contains("one"), "got {said}");
    assert!(said.contains("two"), "got {said}");
    // And the explicit form for the one it did not take.
    assert!(
        said.contains("two::Button"),
        "no explicit form offered: {said}"
    );
}

#[test]
fn a_name_nobody_exports_names_both_places_it_was_looked_for() {
    let Some(said) = ask("rustx.use({ recharts: { LineChart: 1 } });\n\
         console.log(rustx.resolve('Preview').error ?? 'no complaint');")
    else {
        return;
    };
    assert!(said.contains("Preview"), "got {said}");
    // The page itself, and the libraries.
    assert!(
        said.contains("recharts"),
        "the libraries were not named: {said}"
    );
    assert!(
        said.contains("{Preview()}") || said.contains("fn Preview"),
        "a page's own component was not offered: {said}"
    );
}

#[test]
fn a_plain_tag_is_its_own_name() {
    // What React builds from a string. Nothing to look up.
    let Some(said) = ask("console.log(rustx.resolve('div').component);") else {
        return;
    };
    assert_eq!(said, "div");
}

#[test]
fn the_applications_own_component_wins_over_the_library_it_was_built_from() {
    // `components/ui/dialog.tsx` is built *on* `@radix-ui/react-dialog`,
    // and both export `Dialog`. A page that writes `<Dialog>` means its
    // own — which is why nobody has to write `<ui::Dialog>`.
    let Some(said) = ask(
        "rustx.use({ radixUiReactDialog: { Dialog: 'the primitive' }, \
         ui: { Dialog: 'the own one' } });\n\
         console.log(rustx.resolve('Dialog').component);",
    ) else {
        return;
    };
    assert_eq!(said, "the own one");
}

#[test]
fn the_explicit_form_still_asks_for_the_primitive() {
    // Removing the prefix must not remove the way back to the library.
    let Some(said) = ask(
        "rustx.use({ radixUiReactDialog: { Dialog: 'the primitive' }, \
         ui: { Dialog: 'the own one' } });\n\
         console.log(rustx.resolve('radixUiReactDialog::Dialog').component);",
    ) else {
        return;
    };
    assert_eq!(said, "the primitive");
}
