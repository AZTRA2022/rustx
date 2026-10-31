//! A capitalised tag is a call to a component.
//!
//! `<Nav />` is the whole of what a layout needs: a component that
//! renders another one. The props go by the name of the parameter they
//! fill, not by the order they were written in, because the order of an
//! argument list is not something a reader of the `.rsx` can see.

use rustx_compiler::transpile::transpile;

fn rewrite_one(source: &str) -> String {
    transpile(source).expect("transpilable").rust
}

const NAV: &str = "fn Nav() -> Element {\n    return <p>nav</p>;\n}\n";

#[test]
fn a_capitalised_tag_becomes_a_call_to_the_component() {
    let rust = rewrite_one(&format!(
        "{NAV}fn Page() -> Element {{\n    return <div><Nav /></div>;\n}}"
    ));
    assert!(rust.contains("component_at"), "got {rust}");
    // A component with nothing to pass is handed over as the function
    // itself, not wrapped in a closure clippy would ask the developer to
    // simplify in a file they never wrote.
    assert!(rust.contains("} , Nav ,"), "got {rust}");
}

#[test]
fn the_call_carries_the_place_it_was_written() {
    let rust = rewrite_one(&format!(
        "{NAV}fn Page() -> Element {{\n    return <div><Nav /></div>;\n}}"
    ));
    assert!(rust.contains("line : 5u32"), "got {rust}");
}

#[test]
fn a_lowercase_tag_is_still_html() {
    let rust = rewrite_one("fn Page() -> Element {\n    return <section><p>hi</p></section>;\n}");
    assert!(!rust.contains("component_at"), "got {rust}");
}

#[test]
fn an_attribute_is_passed_as_the_parameter_it_names() {
    // Written in the other order than declared: what matters is the
    // name, not the position.
    let rust = rewrite_one(
        "fn Row(label: String, count: u8) -> Element {\n    return <li>{label}</li>;\n}\nfn Page() -> Element {\n    return <ul><Row count={2} label={\"a\".to_string()} /></ul>;\n}",
    );
    assert!(
        rust.contains("Row (\"a\" . to_string () , 2)"),
        "got {rust}"
    );
}

#[test]
fn what_is_written_between_the_tags_arrives_as_children() {
    let rust = rewrite_one(
        "fn Layout(children: Element) -> Element {\n    return <main>{children}</main>;\n}\nfn Page() -> Element {\n    return <Layout><p>hi</p></Layout>;\n}",
    );
    assert!(
        rust.contains("Layout (rustx_dom :: Element :: tag (\"p\")"),
        "got {rust}"
    );
}

#[test]
fn several_children_arrive_as_one_fragment() {
    let rust = rewrite_one(
        "fn Layout(children: Element) -> Element {\n    return <main>{children}</main>;\n}\nfn Page() -> Element {\n    return <Layout><p>a</p><p>b</p></Layout>;\n}",
    );
    assert!(
        rust.contains("Layout (rustx_dom :: Element :: Fragment"),
        "got {rust}"
    );
}

#[test]
fn a_component_this_file_does_not_declare_is_a_library_component() {
    // It used to be a refusal, and the refusal was right while a tag
    // could only be a component written beside it. A tag is now also how
    // a component off npm is written, and the compiler cannot know what
    // the application installed — so a name this file does not declare
    // is resolved in the page, among the libraries it brought, and the
    // complaint for a name nobody has names both places it was looked
    // for.
    let rust = transpile("fn Page() -> Element {\n    return <Preview />;\n}")
        .expect("transpilable")
        .rust;
    assert!(rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("\"Preview\""), "got {rust}");
}

#[test]
fn an_attribute_that_names_no_parameter_is_refused() {
    let error = transpile(
        "fn Row(label: String) -> Element {\n    return <li>{label}</li>;\n}\nfn Page() -> Element {\n    return <Row labell={1} />;\n}",
    )
    .expect_err("should be refused");
    assert!(error.message.contains("labell"), "got {}", error.message);
}

#[test]
fn a_parameter_the_tag_never_gives_is_refused() {
    // Fail closed: a missing prop is a missing argument, and `rustc`
    // naming the generated call is a worse message than this one.
    let error = transpile(
        "fn Row(label: String) -> Element {\n    return <li>{label}</li>;\n}\nfn Page() -> Element {\n    return <Row />;\n}",
    )
    .expect_err("should be refused");
    assert!(error.message.contains("label"), "got {}", error.message);
}

