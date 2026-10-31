//! What crosses a relaunch.
//!
//! `rustx dev` rebuilds and relaunches; it does not keep a process alive
//! across a recompilation of native code, and says so. What it can do is
//! bring the screen back where it was, and that is this module: the
//! values that have a *name* — the `let mut` written at the top of a
//! page — and the page that was open, written to a file as they change
//! and read back when the new process starts.
//!
//! Names, and only names. A component's own cell is identified by a
//! place in a file that has just been edited, so there is nothing to
//! match it against; it starts from its initialiser, and the limit is
//! said out loud rather than papered over.
//!
//! # Why it writes as it goes
//!
//! Because the process is killed, not asked to leave. A window that is
//! about to be replaced is killed, and nothing that waits for a tidy
//! shutdown would ever run. So each change writes the file — a handful
//! of short lines, and only when something asked for them: outside
//! `rustx dev` nothing here touches the disk.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A value that crossed, as it was written: what kind it was, and what
/// it said.
#[derive(Clone)]
struct Kept {
    kind: String,
    text: String,
}

/// How to ask a live value what it says now.
type Reporter = Box<dyn Fn() -> Option<Kept>>;

thread_local! {
    /// The file to write, when there is one.
    static WHERE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// What the file said when this process started.
    static READ: RefCell<HashMap<String, Kept>> = RefCell::new(HashMap::new());
    /// The live values, by name, in the order they appeared.
    static REPORTERS: RefCell<Vec<(String, Reporter)>> = const { RefCell::new(Vec::new()) };
    /// The page that is open.
    static PAGE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The name under which the open page is written.
const PAGE_KEY: &str = "\u{1}page";

/// Carry state in this file: read what is in it, and write to it from
/// now on.
///
/// Called by the engine when `RUSTX_CARRIED` names a file, which is what
/// `rustx dev` sets and nothing else does.
pub fn carrying(file: &Path) {
    WHERE.with(|held| *held.borrow_mut() = Some(file.to_path_buf()));
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let read = parse(&text);
    if let Some(page) = read.get(PAGE_KEY) {
        PAGE.with(|held| *held.borrow_mut() = Some(page.text.clone()));
    }
    READ.with(|held| *held.borrow_mut() = read);
}

/// Forget everything: the file, what was read, and who reports.
///
/// For a test, and for a context being replaced.
pub fn forget() {
    WHERE.with(|held| *held.borrow_mut() = None);
    READ.with(|held| held.borrow_mut().clear());
    REPORTERS.with(|held| held.borrow_mut().clear());
    PAGE.with(|held| *held.borrow_mut() = None);
}

/// The page that was open when the last process was replaced.
pub fn page() -> Option<String> {
    PAGE.with(|held| held.borrow().clone())
}

/// Say which page is open now.
pub fn be_at(path: &str) {
    let changed = PAGE.with(|held| {
        let mut held = held.borrow_mut();
        if held.as_deref() == Some(path) {
            return false;
        }
        *held = Some(path.to_string());
        true
    });
    if changed {
        keep();
    }
}

/// What this name was carried in with, if it was, and if it is still the
/// same kind of value.
///
/// A value whose shape changed between two builds is thrown away rather
/// than forced in: `let mut n = 0;` that became `let mut n = "0";` is a
/// different value with the same name, and injecting the old one is how
/// a reload corrupts an application instead of restoring it.
pub fn remembered<T: 'static>(name: &str) -> Option<T> {
    let kept = READ.with(|held| held.borrow().get(name).cloned())?;
    if kept.kind != std::any::type_name::<T>() {
        return None;
    }
    from_text::<T>(&kept.text)
}

/// Register a live value, so that it can be asked what it says.
///
/// `read` is called at every write; it must not panic, and must return
/// `None` for a value this module cannot write down.
pub(crate) fn report<T: 'static>(name: &str, read: impl Fn() -> Option<String> + 'static) {
    let kind = std::any::type_name::<T>().to_string();
    let reporter: Reporter = Box::new(move || {
        read().map(|text| Kept {
            kind: kind.clone(),
            text,
        })
    });
    REPORTERS.with(|held| {
        let mut held = held.borrow_mut();
        match held.iter_mut().find(|(known, _)| known == name) {
            Some(place) => place.1 = reporter,
            None => held.push((name.to_string(), reporter)),
        }
    });
}

/// Write what everything says now, if anything asked for a file.
///
/// Called on every write to a named value: a few short lines, and only
/// under `rustx dev`.
pub fn keep() {
    let Some(file) = WHERE.with(|held| held.borrow().clone()) else {
        return;
    };
    let mut text = String::new();
    if let Some(page) = page() {
        line(&mut text, PAGE_KEY, "page", &page);
    }
    REPORTERS.with(|held| {
        for (name, reporter) in held.borrow().iter() {
            if let Some(kept) = reporter() {
                line(&mut text, name, &kept.kind, &kept.text);
            }
        }
    });
    // A failure here is not the application's problem: the worst it
    // costs is a screen that comes back at the beginning.
    let _ = std::fs::write(file, text);
}

fn line(into: &mut String, name: &str, kind: &str, text: &str) {
    into.push_str(&escape(name));
    into.push('\t');
    into.push_str(kind);
    into.push('\t');
    into.push_str(&escape(text));
    into.push('\n');
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut letters = text.chars();
    while let Some(letter) = letters.next() {
        if letter != '\\' {
            out.push(letter);
            continue;
        }
        match letters.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// What a file says. A line this cannot read is a line it leaves out:
/// the file may well be half written, because the process that was
/// writing it was killed.
fn parse(text: &str) -> HashMap<String, Kept> {
    let mut found = HashMap::new();
    for row in text.lines() {
        let mut pieces = row.split('\t');
        let (Some(name), Some(kind), Some(value)) = (pieces.next(), pieces.next(), pieces.next())
        else {
            continue;
        };
        found.insert(
            unescape(name),
            Kept {
                kind: kind.to_string(),
                text: unescape(value),
            },
        );
    }
    found
}

/// A value as text, for the kinds of value a name is usually given.
///
/// A number, a string, a yes or no: what a `let mut` at the top of a
/// page holds, nine times in ten. Anything else is not written, and
/// comes back from its initialiser — said in the documentation, because
/// a limit nobody is told about is a bug.
pub(crate) fn text_of<T: 'static>(value: &T) -> Option<String> {
    let value = value as &dyn Any;
    macro_rules! as_text {
        ($($kind:ty),*) => {
            $(if let Some(it) = value.downcast_ref::<$kind>() {
                return Some(it.to_string());
            })*
        };
    }
    as_text!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char,
        String
    );
    None
}

/// The other way: text back into the value it was.
fn from_text<T: 'static>(text: &str) -> Option<T> {
    macro_rules! from {
        ($($kind:ty),*) => {
            $(if TypeId::of::<T>() == TypeId::of::<$kind>() {
                let it: $kind = text.parse().ok()?;
                return (Box::new(it) as Box<dyn Any>).downcast::<T>().ok().map(|it| *it);
            })*
        };
    }
    from!(
        i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char,
        String
    );
    None
}
