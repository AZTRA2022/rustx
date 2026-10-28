# rustx — Plan 2 : le compilateur `.rsx`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Écrire un `.rsx` contenant du JSX nu, le compiler, et voir le résultat à l'écran — avec un `onClick` qui exécute du Rust.

**Architecture:** Un transpileur lit le `.rsx`, repère les régions JSX dans un flux de jetons Rust, les remplace par des appels de construction d'arbre, et écrit un `.rs` ordinaire accompagné d'une carte de correspondance position à position. Un script de build de cargo fait la traduction avant la compilation. Un arbre `Element` en mémoire est monté dans le DOM par la couche `rustx-engine` du plan 1.

**Tech Stack:** Rust edition 2024, `proc-macro2` pour la tokenisation, `rustx-engine` (CEF 156.2.0), Windows 10 x64.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md`

**Portée :** ce plan couvre §7.3 du document (le compilateur et sa carte) et le strict minimum de §7.4 pour qu'un résultat soit visible : un arbre d'éléments et son montage statique dans le DOM.

**Hors périmètre, explicitement :** la promotion automatique d'état (`let mut n = 0` qui survit aux re-rendus), la réconciliation, le re-rendu, `key`, les effets, le contexte partagé — tout §7.4 bis. Ils font l'objet du plan 3, et un `onClick` de ce plan-ci peut muter du Rust mais ne redessine rien. Également hors périmètre : le serveur de langage, le routing par fichiers, l'interopérabilité React.

## Global Constraints

- Plateforme cible : Windows 10 x64 uniquement.
- Rust edition 2024.
- Aucune dépendance sous GPL ; aucune obligation de licence propagée aux utilisateurs du framework.
- Aucun contenu web distant.
- `unsafe` reste confiné à `rustx-engine`. Le compilateur et le code qu'il **génère** n'en contiennent pas : c'est l'anti-objectif n°4 du document, et il porte précisément sur le code généré.
- L'état vit sur le thread d'interface du processus de rendu ; il n'est ni `Send` ni `Sync`.
- La syntaxe est celle que le commanditaire a fixée : JSX nu, sans macro enveloppante, et un gestionnaire d'événement est une **vraie closure Rust** (`onClick={|| …}`) pour que `rust-analyzer` sache la lire.
- La transpilation est déterministe : même entrée, même sortie.

## Review Focus

Cinq choses que le document implique et qu'aucune tâche n'exerce d'elle-même. Chacune a son test rattaché à la tâche propriétaire.

1. **Un `.rsx` syntaxiquement faux.** Le compilateur doit nommer le fichier, la ligne et la colonne du `.rsx` — jamais du `.rs` généré, que le développeur n'a pas écrit et n'a aucune raison de lire. → Tâche 2.
2. **Du texte d'interface qui contient `<`, `&` ou des guillemets.** Un prix « 10 < 20 » ou un nom « Dupont & Fils » ne doit ni casser le parseur ni se retrouver injecté comme balise. → Tâche 3.
3. **Une erreur de `rustc` dans le code Rust à l'intérieur du JSX.** Le message doit pointer la bonne ligne du `.rsx` ; sans la carte de correspondance, l'écart grandit avec le fichier et l'outillage devient inutilisable. → Tâche 5.
4. **Un attribut inconnu ou mal orthographié** (`classname` pour `className`). Le document promet « toutes les balises et props » ; le silence doit être impossible, l'erreur explicite. → Tâche 4.
5. **Un composant qui renvoie une liste de nœuds ou rien du tout**, et du texte imbriqué sous plusieurs niveaux. Le montage doit produire exactement l'arbre écrit, sans nœud perdu ni ordre inversé. → Tâche 6.

---

### Task 1: L'arbre d'éléments

**Files:**
- Create: `crates/rustx-dom/Cargo.toml`
- Create: `crates/rustx-dom/src/lib.rs`
- Create: `crates/rustx-dom/src/element.rs`
- Modify: `Cargo.toml` (ajouter le membre)
- Test: `crates/rustx-dom/tests/element.rs`

Le type que les composants renvoient, et ce que le transpileur produit. Il vit dans sa propre crate parce que le compilateur et le moteur en dépendent tous les deux, et que le compilateur ne doit pas dépendre de CEF.

**Interfaces:**
- Consumes: rien.
- Produces:
  - `rustx_dom::Element` (enum : `Tag(TagElement)`, `Text(String)`, `Fragment(Vec<Element>)`, `Empty`)
  - `rustx_dom::TagElement { pub tag: String, pub attributes: Vec<(String, AttributeValue)>, pub handlers: Vec<(String, Handler)>, pub children: Vec<Element> }`
  - `rustx_dom::AttributeValue` (enum : `Text(String)`, `Flag(bool)`)
  - `rustx_dom::Handler` — enveloppe une `Box<dyn Fn(Event) + 'static>`, ni `Send` ni `Sync`
  - `rustx_dom::Event { pub kind: String }`
  - `rustx_dom::Element::tag(name: &str) -> TagElement` et les constructeurs enchaînables `TagElement::attribute`, `TagElement::handler`, `TagElement::child`, `TagElement::build() -> Element` — c'est la forme que le code généré appelle.
  - `rustx_dom::into_element<T: IntoElement>(value: T) -> Element` et le trait `IntoElement`, implémenté pour `Element`, pour `Vec<Element>`, pour `Option<Element>` et, par une implémentation générique, pour tout `T: std::fmt::Display`. C'est ce qui permet d'écrire `{clicks}` au lieu de `{clicks.to_string()}`.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
