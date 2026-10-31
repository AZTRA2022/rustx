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
    // State promotion runs first: it rewrites Rust, never JSX, and it
    // never moves a line, so the scan below sees the same shape.
    let written = source;
    let source = &crate::rewrite::promote_state(source)?;
    // Which lines the rewrite rebuilt, read off the two files rather
    // than reported by the rewrite: every edit it makes stays on the
    // line it started on, so the two have the same lines, and a line
    // that differs is a line nobody wrote. A language server must not
    // offer a column on one of those.
    let rewritten: Vec<usize> = written
        .lines()
        .zip(source.lines())
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(at, _)| at + 1)
        .collect();

    let spans = find_jsx(source).map_err(|error| error.resolve(source))?;

    // What a capitalised tag needs to become a call: the components
    // this file declares, and their parameters in order.
    let components = crate::promote::component_signatures(source);
    let context = crate::emit::Context {
        components: &components,
        source,
    };

    let mut rust = String::with_capacity(source.len());
    let mut points = vec![Breakpoint {
        generated: 1,
        rsx: 1,
    }];
    let mut cursor = 0;
    // Where we are in each file, in one-based lines.
    let mut generated_line = 1;
    let mut rsx_line = 1;

    let mut regions = Vec::new();
    for span in spans {
        let plain = &source[cursor..span.start];
        rust.push_str(plain);
        let crossed = plain.matches('\n').count();
        generated_line += crossed;
        rsx_line += crossed;

        // Where the `<` sits on its line: everything before it is the
        // developer's own Rust and keeps its columns.
        let line_start = source[..span.start].rfind('\n').map_or(0, |at| at + 1);
        let column = source[line_start..span.start].chars().count() + 1;

        let node = parse_jsx(source, span).map_err(|error| error.resolve(source))?;
        let code = emit(&node, &context).map_err(|error| error.resolve(source))?;
        // A region is one Rust expression however many lines it took to
        // write, so the newlines it swallowed are put back after it. That
        // keeps every later line of the generated file on the number it
        // has in the `.rsx` — which is what makes a `rustc` error land
        // where the developer is looking instead of drifting further away
        // with every region above it.
        rust.push_str(&code);
        let swallowed = source[span.start..span.end].matches('\n').count();
        regions.push(crate::map::Region {
            first_line: rsx_line,
            column,
            last_line: rsx_line + swallowed,
        });
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
        map: PositionMap::new(points).with_shape(regions, rewritten),
    })
}
