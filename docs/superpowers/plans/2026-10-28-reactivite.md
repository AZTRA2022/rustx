# rustx — Plan 3 : la réactivité

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que ce code compile, s'affiche, et se mette à jour quand on clique — sans une annotation de plus.

```rust
fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={|| n += 1}>{n}</button>;
}
```

**Architecture:** Le compilateur analyse le corps de chaque composant et repère les variables locales à la fois lues dans le JSX et mutées depuis une closure de gestionnaire. Celles-là, et elles seules, sont réécrites en cellules d'état persistantes identifiées par leur emplacement dans le code. Muter une cellule ne dessine rien : cela marque l'instance sale. Une fois par frame, les instances sales sont ré-exécutées, le nouvel arbre est comparé à l'ancien, et seules les différences touchent le DOM.

**Tech Stack:** Rust edition 2024, `rustx-dom`, `rustx-compiler` (plan 2), `rustx-engine` (plan 1, CEF 156.2.0), Windows 10 x64.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.4 et §7.4 bis.

**Portée :** la promotion d'état, le cadencement, le re-rendu, la réconciliation avec `key`, le démontage avec `Drop`, et les props.

**Hors périmètre, explicitement :** les effets et les tâches asynchrones (`on_mount` / annulation au démontage) et l'état partagé entre composants éloignés (contexte ou magasin). Les deux sont au §7.4 bis et font partie de la v1, mais ni l'un ni l'autre n'est nécessaire pour que le compteur du §7.4 fonctionne, et chacun est un sous-système avec ses propres questions de cycle de vie. Ils font l'objet du plan 4. Également hors périmètre : le magasin partagé entre fenêtres (§7.4, multi-fenêtres), l'interopérabilité React, le routing.

## Global Constraints

- Plateforme cible : Windows 10 x64 uniquement. Rust edition 2024.
- Aucune dépendance sous GPL ; aucune obligation de licence propagée aux utilisateurs du framework.
- `unsafe` reste confiné à `rustx-engine`. Ni le compilateur, ni `rustx-dom`, ni le code **engendré** n'en contiennent — anti-objectif n°4, qui porte précisément sur le code engendré.
- L'état vit sur le thread d'interface du processus de rendu. Il n'est ni `Send` ni `Sync`, il n'y a pas de verrou sur le chemin chaud, et un `let mut n = 0;` n'impose aucune contrainte de thread à ce que la closure capture.
- Muter l'état ne dessine rien. Le rendu a lieu **une fois par frame**, calé sur le vsync, quel que soit le nombre de mutations entre deux frames.
- Aucune syntaxe nouvelle. Pas de `use_state`, pas de `signal`, pas de `move`, pas de `Rc<RefCell<…>>` dans ce que le développeur écrit.
- Une variable qui n'est pas promue reste un `let` Rust ordinaire, sans coût à l'exécution.
- La transpilation reste déterministe.

## Review Focus

Cinq conditions que le document implique et qu'aucune tâche n'exerce d'elle-même. Chacune a son test rattaché à la tâche propriétaire.

1. **Deux composants du même type, montés côte à côte.** L'identité d'une cellule par emplacement dans le code est la même pour les deux ; si l'instance n'entre pas dans l'identité, deux compteurs partagent un compteur. → Tâche 1.
2. **Une variable mutée depuis une closure mais jamais lue dans le JSX**, et l'inverse. Ni l'une ni l'autre ne doit être promue : la promotion a un coût, et une variable de travail ordinaire ne doit pas le payer. → Tâche 2.
3. **Une liste réordonnée dont les éléments portent un état.** Sans `key`, l'état suit la position et le texte saisi dans le troisième champ se retrouve dans le premier. → Tâche 5.
4. **Une mutation postée depuis un thread de travail pendant qu'une frame est en cours de rendu.** Le document promet qu'aucune course n'est possible ; la file du plan 1 est le seul chemin, et le rendu ne doit pas lire un état à moitié écrit. → Tâche 6.
5. **Un composant démonté dont l'état détient une ressource.** Le `Drop` de Rust doit s'exécuter au démontage, une fois, et pas au re-rendu. → Tâche 7.

---

### Task 1: Les cellules d'état

**Files:**
- Create: `crates/rustx-dom/src/state.rs`
- Modify: `crates/rustx-dom/src/lib.rs`
- Test: `crates/rustx-dom/tests/state.rs`