#[test]
fn a_tag_carries_its_attributes_handlers_and_children_in_order() {
    let element = Element::tag("div")
        .attribute("className", AttributeValue::Text("row".into()))
        .child(Element::Text("one".into()))
        .child(Element::Text("two".into()))
        .build();

    let Element::Tag(tag) = element else { panic!("expected a tag") };
    assert_eq!(tag.tag, "div");
    assert_eq!(tag.attributes.len(), 1);
    assert_eq!(tag.children.len(), 2);
    assert!(matches!(&tag.children[0], Element::Text(t) if t == "one"));
}

#[test]
fn a_handler_is_callable_and_needs_no_thread_bound() {
    let seen = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter = seen.clone();
    let element = Element::tag("button")
        .handler("onClick", Handler::new(move |_event| counter.set(counter.get() + 1)))
        .build();

    let Element::Tag(tag) = element else { panic!("expected a tag") };
    tag.handlers[0].1.call(Event { kind: "click".into() });
    assert_eq!(seen.get(), 1);
}

#[test]
fn anything_displayable_can_be_written_between_braces() {
    // `{clicks}` — no `.to_string()`, no `.get()`, no conversion at all.
    assert!(matches!(into_element(7_u32), Element::Text(t) if t == "7"));
    assert!(matches!(into_element("hi"), Element::Text(t) if t == "hi"));
    assert!(matches!(into_element(Element::Empty), Element::Empty));
    assert!(matches!(into_element(Vec::<Element>::new()), Element::Fragment(c) if c.is_empty()));
}
```

Le second test fixe une décision du document : un gestionnaire capture librement du `Rc`, parce qu'il ne quitte jamais le thread d'interface.

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p rustx-dom`
Expected: FAIL, le paquet n'existe pas.

- [ ] **Step 3: Créer la crate et implémenter `element.rs`**

Aucune dépendance. `Handler` ne dérive pas `Debug` ni `Clone` ; les autres types dérivent `Debug`.

- [ ] **Step 4: Lancer le test pour vérifier qu'il passe**

