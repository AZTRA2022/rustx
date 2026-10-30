//! What a `build.rs` contains: one line.
//!
//! ```text
//! // build.rs
//! fn main() {
//!     rustx_build::app();
//! }
//! ```
//!
//! Everything else — which directory holds the pages, where the
//! generated Rust goes, which files cargo should watch, how a mistake in
//! a `.rsx` becomes a cargo error in the coordinates of the `.rsx` — is
//! the same in every application, so it is written once, here.

use std::path::{Path, PathBuf};

use rustx_compiler::error::CompileError;

/// Compile the pages of `app/` and hand the router their table.
///
/// For an application whose screens are its files, which is every
/// application that uses `#[rustx::main]`.
pub fn app() {
    run(rustx_compiler::build::compile_app);
}

/// Compile the pages of `app/` without a route table, for an
/// application that mounts its own components — `#[rustx::main(pages)]`.
pub fn pages() {
    run(rustx_compiler::build::compile_pages);
}

fn run(compile: fn(&Path, &Path) -> Result<Vec<PathBuf>, CompileError>) {
    println!("cargo::rerun-if-changed=app");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    match compile(Path::new("app"), &out) {
        Ok(generated) => {
            // Only the sources are worth watching: a generated file
            // changing is this script having run, not a reason to run
            // it again.
            for path in generated.iter().filter(|path| !path.starts_with(&out)) {
                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
        Err(error) => {
            // Cargo's own error, in the coordinates of the `.rsx`.
            println!("cargo::error={error}");
            std::process::exit(1);
        }
    }
}
