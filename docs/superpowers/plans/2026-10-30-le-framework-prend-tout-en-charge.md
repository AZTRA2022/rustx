# Le framework prend tout en charge — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Installer le crate `rustx` et écrire douze lignes dans `main.rs` suffit : ni `build.rs`, ni `include!`, ni `store::<T>()`, ni composant écrit à la main en `Element::tag`, ni module d'échafaudage de test.

**Architecture:** Ce que l'application écrivait est déplacé d'un cran : le `build.rs` entre dans `#[rustx::main]`, qui compile `app/**/*.rsx` au moment de l'expansion ; la déclaration d'un état partagé devient un `let mut` au sommet d'un `.rsx`, promu par le compilateur comme un `let mut` de composant l'est déjà ; les composants que React monte sont enregistrés par le compilateur, pas par l'application ; et la surface que le harnais de test pilote (`rustx_scenario`, `rustx_report`, `rustx_shared`, `rustx_release`) devient celle du moteur.

**Tech Stack:** Rust 2024, proc-macro (`rustx-macros`), `rustx-compiler`, CEF 156.2.0.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §154 (« Pas de `use_state`, pas de `signal`. On écrit `let mut n = 0;`, le compilateur fait le reste »), §7.4 bis (état partagé), §7.3 (transpilation déterministe), §7.6 (CLI et outillage).

## Global Constraints

- Ce que `main.rs` contient, en entier : les attributs de lint, `#[rustx::main]`, `App::new(Config::new(…).size(…))?`, `app.run()`.
- Aucune application n'a de `build.rs`.
- Un changement dans un `.rsx` doit recompiler l'application : le code généré nomme chaque source par `include_str!`, qui est ce que `rustc` et cargo suivent.
- Le nombre de lignes du `.rs` généré reste celui du `.rsx` (§7.3).
- Rien de ce qui est ajouté n'est magique : chaque ligne que l'attribut écrit est du Rust qu'un développeur aurait pu écrire.

## Review Focus

1. Un `let mut` au sommet d'un `.rsx` lu par deux composants : les deux doivent voir la même valeur, et une écriture doit re-rendre les deux.
2. Un `let mut` partagé dont le nom est aussi celui d'un local de composant : le local gagne, et rien ne s'écrit dans le partagé par accident.
3. Un `.rsx` modifié après une compilation réussie : `cargo build` doit le recompiler, sans quoi le développeur débogue du code qui n'existe plus.
4. Une faute dans un `.rsx` sans `build.rs` : la plainte doit nommer le fichier, la ligne et la colonne du `.rsx`, pas une ligne d'expansion de macro.
5. Deux applications du même atelier compilées en parallèle : le compilateur tourne maintenant dans le processus de `rustc`, et deux expansions simultanées ne doivent pas se marcher dessus.

---

### Task 1 : Un état partagé avec un initialisateur

**Files:**
- Modify: `crates/rustx-dom/src/store.rs`
- Test: `crates/rustx-dom/tests/store.rs`

**Interfaces:**
- Produces: `rustx_dom::store::shared::<K, T>(initial: impl FnOnce() -> T) -> Shared<T>` avec `get`/`with`/`set`/`update`, où `K` est un type-marqueur que le compilateur fabrique par déclaration. `store::<T>()` devient `shared::<T, T>(T::default)` et ne change pas de comportement.

- [ ] **Step 1 : les tests qui échouent** — `a_shared_value_is_built_once_and_found_again`, `two_markers_are_two_values_even_of_the_same_type`, `writing_a_shared_value_marks_everyone_who_read_it`, `a_store_is_a_shared_value_whose_marker_is_its_type`.
- [ ] **Step 2** : `cargo test -p rustx-dom --test store` → échec de compilation sur `shared` absent.
- [ ] **Step 3** : la registry prend la clé en paramètre ; `Entry` ne change pas.
- [ ] **Step 4** : vert. **Step 5** : commit `feat(dom): a shared value with an initializer, named by a marker`.

---

### Task 2 : `let mut` au sommet d'un `.rsx`

**Files:**
- Modify: `crates/rustx-compiler/src/promote.rs`, `crates/rustx-compiler/src/rewrite.rs`
- Test: `crates/rustx-compiler/tests/shared.rs`

**Interfaces:**
- Consumes: tâche 1.
- Produces: `promote::shared_of(source) -> Vec<SharedDeclaration { name, declaration: Span, initializer: Span }>`.

La déclaration devient un type-marqueur ; chaque composant qui s'en sert reçoit, au début de son corps, `let <nom> = rustx_dom::store::shared::<__shared_<nom>, _>(|| <init>);`. Les lectures et les mutations sont ensuite réécrites par le chemin qui existe déjà pour un `let mut` de composant — c'est tout l'intérêt : un état partagé est une cellule dont le rangement est la registry.

