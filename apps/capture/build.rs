//! Turns every `.rsx` under `app/` into Rust before cargo compiles it.
//!
//! This is the whole build story of a `rustx` application: one build
//! script, one command. A later plan replaces it with a `rustx` CLI; the
//! translation it calls is the same.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=app");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    match rustx_compiler::build::compile_app(Path::new("app"), &out) {
        Ok(generated) => {
            for path in generated {
                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
        Err(error) => {
            // Printed as cargo's own error, in coordinates of the `.rsx`.
            println!("cargo::error={error}");
            std::process::exit(1);
        }
    }
}
