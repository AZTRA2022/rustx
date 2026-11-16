//! What has to be true of the compiler for **every** input.
//!
//! There is no network here, so `cargo-fuzz` cannot be installed. That
//! is no loss: a fuzzer whose generator is a seeded pseudo-random
//! function is reproducible, runs in CI with the rest of the suite, and
//! prints the seed of whatever it finds. Everything below is an ordinary
//! `#[test]`.
//!
//! The three laws being checked, in the order they matter:
//!
//! 1. **No panic, on any input.** A `.rsx` is a file a developer is in
//!    the middle of typing: half a paste, a minified blob, twenty
//!    thousand unclosed braces, a line that ends in the middle of a
//!    character. Every one of those is either clean generated Rust or an
//!    honest refusal, and never a backtrace naming a file in this crate.
//! 2. **The line count is preserved.** The position map rests on it: a
//!    rewrite that loses a newline moves every `rustc` diagnostic below
//!    it, silently and for ever.
//! 3. **Nothing outside a JSX region moves.** The contract is that the
//!    developer's Rust is their Rust. What state promotion rewrites is
//!    rewritten in place; everything else comes out byte for byte.

use rustx_compiler::rewrite::promote_state;
use rustx_compiler::scan::find_jsx;
use rustx_compiler::transpile::transpile;

/// SplitMix64. Chosen because it is ten lines, has no dependency, and
/// gives the same stream on every platform — which is what makes a
/// failure here something another engineer can reproduce from the seed
/// printed beside it.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, ceiling: usize) -> usize {
        if ceiling == 0 {
            return 0;
        }
        (self.next() % ceiling as u64) as usize
    }

    fn pick<'a, T>(&mut self, from: &'a [T]) -> &'a T {
        &from[self.below(from.len())]
    }

    /// One chance in `odds`.
    fn rarely(&mut self, odds: usize) -> bool {
        self.below(odds.max(1)) == 0
    }
}

/// The pieces a `.rsx` is made of, as a scanner sees them.
///
/// Not random bytes: random bytes are almost all rejected by the first
/// branch they meet, and the interesting failures live where two of
/// these meet. A lone `"` next to a `{` next to a `</` is the shape that
/// breaks a scanner, so that is the shape this generates.
const PIECES: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "<>",
    "</>",
    "{",
    "}",
    "(",
    ")",
    "[",
    "]",
    "\"",
    "'",
    "\\",
    "|",
    "||",
    "=",
    "==",
    "=>",
    ";",
    ",",
    ".",
    "..",
    ":",
    "::",
    "!",
    "#",
    "*",
    "&",
    "+",
    "-",
    "?",
    "@",
    "$",
    "%",
    "^",
    "~",
    "`",
    " ",
    "\n",
    "\r\n",
    "\r",
    "\t",
    "//",
    "/*",
    "*/",
    "r#\"",
    "\"#",
    "b\"",
    "fn ",
    "let ",
    "mut ",
    "return ",
    "async ",
    "await",
    "move ",
    "Element",
    "div",
    "span",
    "p",
    "button",
    "Card",
    "js::Chart",
    "onClick",
    "className",
    "style",
    "key",
    "keep",
    "effect",
    "task",
    "format!",
    "println!",
    "self",
    "Self",
    "impl",
    "struct",
    "match",
    "if ",
    "else ",
    "for ",
    "while ",
    "0",
    "1",
    "42",
    "n",
    "x",
    "items",
    "count",
    "label",
    "é",
    "日本語",
    "🎉",
    "\u{200b}",
    "\u{feff}",
    "\u{1}",
    "ᚠ",
    "A",
    "Z",
    "_",
];

/// A pile of pieces, with no structure asked of it at all.
fn soup(rng: &mut Rng, pieces: usize) -> String {
    let mut out = String::new();
    for _ in 0..pieces {
        out.push_str(rng.pick(PIECES));
    }
    out
}

