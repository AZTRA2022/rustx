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
    let runtime = directory.join("runtime.js");
    std::fs::write(&runtime, RUNTIME).expect("a file");
    // A file of its own per question: the tests run at once, and three
    // of them writing one `asked.js` ran node on a file half of another
    // question had written.
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let asked = directory.join(format!(
        "asked-{}.js",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
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
    assert!(said.contains("never said it uses"), "got {said}");
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