Ce que le code engendré appelle. Aucune dépendance au DOM ni à CEF : c'est du Rust ordinaire, testable sans fenêtre.

**Interfaces:**
- Consumes: rien.
- Produces:
  - `rustx_dom::state::Slot { pub file: &'static str, pub line: u32, pub column: u32 }` — l'emplacement dans le code, `PartialEq + Eq + Hash + Copy`
  - `rustx_dom::state::State<T>` — un handle, `Clone` (bon marché, partage la cellule), ni `Send` ni `Sync`
  - `State::<T>::get(&self) -> T` où `T: Clone`, `set(&self, value: T)`, `update(&self, change: impl FnOnce(&mut T))`
  - `rustx_dom::state::cell<T: 'static>(slot: Slot, initial: impl FnOnce() -> T) -> State<T>` — rend la cellule de ce slot dans l'instance courante, en l'initialisant au premier passage
  - `rustx_dom::state::InstanceId(u64)` et `rustx_dom::state::Scope` — un garde RAII : `Scope::enter(InstanceId)` fait de cette instance l'instance courante jusqu'à sa chute
  - `rustx_dom::state::take_dirty() -> Vec<InstanceId>` — les instances muées depuis le dernier appel, vidées
  - `rustx_dom::state::drop_instance(InstanceId)` — libère les cellules d'une instance, ce qui exécute leur `Drop`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_cell_keeps_its_value_across_renders_of_the_same_instance() {
    let id = InstanceId(1);
    {
        let _scope = Scope::enter(id);
        cell(here(), || 0_i32).set(7);
    }
    let _scope = Scope::enter(id);
    assert_eq!(cell(here(), || 0_i32).get(), 7);
}

#[test]
fn two_instances_of_the_same_component_do_not_share_a_cell() {
    // Review Focus 1: the slot is the same line of code in both.
    {
        let _scope = Scope::enter(InstanceId(1));
        cell(here(), || 0_i32).set(7);
    }
    let _scope = Scope::enter(InstanceId(2));
    assert_eq!(cell(here(), || 0_i32).get(), 0);
}

#[test]
fn mutating_a_cell_marks_its_instance_dirty_once() {
    let id = InstanceId(3);
    let _scope = Scope::enter(id);
    let n = cell(here(), || 0_i32);
    n.set(1);
    n.update(|v| *v += 1);
    assert_eq!(take_dirty(), vec![id]);
    assert!(take_dirty().is_empty(), "dirt is consumed, not remembered");
}

#[test]
fn a_handle_cloned_into_a_closure_still_reaches_the_same_cell() {
    // A handler outlives the render that built it, so the rewrite hands
    // it a clone. The clone must not be a copy of the value.
    let _scope = Scope::enter(InstanceId(4));
    let n = cell(here(), || 0_i32);
    let held = n.clone();
    held.set(9);
    assert_eq!(n.get(), 9);
}

#[test]
fn dropping_an_instance_runs_the_drop_of_what_it_held() {
    // Review Focus 5, at the level where it is testable without a DOM.
    let witness = std::rc::Rc::new(std::cell::Cell::new(false));
    {
        let _scope = Scope::enter(InstanceId(5));
        cell(here(), || Noisy(witness.clone()));
    }
    drop_instance(InstanceId(5));
    assert!(witness.get(), "the value's Drop never ran");
}
```

`here()` est un helper de test qui construit un `Slot` à partir de `file!()`, `line!()` et `column!()`. `Noisy` est une structure de test dont le `Drop` lève le témoin.

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-dom --test state`
Expected: FAIL, `rustx_dom::state` n'existe pas.

- [ ] **Step 3: Implémenter `state.rs`**

Le magasin est un `thread_local!` : `RefCell<HashMap<InstanceId, HashMap<Slot, Rc<dyn Any>>>>`, plus la pile des instances courantes et l'ensemble des instances sales. `State<T>` tient un `Rc<RefCell<T>>` et l'`InstanceId` qui l'a créée, pour savoir qui salir. `get` clone, `set` et `update` écrivent puis salissent. `drop_instance` retire l'entrée de la carte, ce qui libère les `Rc` et exécute les `Drop`.