- [ ] **Step 1 : les tests qui échouent** — `a_let_mut_at_the_top_of_a_file_is_shared`, `two_components_see_one_shared_value`, `a_local_of_the_same_name_wins_inside_its_component` (Review Focus 2), `a_shared_write_in_a_handler_becomes_an_update`, `the_file_keeps_its_line_count`.
- [ ] **Step 2** : `cargo test -p rustx-compiler --test shared` → rouge.
- [ ] **Step 3 à 5** : implémenter, vert, commit `feat(compiler): a let mut at the top of a page is shared by the application`.

---

### Task 3 : Les composants s'enregistrent eux-mêmes

**Files:**
- Modify: `crates/rustx-compiler/src/routes.rs`, `crates/rustx/src/lib.rs`
- Test: `crates/rustx-compiler/tests/routes.rs`

**Interfaces:**
- Produces: dans le fichier généré, `pub fn component_table() -> &'static [(&'static str, fn() -> rustx_dom::Element)]` — tous les composants sans paramètre du dossier `app/`. `App::new` les donne à `foreign::register_component`.

- [ ] **Step 1** : `every_component_without_parameters_is_in_the_table`, `a_component_with_parameters_is_not` (React n'a pas de quoi les remplir).
- [ ] **Step 2 à 5** : rouge, implémenter, vert, commit `feat(compiler): React may mount any component a page declares`.

---

### Task 4 : Plus de `build.rs`

**Files:**
- Modify: `crates/rustx-macros/src/lib.rs`, `crates/rustx-macros/Cargo.toml`
- Delete: `crates/rustx-build/`, `apps/*/build.rs`
- Test: `crates/rustx-macros/tests/expand.rs`, et les applications elles-mêmes

**Interfaces:**
- Consumes: tâches 2 et 3.
- Produces: `#[rustx::main]` compile `app/` au moment de l'expansion, émet le Rust généré en place, et émet un `include_str!` par source pour que cargo suive les fichiers.

- [ ] **Step 1** : `a_page_is_compiled_without_a_build_script`, `a_mistake_in_a_page_names_the_file_and_the_line` (Review Focus 4), `every_source_is_named_so_cargo_follows_it` (Review Focus 3).
- [ ] **Step 2 à 5** : rouge, implémenter, vert, commit `feat(rustx): no build script — the attribute compiles the pages`.

---

### Task 5 : La surface de test est celle du moteur

**Files:**
- Modify: `crates/rustx-engine/src/renderer/mod.rs`, `crates/rustx/src/lib.rs`
- Delete: `apps/*/src/scenario.rs`
- Test: les scénarios des cinq applications

**Interfaces:**
- Produces: natives installées par le moteur — `rustx_scenario()`, `rustx_report(key, value)` (lisant `--scenario=` et `--report=`), `rustx_shared(name)`, `rustx_shared_set(name, value)`, `rustx_release(id)` (déjà là).

Un état partagé a un nom, donc le harnais peut le lire et l'écrire par son nom : c'est ce qui remplace `capture_ticks`, `hybrid_points`, `catalogue_clicked` et `counter_rotate`.

- [ ] **Step 1** : `a_scenario_reads_a_shared_value_by_its_name`, `the_report_native_is_there_without_an_application_registering_it`.
- [ ] **Step 2 à 6** : rouge, implémenter, réécrire les scénarios des applications, vert, commit `feat(engine): the test surface belongs to the framework`.

---

### Task 6 : Les applications, dans leur forme finale

**Files:**
- Modify: `apps/*/src/main.rs`, `apps/*/app/*.rsx`, `apps/*/assets/index.html`, `apps/*/Cargo.toml`
- Delete: `apps/*/src/{devices,chart,rows,scenario}.rs`
- Test: `cargo test --workspace -- --test-threads=1`

Chaque application devient routée (un seul hôte, `#app`), ses composants écrits à la main passent en `.rsx`, son état partagé devient un `let mut` au sommet de sa page, et son `main.rs` est exactement l'extrait de la consigne. Un document par défaut est servi quand il n'y a pas d'`assets/index.html`, pour qu'une application neuve n'ait rien à écrire du tout.

- [ ] **Step 1 à 6** : par application, scénario d'abord, puis la page, puis `main.rs` ; `cargo test -p <app>` à chaque fois ; commit `refactor(apps): an application is its pages and nothing else`.

---

## Ce que ce plan livre

`cargo add rustx`, un `app/page.rsx`, et un `main.rs` de douze lignes qui ne nomme que le titre et la taille de la fenêtre.
