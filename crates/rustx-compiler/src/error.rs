//! What the compiler says when it cannot go on.
//!
//! Always in coordinates of the `.rsx` the developer wrote. The generated
//! `.rs` is an artefact of the build; nobody opens it on purpose, and a
//! message pointing into it is a message that wastes someone's afternoon.

use std::path::{Path, PathBuf};

/// A refusal, placed in the source the developer wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
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
