# rustx — Plan 6 : l'interopérabilité React et npm

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Qu'une page écrite comme on écrit du TSX compile — `style={{…}}`, `className`, les attributs en camelCase — et qu'un composant React vive à l'intérieur d'un arbre `rustx`, et un composant `rustx` à l'intérieur d'un arbre React, dans le même DOM et la même boucle de frames.

```rust
fn Panel() -> Element {
    return <div style={{ display: "flex", gap: "12px" }}>
        <js::Chart data={series} />
    </div>;
}
```

**Architecture:** Deux moitiés indépendantes, et c'est pour ça qu'elles tiennent dans un plan. La première est du compilateur seul : le dialecte de props de React — un objet de style devient une chaîne CSS, les clés camelCase deviennent du kebab-case. La seconde est une frontière de nœud : un `Element::Foreign` est un conteneur que le réconciliateur de `rustx` construit puis ne regarde plus jamais, et à l'intérieur duquel React est seul maître ; symétriquement, un nœud que React possède peut recevoir un composant `rustx` par son nom. Rien n'est emprunté à travers : une prop est une valeur scalaire copiée dans V8.

**Tech Stack:** Rust edition 2024, `rustx-dom`, `rustx-compiler`, `rustx-engine` (CEF 156.2.0), Windows 10 x64, JavaScript (ES2020) côté page.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.5 en entier (les deux sens, les deux règles d'appropriation), §4 anti-objectif n°4 (pas d'`unsafe` pour régler la frontière), §7.3 (transpilation déterministe).

**Portée :** le dialecte de props de React (`style={{…}}`, `className`, camelCase), `Element::Foreign` et son traitement opaque par le diff, la syntaxe `<js::Nom />` avec ses props scalaires, le montage et le démontage du sous-arbre React par la page, le sens inverse (`rustx` monté dans un nœud React), et un logiciel de preuve hybride.

**Hors périmètre, explicitement :** l'empaquetage des dépendances npm **dans le binaire**, qui appartient au plan 8 (§7.7, `rustx build` vers un `.exe` unique) — ici le bundle de la page est un fichier d'`assets/`, chargé localement comme le reste. Également hors périmètre : le transfert de propriété d'un gros tampon (une image de caméra) à travers la frontière, qui a besoin du matériel de la couche OS pour avoir un sens et qui est le seul point de §7.5 que ce plan laisse ouvert ; et l'écriture de `.tsx` par le compilateur `rustx`, qui n'a jamais été promise — un `.tsx` est compilé par la chaîne JavaScript du développeur.

## Global Constraints

- Plateforme cible : Windows 10 x64 uniquement. Rust edition 2024.
- Aucune dépendance sous GPL ; `unsafe` reste confiné à `rustx-engine`, et **l'anti-objectif n°4 interdit de régler la frontière avec de l'`unsafe`** : une prop traverse en valeur, jamais en référence.
- **React possède son sous-arbre.** Le réconciliateur de `rustx` donne un nœud conteneur et n'y redescend plus jamais : il peut le remplacer ou l'enlever, jamais en modifier l'intérieur. Symétriquement pour un composant `rustx` monté dans un nœud React.
- **Rien n'est emprunté à travers la frontière.** Une prop est `Value` (null, booléen, nombre, chaîne), copiée dans V8. Aucune référence Rust ne survit côté JavaScript : V8 décide quand il lâche, et il peut lâcher après la mort du propriétaire Rust.
- Pas de sérialisation JSON entre les deux mondes : les props passent par les valeurs V8 du plan 1.
- Le contenu reste local (plan 1, §5) : un bundle React est un fichier de l'application, jamais une URL.
- La transpilation reste déterministe.

## Review Focus

