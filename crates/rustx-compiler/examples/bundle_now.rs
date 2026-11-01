//! Build one application's bundle and stylesheet, and say what came out.
//!
//! `cargo run -p rustx-compiler --example bundle_now -- <app>`
//!
//! A diagnostic: the bundle is normally built from inside a build, where
//! a failure arrives as one line of a compiler error. This runs the same
//! code with the error in full and the sizes printed, which is what
//! answers "did Tailwind actually read my pages".

fn main() {
    let Some(app) = std::env::args().nth(1) else {
        eprintln!("bundle_now <app>");
        std::process::exit(2);
    };
    let app = std::path::Path::new(&app);
    println!("tailwind: {:?}", rustx_compiler::bundle::dialect_in(app));
    match rustx_compiler::bundle::build(app) {
        Ok(built) => {
            for (what, path) in [("bundle", built.bundle), ("stylesheet", built.stylesheet)] {
                match path {
                    Some(path) => {
                        let size = std::fs::metadata(&path).map(|it| it.len()).unwrap_or(0);
                        println!("{what}: {} ({size} bytes)", path.display());
                    }
                    None => println!("{what}: none"),
                }
            }
        }
        Err(why) => {
            println!("{why}");
            std::process::exit(1);
        }
    }
}
