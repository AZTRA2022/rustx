//! What `rustx` says when it is asked what it does.
//!
//! Four commands, one line each, and the sentence that says what a
//! reload keeps and what it does not. That last part is here rather than
//! only in the documentation because the place a developer finds out
//! that their component's state went back to zero is the moment they
//! watched it happen, and `--help` is the nearest thing to hand.

/// The whole of `rustx --help`.
pub fn text() -> String {
    format!(
        "rustx {} — native software, written in Rust, drawn by a browser engine.\n\
         \n\
         \x20 rustx new <name>   write a project that compiles the first time\n\
         \x20 rustx dev          watch the files, rebuild, bring the window back\n\
         \x20 rustx build        build, and give the .exe its own name and icon\n\
         \x20 rustx run          build and run once, without watching\n\
         \n\
         `rustx dev` relaunches the application; it does not reload native code\n\
         with the process still alive. What comes back across a relaunch: the\n\
         shared state declared with a `let mut` at the top of a page, and the\n\
         page that was open. What does not: the cells inside a component, whose\n\
         identity is a place in code that has just changed, and any value this\n\
         cannot write down — anything that is not a number, a string or a yes.\n\
         \n\
         `rustx build` adds nothing to what cargo does except the icon and the\n\
         version in the file's properties. `cargo build` still works; the\n\
         binary it makes simply looks anonymous in an explorer window.\n",
        env!("CARGO_PKG_VERSION")
    )
}