Run: `cargo test -p rustx-dom`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/rustx-dom
git commit -m "feat(dom): the element tree a component returns"
```

---

### Task 2: Repérer le JSX dans un fichier Rust

**Files:**
- Create: `crates/rustx-compiler/Cargo.toml`
- Create: `crates/rustx-compiler/src/lib.rs`
- Create: `crates/rustx-compiler/src/scan.rs`
- Create: `crates/rustx-compiler/src/error.rs`
- Modify: `Cargo.toml`
- Test: `crates/rustx-compiler/tests/scan.rs`

Un `.rsx` est du Rust auquel s'ajoute le JSX. Avant de traduire quoi que ce soit il faut savoir **où** commence et où finit chaque région JSX, et le dire en coordonnées du `.rsx`.

**Interfaces:**
- Consumes: rien.
- Produces:
  - `rustx_compiler::Span { pub start: usize, pub end: usize }` — décalages en octets dans le `.rsx`
  - `rustx_compiler::scan::find_jsx(source: &str) -> Result<Vec<Span>, CompileError>` — les régions JSX de plus haut niveau, dans l'ordre du fichier
  - `rustx_compiler::CompileError { pub file: PathBuf, pub line: usize, pub column: usize, pub message: String }`, `impl Display` au format `path:line:column: message`
  - `CompileError::at(source: &str, offset: usize, message: impl Into<String>) -> Self`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_return_of_jsx_is_one_region() {
    let source = "fn App() -> Element { return <div>hi</div>; }";
    let spans = find_jsx(source).expect("valid");
    assert_eq!(spans.len(), 1);
    assert_eq!(&source[spans[0].start..spans[0].end], "<div>hi</div>");
}

#[test]
fn comparisons_are_not_mistaken_for_tags() {
    // The hardest part of the whole scanner: `<` means two things in Rust.
    let source = "fn f() { let ok = a < b && c > d; let g: Vec<u8> = vec![]; }";
    assert_eq!(find_jsx(source).expect("valid"), Vec::new());
}

#[test]
fn nested_tags_are_one_region_not_three() {
    let source = "fn App() -> Element { return <ul><li>a</li></ul>; }";
    let spans = find_jsx(source).expect("valid");
    assert_eq!(spans.len(), 1);
}

#[test]
fn an_unclosed_tag_names_the_rsx_line_and_column() {
    // Review Focus 1.
    let source = "fn App() -> Element {\n    return <div>oops;\n}";
    let error = find_jsx(source).expect_err("unclosed");
    assert_eq!(error.line, 2);
    assert_eq!(error.column, 12);
    assert!(error.message.contains("<div>"), "got {}", error.message);
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-compiler --test scan`
Expected: FAIL, `find_jsx` non défini.

- [ ] **Step 3: Implémenter `find_jsx` dans `scan.rs`**

La règle qui distingue un `<` ouvrant une balise d'un `<` de comparaison, et qui est la décision que l'implémenteur ne peut pas prendre seul : **un `<` ouvre une balise lorsqu'il est immédiatement suivi d'un identifiant ou de `/` ou de `>` sans espace intercalaire, et que le jeton qui le précède est l'un de `return`, `=>`, `{`, `(`, `,`, `=` ou le début du fichier.** Partout ailleurs c'est un opérateur. Les génériques (`Vec<u8>`) échouent sur le test du jeton précédent, puisqu'ils suivent un identifiant ou `:`.

Une fois une balise ouverte, la région s'étend jusqu'à la fermeture correspondante, en comptant les imbrications et en ignorant les `<` à l'intérieur des littéraux de chaîne et des blocs `{}` d'expression Rust.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-compiler --test scan`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-compiler Cargo.toml
git commit -m "feat(compiler): find JSX regions in Rust source"
```

---

### Task 3: Analyser une région JSX

**Files:**
- Create: `crates/rustx-compiler/src/parse.rs`
- Create: `crates/rustx-compiler/src/ast.rs`
- Test: `crates/rustx-compiler/tests/parse.rs`

**Interfaces:**
- Consumes: Tâche 2 (`Span`, `CompileError`).
- Produces:
  - `rustx_compiler::ast::Node` (enum : `Tag(TagNode)`, `Text(String)`, `Expression(String)`)
  - `rustx_compiler::ast::TagNode { pub name: String, pub attributes: Vec<Attribute>, pub children: Vec<Node>, pub offset: usize }`
  - `rustx_compiler::ast::Attribute { pub name: String, pub value: AttributeSource, pub offset: usize }`
  - `rustx_compiler::ast::AttributeSource` (enum : `Literal(String)`, `Expression(String)`, `Flag`)
  - `rustx_compiler::parse::parse_jsx(source: &str, span: Span) -> Result<Node, CompileError>`

