# La complétion dans le JSX

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Écrire une balise, un attribut, un composant ou une expression dans le `return` d'un `.rsx` et recevoir les mêmes suggestions qu'ailleurs — les balises et les attributs du serveur lui-même, les composants du fichier et des bibliothèques installées, et, dans un `{…}`, les vraies complétions de `rust-analyzer` avec les types.

**Architecture:** Deux moitiés, parce qu'il y a deux sortes de position. Dans le balisage — un nom de balise, un nom d'attribut, une balise fermante — c'est le serveur qui répond, à partir de ce que le compilateur sait déjà : ses listes HTML et SVG, ses attributs et ses événements, les signatures des composants du fichier, et les bibliothèques du `package.json` plus les composants installés dans `components/ui`. Dans un `{…}`, c'est `rust-analyzer` qui répond, et la pièce qui manquait est la correspondance : une région JSX est émise sur **une seule ligne** de Rust, donc une colonne dedans n'a pas d'équivalent — alors le serveur fabrique, pour cette requête-là, un fichier où l'expression sous le curseur est posée à sa ligne et à sa colonne du `.rsx`, entourée du vrai corps de la fonction. Les positions se correspondent au caractère, et `rust-analyzer` voit les types parce que tout ce qui précède est le vrai code.

**Tech Stack:** Rust. `rustx-lsp` (déjà là : `rpc`, `json`, `shadow`, `downstream`, `crate_for_pages`), `rustx-compiler` pour les listes et la carte des positions, et l'extension VS Code pour les caractères déclencheurs. Aucune dépendance nouvelle.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.7 (`rustx-lsp` : « c'est ce qui rend la syntaxe nue viable »), §7.3 (le compilateur garde les positions), objectif n°7.

## Global Constraints

- **Aucune suggestion inventée.** Une position dont le serveur ne sait rien rend une liste vide, jamais une suggestion « d'à côté » : une complétion fausse coûte plus cher qu'une complétion absente.
- **Une frappe n'attend pas.** Ce que le serveur répond lui-même est calculé sur le texte que l'éditeur vient d'envoyer, sans compiler et sans attendre `rust-analyzer` : un test pose une page de mille lignes et exige une réponse sous 50 ms.
- **Le serveur ne meurt pas sur un fichier à moitié tapé**, qui est l'état d'un `.rsx` la plupart du temps : `<div class` sans `=`, une balise jamais fermée, un `{` seul. Chaque cas est un test.
- **Rien ne panique** : `#![deny(clippy::unwrap_used, clippy::expect_used)]` sur `rustx-lsp`, et toute lecture du texte passe par des bornes vérifiées.
- Ce que `rust-analyzer` ne peut pas donner — parce qu'il n'est pas installé, parce que l'application n'a jamais été construite — laisse les balises et les attributs marcher quand même. Les deux moitiés sont indépendantes.

## Review Focus

1. **Une balise à moitié écrite au milieu du fichier** (`<di` suivi de trois cents lignes de JSX valide) : les suggestions sont celles d'un nom de balise, pas celles d'un attribut ni d'une expression. Test Task 1 (`a_half_written_tag_is_a_tag_and_not_an_attribute`).
2. **Un `{` jamais fermé** : le serveur répond quelque chose d'utile pour l'expression commencée et ne tombe pas, alors que le fichier ne compile pas — ce qui est l'état normal pendant la frappe. Test Task 3 (`an_expression_nobody_closed_still_gets_its_types`).
3. **Un attribut sur un composant** — du fichier, ou installé : les paramètres du composant du fichier sont proposés par leur nom, et pour un composant d'une bibliothèque le serveur dit ce qu'il sait (les attributs communs) sans prétendre connaître ses props. Test Task 2 (`a_components_own_parameters_are_the_attributes_offered`).
4. **Une position dans un `style={{ … }}`** : les clés d'un objet de style ne sont pas des attributs de balise, et le serveur ne doit pas proposer `onClick` dans un objet CSS. Test Task 2 (`inside_a_style_object_the_keys_are_css_and_not_attributes`).
5. **Un fichier dont le `rust-analyzer` est absent ou lent** : le balisage se complète quand même, et la requête d'expression rend une liste vide au bout de son délai au lieu de bloquer l'éditeur. Test Task 3 (`without_a_downstream_the_markup_still_completes`).

---

### Task 1 : Les noms de balises, et ce qui est installé

