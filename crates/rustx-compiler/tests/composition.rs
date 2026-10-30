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
    assert!(rust.contains("Nav ()"), "got {rust}");
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
    assert!(rust.contains("Layout (rustx_dom :: Element :: Fragment"), "got {rust}");
}

#[test]
fn a_component_that_is_not_declared_in_this_file_is_refused() {
    // Review Focus 5: a silent wrong call is worse than a refusal.
    let error = transpile("fn Page() -> Element {\n    return <Preview />;\n}")
        .expect_err("should be refused");
    assert_eq!(error.line, 2);
    assert!(error.message.contains("Preview"), "got {}", error.message);
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