1. **Un sous-arbre React dont le conteneur disparaît d'un rendu `rustx`.** React doit être prévenu avant que le nœud ne quitte le document, sinon ses effets ne se nettoient jamais et ses écouteurs restent sur un nœud détaché. → Tâche 5.
2. **Un `style={{…}}` dont une valeur est un nombre, ou dont une clé est déjà en kebab-case.** React ajoute `px` à certains nombres et pas à d'autres ; `rustx` ne doit pas deviner. → Tâche 1.
3. **Une prop qui n'est pas un scalaire.** `<js::Chart camera={camera} />` ne peut pas traverser, et doit être refusé à la compilation plutôt que de devenir une chaîne « Camera { … } » dans le DOM. → Tâche 4.
4. **Un composant `rustx` monté dans un nœud React que React enlève ensuite.** L'instance `rustx` doit être libérée, avec ses tâches, et ne pas rester dans la liste des racines à re-rendre à chaque frame. → Tâche 6.
5. **Un nom de composant JavaScript que la page n'a pas enregistré.** Doit le dire à l'écran et sur stderr, plutôt que laisser un trou. → Tâche 5.

---

### Task 1: Les props d'une balise, celles de HTML et celles de JSX

**Files:**
- Modify: `crates/rustx-compiler/src/emit.rs`, `crates/rustx-compiler/src/attributes.rs`
- Test: `crates/rustx-compiler/tests/style.rs`, `crates/rustx-compiler/tests/props.rs`

Ce que `.rsx` doit accepter sur une balise, c'est **ce que HTML et JSX acceptent** — pas une liste choisie. Aujourd'hui `class`, `for`, `tabindex`, `maxlength` et la plupart des attributs de présentation SVG sont refusés parce que seule l'orthographe camelCase de React est dans la table ; et `style={{ display: "flex" }}` est recopié tel quel dans du Rust, d'où cent une erreurs `struct literal body without path`. Les deux sont la même tâche : la surface des props.

