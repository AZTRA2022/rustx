//! What `rustx` says when it is asked what it does.
//!
//! Every command, one line each, every flag any of them takes, and the
//! sentence that says what a reload keeps and what it does not. That
//! last part is here rather than only in the documentation because the
//! place a developer finds out that their component's state went back
//! to zero is the moment they watched it happen, and `--help` is the
//! nearest thing to hand.
//!
//! The parity between this text and what `arguments::read` accepts is a
//! law of the test suite, not a habit: a flag the command line takes
//! and the help does not mention is a feature nobody can find, and a
//! flag the help promises and the command line refuses is worse.

/// The whole of `rustx --help`.
pub fn text() -> String {
    format!(
        "rustx {} — native software, written in Rust, drawn by a browser engine.\n\
         \n\
         \x20 rustx new <name>    write a project that compiles the first time\n\
         \x20 rustx dev           watch the files, rebuild, bring the window back\n\
         \x20 rustx build         build, and give the .exe its own name and icon\n\
         \x20 rustx build --release\n\
         \x20                     the build that ships, and what ships beside it\n\
         \x20 rustx run           build and run once, without watching\n\
         \x20 rustx run --release the same, from the release build\n\
         \x20 rustx --version     the version of this command line\n\
         \x20 rustx --help        this\n\
         \n\
         `rustx new` writes the project into the directory you are in, under the\n\
         name you gave it. There is no flag for anywhere else: `cd` first.\n\
         \n\
         `rustx dev` relaunches the application; it does not reload native code\n\
         with the process still alive. What comes back across a relaunch: the\n\
         shared state declared with a `let mut` at the top of a page, and the\n\
         page that was open. What does not: the cells inside a component, whose\n\
         identity is a place in code that has just changed, and any value this\n\
         cannot write down — anything that is not a number, a string or a yes.\n\
         It always builds in debug; a release build of a browser engine is not\n\
         something to wait for between two keystrokes.\n\
         \n\
         `rustx build` adds nothing to what cargo does except the icon and the\n\
         version in the file's properties. `cargo build` still works; the\n\
         binary it makes simply looks anonymous in an explorer window. The one\n\
         path it prints is the binary; a release build also says, on the error\n\
         stream, what Chromium needs beside that binary to start.\n\
         \n\
         Where it builds is cargo's business and cargo's alone: CARGO_TARGET_DIR,\n\
         then `[build] target-dir` in a `.cargo/config.toml` at or above the\n\
         project, then `target/` at the root of the workspace. This command line\n\
         reads the same three in the same order, so it looks where cargo wrote.\n\
         \n\
         It needs cargo on the PATH, and nothing else: no node, no npm, no\n\
         Python, no shell. A project that reaches for a JavaScript library needs\n\
         npm, and says so itself when it does.\n",
        env!("CARGO_PKG_VERSION")
    )
}
