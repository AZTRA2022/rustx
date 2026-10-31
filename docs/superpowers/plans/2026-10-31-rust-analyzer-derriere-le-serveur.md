# `rust-analyzer` derrière le serveur

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dans un `.rsx`, compléter un nom, survoler un type, aller à une définition — avec les réponses de `rust-analyzer`, aux positions du fichier qu'on a écrit.

**Architecture:** `rustx-lsp` devient un intermédiaire. Il garde ce qu'il fait déjà (compiler à chaque frappe, souligner ce que le compilateur refuse) et, pour tout ce qu'il ne sait pas faire, il parle à un `rust-analyzer` qu'il a lancé lui-même. Entre les deux il traduit : le `.rsx` que l'éditeur tient devient le `.rs` que le compilateur produit, la position du curseur passe par la carte de correspondance du §7.3, et ce que `rust-analyzer` répond repasse par la carte en sens inverse. L'éditeur ne voit qu'un serveur ; `rust-analyzer` ne voit qu'un `.rs`.

**Tech Stack:** Rust, toujours sans dépendance : le même JSON-RPC écrit à la main qu'au plan précédent, cette fois dans les deux sens. `rust-analyzer` est un processus enfant, trouvé sur le `PATH` ou nommé par un réglage.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.7 (l'outillage) et §7.3 (la carte de correspondance), objectif n°7.

## Global Constraints

- La carte est la seule traduction de positions. Rien ne devine une ligne : ce qui n'est pas dans la carte est signalé comme tel et la requête est abandonnée plutôt que répondue à côté.
- `rust-analyzer` absent n'est pas une erreur : les diagnostics du plan précédent continuent, et la complétion n'existe simplement pas. Aucune boîte de dialogue, une ligne sur le canal de sortie.
- Ouvrir un `.rsx` ne lance toujours pas `cargo`. C'est `rust-analyzer` qui décide de ce qu'il construit, comme pour n'importe quel projet Rust.
- Le serveur reste le seul interlocuteur de l'éditeur : il ne relaie jamais une requête telle quelle, il répond toujours lui-même, avec un identifiant à lui.
- Une frappe ne doit jamais attendre `rust-analyzer` : les diagnostics du compilateur partent avant, et une réponse qui n'arrive pas dans le temps imparti est une réponse vide.

## Review Focus

1. Un curseur **à l'intérieur d'une balise** : la région JSX tient plusieurs lignes du `.rsx` et une seule du `.rs`, donc la colonne ne veut plus rien dire. La requête doit rendre une réponse vide, pas une réponse fausse.
2. `rust-analyzer` qui meurt en pleine session : le serveur le relance une fois, et continue à souligner entre-temps.
3. Une définition qui tombe dans le `.rs` généré : l'éditeur ouvrirait un fichier de `%TEMP%` que le développeur n'a jamais écrit. Elle doit revenir sur le `.rsx`.
4. Une définition qui tombe dans une autre caisse (`rustx-dom`, `std`) : elle passe telle quelle, c'est un vrai fichier Rust.
5. Deux requêtes en vol en même temps — l'éditeur n'attend pas — et une réponse qui arrive après que le document a changé : chaque réponse doit retrouver sa question, ou être jetée.

---

### Task 1 : La carte connaît les colonnes

**Files:**
- Modify: `crates/rustx-compiler/src/map.rs`, `crates/rustx-compiler/src/transpile.rs` (ce qui remplit la carte), `crates/rustx-compiler/src/rewrite.rs` (ce que chaque édition déplace)
- Test: `crates/rustx-compiler/tests/map.rs`

**Interfaces:**
- Produces: `PositionMap::to_generated_position(line: usize, column: usize) -> Option<(usize, usize)>` et `PositionMap::to_rsx_position(line: usize, column: usize) -> Option<(usize, usize)>`, en coordonnées à partir de 1. `None` veut dire « cette position n'a pas d'équivalent » — à l'intérieur d'une région JSX, ou sur une ligne que la réécriture a reconstruite. Une ligne que rien n'a touchée garde ses colonnes, et c'est le cas courant.
- Le texte de la carte gagne les colonnes sans casser ce qui la lit : `generated:rsx` reste une ligne valide, `generated:rsx:from:to` est la forme longue, et `from_text` lit les deux.

