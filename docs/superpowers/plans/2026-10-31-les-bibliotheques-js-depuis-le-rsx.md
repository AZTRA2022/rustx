# Les bibliothèques JavaScript s'utilisent depuis le `.rsx`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Écrire `<js::recharts::LineChart data={points}>…</js::recharts::LineChart>` dans une page `.rsx` et obtenir le composant de la bibliothèque, sans écrire une ligne de JavaScript de colle.

**Architecture:** Un nœud étranger cesse d'être une boîte opaque à props plates : il porte des enfants, ses props portent des listes et des enregistrements, et son nom est un chemin — `module::Composant`. Le moteur injecte lui-même la moitié JavaScript du pont (le runtime React), si bien que le seul JavaScript d'une application est la liste de ses bibliothèques : `rustx.use({ recharts })`.

**Tech Stack:** Rust, CEF 156.2.0 (V8 dans le processus de rendu), React 18.3.1 + react-dom 18.3.1 + recharts 2.12.7 via npm, esbuild 0.23.1.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.5 (interopérabilité), objectif n°3 (l'écosystème React disponible sans couture).

## Global Constraints

- Le développeur n'écrit que du `.rsx` et un `package.json`. Aucun fichier `.js` écrit à la main ne doit être nécessaire pour utiliser un composant npm.
- Aucune sérialisation du rendu : les props traversent comme valeurs V8 construites dans le processus de rendu, jamais comme texte JSON.
- `rustx-dom` ne dépend pas du moteur : le type des valeurs reste dans `rustx-dom`, la moitié V8 reste dans `rustx-engine`.
- Les deux règles de propriété ne bougent pas : rustx ne regarde pas dans un nœud que React possède, React ne touche pas à un nœud que rustx possède.
- React est prévenu **avant** que le conteneur quitte le document.
- Versions npm exactes : `react@18.3.1`, `react-dom@18.3.1`, `recharts@2.12.7`, `esbuild@0.23.1`.

## Review Focus

1. Un composant npm dont le nom n'existe pas dans le module (`<js::recharts::Lne />`) : le développeur doit lire le nom qu'il a écrit et le module où il a été cherché, pas voir un conteneur vide.
2. Une bibliothèque que la page utilise et que le bundle n'a pas déclarée (`rustx.use` sans elle) : même exigence, nommer le module manquant.
3. Un enfant étranger dont les props changent sans que le parent change : React doit recevoir un arbre à jour, pas l'arbre du premier rendu.
4. Une prop `Value::Record` dont une clé n'est pas un identifiant JavaScript (`"data-key"`) : elle doit traverser telle quelle.
5. Une liste vide et une liste de listes : `data={vec![]}` ne doit pas devenir `null`, et une imbrication ne doit pas s'aplatir.

---

### Task 1 : Une valeur peut être une liste ou un enregistrement

**Files:**
- Modify: `crates/rustx-dom/src/value.rs`
- Test: `crates/rustx-dom/tests/value.rs` (créer)

**Interfaces:**
- Produces: `Value::List(Vec<Value>)`, `Value::Record(Vec<(String, Value)>)` ; `trait ToValue { fn to_value(&self) -> Value; }` implémenté pour `bool`, les entiers, `f32`/`f64`, `&str`, `String`, `Option<T: ToValue>`, `Vec<T: ToValue>`, `&[T]` ; `pub fn record(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value`.

- [x] **Step 1** : `a_list_of_records_is_a_value`, `an_empty_list_is_a_list_not_null`, `a_record_keeps_a_key_that_is_not_an_identifier` (`record([("data-key", "at".to_value())])`), `an_option_is_null_or_the_value`.
- [x] **Step 2** : rouge (`cargo test -p rustx-dom --test value`).
- [x] **Step 3** : les deux variantes, le trait et ses implémentations.
- [x] **Step 4** : vert, plus `cargo test -p rustx-dom`.
- [x] **Step 5** : commit `feat(dom): a prop can be a list or a record`.

---

### Task 2 : V8 les construit et les relit

**Files:**
- Modify: `crates/rustx-engine/src/value.rs`
- Test: `crates/rustx-engine/assets/hello/index.html` + `crates/rustx-engine/tests/native_call.rs`

**Interfaces:**
- Consumes: `Value::List`, `Value::Record` (Task 1).
- Produces: `into_v8` construit un tableau (`v8_value_create_array`) et un objet ; `from_v8` relit un tableau et un objet simple ; `object_of` reste la porte d'entrée des props.

- [x] **Step 1** : dans le scénario `native_call`, une native `rustx_echo` qui renvoie ce qu'on lui donne ; le test `a_list_of_records_survives_the_round_trip` lit `rustx_echo([{at: 1, value: 2}])` et rapporte `JSON.stringify` du retour.
- [x] **Step 2** : rouge — « unsupported argument ».
- [x] **Step 3** : la récursion dans les deux sens. Une profondeur maximale de 32 refuse un cycle avec un message qui le dit (`rustx: a prop is nested more than 32 deep — a cycle cannot cross the boundary`).
- [x] **Step 4** : vert, plus `cargo test -p rustx-engine --test native_call`.
- [x] **Step 5** : commit `feat(engine): lists and records cross the boundary`.

---

### Task 3 : Une structure Rust devient une prop

**Files:**
- Modify: `crates/rustx-macros/src/lib.rs`
- Test: `crates/rustx-macros/tests/to_value.rs` (créer)

**Interfaces:**
- Consumes: `ToValue` (Task 1).
- Produces: `#[derive(rustx::ToValue)]` sur une structure à champs nommés, qui l'implémente en `Value::Record` avec le nom des champs tel qu'écrit. Un `#[rustx(rename = "data-key")]` sur un champ remplace son nom.

- [x] **Step 1** : `a_struct_becomes_a_record_of_its_fields`, `a_renamed_field_crosses_under_its_new_name`, `a_struct_with_unnamed_fields_is_refused_by_name` (message nommant la structure).
- [x] **Step 2** : rouge.
- [x] **Step 3** : le derive. Pas de dépendance nouvelle : `rustx-macros` lit déjà des tokens à la main.
- [x] **Step 4** : vert.
- [x] **Step 5** : commit `feat(macros): a struct crosses the boundary as a record`.

---

### Task 4 : Un nœud étranger a des enfants et un chemin

**Files:**
- Modify: `crates/rustx-dom/src/element.rs`, `crates/rustx-dom/src/diff.rs`
- Modify: `crates/rustx-compiler/src/parse.rs`, `crates/rustx-compiler/src/emit.rs`
- Test: `crates/rustx-compiler/tests/foreign.rs`, `crates/rustx-dom/tests/diff.rs`

**Interfaces:**
- Produces: `Element::Foreign { component: String, props: Vec<(String, Value)>, children: Vec<Element>, id: String }` où `component` est `"recharts::LineChart"` ; `Patch::ForeignProps` compare props **et** enfants.
- Consumes: rien de nouveau.

- [x] **Step 1** : côté compilateur — `a_library_component_is_named_by_its_module` (`<js::recharts::LineChart />` → `component: "recharts::LineChart"`), `a_foreign_tag_may_hold_foreign_children`, `a_foreign_tag_may_not_hold_a_rust_tag` (refus nommant la ligne : un nœud que React possède ne contient pas de DOM que rustx possède) ; côté diff — `a_child_of_a_foreign_node_whose_props_changed_is_a_patch`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : le champ, la comparaison, le chemin dans le parseur (`js::` suivi d'un ou plusieurs segments), les enfants dans l'émetteur.
- [x] **Step 4** : vert, plus `cargo test -p rustx-compiler` et `cargo test -p rustx-dom`.
- [x] **Step 5** : commit `feat(compiler): a React component is written with its module and its children`.

---

### Task 5 : Le moteur passe l'arbre, pas seulement les props

**Files:**
- Modify: `crates/rustx-engine/src/foreign.rs`, `crates/rustx-engine/src/render.rs`
- Test: `crates/rustx-engine/tests/foreign.rs`, `crates/rustx-engine/assets/hello/index.html`

**Interfaces:**
- Consumes: `Element::Foreign` avec enfants (Task 4), `object_of` (Task 2).
- Produces: `hand_over(name, node, props, children)` ; la moitié JavaScript reçoit `mount(name, node, props, children)` où `children` est une liste de `{ component, props, children }`.

- [x] **Step 1** : dans le scénario `foreign`, un composant étranger avec un enfant étranger ; `a_foreign_child_is_handed_over_with_its_parent` et `a_childs_props_reach_react_without_the_parent_changing`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : la description de l'arbre comme `Value::Record`, construite une fois par remise.
- [x] **Step 4** : vert, plus `cargo test -p rustx-engine --test foreign`.
- [x] **Step 5** : commit `feat(engine): a boundary hands over a tree`.

---

### Task 6 : Le moteur fournit la moitié JavaScript

**Files:**
- Create: `crates/rustx/assets/runtime.js`
- Modify: `crates/rustx/src/lib.rs`
- Test: `crates/rustx/tests/runtime.rs` (créer)

**Interfaces:**
- Consumes: le contrat de la Task 5.
- Produces: `window.rustx.use({ react, reactDom, ...libraries })` enregistre les modules ; le runtime définit `window.__rustx_react.mount/unmount` à partir d'eux, résout `"recharts::LineChart"` dans `libraries.recharts`, construit l'arbre React récursivement avec `createElement`, garde une racine par nœud remis (`WeakMap`) et rend avec `flushSync`. Un nom introuvable écrit `rustx: the page asked for <Name> in <module>, which does not export it` dans la console et renvoie `false` ; un module absent dit lequel. Le runtime est exécuté par le moteur à la création du contexte, avant les scripts de la page, donc `window.rustx` existe toujours quand le bundle s'exécute.
- `App::new` injecte le runtime via `rustx_engine::on_context_created`, avec `include_str!("../assets/runtime.js")`.

- [x] **Step 1** : `the_runtime_is_in_the_binary_not_on_disk` (le `include_str!` contient `__rustx_react`), `a_missing_module_is_named` et `a_missing_export_is_named` — ces deux-là sur la fonction de résolution, extraite et testée depuis Node (`node --test` sur `crates/rustx/assets/runtime.js`, importable parce que le fichier expose aussi `module.exports` quand il tourne hors navigateur). Si Node n'est pas disponible sur la machine, le test est `#[ignore]` avec la raison écrite dedans.
- [x] **Step 2** : rouge.
- [x] **Step 3** : le runtime et l'injection.
- [x] **Step 4** : vert.
- [x] **Step 5** : commit `feat(rustx): the framework brings its own half of the React bridge`.

---

### Task 7 : `apps/hybrid` utilise recharts depuis sa page

**Files:**
- Modify: `apps/hybrid/app/page.rsx`, `apps/hybrid/web/src/main.jsx`, `apps/hybrid/assets/index.html`, `apps/hybrid/tests/app.rs`
- Test: `cargo test -p hybrid`

**Interfaces:**
- Consumes: tout ce qui précède.

La page écrit le graphique elle-même :

```rsx
<js::recharts::LineChart width={320} height={120} data={points}>
    <js::recharts::XAxis dataKey="at" />
    <js::recharts::Line type="monotone" dataKey="value" stroke="#7f5af0" />
</js::recharts::LineChart>
```

et le seul JavaScript de l'application devient `import * as recharts from "recharts"; rustx.use({ react, reactDom, recharts });`.

- [x] **Step 1** : le scénario lit le `<svg>` que recharts a produit (`#chart svg path` existe) et le nombre de points (`.recharts-line-curve` a un `d` qui change après un clic) ; `the_page_draws_a_library_component_it_wrote_itself`, `a_point_added_in_rust_reaches_the_library`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : la page, le manifeste, le bundle (`npm run build`).
- [x] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [x] **Step 5** : commit `feat(hybrid): the page writes the chart, not the bridge`.

---

## Ce que ce plan livre

Une page `.rsx` qui utilise un composant npm comme une balise, avec ses enfants et ses props, et une application dont tout le JavaScript tient en une ligne : la liste de ses bibliothèques.
