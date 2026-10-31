//! What a developer types, and what it says back.

use rustx_cli::{arguments, help};

#[test]
fn help_names_every_command() {
    let said = help::text();
    for command in ["rustx new", "rustx dev", "rustx build", "rustx run"] {
        assert!(said.contains(command), "`{command}` is not in:\n{said}");
    }
}

#[test]
fn help_says_what_a_reload_keeps() {
    // The limit, where a developer finds it: the place they discover
    // that a component's count went back to zero is the moment they
    // watched it happen.
    let said = help::text().to_lowercase();
    assert!(
        said.contains("does not reload native code"),
        "the limit is not said:\n{said}"
    );
    assert!(
        said.contains("shared state") && said.contains("page that was open"),
        "what is kept is not said:\n{said}"
    );
    assert!(
        said.contains("cells inside a component"),
        "what is not kept is not said:\n{said}"
    );
}

#[test]
fn an_unknown_command_suggests_the_nearest() {
    assert_eq!(arguments::nearest("biuld"), Some("build"));
    assert_eq!(arguments::nearest("buidl"), Some("build"));
    assert_eq!(arguments::nearest("ne"), Some("new"));
    assert_eq!(
        arguments::nearest("publish"),
        None,
        "a suggestion nobody meant is worse than none"
    );
}

#[test]
fn nothing_typed_is_a_question_not_a_mistake() {
    let nothing: Vec<String> = Vec::new();
    assert_eq!(arguments::read(&nothing), arguments::Command::Help);
    assert_eq!(
        arguments::read(&["new".to_string()]),
        arguments::Command::Help,
        "`rustx new` with no name is somebody asking what goes there"
    );
}
