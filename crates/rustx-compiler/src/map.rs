//! Keeping the generated file's lines tied to the ones that were written.
//!
//! A JSX region takes several lines in the `.rsx` and one line in the
//! `.rs`, so from the first region onwards the two files drift apart. A
//! `rustc` error names a line of the `.rs`; without this map the developer
//! is handed a line number for a file they never wrote, in a directory
//! they have no reason to know about, and the gap grows with the file.
//!
//! The map is a list of breakpoints — one at the start, one after each
//! region — and between two breakpoints the correspondence is a constant
//! translation, because nothing in between changes the line count.

/// One point where the two files agree, in one-based line numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breakpoint {
    pub generated: usize,
    pub rsx: usize,
}

/// A JSX region, in one-based lines and columns of the `.rsx`.
///
/// From its `<` to its last line, several lines of `.rsx` are one Rust
/// expression. A column in there has no equivalent, and a language
/// server that invented one would complete the wrong word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub first_line: usize,
    /// Where the `<` is on that first line. What precedes it — a
    /// `return`, a `let x =` — is the developer's own Rust, untouched.
    pub column: usize,
    pub last_line: usize,
}

/// Which `.rsx` line a `.rs` line came from, and back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMap {
    points: Vec<Breakpoint>,
    /// The regions, in order.
    regions: Vec<Region>,
    /// The lines state promotion rebuilt, in order. A `let mut` that
    /// became a cell is a line nobody wrote.
    rewritten: Vec<usize>,
}

impl PositionMap {
    pub(crate) fn new(points: Vec<Breakpoint>) -> Self {
        Self {
            points,
            regions: Vec::new(),
            rewritten: Vec::new(),
        }
    }

    pub(crate) fn with_shape(mut self, regions: Vec<Region>, rewritten: Vec<usize>) -> Self {
        self.regions = regions;
        self.rewritten = rewritten;
        self
    }

    /// The regions this file holds.
    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// Where a position of the `.rsx` is in the generated Rust.
    ///
    /// `None` when it is nowhere: inside a region, or on a line the
    /// rewrite rebuilt. Lines are one-based, and so are columns.
    pub fn to_generated_position(&self, line: usize, column: usize) -> Option<(usize, usize)> {
        if self.opaque(line, column) {
            return None;
        }
        Some((self.to_generated(line)?, column))
    }

    /// Where a position of the generated Rust is in the `.rsx`.
    pub fn to_rsx_position(&self, line: usize, column: usize) -> Option<(usize, usize)> {
        let rsx_line = self.to_rsx(line)?;
        if self.opaque(rsx_line, column) {
            return None;
        }
        Some((rsx_line, column))
    }

    /// Whether this position of the `.rsx` has no counterpart.
    fn opaque(&self, line: usize, column: usize) -> bool {
        if self.rewritten.contains(&line) {
            return true;
        }
        self.regions.iter().any(|region| {
            if line == region.first_line {
                column >= region.column
            } else {
                line > region.first_line && line <= region.last_line
            }
        })
    }

    /// The `.rsx` line that produced this line of generated Rust.
    pub fn to_rsx(&self, generated_line: usize) -> Option<usize> {
        let point = self
            .points
            .iter()
            .rev()
            .find(|point| point.generated <= generated_line)?;
        Some(point.rsx + (generated_line - point.generated))
    }

    /// Where a `.rsx` line ended up.
    ///
    /// The two files have the same number of lines — a region puts back
    /// the newlines its expression swallowed — so this is the identity
    /// today. It stays here because the day a region places its children
    /// on their own lines, it will not be.
    pub fn to_generated(&self, rsx_line: usize) -> Option<usize> {
        let point = self
            .points
            .iter()
            .rev()
            .find(|point| point.rsx <= rsx_line)?;
        Some(point.generated + (rsx_line - point.rsx))
    }

    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.points
    }

    /// The map as it is written next to the generated file: one
    /// `generated:rsx` pair per line, in order.
    /// The regions and the rewritten lines follow the breakpoints, each
    /// on a line of its own under a name: a reader that does not know
    /// them skips them, which is what the old short form needs.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for point in &self.points {
            text.push_str(&format!("{}:{}\n", point.generated, point.rsx));
        }
        for region in &self.regions {
            text.push_str(&format!(
                "region {} {} {}\n",
                region.first_line, region.column, region.last_line
            ));
        }
        for line in &self.rewritten {
            text.push_str(&format!("rewrote {line}\n"));
        }
        text
    }

    /// Read back what [`PositionMap::to_text`] wrote. Unreadable lines are
    /// skipped: a damaged map should degrade into worse line numbers, not
    /// into a build that refuses to run.
    pub fn from_text(text: &str) -> Self {
        let numbers = |line: &str, name: &str| -> Option<Vec<usize>> {
            let rest = line.strip_prefix(name)?;
            rest.split_whitespace()
                .map(|word| word.parse().ok())
                .collect()
        };
        let mut regions = Vec::new();
        let mut rewritten = Vec::new();
        for line in text.lines() {
            if let Some(found) = numbers(line, "region ")
                && let [first_line, column, last_line] = found[..]
            {
                regions.push(Region {
                    first_line,
                    column,
                    last_line,
                });
            }
            if let Some(found) = numbers(line, "rewrote ")
                && let [at] = found[..]
            {
                rewritten.push(at);
            }
        }

        let points = text
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter_map(|(generated, rsx)| {
                Some(Breakpoint {
                    generated: generated.trim().parse().ok()?,
                    rsx: rsx.trim().parse().ok()?,
                })
            })
            .collect();
        Self {
            points,
            regions,
            rewritten,
        }
    }
}