#[test]
fn children_given_to_a_component_that_takes_none_is_refused() {
    let error = transpile(
        "fn Nav() -> Element {\n    return <p>nav</p>;\n}\nfn Page() -> Element {\n    return <Nav><p>hi</p></Nav>;\n}",
    )
    .expect_err("should be refused");
    assert!(error.message.contains("children"), "got {}", error.message);
}

#[test]
fn composing_keeps_the_file_the_same_length() {
    let source = format!("{NAV}fn Page() -> Element {{\n    return <div><Nav /></div>;\n}}\n");
    assert_eq!(
        transpile(&source).unwrap().rust.lines().count(),
        source.lines().count()
    );
}

#[test]
fn a_component_tag_inside_a_handler_is_refused_where_it_was_written() {
    // I6 of the plan 5 review. It compiled, and the panic came at click
    // time with a message about the renderer — for code the developer
    // wrote. A component is rendered by the render that declares it,
    // and a handler runs between renders, so there is no render to
    // belong to.
    let source = "fn Row(name: String) -> Element {\n    return <li>{name}</li>;\n}\n\n\
                  fn P() -> Element {\n    \
                  return <button onClick={move |_| { let _ = <Row name={\"a\".to_string()} />; }}>go</button>;\n}\n";
    let error = rustx_compiler::transpile::transpile(source).expect_err("should be refused");
    assert!(
        error.message.contains("Row") && error.message.contains("handler"),
        "got {}",
        error.message
    );
    assert_eq!(error.line, 6, "it was written on line 6: {}", error.message);
}

/// The columns the `Slot`s in this generated Rust were given, in order.
fn columns_of(rust: &str) -> Vec<String> {
    rust.split("column : ")
        .skip(1)
        .map(|rest| {
            rest.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect()
        })
        .collect()
}

#[test]
fn a_call_in_braces_to_a_component_of_this_file_is_an_instance_of_its_own() {
    // A component is called the way Rust calls a function. What that
    // needs is not the syntax — `{Row(…)}` already compiled — but the
    // instance: called plainly, the function runs in the middle of its
    // parent's render and its cells belong to the parent.
    let rust = rewrite_one(
        "fn Row(label: String) -> Element {\n    return <li>{label}</li>;\n}\n\
         fn Page() -> Element {\n    return <ul>{Row(\"a\".to_string())}</ul>;\n}",
    );
    assert!(rust.contains("component_at"), "got {rust}");
    assert!(rust.contains("Row (\"a\" . to_string ())"), "got {rust}");
}

#[test]
fn two_calls_in_braces_are_two_instances() {
    let rust = rewrite_one(
        "fn Row() -> Element {\n    return <li>x</li>;\n}\n\
         fn Page() -> Element {\n    return <ul>{Row()}{Row()}</ul>;\n}",
    );
    assert_eq!(rust.matches("component_at").count(), 2, "got {rust}");
    let columns = columns_of(&rust);
    assert_ne!(columns[0], columns[1], "two places, two instances: {rust}");
}

#[test]
fn a_call_to_something_that_is_not_a_component_is_left_alone() {
    let rust = rewrite_one(
        "fn helper() -> u8 { 1 }\nfn Page() -> Element {\n    return <p>{helper()}</p>;\n}",
    );
    assert!(!rust.contains("component_at"), "got {rust}");
}

#[test]
fn a_read_that_only_looks_like_a_call_is_left_alone() {
    let rust = rewrite_one(
        "fn Page() -> Element {\n    let n = 1;\n    return <p>{n.to_string()}</p>;\n}",
    );
    assert!(!rust.contains("component_at"), "got {rust}");
}

#[test]
fn a_call_inside_a_larger_expression_is_left_alone() {
    // What surrounds it can call it zero times or ten, and identity by
    // the place it is written would stop meaning anything.
    let rust = rewrite_one(
        "fn Row() -> Element {\n    return <li>x</li>;\n}\n\
         fn Page() -> Element {\n    return <ul>{vec![Row()]}</ul>;\n}",
    );
    assert!(!rust.contains("component_at"), "got {rust}");
}