**Interfaces:**
- Consumes: plan 5 (`emit::Context`, les refus d'attributs).
- Produces:
  - `rustx_compiler::attributes::style_string(object: &str) -> Result<String, String>` — un objet JSX de style en une déclaration CSS, ou le nom de ce qui ne va pas
  - `emit` accepte `style={{ … }}` sur un élément
  - `attributes::canonical(name: &str) -> Option<&'static str>` — la table reconnaît les **deux** orthographes de chaque attribut, celle de HTML et celle de JSX, et dit laquelle a été écrite
  - Les tables couvrent les attributs de HTML et les attributs de présentation de SVG, pas un échantillon ; `data-*`, `aria-*`, `xlink:` et `xml:` restent ouverts comme aujourd'hui
  - Une faute de frappe est toujours refusée, avec le nom le plus proche : accepter les deux orthographes n'est pas accepter n'importe quoi

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn an_object_of_styles_becomes_one_css_declaration() {
    assert_eq!(style_string("{ display: \"flex\", gap: \"12px\" }").unwrap(), "display: flex; gap: 12px");
}

#[test]
fn a_camel_case_property_becomes_kebab_case() {
    // Review Focus 2, second half: already-kebab keys are left alone.
    assert_eq!(style_string("{ backgroundColor: \"#09090b\" }").unwrap(), "background-color: #09090b");
    assert_eq!(style_string("{ \"background-color\": \"#09090b\" }").unwrap(), "background-color: #09090b");
}

#[test]
fn a_trailing_comma_is_allowed_because_rustfmt_writes_one() {
    assert_eq!(style_string("{ color: \"red\", }").unwrap(), "color: red");
}

#[test]
fn a_numeric_value_is_refused_by_name() {
    // Review Focus 2: React adds `px` to some numbers and not others.
    // Guessing is worse than asking.
    let complaint = style_string("{ width: 10 }").expect_err("should be refused");
    assert!(complaint.contains("width") && complaint.contains("px"), "got {complaint}");
}

#[test]
fn a_style_object_reaches_the_dom_as_a_style_attribute() {
    let rust = emit_one("<div style={{ display: \"flex\" }}>hi</div>");
    assert!(rust.contains("\"style\""), "got {rust}");
    assert!(rust.contains("display: flex"), "got {rust}");
}

#[test]
fn a_style_string_is_still_a_style_string() {
    let rust = emit_one("<div style=\"display: flex\">hi</div>");
    assert!(rust.contains("display: flex"), "got {rust}");
}

#[test]
fn a_style_expression_that_is_not_an_object_is_passed_through() {
    // `style={computed}` is a value the developer built; it reaches the
    // DOM as it is.
    let rust = emit_one("<div style={computed}>hi</div>");
    assert!(rust.contains("computed"), "got {rust}");
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-compiler --test style`, puis commit `feat(compiler): style objects, the way TSX writes them`.

**Step 3, la décision :** l'objet est lu par le scanner déjà écrit (chaînes et virgules de premier niveau, comme `split_first_argument` du plan 4), pas par un parseur JSON. Une valeur doit être une chaîne littérale : un nombre est refusé en nommant la propriété et en disant d'écrire `"10px"`. La transformation camelCase → kebab-case est mécanique ; une clé déjà en kebab-case, entre guillemets, passe telle quelle.

- [ ] **Step 6: Vérifier sur la vraie page** — `cargo build -p counter` compile la page d'atterrissage du dépôt (588 lignes de `style={{…}}`) — PASS

---

### Task 2: Un appel de composant est du Rust ordinaire

**Files:**
- Modify: `crates/rustx-compiler/src/emit.rs`
- Modify: `apps/capture/app/page.rsx`
- Test: `crates/rustx-compiler/tests/composition.rs`

Un composant déclaré dans le fichier s'appelle comme une fonction Rust s'appelle : `{Device("Webcam".to_string())}`. La balise `<Device name={…} />` du plan 5 reste pour un layout et ses `children`, mais elle ne doit pas être la seule façon, et elle n'est pas celle qu'un lecteur de Rust écrira.

Ce qui manque n'est pas la syntaxe — `{Device(…)}` compile déjà — c'est l'**instance** : une expression entre accolades appelle la fonction au milieu du rendu du parent, donc ses cellules appartiennent au parent. Deux appels sur la même ligne partagent alors un état, et le plan 3 appelle ça exactement « deux composants qui se partagent une cellule ».

**Interfaces:**
- Consumes: plan 5 (`emit::Context`, `component_at`).
- Produces: rien de public ; `emit` reconnaît un appel de composant dans une expression JSX.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_call_in_braces_to_a_component_of_this_file_is_an_instance_of_its_own() {
    let rust = rewrite_one("fn Row(label: String) -> Element {\n    return <li>{label}</li>;\n}\nfn Page() -> Element {\n    return <ul>{Row(\"a\".to_string())}</ul>;\n}");
    assert!(rust.contains("component_at"), "got {rust}");
    assert!(rust.contains("Row (\"a\" . to_string ())"), "got {rust}");
}

#[test]
fn two_calls_in_braces_are_two_instances() {
    // Review Focus of plan 5, number 2, written the Rust way.
    let rust = rewrite_one("fn Row() -> Element {\n    return <li>x</li>;\n}\nfn Page() -> Element {\n    return <ul>{Row()}{Row()}</ul>;\n}");
    assert_eq!(rust.matches("component_at").count(), 2, "got {rust}");
    assert_ne!(columns_of(&rust)[0], columns_of(&rust)[1], "two places, two instances: {rust}");
}

#[test]
fn a_call_to_something_that_is_not_a_component_is_left_alone() {
    let rust = rewrite_one("fn helper() -> u8 { 1 }\nfn Page() -> Element {\n    return <p>{helper()}</p>;\n}");
    assert!(!rust.contains("component_at"), "got {rust}");
}

#[test]
fn a_read_that_only_looks_like_a_call_is_left_alone() {
    let rust = rewrite_one("fn Page() -> Element {\n    let n = 1;\n    return <p>{n.to_string()}</p>;\n}");
    assert!(!rust.contains("component_at"), "got {rust}");
}

#[test]
fn a_call_inside_a_larger_expression_is_left_alone() {
    // `{if ok { Row() } else { Empty() }}` is a branch, and a component
    // under an `if` is what plan 4's reach-check complains about. The
    // compiler does not quietly make it an instance.
    let rust = rewrite_one("fn Row() -> Element {\n    return <li>x</li>;\n}\nfn Page() -> Element {\n    return <ul>{vec![Row()]}</ul>;\n}");
    assert!(!rust.contains("component_at"), "got {rust}");
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-compiler --test composition`, puis commit `feat(compiler): a component called the way Rust calls a function`.

**Step 3, la décision :** la reconnaissance porte sur la forme **entière** de l'expression : `Nom(…)` où `Nom` est un composant du fichier et où la parenthèse fermante termine l'expression. Un appel enfoui dans une expression plus grande est laissé tel quel, parce que ce qui l'entoure peut l'appeler zéro ou dix fois et que l'identité par emplacement n'aurait alors plus de sens.

- [ ] **Step 6: Récrire la page de `capture`** avec `{Device(…)}`, et relancer `cargo test -p capture`.

---

### Task 3: La frontière de nœud

**Files:**
- Modify: `crates/rustx-dom/src/element.rs`, `crates/rustx-dom/src/diff.rs`
- Test: `crates/rustx-dom/tests/foreign.rs`

**Interfaces:**
- Consumes: plan 3 (`Element`, `diff`, `Patch`).
- Produces:
  - `rustx_dom::Element::Foreign { component: String, props: Vec<(String, rustx_dom::Value)>, id: String }`
  - `rustx_dom::Value` — null, booléen, nombre, chaîne : ce qui traverse, et rien d'autre. **`Value` vit aujourd'hui dans `rustx-engine` (`value.rs`)** : il descend dans `rustx-dom`, parce qu'un `Element` le porte maintenant et que `rustx-dom` ne peut pas dépendre du moteur. `rustx_engine::Value` devient une ré-exportation, donc aucun appelant ne change.
  - `rustx_dom::diff::Patch::ForeignProps { path: Path, props: Vec<(String, Value)> }`
  - `Patch::path(&self) -> &Path` et `Path::depth(&self) -> usize`, s'ils n'existent pas déjà — le test de la Review Focus les demande
  - Le diff traite un `Foreign` comme opaque : props changées → un patch de props ; composant changé → remplacement ; jamais de descente dedans

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn two_foreign_nodes_of_the_same_component_with_the_same_props_are_one_patch_free_node() {
    assert!(diff(&chart("a"), &chart("a")).is_empty());
}

#[test]
fn changing_a_prop_patches_the_props_and_nothing_else() {
    assert_eq!(diff(&chart("a"), &chart("b")), vec![Patch::ForeignProps { path: Path::root(), props: vec![("series".into(), Value::String("b".into()))] }]);
}

#[test]
fn changing_the_component_replaces_the_container() {
    assert!(matches!(diff(&chart("a"), &table("a")).first(), Some(Patch::Replace { .. })));
}

#[test]
fn the_diff_never_descends_into_a_foreign_node() {
    // React owns what is inside. Even if the tree says otherwise, no
    // patch may name a path below the container.
    let patches = diff(&chart_with_children(), &chart_with_other_children());
    assert!(patches.iter().all(|patch| patch.path().depth() <= 1), "got {patches:?}");
}

#[test]
fn a_foreign_node_that_leaves_the_tree_is_a_removal_of_its_container() {
    assert!(matches!(diff(&chart("a"), &Element::Empty).first(), Some(Patch::Replace { .. })));
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-dom --test foreign`, puis commit `feat(dom): a node React owns, and a diff that does not look inside it`.

**Step 3, la décision :** `Foreign` porte un `id` stable (fourni par le compilateur, comme un `Slot`) pour que le moteur retrouve le conteneur dans le DOM sans le chercher par position.

---

### Task 4: `<js::Nom />`

**Files:**
- Modify: `crates/rustx-compiler/src/parse.rs`, `crates/rustx-compiler/src/emit.rs`, `crates/rustx-compiler/src/tags.rs`
- Test: `crates/rustx-compiler/tests/foreign.rs`

**Interfaces:**
- Consumes: tâche 2, plan 5 (le refus d'un composant inconnu).
- Produces: `<js::Nom prop={…} />` → `Element::Foreign { component: "Nom", … }`. Rien de public de plus.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_js_tag_becomes_a_foreign_element() {
    let rust = rewrite_one("fn P() -> Element {\n    return <js::Chart series={\"a\".to_string()} />;\n}");
    assert!(rust.contains("Element :: Foreign"), "got {rust}");
    assert!(rust.contains("\"Chart\""), "got {rust}");
}

#[test]
fn a_js_tag_gets_an_id_of_its_own() {
    let rust = rewrite_one("fn P() -> Element {\n    return <div><js::Chart /><js::Chart /></div>;\n}");
    let ids: Vec<&str> = rust.matches("rustx-js-").collect();
    assert_eq!(ids.len(), 2, "each boundary needs its own container: {rust}");
}

#[test]
fn a_prop_that_is_not_a_scalar_is_refused_by_name() {
    // Review Focus 3, and Global Constraint: nothing is borrowed across
    // the boundary.
    let error = transpile("fn P(camera: Camera) -> Element {\n    return <js::Chart camera={camera} />;\n}").expect_err("should be refused");
    assert!(error.message.contains("camera"), "got {}", error.message);
}

#[test]
fn children_of_a_js_tag_are_refused() {
    // React owns what is inside; a child written here could never be
    // reconciled by either side.
    let error = transpile("fn P() -> Element {\n    return <js::Chart><p>hi</p></js::Chart>;\n}").expect_err("should be refused");
    assert!(error.message.contains("children"), "got {}", error.message);
}

#[test]
fn a_lowercase_js_tag_is_still_refused_as_an_unknown_tag() {
    assert!(transpile("fn P() -> Element {\n    return <js::chart />;\n}").is_err());
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-compiler --test foreign`, puis commit `feat(compiler): a js:: tag is a React component`.

**Step 3, la décision :** une prop doit être un littéral ou une expression dont le type est scalaire ; comme le compilateur ne connaît pas les types, le refus porte sur ce qu'il peut voir — une expression qui n'est ni un littéral, ni un appel à `.to_string()`, ni un nombre, ni un booléen est refusée en nommant la prop et en disant ce qui traverse. L'`id` du conteneur est `rustx-js-<ligne>-<colonne>`, déterministe comme un `Slot`.

---

### Task 5: Le moteur passe le relais à React

**Files:**
- Create: `crates/rustx-engine/src/foreign.rs`
- Modify: `crates/rustx-engine/src/mount.rs`, `crates/rustx-engine/src/render.rs`, `crates/rustx-engine/src/lib.rs`
- Test: `crates/rustx-engine/tests/foreign.rs`

**Interfaces:**
- Consumes: tâches 2 et 3, plan 1 (`Document`, `Node`, `register_native`), plan 3 (`mount_tree`, `apply`).
- Produces:
  - La page expose `window.__rustx_react = { mount(name, node, props), unmount(node) }` ; le moteur l'appelle
  - `rustx_engine::foreign::handed_over() -> u64` et `rustx_engine::foreign::given_back() -> u64` — pour les tests et le diagnostic

- [ ] **Step 1: Écrire les tests qui échouent — scénario `foreign` dans `hello`**

```rust
#[test]
fn a_react_component_is_mounted_into_the_container_rustx_built() {
    assert_eq!(report().get("chart_text"), "chart of a");
}

#[test]
fn changing_a_prop_reaches_react_without_rebuilding_it() {
    assert_eq!(report().get("chart_text_after_prop"), "chart of b");
    assert_eq!(report().get("react_mounts"), "1");
}

#[test]
fn rustx_does_not_touch_what_react_put_inside() {
    // React's own node is still there after a rustx re-render.
    assert_eq!(report().get("react_node_survived"), "yes");
}

#[test]
fn react_is_told_before_its_container_leaves_the_document() {
    // Review Focus 1: otherwise its effects never clean up and its
    // listeners stay on a detached node.
    assert_eq!(report().get("react_unmounted"), "yes");
    assert_eq!(report().get("unmount_came_first"), "yes");
}

#[test]
fn a_component_the_page_never_registered_says_so() {
    // Review Focus 5.
    assert_eq!(report().get("unknown_component"), "rustx: the page has no React component named 'Nope'");
}
```

- [ ] **Step 2 à 6** : scénario `foreign` dans `hello`, `cargo test -p rustx-engine --test foreign`, puis commit `feat(engine): hand a node to React, and tell it before taking it back`.

**Step 3, la décision :** le moteur appelle `__rustx_react.mount` juste après avoir attaché le conteneur, et `__rustx_react.unmount` **avant** de détacher le conteneur — l'ordre est la moitié de la tâche. Un composant inconnu est signalé sur stderr et laissé vide ; la page n'est pas prise en otage par une faute de frappe.

---

### Task 6: Le sens inverse

**Files:**
- Modify: `crates/rustx-engine/src/foreign.rs`, `crates/rustx-engine/src/renderer/mod.rs`
- Test: `crates/rustx-engine/tests/foreign.rs` (mêmes scénario et rapport)

**Interfaces:**
- Consumes: tâche 4, plan 3 (`render::mount_app`, `render::unmount`).
- Produces:
  - `rustx_engine::foreign::register_component(name: &str, component: impl Fn() -> Element + 'static)`
  - La page appelle `rustx_component(name, node)` pour monter, et `rustx_release(node)` pour rendre le nœud

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn react_can_mount_a_rustx_component_into_a_node_it_owns() {
    assert_eq!(report().get("rustx_inside_react"), "0");
}

#[test]
fn that_component_is_live_and_not_a_snapshot() {
    assert_eq!(report().get("rustx_inside_react_after_click"), "1");
}

#[test]
fn releasing_the_node_releases_the_component_and_its_tasks() {
    // Review Focus 4.
    assert_eq!(report().get("released_inside_react"), "yes");
    assert_eq!(report().get("roots_after_release"), report().get("roots_before_mount"));
}

#[test]
fn mounting_an_unregistered_rustx_component_is_an_error_the_page_can_catch() {
    assert!(report().get("unknown_rustx_component").contains("Nope"));
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-engine --test foreign`, puis commit `feat(engine): a rustx component mounted inside a node React owns`.

**Step 3, la décision :** `rustx_component` est une native ordinaire qui appelle `render::mount_app` sur le nœud que JavaScript lui donne — donc le composant est une racine de plus, re-rendue par la même frame que tout le reste, ce qui est exactement ce que §7.5 demande (« le même cycle de rendu »). `rustx_release` appelle `render::unmount`, donc la libération est celle du plan 3.

---

### Task 7: Le logiciel de preuve hybride

**Files:**
- Create: `apps/hybrid/` (`Cargo.toml`, `build.rs`, `app/page.rsx`, `src/main.rs`, `assets/index.html`, `assets/components.js`, `tests/`)
- Modify: `Cargo.toml`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_javascript_component_lives_inside_a_rustx_page() {
    assert_eq!(report().get("chart"), "7 points");
}

#[test]
fn a_rustx_component_lives_inside_a_javascript_panel() {
    assert_eq!(report().get("counter_in_panel"), "2");
}

#[test]
fn both_trees_share_one_frame_loop() {
    // One frame drew both sides: the counter and the chart moved on the
    // same tick, which is what forbids a second render loop.
    assert_eq!(report().get("drawn_on_one_frame"), "yes");
}

#[test]
fn leaving_the_page_tells_javascript_before_the_node_goes() {
    assert_eq!(report().get("js_unmounted"), "yes");
}
```

- [ ] **Step 2 à 7** : comme aux plans 4 et 5 — `cargo test -p hybrid`, puis `cargo test --workspace -- --test-threads=1`, puis commit `feat(apps): a page where Rust and JavaScript own one DOM between them`.

**Step 3, le bundle :** `assets/components.js` est écrit à la main et tient en trente lignes : il expose `__rustx_react.mount/unmount` et un panneau qui appelle `rustx_component`. Ce n'est pas React — React est une dépendance npm, et l'empaquetage npm est au plan 8 ; ce qui est prouvé ici est le **contrat** des deux règles d'appropriation, qui est ce que React consomme. Le fichier dit en commentaire ce qu'il remplace.

---

## Ce que ce plan livre, et ce qui vient après

À la fin, une page `rustx` s'écrit comme du TSX s'écrit, un composant JavaScript vit dans un arbre `rustx`, un composant `rustx` vit dans un arbre JavaScript, et les deux partagent un DOM et une boucle de frames sans qu'aucun des deux réconciliateurs n'entre chez l'autre. Le §7.5 est tenu, à l'exception du transfert d'un gros tampon, qui attend le matériel.

Ensuite : la couche OS — fenêtres, périphériques, capture, notifications, fichiers — qui est ce qui donne au logiciel de preuve son flux caméra, puis la CLI et le hot reload.
