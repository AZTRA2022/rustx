//! The system's own file dialogs.
//!
//! A modal dialog waits for a human hand, so what is proved here is
//! everything around it: the filters becoming patterns, the words of
//! each refusal, and the shape of what is asked for. The dialog itself
//! is looked at with `cargo run -p rustx-os --example choose_a_file`.

use rustx_os::dialog::{self, Ask, DialogError};

#[test]
fn the_filters_become_patterns_every_system_reads() {
    let asked = vec![
        (
            "Images".to_string(),
            vec!["png".to_string(), ".jpg".to_string(), "*.gif".to_string()],
        ),
        ("Text".to_string(), vec!["txt".to_string()]),
    ];
    assert_eq!(
        dialog::patterns_for(&asked),
        vec![
            ("Images".to_string(), "*.png;*.jpg;*.gif".to_string()),
            ("Text".to_string(), "*.txt".to_string()),
        ],
        "the pattern is the same however the extension is written"
    );
    assert!(dialog::patterns_for(&[]).is_empty());
}

#[test]
fn what_is_asked_is_what_is_written() {
    // `Ask` is written by hand in a page, so it has to read well.
    let ask = Ask::new("Choose an image")
        .filter("Images", ["png"])
        .starting_in("C:/Users")
        .named("untitled.png")
        .many();
    assert_eq!(ask.title, "Choose an image");
    assert_eq!(ask.filters.len(), 1);
    assert_eq!(ask.start_in, Some(std::path::PathBuf::from("C:/Users")));
    assert_eq!(ask.named.as_deref(), Some("untitled.png"));
    assert!(ask.many);
    // And nothing is asked for by default.
    let plain = Ask::new("Open");
    assert!(plain.filters.is_empty() && plain.start_in.is_none() && !plain.many);
}

#[test]
fn a_cancelled_dialog_is_cancelled_and_not_a_failure() {
    // Named, and said plainly: an application that logs a failure every
    // time somebody closes a dialog is an application whose log is of
    // no further use.
    //
    // This assertion is why the `test` gate was red: the message was
    // translated to English and the sentence being looked for was left
    // in French, so a cancelled dialog read as a failing test.
    let said = DialogError::Cancelled.to_string();
    assert!(said.contains("closed"), "got {said}");
    assert!(!said.contains("error"), "got {said}");
}

#[test]
fn a_platform_that_cannot_do_it_says_which_file_to_write() {
    // The refusal of a platform that is not written names the platform
    // *and* the file: that is a list of work, not an excuse.
    let said = DialogError::Unsupported("the file dialog").to_string();
    assert!(said.contains("the file dialog"), "got {said}");
    assert!(said.contains(std::env::consts::OS), "got {said}");
    assert!(said.contains("dialog.rs"), "got {said}");
}

#[test]
fn this_platform_says_whether_it_has_one() {
    // Les trois l'ont maintenant : COM sur Windows, le portail ou
    // zenity sur Linux, NSOpenPanel sur macOS. C'est ce que lit une
    // page pour ne pas dessiner un bouton qui refuserait.
    assert!(dialog::supported());
}