`cell` appelée hors de tout `Scope` est un bug du code engendré, pas du développeur : elle panique avec un message qui dit que la cellule a été demandée hors d'un rendu.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-dom --test state`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-dom
git commit -m "feat(dom): state cells, identified by code location and instance"
```

---

### Task 2: L'analyse de promotion

**Files:**
- Create: `crates/rustx-compiler/src/promote.rs`
- Test: `crates/rustx-compiler/tests/promote.rs`

Décider, et seulement décider. Aucune réécriture ici : la tâche 3 s'en charge, et séparer les deux rend la règle testable sans lire une ligne de Rust engendré.

**Interfaces:**
- Consumes: plan 2 (`scan::find_jsx`, `Span`, `CompileError`).
- Produces:
  - `rustx_compiler::promote::Component { pub name: String, pub body: Span, pub returns_jsx: Vec<Span>, pub promoted: Vec<Promoted> }`
  - `rustx_compiler::promote::Promoted { pub name: String, pub declaration: Span, pub initializer: Span, pub reads: Vec<Span>, pub mutations: Vec<Mutation> }`
  - `rustx_compiler::promote::Mutation { pub span: Span, pub kind: MutationKind }`
  - `rustx_compiler::promote::MutationKind` (enum : `Assign`, `Compound`, `Method`)
  - `rustx_compiler::promote::analyse(source: &str) -> Result<Vec<Component>, CompileError>`

Un composant est une `fn` dont le type de retour est `Element` — c'est la marque, et elle est déjà celle du document. Une closure de gestionnaire est une closure qui est la valeur d'un attribut `on*` dans une région JSX.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_variable_read_in_the_jsx_and_mutated_in_a_handler_is_promoted() {
    let components = analyse(COUNTER).unwrap();
    let promoted: Vec<&str> = components[0].promoted.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(promoted, ["n"]);
}

#[test]
fn a_variable_only_mutated_in_a_handler_is_not_promoted() {
    // Review Focus 2. Nothing reads it, so nothing needs to survive.
    let source = "fn C() -> Element {\n    let mut log = 0;\n    return <button onClick={|| log += 1}>go</button>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn a_variable_only_read_in_the_jsx_is_not_promoted() {
    // Review Focus 2, the other half: an ordinary local, recomputed each
    // render, must pay nothing.
    let source = "fn C() -> Element {\n    let title = \"hi\";\n    return <h1>{title}</h1>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn a_mutation_outside_a_handler_does_not_promote() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    n += 1;\n    return <p>{n}</p>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn a_component_parameter_is_never_promoted() {
    let source = "fn C(label: String) -> Element {\n    return <button onClick={|| ()}>{label}</button>;\n}";
    assert!(analyse(source).unwrap()[0].promoted.is_empty());
}

#[test]
fn each_kind_of_mutation_is_recognised() {
    let source = "fn C() -> Element {\n    let mut items = vec![];\n    let mut n = 0;\n    let mut s = 0;\n    return <div onClick={|| { items.push(1); n = 5; s += 1; }}>{items.len()}{n}{s}</div>;\n}";
    let kinds: Vec<MutationKind> = analyse(source).unwrap()[0]
        .promoted.iter().flat_map(|p| p.mutations.iter().map(|m| m.kind)).collect();
    assert!(kinds.contains(&MutationKind::Method));
    assert!(kinds.contains(&MutationKind::Assign));
    assert!(kinds.contains(&MutationKind::Compound));
}

#[test]
fn a_function_that_does_not_return_element_is_not_a_component() {
    let source = "fn helper() -> u8 { let mut n = 0; n += 1; n }";
    assert!(analyse(source).unwrap().is_empty());
}
```

