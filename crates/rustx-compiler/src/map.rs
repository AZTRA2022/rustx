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

/// Which `.rsx` line a `.rs` line came from, and back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionMap {
    points: Vec<Breakpoint>,
}

impl PositionMap {
    pub(crate) fn new(points: Vec<Breakpoint>) -> Self {
        Self { points }
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

    /// Where a `.rsx` line ended up. A line inside a JSX region collapses
    /// into the single generated line that region became.
    pub fn to_generated(&self, rsx_line: usize) -> Option<usize> {
        let point = self.points.iter().rev().find(|point| point.rsx <= rsx_line)?;
        Some(point.generated + (rsx_line - point.rsx))
    }

    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.points
    }

    /// The map as it is written next to the generated file: one
    /// `generated:rsx` pair per line, in order.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for point in &self.points {
            text.push_str(&format!("{}:{}\n", point.generated, point.rsx));
        }
        text
    }

    /// Read back what [`PositionMap::to_text`] wrote. Unreadable lines are
    /// skipped: a damaged map should degrade into worse line numbers, not
    /// into a build that refuses to run.
    pub fn from_text(text: &str) -> Self {
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
        Self { points }
    }
}