- [ ] **Step 1** : les tests — `a_line_nothing_touched_keeps_its_columns`, `a_position_inside_a_jsx_region_has_no_equivalent`, `a_rewritten_line_has_no_equivalent`, `a_map_written_and_read_back_is_the_same_map`, `an_old_map_without_columns_still_reads`.
- [ ] **Step 2** : rouge (`cargo test -p rustx-compiler --test map`).
- [ ] **Step 3** : les deux conversions et la forme longue du texte.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(compiler): the position map knows about columns`.

---

### Task 2 : Le serveur parle à un enfant

**Files:**
- Create: `crates/rustx-lsp/src/downstream.rs`
- Modify: `crates/rustx-lsp/src/lib.rs`
- Test: `crates/rustx-lsp/tests/downstream.rs`

**Interfaces:**
- Produces: `downstream::Server` — `start(command: &str) -> Option<Server>` (lance le processus, écrit `initialize`, attend sa réponse), `ask(&mut self, method: &str, params: &str) -> Option<String>` (envoie avec un identifiant à lui, lit jusqu'à la réponse qui porte cet identifiant, jette les notifications), `tell(&mut self, method: &str, params: &str)`, `restart(&mut self)`. `None` partout veut dire « pas de serveur en face », jamais une erreur remontée à l'éditeur.
- Consumes: `rpc::read`, `rpc::write`, `json` (plan précédent).
- Le test n'a pas besoin de `rust-analyzer` : il lance le binaire du test lui-même avec un argument qui le fait répondre en serveur de langage minimal.

- [ ] **Step 1** : les tests — `a_child_that_answers_initialize_is_held`, `a_question_finds_its_own_answer_among_notifications`, `a_child_that_is_not_there_is_no_server` (une commande qui n'existe pas rend `None`), `a_child_that_dies_is_started_again_once` (Review Focus 2), `an_answer_that_never_comes_is_an_empty_answer` (une échéance, Review Focus 5).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le processus, les identifiants, l'échéance, le redémarrage.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(lsp): the server holds a language server of its own`.

---

### Task 3 : Le `.rsx` que l'éditeur tient devient le `.rs` que Rust voit

**Files:**
- Create: `crates/rustx-lsp/src/shadow.rs`
- Modify: `crates/rustx-lsp/src/lib.rs`
- Test: `crates/rustx-lsp/tests/shadow.rs`

**Interfaces:**
- Produces: `shadow::Shadow { uri: String, rust: String, map: PositionMap }` et `shadow::of(uri: &str, source: &str) -> Option<Shadow>` — compile le texte, écrit le `.rs` à côté de celui que la construction écrirait déjà (le dossier des pages générées, nommé d'après le `.rsx`), et garde la carte. Le `.rs` est écrit sur le disque parce que `rust-analyzer` lit des fichiers, et il est tenu à jour par un `didChange` sur l'URI du `.rs`.
- Consumes: `diagnostics` (plan précédent), `PositionMap` (Task 1).

- [ ] **Step 1** : les tests — `a_page_becomes_a_rust_file_next_to_the_generated_ones`, `a_change_rewrites_the_shadow`, `a_page_that_does_not_compile_leaves_the_last_shadow_alone` (sinon la complétion s'éteint à chaque demi-balise tapée), `the_shadow_of_a_page_is_the_same_file_twice` (déterminisme, §7.3).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : l'ombre et son écriture.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(lsp): a page has a shadow Rust file`.

---

### Task 4 : Compléter, survoler, aller à la définition

**Files:**
- Modify: `crates/rustx-lsp/src/lib.rs`, `crates/rustx-lsp/src/diagnostics.rs` (rien d'autre ne bouge)
- Test: `crates/rustx-lsp/tests/assistance.rs`

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: la réponse à `textDocument/completion`, `textDocument/hover` et `textDocument/definition` pour un `.rsx` ; `initialize` annonce `completionProvider`, `hoverProvider` et `definitionProvider` **seulement** si un serveur en face a répondu. Une position sans équivalent rend une réponse vide. Un `location` qui pointe sur l'ombre repasse par la carte et revient sur le `.rsx` ; un `location` ailleurs passe tel quel.

- [ ] **Step 1** : les tests — `a_completion_inside_a_component_body_is_asked_of_the_child`, `a_completion_inside_a_tag_answers_nothing` (Review Focus 1), `a_hover_comes_back_with_the_type`, `a_definition_in_the_shadow_comes_back_on_the_rsx` (Review Focus 3), `a_definition_in_another_crate_passes_through` (Review Focus 4), `capabilities_do_not_promise_what_there_is_no_server_for`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : les trois méthodes et la traduction dans les deux sens.
- [ ] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [ ] **Step 5** : commit `feat(lsp): completion, hovers and definitions in a .rsx`.

---

### Task 5 : L'extension et ce qu'un développeur lit

**Files:**
- Modify: `editors/vscode/package.json` (le réglage `rustx.rustAnalyzer.path`), `editors/vscode/README.md`
- Test: `editors/vscode/test/wiring.test.mjs`

**Interfaces:**
- Consumes: Task 4.
- Produces: le réglage qui nomme `rust-analyzer` quand il n'est pas sur le `PATH`, passé au serveur par son environnement ; et le paragraphe du README qui dit ce qui marche, ce qui ne marche pas dans une balise, et pourquoi.

- [ ] **Step 1** : `the_extension_declares_where_rust_analyzer_is`.
- [ ] **Step 2** : rouge (`npm test` dans `editors/vscode`).
- [ ] **Step 3** : le réglage et le README.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(vscode): say where rust-analyzer is`.

---

## Ce que ce plan livre

Un `.rsx` où la complétion, le survol et « aller à la définition » répondent comme dans un `.rs`, partout où une position veut encore dire quelque chose — c'est-à-dire dans le code, qui est là où on en a besoin. Dans une balise, la réponse est vide et assumée : la colonne d'une région JSX n'a pas d'équivalent, et une réponse fausse serait pire que pas de réponse.
