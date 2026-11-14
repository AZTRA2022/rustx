//! Turning a JSX region into a node tree.

use rustx_compiler::ast::{AttributeSource, Node};
use rustx_compiler::parse::parse_jsx;
use rustx_compiler::{CompileError, scan::find_jsx};

fn parse_one(source: &str) -> Node {
    let span = find_jsx(source).expect("scannable")[0];
    parse_jsx(source, span).expect("parsable")
}

fn parse_one_err(source: &str) -> CompileError {
    match find_jsx(source) {
        Err(error) => error,
        Ok(spans) => parse_jsx(source, spans[0]).expect_err("should refuse"),
    }
}

#[test]
fn text_children_keep_their_punctuation() {
    // Review Focus 2: `<`, `&` and quotes in UI text are ordinary text.
    let node = parse_one(r#"<p>10 < 20 & "quoted"</p>"#);
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 1);
    assert!(matches!(&tag.children[0], Node::Text(t) if t == r#"10 < 20 & "quoted""#));
}

#[test]
fn an_expression_child_keeps_its_rust_verbatim() {
    let node = parse_one("<p>{ user.name.to_uppercase() }</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert!(
        matches!(&tag.children[0], Node::Expression { code, .. } if code.trim() == "user.name.to_uppercase()")
    );
}

#[test]
fn attributes_come_in_three_shapes() {
    let node = parse_one(r#"<input className="row" value={name} disabled />"#);
    let Node::Tag(tag) = node else { panic!() };
    assert!(matches!(&tag.attributes[0].value, AttributeSource::Literal(v) if v == "row"));
    assert!(
        matches!(&tag.attributes[1].value, AttributeSource::Expression(v) if v.trim() == "name")
    );
    assert!(matches!(tag.attributes[2].value, AttributeSource::Flag));
}

#[test]
fn a_closure_attribute_survives_its_braces_and_body() {
    let node = parse_one("<button onClick={|| { let mut n = 0; n += 1; }}>go</button>");
    let Node::Tag(tag) = node else { panic!() };
    let AttributeSource::Expression(code) = &tag.attributes[0].value else {
        panic!()
    };
    assert!(code.contains("n += 1"), "got {code}");
}

#[test]
fn a_self_closing_tag_has_no_children() {
    let node = parse_one("<br />");
    let Node::Tag(tag) = node else { panic!() };
    assert!(tag.children.is_empty());
}

#[test]
fn a_mismatched_closing_tag_says_which_two_disagree() {
    let error = parse_one_err("<div>hi</span>");
    assert!(error.message.contains("div"), "got {}", error.message);
    assert!(error.message.contains("span"), "got {}", error.message);
}

#[test]
fn a_space_between_text_and_an_inline_tag_survives() {
    // `a <b>bold</b> word` is the most ordinary markup there is, and
    // trimming every text node renders it "aboldword".
    let node = parse_one("<p>Hello <b>world</b> again</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert!(
        matches!(&tag.children[0], Node::Text(t) if t == "Hello "),
        "got {:?}",
        tag.children[0]
    );
    assert!(
        matches!(&tag.children[2], Node::Text(t) if t == " again"),
        "got {:?}",
        tag.children[2]
    );
}

#[test]
fn indentation_does_not_leak_into_the_rendered_text() {
    // Whitespace that crosses a newline is layout, not content: JSX drops
    // it, and a tree written over several indented lines must not print
    // its own indentation.
    let node = parse_one("<p>\n    line one\n    line two\n</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 1);
    assert!(
        matches!(&tag.children[0], Node::Text(t) if t == "line one line two"),
        "got {:?}",
        tag.children[0]
    );
}

#[test]
fn whitespace_between_tags_on_separate_lines_is_dropped() {
    let node = parse_one("<ul>\n    <li>a</li>\n    <li>b</li>\n</ul>");
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 2, "got {:?}", tag.children);
}

#[test]
fn an_attribute_string_has_no_escapes_just_like_in_jsx() {
    // A Windows path in a tooltip. The scanner reads Rust escapes, but
    // an attribute's quotes are HTML's, not Rust's: a backslash is a
    // backslash, and the value must not grow one.
    let node = parse_one(r#"<div title="C:\"></div>"#);
    let Node::Tag(tag) = node else { panic!() };
    assert!(
        matches!(&tag.attributes[0].value, AttributeSource::Literal(v) if v == "C:\\"),
        "got {:?}",
        tag.attributes[0].value
    );
}

// ───────────── Les commentaires, dans le markup ─────────────
//
// Ce que le labo a vu à l'écran : « // Les trois boutons que le cadre
// du système dessinait. » écrit dans la fenêtre, en toutes lettres, au
// milieu de la barre. Un commentaire entre deux balises était du texte
// pour le compilateur, donc du texte pour l'utilisateur.
//
// Le `.rsx` est du Rust, et un commentaire s'y écrit comme en Rust.
// Trois places : entre des enfants, entre des attributs, et dans des
// accolades, qui est la forme de JSX.

#[test]
fn a_line_comment_between_children_is_not_text() {
    let node = parse_one(
        "<div>\n    // les trois boutons que le cadre dessinait\n    <b>ok</b>\n</div>",
    );
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(
        tag.children.len(),
        1,
        "le commentaire ne doit pas devenir un enfant : {:?}",
        tag.children
    );
    assert!(matches!(&tag.children[0], Node::Tag(_)));
}

#[test]
fn a_block_comment_between_children_is_not_text_either() {
    let node = parse_one("<div>\n    /* deux lignes\n       de prose */\n    <b>ok</b>\n</div>");
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 1, "{:?}", tag.children);
}

#[test]
fn a_comment_in_braces_is_the_jsx_form_and_disappears_too() {
    // `{/* … */}` est ce qu'un développeur venu de React écrit, et il
    // n'y a aucune raison de le refuser : des accolades qui ne tiennent
    // qu'un commentaire ne tiennent aucune expression.
    let node = parse_one("<div>{/* rien à voir */}<b>ok</b></div>");
    let Node::Tag(tag) = node else { panic!() };
    assert_eq!(tag.children.len(), 1, "{:?}", tag.children);
}

#[test]
fn a_comment_between_attributes_is_not_an_attribute() {
    let node = parse_one(
        "<input\n    // le titre du document\n    id=\"titre\"\n    /* et sa valeur */\n    value={titre}\n/>",
    );
    let Node::Tag(tag) = node else { panic!() };
    let names: Vec<&str> = tag.attributes.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["id", "value"], "{names:?}");
}

#[test]
fn an_address_in_a_text_child_is_not_a_comment() {
    // Le piège de la règle : `//` existe dans de la prose. Un
    // commentaire commence une ligne ; ce qui est au milieu d'une
    // phrase reste la phrase.
    let node = parse_one("<p>voir https://exemple.fr pour la suite</p>");
    let Node::Tag(tag) = node else { panic!() };
    assert!(
        matches!(&tag.children[0], Node::Text(text) if text.contains("https://exemple.fr")),
        "{:?}",
        tag.children
    );
}

#[test]
fn a_comment_does_not_eat_the_text_around_it() {
    let node = parse_one("<p>avant\n    // au milieu\n    après</p>");
    let Node::Tag(tag) = node else { panic!() };
    let Node::Text(text) = &tag.children[0] else {
        panic!("{:?}", tag.children)
    };
    assert_eq!(text, "avant après");
}

/// A tree deeper than the parser's own recursion can afford.
///
/// The parser descends into a tag, and so does the emission under it, so
/// every level of nesting costs stack — about twelve kilobytes of it in
/// an unoptimised build. A few hundred levels therefore overflowed the
/// thread, and a stack overflow is not a refusal: the process dies with
/// no line, no column and no message, whether it is `cargo build` or the
/// language server answering a keystroke. A minified blob pasted into a
/// page is enough to do it.
#[test]
fn a_tree_too_deep_to_descend_is_refused_rather_than_crashing() {
    let depth = rustx_compiler::parse::NESTING_LIMIT + 1;
    let source = format!(
        "fn Page() -> Element {{ return {}{}; }}",
        "<div>".repeat(depth),
        "</div>".repeat(depth)
    );
    let error = parse_one_err(&source);
    assert!(
        error.message.contains("nested more than"),
        "got {}",
        error.message
    );
    assert!(error.line >= 1 && error.column >= 1, "unplaced refusal");
}

/// And the refusal survives a stack far smaller than the one the deep
/// tree needed, which is the whole point of having a limit at all.
///
/// Two hundred and fifty-six kibibytes: less than a quarter of what the
/// limit itself costs, so this would die if the refusal were not raised
/// before the descent. Ten thousand levels, because that is what a
/// minified document looks like.
#[test]
fn a_very_deep_tree_is_refused_on_a_small_stack() {
    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let source = format!(
                "fn Page() -> Element {{ return {}{}; }}",
                "<div>".repeat(10_000),
                "</div>".repeat(10_000)
            );
            rustx_compiler::transpile::transpile(&source).is_err()
        })
        .expect("a worker thread");
    assert!(
        worker.join().expect("the worker did not die"),
        "a tree ten thousand deep was accepted"
    );
}

/// The same limit, reached through braces rather than through tags.
///
/// `<div>{ <div>{ … } }</div>` nests just as deeply, and each `{…}` is
/// parsed by a call of its own — so a limit the cursor kept to itself
/// was no limit at all.
#[test]
fn nesting_through_an_expression_counts_towards_the_same_limit() {
    let depth = rustx_compiler::parse::NESTING_LIMIT;
    let source = format!(
        "fn Page() -> Element {{ return {}<br />{}; }}",
        "<div>{".repeat(depth),
        "}</div>".repeat(depth)
    );
    let error = parse_one_err(&source);
    assert!(
        error.message.contains("nested more than"),
        "got {}",
        error.message
    );
}

/// And a tree exactly at the limit is still compiled.
///
/// A limit that refused what it allows would be a limit nobody could
/// reason about.
#[test]
fn a_tree_at_the_limit_is_still_accepted() {
    let depth = rustx_compiler::parse::NESTING_LIMIT;
    let source = format!(
        "fn Page() -> Element {{ return {}{}; }}",
        "<div>".repeat(depth),
        "</div>".repeat(depth)
    );
    rustx_compiler::transpile::transpile(&source).expect("a tree at the limit compiles");
}
