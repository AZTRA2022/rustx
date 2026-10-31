# L'extension VS Code

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ouvrir un `.rsx` dans VS Code et voir du code coloré, des erreurs soulignées à la bonne ligne pendant qu'on écrit, et les commentaires et les paires de l'éditeur qui marchent comme dans n'importe quel fichier Rust.

**Architecture:** Trois morceaux indépendants. Une grammaire TextMate qui injecte le JSX dans Rust, une configuration de langage, et un serveur de langage en Rust (`rustx-lsp`) qui compile le fichier à chaque frappe et publie ce que le compilateur a refusé — aux coordonnées du `.rsx`, qui sont déjà celles que `CompileError` porte. L'extension ne fait que câbler les trois.

**Tech Stack:** Rust (le serveur, sans dépendance : JSON-RPC sur stdio écrit à la main, comme le reste du projet lit ses tokens), TypeScript compilé par esbuild pour l'extension, `vscode-textmate` pour tester la grammaire.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.7 (l'outillage), objectif n°7.

## Global Constraints

- Le serveur ne dépend d'aucun crate externe : le projet lit déjà des tokens à la main, et un serveur de langage est un en-tête `Content-Length` et du JSON.
- Les positions publiées sont celles du `.rsx`, jamais celles du `.rs` généré : `CompileError` porte déjà `line` et `column`, comptés à partir de 1, et LSP les compte à partir de 0.
- Rien dans l'extension ne compile l'application : ouvrir un fichier ne doit pas lancer `cargo`.
- Ce plan ne fait pas le pont vers `rust-analyzer` — la complétion, le survol, « aller à la définition ». C'est le plan suivant, et il s'appuie sur la carte de correspondance (§7.3) que le compilateur écrit déjà.

## Review Focus

1. Un `.rsx` qui ne compile pas pour une raison que le compilateur ne nomme pas (un panic) : l'éditeur doit rester vivant et ne rien souligner, pas mourir avec le serveur.
2. Un fichier ouvert puis corrigé : le soulignement doit disparaître, donc un fichier sans erreur publie une liste vide plutôt que rien du tout.
3. Un `.rsx` de 5 000 lignes tapé vite : compiler à chaque frappe ne doit pas prendre l'éditeur en otage — ce qui se mesure.
4. Un client qui envoie `shutdown` puis `exit` : le serveur doit s'arrêter, ou VS Code laisse un processus par fenêtre ouverte.
5. Un `Content-Length` coupé en deux par le tube : lire l'en-tête ligne par ligne et le corps à la longueur exacte, sinon le serveur se désynchronise à la première grosse frappe.

---

### Task 1 : Le serveur lit et écrit du LSP

**Files:**
- Create: `crates/rustx-lsp/Cargo.toml`, `crates/rustx-lsp/src/main.rs`, `crates/rustx-lsp/src/rpc.rs`, `crates/rustx-lsp/src/json.rs`
- Test: `crates/rustx-lsp/tests/rpc.rs`
- Modify: `Cargo.toml` (le membre)

**Interfaces:**
- Produces: `rpc::read(&mut impl BufRead) -> Option<String>` (un message, corps seul) ; `rpc::write(&mut impl Write, &str)` (en-tête + corps) ; `json::field(&str, &str) -> Option<&str>` et `json::escape(&str) -> String` — assez de JSON pour lire une méthode, un chemin et un texte, et pour écrire des diagnostics.

- [x] **Step 1** : `a_message_is_read_with_its_length`, `a_header_split_across_reads_is_still_one_message`, `a_string_with_quotes_and_newlines_survives_escaping`, `a_field_of_a_nested_object_is_found`.
- [x] **Step 2** : rouge (`cargo test -p rustx-lsp`).
- [x] **Step 3** : la lecture, l'écriture, le peu de JSON.
- [x] **Step 4** : vert.
- [x] **Step 5** : commit `feat(lsp): the server reads and writes LSP`.

---