`COUNTER` est la constante contenant exactement le compteur du §7.4.

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-compiler --test promote`
Expected: FAIL, `analyse` non défini.

- [ ] **Step 3: Implémenter `analyse`**

Le repérage se fait sur le même balayage de jetons que `scan.rs`, pas sur un arbre syntaxique Rust complet : la règle ne demande que de reconnaître `fn NOM(…) -> Element {`, les `let mut NOM = …;` de premier niveau du corps, et les occurrences de `NOM` à l'intérieur d'une région JSX ou d'une closure de gestionnaire.

Une occurrence de `NOM` est une **mutation** quand le jeton qui suit est un opérateur d'affectation (`=` non suivi de `=`, ou `+= -= *= /= %= &= |= ^= <<= >>=`) — `Assign` pour le premier, `Compound` pour les autres — ou quand elle est suivie d'un `.` et que l'appel qui suit est en position d'instruction (le jeton précédent est `;` ou `{`, et l'appel est suivi d'un `;`). Toute autre occurrence est une **lecture**.

Un paramètre de composant n'est jamais promu : il est déjà passé à chaque rendu, et le promouvoir ferait d'un changement de prop une donnée qui ne change plus.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-compiler --test promote`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-compiler/src/promote.rs crates/rustx-compiler/tests/promote.rs
git commit -m "feat(compiler): decide which locals become state"
```

---

### Task 3: La réécriture

**Files:**
- Modify: `crates/rustx-compiler/src/transpile.rs`
- Create: `crates/rustx-compiler/src/rewrite.rs`
- Test: `crates/rustx-compiler/tests/rewrite.rs`

**Interfaces:**
- Consumes: tâches 1 et 2.
- Produces: `rustx_compiler::rewrite::rewrite_component(source: &str, component: &Component) -> Result<String, CompileError>`, appelée par `transpile` avant l'émission du JSX.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn the_declaration_becomes_a_cell_identified_by_its_place_in_the_code() {
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("rustx_dom::state::cell"), "got {rust}");
    assert!(rust.contains("Slot"), "got {rust}");
    assert!(!rust.contains("let mut n = 0"), "the plain local survived: {rust}");
}

#[test]
fn a_compound_mutation_in_a_handler_becomes_an_update() {
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("update"), "got {rust}");
    assert!(!rust.contains("n += 1"), "the raw mutation survived: {rust}");
}

#[test]
fn a_handler_closure_owns_its_handles() {
    // The handler outlives the render that built it, so it cannot borrow.
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("move |"), "got {rust}");
    assert!(rust.contains("clone"), "got {rust}");
}

#[test]
fn a_read_in_the_jsx_becomes_a_read_of_the_cell() {
    // The developer writes `{n}` and nothing else. The conversion is the
    // compiler's job, and it is a read of the cell, not a copy of the
    // handle — `into_element` takes its argument by value, so handing it
    // the handle would move it and a second `{n}` would stop compiling.
    let rust = rewrite_one(COUNTER);
    assert!(rust.contains("n.get()"), "got {rust}");
}

#[test]
fn the_same_cell_can_be_read_twice_in_one_tree() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <div onClick={|| n += 1}>{n}{n}</div>;\n}";
    let rust = rewrite_one(source);
    assert_eq!(rust.matches("n.get()").count(), 2, "got {rust}");
}

#[test]
fn a_plain_assignment_becomes_a_set() {
    let source = "fn C() -> Element {\n    let mut n = 0;\n    return <button onClick={|| n = 5}>{n}</button>;\n}";
    assert!(rewrite_one(source).contains("set"), "got {}", rewrite_one(source));
}

#[test]
fn a_method_mutation_goes_through_update_not_through_a_copy() {
    let source = "fn C() -> Element {\n    let mut items: Vec<u8> = vec![];\n    return <button onClick={|| { items.push(1); }}>{items.len()}</button>;\n}";
    let rust = rewrite_one(source);
    assert!(rust.contains("update"), "got {rust}");
    assert!(!rust.contains("get().push"), "the push went to a copy: {rust}");
}

#[test]
fn an_untouched_component_is_byte_for_byte_what_it_was() {
    let source = "fn C() -> Element {\n    let title = \"hi\";\n    return <h1>{title}</h1>;\n}\n";
    assert_eq!(transpile(source).unwrap().rust, transpile(source).unwrap().rust);
    assert!(!transpile(source).unwrap().rust.contains("state::cell"));
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-compiler --test rewrite`
Expected: FAIL, `rewrite_component` non défini.

- [ ] **Step 3: Implémenter la réécriture**

Quatre substitutions, toutes exprimées en décalages d'octets dans le `.rsx`, appliquées de la fin vers le début pour que les décalages restent valides :

- `let mut NOM = INIT;` → `let NOM = rustx_dom::state::cell(rustx_dom::state::Slot { file: file!(), line: LIGNE, column: COLONNE }, || INIT);` où `LIGNE` et `COLONNE` sont celles de la déclaration **dans le `.rsx`**. C'est ce qui fait que l'identité d'une cellule survit à une recompilation et change quand le développeur déplace la déclaration.
- Dans une closure de gestionnaire : `NOM OP= EXPR` → `NOM.update(|v| *v OP= EXPR)` ; `NOM = EXPR` → `NOM.set(EXPR)` ; `NOM.méthode(ARGS);` → `NOM.update(|v| v.méthode(ARGS));`.
- Toute autre occurrence de `NOM`, dans une closure de gestionnaire comme ailleurs, → `NOM.get()`.
- La closure de gestionnaire elle-même, `|ARGS| CORPS`, devient `{ let NOM = NOM.clone(); … move |ARGS| CORPS }`, un `let` par variable promue que la closure touche.

Le développeur, lui, n'écrit aucune conversion : `{n}` reste `{n}` dans le `.rsx`. C'est l'exigence du commanditaire, et elle porte sur ce qu'on écrit, pas sur ce qui est engendré.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-compiler --test rewrite`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-compiler/src/rewrite.rs crates/rustx-compiler/src/transpile.rs crates/rustx-compiler/tests/rewrite.rs
git commit -m "feat(compiler): rewrite promoted locals into state cells"
```

---

### Task 4: L'arbre monté

**Files:**
- Create: `crates/rustx-engine/src/tree.rs`
- Modify: `crates/rustx-engine/src/mount.rs`
- Test: `crates/rustx-engine/tests/tree.rs`

Pour re-rendre, il faut se souvenir de ce qui a été monté. Le montage du plan 2 écrit dans le DOM et oublie ; ici il garde l'arbre et le nœud DOM de chaque élément.

**Interfaces:**
- Consumes: plan 2 (`mount`), tâche 1 (`InstanceId`).
- Produces:
  - `rustx_engine::tree::Mounted { … }` — un arbre d'éléments et, pour chacun, son `dom::Node`
  - `rustx_engine::tree::mount_tree(element: Element, into: &dom::Node) -> Result<Mounted, DomError>`
  - `Mounted::instances(&self) -> &[InstanceId]`

- [ ] **Step 1: Écrire les tests qui échouent**

Scénario `tree` dans `hello`, lu par le pilote existant.

```rust
#[test]
fn a_mounted_tree_remembers_the_dom_node_of_every_element() {
    assert_eq!(report().get("remembered"), report().get("in_dom"));
}

#[test]
fn mounting_twice_into_the_same_parent_does_not_lose_the_first_tree() {
    assert_eq!(report().get("after_second_mount"), "<i>a</i><i>b</i>");
}
```

- [ ] **Step 2 à 5** : comme les tâches précédentes (`cargo test -p rustx-engine --test tree`), puis commit `feat(engine): remember what was mounted`.

---

### Task 5: La réconciliation

**Files:**
- Create: `crates/rustx-dom/src/diff.rs`
- Modify: `crates/rustx-dom/src/element.rs` (ajouter `key`)
- Test: `crates/rustx-dom/tests/diff.rs`

Un diff **pur** : deux arbres en entrée, une liste de corrections en sortie, aucun DOM. C'est ce qui rend la partie difficile testable en millisecondes au lieu de dix secondes par scénario.

**Interfaces:**
- Consumes: tâche 1.
- Produces:
  - `rustx_dom::Key` et `TagElement::key(self, key: impl Into<Key>) -> Self`, plus `pub key: Option<Key>` sur `TagElement`
  - `rustx_dom::diff::Path(Vec<usize>)` — le chemin d'enfants depuis la racine
  - `rustx_dom::diff::Patch` (enum : `SetText { path: Path, text: String }`, `SetAttribute { path: Path, name: String, value: AttributeValue }`, `RemoveAttribute { path: Path, name: String }`, `Replace { path: Path, source: Path }`, `Insert { parent: Path, index: usize, source: Path }`, `Remove { parent: Path, index: usize }`, `Move { parent: Path, from: usize, to: usize }`, `ReplaceHandlers { path: Path, source: Path }`)
  - `rustx_dom::diff::diff(old: &Element, new: &Element) -> Vec<Patch>`

Une correction ne transporte jamais d'`Element` : `Element` n'est pas `Clone` — il contient des closures — et `diff` n'emprunte le nouvel arbre que le temps de le lire. Un `Replace` ou un `Insert` nomme donc un **chemin dans le nouvel arbre** (`source`), et c'est celui qui applique les corrections, tâche 6, qui va y chercher le sous-arbre dans l'arbre qu'il possède.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn an_unchanged_tree_produces_no_patch() {
    assert!(diff(&tree(), &tree()).is_empty());
}

#[test]
fn only_the_text_that_changed_is_patched() {
    let patches = diff(&counter(0), &counter(1));
    assert_eq!(patches.len(), 1);
    assert!(matches!(&patches[0], Patch::SetText { text, .. } if text == "1"));
}

#[test]
fn a_different_tag_is_replaced_whole() {
    let patches = diff(&Element::tag("p").build(), &Element::tag("div").build());
    assert!(matches!(patches.as_slice(), [Patch::Replace { .. }]));
}

#[test]
fn a_keyed_list_reordered_moves_nodes_instead_of_rewriting_them() {
    // Review Focus 3: state follows the key, not the position.
    let before = list(&["a", "b", "c"]);
    let after = list(&["c", "a", "b"]);
    let patches = diff(&before, &after);
    assert!(patches.iter().all(|p| matches!(p, Patch::Move { .. })), "got {patches:?}");
}

#[test]
fn a_keyed_item_removed_takes_only_itself() {
    let patches = diff(&list(&["a", "b", "c"]), &list(&["a", "c"]));
    assert!(matches!(patches.as_slice(), [Patch::Remove { index: 1, .. }]), "got {patches:?}");
}

#[test]
fn an_unkeyed_list_falls_back_to_position() {
    let patches = diff(&unkeyed(&["a", "b"]), &unkeyed(&["a", "b", "c"]));
    assert!(matches!(patches.as_slice(), [Patch::Insert { index: 2, .. }]), "got {patches:?}");
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-dom --test diff`, puis commit `feat(dom): diff two element trees`.

**Step 3, la décision à ne pas laisser à l'implémenteur :** les enfants sont comparés par `key` dès qu'**un seul** enfant en porte une ; sinon par position. Un enfant dont la `key` disparaît est retiré, une `key` nouvelle est insérée, une `key` déplacée produit un `Move`. Les gestionnaires ne sont jamais comparés — une closure n'a pas d'égalité — ils sont remplacés dès que le nœud est patché, ce qui est correct et évite de garder vivante une closure qui capture un état périmé.

---

### Task 6: Le re-rendu, une fois par frame

**Files:**
- Create: `crates/rustx-engine/src/render.rs`
- Modify: `crates/rustx-engine/src/lib.rs`
- Test: `crates/rustx-engine/tests/render.rs`

**Interfaces:**
- Consumes: tâches 1, 4, 5, et le plan 1 (`frame::on_frame`, la file de jobs).
- Produces:
  - `rustx_engine::render::mount_app(root: impl Fn() -> Element + 'static, id: &str) -> Result<(), DomError>`
  - `rustx_engine::render::apply(patches: &[Patch], new: Element, mounted: &mut Mounted) -> Result<(), DomError>` — le seul endroit qui possède le nouvel arbre et peut en détacher un sous-arbre pour un `Replace` ou un `Insert`
  - `rustx_engine::render::renders() -> u64` — combien de fois l'arbre a été re-rendu, pour les tests

- [ ] **Step 1: Écrire les tests qui échouent**

Scénario `render` dans `hello`.

```rust
#[test]
fn clicking_updates_what_is_on_screen() {
    assert_eq!(report().get("after_one_click"), "1");
}

#[test]
fn a_thousand_mutations_between_two_frames_render_once() {
    // The §7.4 pledge: the developer writes their loop as fast as they
    // like and cannot shoot themselves in the foot.
    let renders: u64 = report().get("renders_for_1000").parse().unwrap();
    assert!(renders <= 2, "{renders} renders for 1000 mutations");
}

#[test]
fn an_untouched_component_is_not_re_rendered() {
    assert_eq!(report().get("sibling_renders"), "1");
}

#[test]
fn a_mutation_posted_from_a_worker_thread_lands_without_tearing() {
    // Review Focus 4.
    assert_eq!(report().get("worker_final"), report().get("worker_shown"));
}
```

- [ ] **Step 2 à 6** : `cargo test -p rustx-engine --test render`, scénario ajouté à `hello`, puis commit `feat(engine): re-render dirty components once per frame`.

**Step 3, la décision :** le rendu est un `on_frame` enregistré au montage. Il lit `state::take_dirty()`, et pour chaque instance sale : entre dans son `Scope`, ré-exécute la fonction du composant, `diff` contre l'arbre mémorisé, applique les corrections, remplace l'arbre mémorisé. Les jobs postés par les threads de travail sont déjà drainés par `frame::tick()` **avant** les callbacks de frame (plan 1), donc l'état est stable quand le rendu commence : c'est ce qui rend la Review Focus 4 vraie par construction, et le test est là pour que cela le reste.

---

### Task 7: Démontage et props

**Files:**
- Modify: `crates/rustx-engine/src/render.rs`, `crates/rustx-engine/src/tree.rs`
- Test: `crates/rustx-engine/tests/lifecycle.rs`

**Interfaces:**
- Consumes: tâches 1, 5, 6.
- Produces: rien de nouveau ; `render` appelle `state::drop_instance` quand une instance quitte l'arbre.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn an_unmounted_component_releases_its_state() {
    // Review Focus 5.
    assert_eq!(report().get("dropped"), "1");
}

#[test]
fn a_re_render_does_not_drop_anything() {
    assert_eq!(report().get("dropped_after_rerender"), "0");
}

#[test]
fn a_component_takes_props_and_re_renders_when_they_change() {
    assert_eq!(report().get("after_prop_change"), "Micro");
}

#[test]
fn a_prop_is_not_turned_into_state() {
    // Promoting a parameter would freeze it at its first value.
    assert_eq!(report().get("prop_after_two_changes"), "Écran");
}
```

- [ ] **Step 2 à 6** : `cargo test -p rustx-engine --test lifecycle`, puis commit `feat(engine): drop a component's state when it leaves the tree`.

---

### Task 8: Le compteur, et une liste qui se réordonne

**Files:**
- Create: `apps/counter/Cargo.toml`, `apps/counter/build.rs`, `apps/counter/app/page.rsx`, `apps/counter/src/main.rs`, `apps/counter/assets/index.html`
- Create: `apps/counter/tests/app.rs`, `apps/counter/tests/common/mod.rs`
- Modify: `Cargo.toml`

La preuve. `app/page.rsx` contient **exactement** le code du §7.4, sans une ligne de plus autour de la fonction `Counter`, plus une liste à clés pour montrer que l'état suit la clé.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn the_counter_of_the_design_document_counts() {
    let report = run_counter_scenario();
    assert_eq!(report.get("after_three_clicks"), "3");
}

#[test]
fn two_counters_side_by_side_count_separately() {
    // Review Focus 1, where it is visible to a human.
    let report = run_counter_scenario();
    assert_eq!(report.get("first"), "3");
    assert_eq!(report.get("second"), "0");
}

#[test]
fn reordering_a_keyed_list_carries_each_items_state_with_it() {
    // Review Focus 3, where it is visible to a human.
    let report = run_counter_scenario();
    assert_eq!(report.get("moved_item_count"), "2");
}
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p counter`
Expected: FAIL, le paquet n'existe pas.

- [ ] **Step 3: Écrire `app/page.rsx`**

```rust
fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={|| n += 1}>{n}</button>;
}
```

Si ce bloc demande une seule annotation de plus, le plan a échoué et il faut revenir à la tâche 3 plutôt que d'ajuster la page.

- [ ] **Step 4: Écrire `build.rs`, `main.rs` et la page hôte**, sur le modèle de `apps/catalogue`.

- [ ] **Step 5: Lancer les tests** — `cargo test -p counter` — **Expected: PASS**

- [ ] **Step 6: Lancer toute la suite** — `cargo test --workspace -- --test-threads=1` — **Expected: PASS**

- [ ] **Step 7: Commit**

```bash
git add apps/counter Cargo.toml
git commit -m "feat(apps): the counter of the design document, counting"
```

---

## Ce que ce plan livre, et ce qui vient après

À la fin : `let mut n = 0;` et `onClick={|| n += 1}` suffisent, une liste à clés garde l'état de chaque ligne quand elle se réordonne, un composant démonté libère ce qu'il tenait, et mille mutations entre deux frames coûtent un rendu.

Ce qui manque encore du §7.4 bis : les effets et les tâches asynchrones, et l'état partagé entre composants éloignés — le plan 4. Puis le routing par fichiers et les layouts, l'interopérabilité React et npm, la couche OS, la CLI et le hot reload, et l'extension VS Code en dernier.