/// A page that is plausible: a component, some state, a handler, markup.
///
/// This is where a real defect hides. Soup mostly exercises refusals;
/// a page the compiler agrees to compile exercises the rewrite, and the
/// rewrite is what moves characters.
fn page(rng: &mut Rng) -> String {
    const INITIALIZERS: &[&str] = &[
        "0",
        "0_usize",
        "false",
        "String::new()",
        "Vec::new()",
        "vec![\n    1,\n    2,\n]",
        "vec![1, 2]",
        "';'",
        "\";\"",
        "'\\n'",
        "\"a; b\"",
        "r\"C:\\temp\\\"",
        "if true { 1 } else { 2 }",
        "Default::default()",
        "None",
        "format!(\"{}\", 1)",
        "(1, 2)",
        "[0u8; 4]",
    ];
    const ANNOTATIONS: &[&str] = &[
        "",
        ": usize",
        ": bool",
        ": String",
        ": Vec<u8>",
        ": Option<u8>",
        ": [u8; 4]",
        ": Vec<(usize, String)>",
    ];
    const WRITES: &[&str] = &[
        "n = 1",
        "n += 1",
        "n -= 1",
        "n <<= 1",
        "items.push(1)",
        "items.clear()",
        "items.push(label.clone())",
        "n = n + 1",
        "n = if true { 1 } else { 2 }",
        "items.push(Item { label: if short { a() } else { label.clone() } })",
    ];
    const BODIES: &[&str] = &[
        "{n}",
        "{items.len()}",
        "{format!(\"n: {n}\")}",
        "{ let m = n; format!(\"{m}\") }",
        "10 < 20 & \"quoted\"",
        "{items.iter().map(|each| <li>{each}</li>).collect::<Vec<_>>()}",
        "<span className=\"row\">{n}</span>",
        "<br />",
        "{/* a comment */}",
        "// not a comment, a sentence\n        plain text",
    ];
    const ATTRIBUTES: &[&str] = &[
        "",
        " className=\"row\"",
        " disabled",
        " disabled={n > 0}",
        " style={{ backgroundColor: \"#09090b\" }}",
        " style={format!(\"width: {}px\", n)}",
        " key={n}",
        " title=\"C:\\\"",
        " data-at={n}",
    ];

    let mut out = String::new();
    if rng.rarely(2) {
        out.push_str(&format!(
            "let mut shared{} = {};\n\n",
            rng.below(3),
            rng.pick(INITIALIZERS)
        ));
    }
    out.push_str("fn Page() -> Element {\n");
    out.push_str(&format!(
        "    let mut n{} = {};\n",
        rng.pick(ANNOTATIONS),
        rng.pick(INITIALIZERS)
    ));
    if rng.rarely(2) {
        out.push_str(&format!(
            "    let mut items{} = {};\n",
            rng.pick(ANNOTATIONS),
            rng.pick(INITIALIZERS)
        ));
    }
    if rng.rarely(3) {
        out.push_str("    keep(Camera::open());\n");
    }
    if rng.rarely(3) {
        out.push_str("    effect((), || { let _ = 1; });\n");
    }
    if rng.rarely(4) {
        out.push_str(&format!("    async move {{ {}; }}\n", rng.pick(WRITES)));
    }
    out.push_str("    return <div");
    out.push_str(rng.pick(ATTRIBUTES));
    out.push_str(">\n        <button onClick={|| ");
    out.push_str(rng.pick(WRITES));
    out.push_str("}>go</button>\n        ");
    out.push_str(rng.pick(BODIES));
    out.push_str("\n    </div>;\n}\n");
    out
}

/// A page with one wound in it: the half-typed file a language server
/// sees on every keystroke.
fn wounded(rng: &mut Rng, source: &str) -> String {
    let characters: Vec<char> = source.chars().collect();
    if characters.is_empty() {
        return String::new();
    }
    let at = rng.below(characters.len());
    match rng.below(6) {
        // Cut the tail off: the paste that is still arriving.
        0 => characters[..at].iter().collect(),
        // Cut the head off.
        1 => characters[at..].iter().collect(),
        // Drop one character — the missing brace, the missing quote.
        2 => characters
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != at)
            .map(|(_, each)| *each)
            .collect(),
        // Insert a piece in the middle.
        3 => {
            let mut out: String = characters[..at].iter().collect();
            out.push_str(rng.pick(PIECES));
            out.extend(&characters[at..]);
            out
        }
        // Duplicate a stretch.
        4 => {
            let until = (at + 1 + rng.below(40)).min(characters.len());
            let mut out: String = characters[..until].iter().collect();
            out.extend(&characters[at..until]);
            out.extend(&characters[until..]);
            out
        }
        // Replace a character with another piece.
        _ => {
            let mut out: String = characters[..at].iter().collect();
            out.push_str(rng.pick(PIECES));
            out.extend(&characters[at + 1..]);
            out
        }
    }
}