### Task 2 : Il publie ce que le compilateur a refusé

**Files:**
- Modify: `crates/rustx-lsp/src/main.rs`
- Create: `crates/rustx-lsp/src/diagnostics.rs`
- Test: `crates/rustx-lsp/tests/session.rs`

**Interfaces:**
- Consumes: `rustx_compiler::transpile::transpile`, `rpc` (Task 1).
- Produces: `diagnostics::of(source: &str) -> Vec<Diagnostic>` où `Diagnostic { line: u32, column: u32, length: u32, message: String }`, et `diagnostics::notification(uri: &str, found: &[Diagnostic]) -> String` qui écrit le `textDocument/publishDiagnostics`. Un panic du compilateur est attrapé et ne publie rien.
- Le serveur répond à `initialize` avec `textDocumentSync: 1` (le texte complet), et publie sur `didOpen` et `didChange`.

- [x] **Step 1** : `a_page_that_does_not_compile_is_reported_at_its_line`, `a_page_that_compiles_publishes_an_empty_list`, `a_compiler_panic_publishes_nothing_and_the_server_lives`, `initialize_answers_with_full_text_sync`, `shutdown_then_exit_stops_the_server`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : la boucle, les diagnostics, l'arrêt.
- [x] **Step 4** : vert, plus un `.rsx` de 5 000 lignes compilé en moins de 50 ms (`a_long_page_is_compiled_in_a_time_a_keystroke_can_hold`).
- [x] **Step 5** : commit `feat(lsp): a page's mistakes are underlined where they were written`.

---

### Task 3 : L'éditeur sait ce qu'est un `.rsx`

**Files:**
- Create: `editors/vscode/package.json`, `editors/vscode/language-configuration.json`, `editors/vscode/syntaxes/rsx.tmLanguage.json`, `editors/vscode/README.md`
- Create: `editors/vscode/test/grammar.test.mjs`

**Interfaces:**
- Produces: le langage `rsx` pour `.rsx` ; une grammaire qui est Rust, plus une injection : à l'intérieur d'un `return <…>;` les balises sont du JSX (`entity.name.tag`), les attributs des attributs, et ce qui est entre `{}` redevient du Rust.
- La configuration : `//` et `/* */`, les paires `() [] {} <>`, l'indentation après une balise ouverte.

- [x] **Step 1** : `vscode-textmate` tokenise un `.rsx` d'exemple ; `a_tag_is_a_tag`, `rust_inside_braces_is_rust`, `a_let_mut_is_a_keyword_everywhere`.
- [x] **Step 2** : rouge (`node --test editors/vscode/test`).
- [x] **Step 3** : la grammaire et la configuration.
- [x] **Step 4** : vert.
- [x] **Step 5** : commit `feat(vscode): an editor knows what a .rsx is`.

---

### Task 4 : L'extension câble le serveur

**Files:**
- Create: `editors/vscode/src/extension.ts`, `editors/vscode/build.mjs`
- Modify: `editors/vscode/package.json`
- Test: `editors/vscode/test/wiring.test.mjs`

**Interfaces:**
- Consumes: le serveur (Task 2), la grammaire (Task 3).
- Produces: une extension qui démarre `rustx-lsp` — trouvé sur le `PATH`, ou nommé par `rustx.server.path` — et parle LSP avec lui ; `vscode-languageclient` est la seule dépendance.
- Ce qu'un test peut vérifier sans VS Code : le `package.json` déclare le langage, l'activation, le réglage, et le bundle se construit.

- [x] **Step 1** : `the_extension_declares_the_language_and_its_server`, `the_bundle_builds`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : l'extension et son build.
- [x] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [x] **Step 5** : commit `feat(vscode): the extension starts the language server`.

---

## Ce que ce plan livre

Un `.rsx` ouvert dans VS Code : coloré, avec ses erreurs soulignées à la bonne ligne pendant la frappe. La complétion et le survol viennent du plan suivant, qui met `rust-analyzer` derrière le même serveur.
