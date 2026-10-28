//! What the compiler says when it cannot go on.
//!
//! Always in coordinates of the `.rsx` the developer wrote. The generated
//! `.rs` is an artefact of the build; nobody opens it on purpose, and a
//! message pointing into it is a message that wastes someone's afternoon.

use std::path::{Path, PathBuf};

/// A refusal, placed in the source the developer wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// Byte offset in the `.rsx`, kept so a line and column can be
    /// worked out later by whoever holds the source.
    pub offset: usize,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl CompileError {
    /// Place a message at a byte offset in `source`, counting the line and
    /// column the way an editor does: both start at one.
    pub fn at(source: &str, offset: usize, message: impl Into<String>) -> Self {
        let offset = offset.min(source.len());
        let before = &source[..offset];
        let line = before.matches('\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let column = source[line_start..offset].chars().count() + 1;

        Self {
            offset,
            file: PathBuf::new(),
            line,
            column,
            message: message.into(),
        }
    }

    /// Name the file the offset was in. The scanner and the parser work on
    /// a string and do not know it; whoever read the file does.
    pub fn in_file(mut self, path: &Path) -> Self {
        self.file = path.to_path_buf();
        self
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file.display(),
            self.line,
            self.column,
            self.message
        )
    }
}

impl std::error::Error for CompileError {}

impl CompileError {
    /// A refusal that knows where it happened but not what the source
    /// looks like. The emitter is in that position: it works on a node
    /// tree, and the tree carries offsets, not text.
    pub fn flagged(offset: usize, message: impl Into<String>) -> Self {
        Self {
            file: PathBuf::new(),
            offset,
            line: 0,
            column: 0,
            message: message.into(),
        }
    }

    /// Fill in the line and column of a flagged error, once somebody who
    /// has the source is holding it.
    pub fn resolve(self, source: &str) -> Self {
        if self.line != 0 {
            return self;
        }
        Self {
            file: self.file,
            ..Self::at(source, self.offset, self.message)
        }
    }
}