`offset` est le décalage dans le `.rsx` d'origine ; c'est ce qui alimente la carte de la tâche 5.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
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
    assert!(matches!(&tag.children[0], Node::Expression(e) if e.trim() == "user.name.to_uppercase()"));
}

#[test]
fn attributes_come_in_three_shapes() {
    let node = parse_one(r#"<input className="row" value={name} disabled />"#);
    let Node::Tag(tag) = node else { panic!() };
    assert!(matches!(&tag.attributes[0].value, AttributeSource::Literal(v) if v == "row"));
    assert!(matches!(&tag.attributes[1].value, AttributeSource::Expression(v) if v.trim() == "name"));
    assert!(matches!(tag.attributes[2].value, AttributeSource::Flag));
}

#[test]
fn a_closure_attribute_survives_its_braces_and_body() {
    let node = parse_one("<button onClick={|| { let mut n = 0; n += 1; }}>go</button>");
    let Node::Tag(tag) = node else { panic!() };
    let AttributeSource::Expression(code) = &tag.attributes[0].value else { panic!() };
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
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-compiler --test parse`
Expected: FAIL, `parse_jsx` non défini.

- [ ] **Step 3: Implémenter `parse_jsx` dans `parse.rs` et les types dans `ast.rs`**

Le texte entre balises est pris littéralement jusqu'au prochain `<` ou `{`, espaces de bord compris puis réduits comme en JSX. Un `{` ouvre une expression Rust qui s'étend jusqu'à son `}` apparié, en comptant les accolades et en ignorant celles des littéraux de chaîne et de caractère : c'est ce qui laisse passer une closure avec un corps.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-compiler --test parse`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-compiler/src/parse.rs crates/rustx-compiler/src/ast.rs crates/rustx-compiler/tests/parse.rs
git commit -m "feat(compiler): parse a JSX region into a node tree"
```

---

### Task 4: Émettre du Rust

**Files:**
- Create: `crates/rustx-compiler/src/emit.rs`
- Create: `crates/rustx-compiler/src/attributes.rs`
- Test: `crates/rustx-compiler/tests/emit.rs`

**Interfaces:**
- Consumes: Tâche 1 (`rustx_dom::Element` et ses constructeurs), Tâche 3 (`ast::Node`).
- Produces:
  - `rustx_compiler::emit::emit(node: &ast::Node) -> Result<String, CompileError>` — l'expression Rust qui construit l'arbre
  - `rustx_compiler::attributes::is_event(name: &str) -> bool` — `true` pour `onClick`, `onChange`, … de la liste du DOM
  - `rustx_compiler::attributes::is_known(name: &str) -> bool`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_tag_becomes_a_chain_of_constructors() {
    let rust = emit_one(r#"<div className="row">hi</div>"#);
    assert!(rust.contains(r#"Element :: tag ("div")"#) || rust.contains(r#"Element::tag("div")"#), "got {rust}");
    assert!(rust.contains("className"), "got {rust}");
    assert!(rust.contains("hi"), "got {rust}");
}

#[test]
fn an_event_attribute_becomes_a_handler_not_an_attribute() {
    let rust = emit_one("<button onClick={|| ()}>go</button>");
    assert!(rust.contains("handler"), "got {rust}");
    assert!(!rust.contains("attribute (\"onClick\""), "got {rust}");
}

#[test]
fn an_unknown_attribute_is_refused_by_name() {
    // Review Focus 4: a typo must not be silently dropped.
    let error = emit_one_err(r#"<div classname="row" />"#);
    assert!(error.message.contains("classname"), "got {}", error.message);
    assert!(error.message.contains("className"), "a suggestion is expected: {}", error.message);
}

#[test]
fn text_is_emitted_as_a_string_literal_not_as_code() {
    // Review Focus 2, the other half: text must never become source.
    let rust = emit_one(r#"<p>{"}"} and " and \</p>"#);
    assert!(rust.contains(r#"Element :: Text"#) || rust.contains("Element::Text"), "got {rust}");
}

#[test]
fn an_expression_child_needs_no_conversion_from_the_developer() {
    // `{clicks}`, never `{clicks.to_string()}`. Anything that can be
    // displayed can be written between braces, and the emitter converts.
    let rust = emit_one("<p>{count}</p>");
    assert!(rust.contains("into_element"), "got {rust}");
    assert!(!rust.contains("to_string"), "the developer's expression was rewritten: {rust}");
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-compiler --test emit`
Expected: FAIL, `emit` non défini.

- [ ] **Step 3: Implémenter `emit` et la table d'attributs**

Un attribut dont `is_event` est vrai devient `.handler("onClick", Handler::new(<expression>))` ; les autres `.attribute(...)`. Un nom inconnu de `is_known` produit une `CompileError` qui nomme l'attribut écrit et le plus proche de la table, mesuré par distance de Levenshtein.

Une expression entre accolades est émise telle quelle, enveloppée dans
`rustx_dom::into_element(<expression>)`. Le développeur n'écrit jamais de
conversion : `{clicks}`, `{user.name}`, `{count + 1}` suffisent. C'est une
exigence du commanditaire, et elle est aussi ce qui rend l'écriture d'une
interface indiscernable du TSX.

La table couvre les attributs HTML du DOM et les gestionnaires `on*` de React. Elle vit dans `attributes.rs`, en une constante triée, et c'est le seul endroit à modifier quand il en manque un.

Le texte est émis avec `proc_macro2::Literal::string`, jamais par concaténation : c'est ce qui rend l'injection impossible.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-compiler --test emit`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-compiler/src/emit.rs crates/rustx-compiler/src/attributes.rs crates/rustx-compiler/tests/emit.rs
git commit -m "feat(compiler): emit Rust that builds the element tree"
```

---

### Task 5: Transpiler un fichier, et la carte de correspondance

**Files:**
- Create: `crates/rustx-compiler/src/transpile.rs`
- Create: `crates/rustx-compiler/src/map.rs`
- Create: `crates/rustx-compiler/src/build.rs`
- Test: `crates/rustx-compiler/tests/transpile.rs`

Sans la carte, un message de `rustc` désigne une ligne du `.rs` généré que le développeur n'a jamais écrite, et l'écart grandit avec le fichier. C'est ce qui décide si l'outillage du plan 6 sera possible.

**Interfaces:**
- Consumes: Tâches 2, 3, 4.
- Produces:
  - `rustx_compiler::transpile::transpile(source: &str) -> Result<Transpiled, CompileError>`
  - `rustx_compiler::transpile::Transpiled { pub rust: String, pub map: PositionMap }`
  - `rustx_compiler::map::PositionMap` avec `PositionMap::to_rsx(&self, generated_line: usize) -> Option<usize>` et `PositionMap::to_generated(&self, rsx_line: usize) -> Option<usize>`
  - `rustx_compiler::build::compile_directory(from: &Path, to: &Path) -> Result<Vec<PathBuf>, CompileError>` — pour le script de build

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn rust_outside_jsx_is_copied_through_untouched() {
    let source = "use std::fmt;\n\nfn helper(x: u8) -> u8 { x + 1 }\n";
    assert_eq!(transpile(source).unwrap().rust, source);
}

#[test]
fn every_rsx_line_maps_to_the_line_it_generated() {
    // Review Focus 3: a rustc error must point at the .rsx line.
    let source = "fn App() -> Element {\n    let name = missing_fn();\n    return <div>{name}</div>;\n}\n";
    let out = transpile(source).unwrap();

    let generated = out.rust.lines().position(|l| l.contains("missing_fn")).unwrap() + 1;
    assert_eq!(out.map.to_rsx(generated), Some(2));
}

#[test]
fn the_mapping_holds_after_jsx_that_expands_over_several_lines() {
    let source = "fn App() -> Element {\n    return <ul>\n        <li>a</li>\n    </ul>;\n}\nfn after() -> u8 { 7 }\n";
    let out = transpile(source).unwrap();

    let generated = out.rust.lines().position(|l| l.contains("fn after")).unwrap() + 1;
    assert_eq!(out.map.to_rsx(generated), Some(6));
}

#[test]
fn transpiling_is_deterministic() {
    let source = "fn App() -> Element { return <div><span>a</span><b>c</b></div>; }";
    assert_eq!(transpile(source).unwrap().rust, transpile(source).unwrap().rust);
}
```

Le troisième test est le cœur du sujet : le JSX occupe trois lignes de `.rsx` et un nombre différent de lignes de `.rs`, et tout ce qui suit doit rester correctement aligné.

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-compiler --test transpile`
Expected: FAIL, `transpile` non défini.

- [ ] **Step 3: Implémenter `transpile`, `PositionMap` et `compile_directory`**

La carte est une liste de couples (ligne générée, ligne `.rsx`) aux points de rupture, c'est-à-dire au début et à la fin de chaque région JSX ; entre deux ruptures la correspondance est une translation constante.

`compile_directory` traduit chaque `.rsx` de `from` en un `.rs` de même nom relatif sous `to`, et écrit la carte à côté, en `<name>.rsx.map`.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-compiler --test transpile`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-compiler/src/transpile.rs crates/rustx-compiler/src/map.rs crates/rustx-compiler/src/build.rs crates/rustx-compiler/tests/transpile.rs
git commit -m "feat(compiler): transpile a .rsx file and map its positions back"
```

---

### Task 6: Monter un arbre dans le DOM

**Files:**
- Create: `crates/rustx-engine/src/mount.rs`
- Modify: `crates/rustx-engine/Cargo.toml`, `crates/rustx-engine/src/lib.rs`
- Test: `crates/rustx-engine/tests/mount.rs`

**Interfaces:**
- Consumes: Tâche 1 (`rustx_dom::Element`), plan 1 (`dom::Document`, `dom::Node`, `renderer::register_native`).
- Produces:
  - `rustx_engine::mount::mount(element: Element, into: &dom::Node) -> Result<(), DomError>` — construit les nœuds et les attache
  - `rustx_engine::mount::mount_at_id(element: Element, id: &str) -> Result<(), DomError>`

- [ ] **Step 1: Écrire les tests qui échouent**

Scénario `mount` dans l'application de démonstration, lu par le pilote de test existant (`crates/rustx-engine/tests/common/mod.rs`).

```rust
#[test]
fn the_mounted_tree_matches_what_was_written() {
    // Review Focus 5: nothing lost, nothing reordered, text at depth.
    assert_eq!(report().get("html"), "<ul><li>a</li><li><b>c</b></li></ul>");
}

#[test]
fn an_empty_component_mounts_nothing_and_does_not_fail() {
    assert_eq!(report().get("empty_children"), "0");
}

#[test]
fn a_fragment_mounts_its_nodes_side_by_side_without_a_wrapper() {
    assert_eq!(report().get("fragment_html"), "<i>a</i><i>b</i>");
}

#[test]
fn a_click_on_a_mounted_node_runs_the_rust_closure() {
    assert_eq!(report().get("clicked"), "1");
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-engine --test mount`
Expected: FAIL, le scénario ne rapporte rien.

- [ ] **Step 3: Implémenter `mount.rs`**

Parcours en profondeur : `Element::Tag` devient `Document::create_element` puis ses attributs, puis ses enfants dans l'ordre, puis `append_child`. `Element::Text` devient un nœud texte. `Element::Fragment` monte ses enfants directement dans le parent, sans nœud enveloppant. `Element::Empty` ne monte rien.

Un gestionnaire est enregistré comme fonction native sous un nom engendré, et `addEventListener` est posé sur le nœud via une expression JavaScript qui l'appelle. C'est le seul endroit où le montage passe par du JavaScript plutôt que par un appel direct ; la raison est que `addEventListener` prend une fonction JavaScript, pas un pointeur natif.

- [ ] **Step 4: Étoffer la démonstration**

Ajouter au binaire `hello` un scénario `mount` qui monte les trois arbres ci-dessus et rapporte `html`, `empty_children`, `fragment_html` et `clicked`.

- [ ] **Step 5: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-engine --test mount`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/rustx-engine/src/mount.rs crates/rustx-engine/tests/mount.rs crates/rustx-engine/src/bin/hello.rs crates/rustx-engine/assets crates/rustx-engine/Cargo.toml crates/rustx-engine/src/lib.rs
git commit -m "feat(engine): mount an element tree into the DOM"
```

---

### Task 7: Une application écrite en `.rsx`

**Files:**
- Create: `apps/catalogue/Cargo.toml`
- Create: `apps/catalogue/build.rs`
- Create: `apps/catalogue/app/page.rsx`
- Create: `apps/catalogue/src/main.rs`
- Create: `apps/catalogue/assets/index.html`
- Modify: `Cargo.toml`
- Test: `apps/catalogue/tests/app.rs`

La preuve du plan : un projet séparé, qui consomme le framework comme n'importe quel utilisateur le fera, et dont l'interface est écrite en `.rsx`.

**Interfaces:**
- Consumes: toutes les tâches précédentes.
- Produces: aucune API ; c'est une application.

- [ ] **Step 1: Écrire le test qui échoue**

```rust
#[test]
fn the_page_written_in_rsx_is_on_screen() {
    let report = run_catalogue_scenario();
    assert_eq!(
        report.get("html"),
        "<h1>Catalogue</h1><ul class=\"devices\"><li>Webcam</li><li>Micro</li><li>Écran</li></ul><button id=\"shout\">3</button>"
    );
}

#[test]
fn clicking_the_button_runs_the_rust_written_in_the_rsx() {
    let report = run_catalogue_scenario();
    assert_eq!(report.get("clicked"), "yes");
}
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p catalogue`
Expected: FAIL, le paquet n'existe pas.

- [ ] **Step 3: Écrire `app/page.rsx`**

C'est la page entière, et elle doit se lire comme du TSX :

```rust
fn Page() -> Element {
    let title = "Catalogue";
    let items = vec!["Webcam", "Micro", "Écran"];

    return <>
        <h1>{title}</h1>
        <ul className="devices">
            {items.iter().map(|name| <li>{name}</li>).collect::<Vec<_>>()}
        </ul>
        <button id="shout" onClick={|_| println!("clicked from Rust")}>
            {items.len()}
        </button>
    </>;
}
```

Aucune conversion, aucun `.to_string()`, aucun `.get()` : `{title}`,
`{name}` et `{items.len()}` s'écrivent tels quels, et `into_element` de la
tâche 1 fait le reste. Une liste est une expression Rust ordinaire qui rend
un `Vec<Element>`.

**Pas de compteur dans ce plan, et c'est délibéré.** Un compteur exige
`let mut clicks = 0;` avec `{clicks}` qui se met à jour tout seul, donc la
promotion automatique d'état et le re-rendu — le plan 3. Écrire ici un
compteur à base de `Rc<Cell<…>>` donnerait une page qui a l'air de marcher
tout en enseignant une syntaxe qu'on s'est engagé à ne jamais demander.
Mieux vaut une page qui ne ment pas.

- [ ] **Step 4: Écrire `build.rs` et `main.rs`**

`build.rs` appelle `compile_directory("app", &out_dir)` et redéclare `cargo::rerun-if-changed=app`. `main.rs` inclut le `.rs` engendré avec `include!`, puis fait ce que fait `hello` : `bootstrap`, `App::new`, et `mount_at_id(Page(), "root")` depuis un `on_context_created`.

- [ ] **Step 5: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p catalogue`
Expected: PASS

- [ ] **Step 6: Lancer toute la suite**

Run: `cargo test --workspace -- --test-threads=1`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add apps/catalogue Cargo.toml
git commit -m "feat(apps): a catalogue page written in .rsx"
```

---

## Ce que ce plan livre, et ce qui vient après

À la fin : on écrit du JSX nu dans un `.rsx`, `cargo build` le compile, l'interface s'affiche, un `onClick` exécute du Rust, et une erreur de compilation désigne la ligne du `.rsx`.

Ce qui manque encore pour que ce soit « comme du Next.js », dans l'ordre des
plans suivants : la promotion automatique d'état et le re-rendu — le plan 3,
celui qui fait que

```rust
fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={|| n += 1}>{n}</button>;
}
```

compile, s'affiche, et se met à jour quand on clique. C'est la syntaxe cible
du document, et aucun plan antérieur n'a le droit d'en livrer une imitation.
Ensuite : le routing par fichiers et les layouts, l'interopérabilité React et
npm, la couche OS, puis la CLI et le hot reload, et l'extension VS Code en
dernier.
