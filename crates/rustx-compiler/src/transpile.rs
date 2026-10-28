//! A `.rsx` in, ordinary Rust out.
//!
//! Everything outside a JSX region is copied byte for byte. That is not
//! an optimisation — it is the contract: the developer's Rust is their
//! Rust, and a compiler that reformats it has started making decisions
//! nobody asked it to make.

use crate::emit::emit;
use crate::error::CompileError;
use crate::map::{Breakpoint, PositionMap};
use crate::parse::parse_jsx;
use crate::scan::find_jsx;

/// Generated Rust and the map back to the file it came from.
#[derive(Debug, Clone)]
pub struct Transpiled {
    pub rust: String,
    pub map: PositionMap,
}

/// Translate one `.rsx` source.
pub fn transpile(source: &str) -> Result<Transpiled, CompileError> {
    let spans = find_jsx(source).map_err(|error| error.resolve(source))?;

    let mut rust = String::with_capacity(source.len());
    let mut points = vec![Breakpoint {
        generated: 1,
        rsx: 1,
    }];
    let mut cursor = 0;
    // Where we are in each file, in one-based lines.
    let mut generated_line = 1;
    let mut rsx_line = 1;

    for span in spans {
        let plain = &source[cursor..span.start];
        rust.push_str(plain);
        let crossed = plain.matches('\n').count();
        generated_line += crossed;
        rsx_line += crossed;

        let node = parse_jsx(source, span).map_err(|error| error.resolve(source))?;
        let code = emit(&node).map_err(|error| error.resolve(source))?;
        // A region is one Rust expression however many lines it took to
        // write, so the newlines it swallowed are put back after it. That
        // keeps every later line of the generated file on the number it
        // has in the `.rsx` — which is what makes a `rustc` error land
        // where the developer is looking instead of drifting further away
        // with every region above it.
        rust.push_str(&code);
        let swallowed = source[span.start..span.end].matches('\n').count();
        for _ in 0..swallowed {
            rust.push('\n');
        }
        generated_line += swallowed;
        rsx_line += swallowed;

        points.push(Breakpoint {
            generated: generated_line,
            rsx: rsx_line,
        });
        cursor = span.end;
    }
    rust.push_str(&source[cursor..]);

    Ok(Transpiled {
        rust,
        map: PositionMap::new(points),
    })
}
