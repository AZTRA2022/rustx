//! The clipboard, round trip.
//!
//! These tests use the real clipboard of the machine they run on,
//! because there is no other clipboard to use and a fake one would
//! prove nothing: the whole question is whether `OpenClipboard` and its
//! friends were called in the order Windows wants. They put back what
//! they found when they are done, so a developer who had something
//! copied still has it.
//!
//! They run one after another, in one thread, since the clipboard is one
//! machine-wide object and two tests holding it at once would be two
//! tests failing for no reason of their own.

#![cfg(windows)]

use rustx_os::clipboard;

/// Everything here, in order, inside one test: the clipboard is shared
/// and `cargo test` runs tests in parallel.
#[test]
fn the_clipboard_carries_what_it_was_given() {
    let before = clipboard::text().ok();

    text_written_is_text_read();
    an_image_survives_the_round_trip();
    a_custom_format_is_read_back_by_the_same_name();
    an_empty_clipboard_is_not_an_error_it_is_an_empty_string();

    if let Some(before) = before {
        let _ = clipboard::set_text(&before);
    }
}

fn text_written_is_text_read() {
    // Accents and an emoji: the clipboard is UTF-16 and the conversion
    // is the one thing here that can quietly lose a character.
    let written = "Bonjour, les éléphants — et un 🐘";
    clipboard::set_text(written).expect("the clipboard took the text");
    assert_eq!(clipboard::text().expect("the text came back"), written);
}

fn an_image_survives_the_round_trip() {
    // Two by two, four pixels, BGRA: small enough to write by hand and
    // big enough to have a stride.
    let written = clipboard::Image {
        width: 2,
        height: 2,
        bgra: vec![
            0, 0, 255, 255, // red
            0, 255, 0, 255, // green
            255, 0, 0, 255, // blue
            255, 255, 255, 255, // white
        ],
    };
    clipboard::set_image(&written).expect("the clipboard took the image");
    let read = clipboard::image().expect("the image came back");
    assert_eq!((read.width, read.height), (2, 2));
    assert_eq!(read.bgra, written.bgra, "the pixels changed on the way");
}

fn a_custom_format_is_read_back_by_the_same_name() {
    // What a large application copies between two of its own windows:
    // its own objects, in its own format, which nothing else on the
    // machine can read or spoil.
    let bytes = [7_u8, 0, 1, 255, 42];
    clipboard::set_custom("rustx/test-object", &bytes).expect("the clipboard took the object");
    assert_eq!(
        clipboard::custom("rustx/test-object").expect("the object came back"),
        bytes.to_vec()
    );
    // Another name is another format, and reading it is not an error
    // with a surprise in it.
    assert!(clipboard::custom("rustx/not-that-one").is_err());
}

fn an_empty_clipboard_is_not_an_error_it_is_an_empty_string() {
    clipboard::set_text("").expect("the clipboard took nothing");
    assert_eq!(clipboard::text().expect("nothing came back"), "");
}

#[test]
fn an_open_that_keeps_failing_is_retried_before_it_is_refused() {
    // Every program on the machine opens the clipboard now and then,
    // and it is open for one program at a time. A single try is a
    // feature that fails a few times a day for no reason anybody can
    // reproduce.
    //
    // The contention itself cannot be staged from inside one process:
    // two threads of the same process both get in, which is what this
    // test found when it tried. What a test can pin is the policy and
    // its two numbers, which is what the clipboard's open is made of.
    let started = std::time::Instant::now();
    let tried: Result<(), &str> = rustx_os::patience::retrying(
        || Err("busy"),
        clipboard::PATIENCE,
        clipboard::BETWEEN,
    );
    let waited = started.elapsed();

    assert_eq!(tried, Err("busy"), "a refusal is what comes back");
    assert!(
        waited >= std::time::Duration::from_millis(150),
        "it gave up after {waited:?}, which is not a retry"
    );
    assert!(
        waited < std::time::Duration::from_millis(1000),
        "it waited {waited:?}, which is long enough to look frozen"
    );
}

#[test]
fn what_this_platform_cannot_do_it_says() {
    // On Windows it can. The point of the test is that the question has
    // an answer at all, so that a page can ask before it offers a
    // button that would refuse.
    assert!(clipboard::supported());
}