/// Every law that holds whatever the input is.
///
/// Returns nothing: a failure is an assertion, and the seed is in the
/// message so that the case can be replayed.
fn judge(source: &str, what: &str) {
    // 1. The rewrite alone. It runs before anything else, so a panic
    //    here is a panic in every caller.
    let promoted = match promote_state(source) {
        Ok(promoted) => promoted,
        // A refusal is an answer. It only has to be an honest one.
        Err(error) => {
            assert!(
                !error.message.is_empty(),
                "{what}: a refusal with no message"
            );
            assert!(
                error.offset <= source.len(),
                "{what}: a refusal placed past the end of the file"
            );
            return;
        }
    };

    // 2. The line count. Every edit the rewrite makes stays on the line
    //    it started on — `transpile` reads the two files side by side to
    //    decide which lines a language server may not offer a column in,
    //    and that reading is only meaningful while the two agree.
    assert_eq!(
        promoted.matches('\n').count(),
        source.matches('\n').count(),
        "{what}: state promotion changed the line count\n--- before ---\n{source}\n--- after ---\n{promoted}"
    );

    // 3. And the whole pipeline.
    let built = match transpile(source) {
        Ok(built) => built,
        Err(error) => {
            assert!(
                !error.message.is_empty(),
                "{what}: a refusal with no message"
            );
            assert!(
                error.offset <= source.len(),
                "{what}: a refusal placed past the end of the file"
            );
            return;
        }
    };
    assert_eq!(
        built.rust.matches('\n').count(),
        source.matches('\n').count(),
        "{what}: transpiling changed the line count\n--- in ---\n{source}\n--- out ---\n{}",
        built.rust
    );

    // 4. Determinism: the same input, twice, byte for byte. A build that
    //    is not reproducible cannot be cached, and a diagnostic that
    //    moves between runs cannot be trusted.
    let again = transpile(source).expect("the second run agrees with the first");
    assert_eq!(built.rust, again.rust, "{what}: two runs, two answers");
    assert_eq!(built.map.to_text(), again.map.to_text(), "{what}: two maps");

    // 5. What is outside a JSX region is the developer's. The rewrite
    //    may have changed it — that is its job — but the emission may
    //    not: the text between two regions is copied byte for byte.
    if let Ok(regions) = find_jsx(&promoted) {
        let mut cursor = 0;
        for region in &regions {
            let plain = &promoted[cursor..region.start];
            assert!(
                built.rust.contains(plain) || plain.trim().is_empty(),
                "{what}: Rust outside a region was not copied through:\n{plain:?}"
            );
            cursor = region.end;
        }
    }
}

#[test]
fn no_soup_of_syntax_makes_the_compiler_panic() {
    for seed in 0..4_000_u64 {
        let mut rng = Rng::new(seed);
        let pieces = 1 + rng.below(60);
        let source = soup(&mut rng, pieces);
        judge(&source, &format!("soup, seed {seed}"));
    }
}

#[test]
fn no_plausible_page_loses_a_line_or_panics() {
    for seed in 0..4_000_u64 {
        let mut rng = Rng::new(seed ^ 0x5EED);
        let source = page(&mut rng);
        judge(&source, &format!("page, seed {seed}"));
    }
}

#[test]
fn no_half_typed_page_makes_the_compiler_panic() {
    for seed in 0..6_000_u64 {
        let mut rng = Rng::new(seed ^ 0xDEAD_BEEF);
        let mut source = page(&mut rng);
        for _ in 0..1 + rng.below(3) {
            source = wounded(&mut rng, &source);
        }
        judge(&source, &format!("wounded page, seed {seed}"));
    }
}

#[test]
fn the_pathological_shapes_are_answers_and_not_crashes() {
    // Written out rather than generated: these are the shapes a
    // generator reaches rarely and a human reaches on their first day.
    let twenty_thousand_braces = "{".repeat(20_000);
    let deep = format!(
        "fn Page() -> Element {{ return {}{}; }}",
        "<div>".repeat(500),
        "</div>".repeat(500)
    );
    let cases: Vec<String> = vec![
        String::new(),
        "\n".to_string(),
        "\r\n\r\n".to_string(),
        "\r".to_string(),
        twenty_thousand_braces,
        "}".repeat(20_000),
        "<".repeat(20_000),
        "\"".repeat(20_000),
        "/*".repeat(10_000),
        "r#\"".to_string(),
        deep,
        "fn Page() -> Element { return <div>".to_string(),
        "fn Page() -> Element { return <>".to_string(),
        "fn Page() -> Element { return </div>; }".to_string(),
        // A character that is not ASCII right where an offset lands.
        "fn Page() -> Element { return <div>é</div> }".to_string(),
        "fn Page() -> Element { let mut n = 'é'; return <p>{n}</p>; }".to_string(),
        "// é\nfn Page() -> Element { return <p onClick={é}>x</p>; }".to_string(),
        "fn Page() -> Element { return <p titre=\"é\">é</p>; }".to_string(),
        // A lone surrogate cannot exist in a Rust `str`; the nearest
        // thing a file can hold is a replacement character.
        "fn Page() -> Element { return <p>\u{fffd}</p>; }".to_string(),
        // Mixed line endings, which is what a file edited on two
        // machines looks like.
        "fn Page() -> Element {\r\n    let mut n = 0;\n    return <p>{n}</p>;\r\n}\n".to_string(),
        // A `let mut` whose initializer holds the statement terminator.
        "fn Page() -> Element { let mut s = ';'; return <button onClick={|| s = ':'}>{s}</button>; }"
            .to_string(),
        // And one whose initializer holds it inside a string.
        "fn Page() -> Element { let mut s = \"a;b\"; return <button onClick={|| s = \"c\"}>{s}</button>; }"
            .to_string(),
        // A `keep` whose argument holds a closing parenthesis in a string.
        "fn Page() -> Element { keep(format!(\"a)\")); return <p>x</p>; }".to_string(),
        "fn Page() -> Element { let mut n = 0; keep(format!(\"a) {n}\")); return <button onClick={|| n += 1}>x</button>; }".to_string(),
    ];
    for (at, source) in cases.iter().enumerate() {
        eprintln!("pathological case {at}");
        judge(source, &format!("pathological case {at}"));
    }
}