**Files:**
- Create: `crates/rustx-lsp/src/inside.rs` (où se trouve le curseur), `crates/rustx-lsp/src/suggest.rs` (ce qu'on propose)
- Modify: `crates/rustx-lsp/src/lib.rs` (`assist` demande d'abord au serveur lui-même), `crates/rustx-compiler/src/tags.rs` et `src/attributes.rs` (les listes deviennent lisibles)
- Test: `crates/rustx-lsp/tests/suggest.rs`

**Interfaces:**
- Produces: `inside::at(source: &str, line: usize, column: usize) -> Place` où `enum Place { Rust, TagName { typed: String }, ClosingTag { open: String }, AttributeName { tag: String, typed: String }, AttributeValue { tag: String, attribute: String }, StyleKey { typed: String }, Expression { span: (usize, usize) } }` — une seule lecture du texte, et le reste du serveur s'en sert pour savoir qui répond.
- Produces: `suggest::tags(typed: &str, page: &Page) -> Vec<Item>` et `struct Item { pub label: String, pub kind: Kind, pub detail: String, pub insert: Option<String> }` avec `enum Kind { Tag, Component, Attribute, Event, Property }` ; `suggest::json(items: &[Item]) -> String` (la liste LSP, échappée par `json::escape`).
- Produces: `struct Page { pub components: Vec<(String, Vec<String>)>, pub libraries: Vec<String> }` et `suggest::page_of(source: &str, path: &Path) -> Page` — les composants du fichier avec leurs paramètres (par `promote::component_signatures`), et les noms que les bibliothèques exportent : les paquets du `package.json` de l'application et les fichiers de `components/ui`, par leurs noms de fichiers.
- Consumes: `rustx_compiler::tags::html()` et `svg()` (nouveaux, les listes derrière les constantes existantes).
- Une balise fermante s'insère entière : taper `</` propose `</div>` quand c'est un `div` qui est ouvert.

- [ ] **Step 1** : les tests — `a_half_written_tag_is_a_tag_and_not_an_attribute` (Review Focus 1), `every_html_tag_is_offered_and_nothing_else_is` (sur `<di` : `div`, `dialog`, `dir` ; pas `onClick`), `a_component_of_the_file_is_offered_by_its_name`, `an_installed_component_is_offered_by_its_name` (un `components/ui/button.tsx` donne `Button`), `a_closing_tag_closes_what_is_open`, `a_thousand_line_page_answers_in_a_blink` (Global Constraint : sous 50 ms).
- [ ] **Step 2** : rouge (`cargo test -p rustx-lsp --test suggest`).
- [ ] **Step 3** : la lecture de la position, les listes, le JSON.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(lsp): the tags, inside the markup`.

---

### Task 2 : Les attributs, les événements, les props et les clés de style

**Files:**
- Modify: `crates/rustx-lsp/src/suggest.rs`, `crates/rustx-lsp/src/inside.rs`
- Test: `crates/rustx-lsp/tests/suggest.rs`

**Interfaces:**
- Produces: `suggest::attributes(tag: &str, typed: &str, page: &Page) -> Vec<Item>` — pour une balise connue, les attributs et les événements du compilateur (`attributes::html()`, `attributes::events()`, nouveaux de la même façon que les balises) ; pour un composant du fichier, ses paramètres, chacun avec son type en `detail` et `nom={}` en insertion ; pour un composant installé, les attributs communs seulement, et un `detail` qui dit d'où vient la balise.
- Produces: `suggest::style_keys(typed: &str) -> Vec<Item>` — les propriétés CSS en casse JavaScript (`backgroundColor`), que le compilateur connaît déjà pour les traduire.
- Un attribut déjà écrit sur la balise n'est pas proposé une seconde fois.

- [ ] **Step 1** : les tests — `a_components_own_parameters_are_the_attributes_offered` (Review Focus 3), `inside_a_style_object_the_keys_are_css_and_not_attributes` (Review Focus 4), `an_event_is_offered_with_the_shape_of_its_handler` (`onClick={move |_| }`), `an_attribute_already_written_is_not_offered_twice`, `an_installed_component_says_where_it_comes_from`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : les listes, les paramètres, les clés.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(lsp): the attributes, the events and the props`.

---

### Task 3 : Dans un `{…}`, les vraies complétions de `rust-analyzer`

**Files:**
- Create: `crates/rustx-lsp/src/picked.rs`
- Modify: `crates/rustx-lsp/src/lib.rs`, `crates/rustx-lsp/src/shadow.rs`, `crates/rustx-compiler/src/map.rs` (les expressions d'une région)
- Test: `crates/rustx-lsp/tests/picked.rs`, `crates/rustx-compiler/tests/map.rs`

**Interfaces:**
- Produces: `map::Embedded { pub line: usize, pub column: usize, pub length: usize }` et `PositionMap::embedded(&self) -> &[Embedded]` — chaque `{…}` d'une région, aux positions du `.rsx`, relevé par le transpileur qui les connaît déjà (`emit` lit l'expression dans la source).
- Produces: `picked::file(generated: &str, map: &PositionMap, line: usize, column: usize) -> Option<String>` — le fichier à envoyer à `rust-analyzer` pour cette requête : la région qui contient le curseur remplacée par
  ```
  <ce qui précède le `<`>{ let __rustx_picked = (
  <autant de lignes vides qu'il faut>
  <colonne - 1 espaces><l'expression telle quelle>
  ); unreachable!() }
  ```
  de sorte que l'expression se trouve à **sa** ligne et **sa** colonne du `.rsx`, et que tout le reste du fichier — la signature de la fonction, les `let` au-dessus, les autres fonctions — soit le vrai code généré. Les positions se correspondent alors au caractère, dans les deux sens, et il n'y a plus de traduction à faire.
- Modifies: `assist` — pour une `Place::Expression`, le serveur envoie un `didChange` du fichier fabriqué, pose la question à la position telle quelle, et remet le vrai fichier après. Le survol et l'aller-à-la-définition passent par le même chemin.
- Le nombre de lignes du fichier fabriqué est exactement celui du vrai : un fichier plus court décalerait tout ce qui suit, et `rust-analyzer` répondrait sur la mauvaise ligne.

- [ ] **Step 1** : les tests — dans `rustx-compiler` : `every_expression_of_a_region_is_in_the_map` ; dans `rustx-lsp` : `the_expression_under_the_cursor_is_where_it_is_in_the_rsx` (ligne et colonne du fichier fabriqué), `the_made_file_has_as_many_lines_as_the_real_one`, `the_made_file_is_rust_a_compiler_accepts` (il transpile et `syn` le parse), `an_expression_nobody_closed_still_gets_its_types` (Review Focus 2), `without_a_downstream_the_markup_still_completes` (Review Focus 5), `a_completion_in_a_handler_sees_the_pages_own_state` (dans `onClick={move |_| points.|}`, `points` est connu).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le relevé des expressions, la fabrication du fichier, l'aiguillage.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(lsp): real completions inside a brace`.

---

### Task 4 : Ce que l'éditeur doit savoir

**Files:**
- Modify: `editors/vscode/package.json`, `editors/vscode/src/extension.ts`, `crates/rustx-lsp/src/lib.rs` (les caractères déclencheurs de `initialize`), `editors/vscode/README.md`
- Test: `editors/vscode/test/wiring.test.mjs`, `crates/rustx-lsp/tests/session.rs`

**Interfaces:**
- `initialize` annonce `completionProvider.triggerCharacters = ["<", " ", "{", ".", ":", "/"]` — `<` pour une balise, l'espace pour un attribut, `{` et `.` pour une expression, `/` pour une fermante.
- Le `resolveProvider` reste `false` : tout ce qu'un élément porte est déjà dans sa première réponse.
- Le README dit ce qui se complète, et ce qui demande un `cargo build` d'abord (les expressions, parce que `rust-analyzer` lit un projet construit).

- [ ] **Step 1** : les tests — `the_server_promises_the_trigger_characters_the_markup_needs`, `the_extension_asks_for_completion_on_those_characters`, `a_session_completes_a_tag_without_any_downstream` (bout en bout, par le vrai serveur, sans `rust-analyzer`).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : les déclencheurs, l'extension, le README.
- [ ] **Step 4** : vert, plus `cargo test --workspace`, `cargo clippy --workspace --all-targets` et `npm test` dans `editors/vscode`.
- [ ] **Step 5** : commit `feat(vscode): completion where a developer types`.

---

## Ce que ce plan livre

La complétion dans le `return` : les balises, les attributs, les événements, les composants du fichier et ceux installés avec `npm` ou `shadcn add`, les clés d'un objet de style — et, dans un `{…}`, les complétions, les survols et les définitions de `rust-analyzer`, avec les types, à la bonne position. C'est la pièce que la spec appelle « ce qui rend la syntaxe nue viable ».

## Ce que ce plan ne fait pas

- Les props d'un composant venu de npm : elles sont dans un `.d.ts`, et les lire demande un serveur TypeScript derrière celui-ci, comme `rust-analyzer` est derrière pour le Rust. Même forme, autre plan.
- Le renommage et la recherche de références à travers une région.
- La complétion des valeurs d'attributs autres que le style (une couleur, une route, un identifiant d'élément).
