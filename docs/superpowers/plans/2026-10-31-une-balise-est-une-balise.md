# Une balise est une balise

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `<LineChart data={points}>` sans préfixe, `<Dialog.Root>` de Radix stylé par Tailwind, les fonctions publiques du projet appelables depuis un `.rsx`, et plus un seul fichier `.js` ou `.jsx` écrit à la main dans une application.

**Architecture:** Une balise capitalisée que le fichier ne déclare pas est un composant de bibliothèque, résolu à l'exécution parmi les paquets que l'application a installés. Le `package.json` de l'application est sa déclaration de bibliothèques : le framework en génère le point d'entrée JavaScript, le bundle avec esbuild et l'injecte dans la page — le développeur n'écrit que du `.rsx`.

**Tech Stack:** Rust, CEF 156.2.0, esbuild 0.23.1, react 18.3.1, react-dom 18.3.1, recharts 2.12.7, @radix-ui/react-dialog 1.1.1, tailwindcss 3.4.10.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.5 et objectif n°3 (l'écosystème React sans couture).

## Global Constraints

- Aucun `.js`, `.jsx`, `.ts` ou `.tsx` écrit à la main dans une application. Un `package.json` et des `.rsx`, rien d'autre.
- Pas de préfixe `js::` dans une page. `<Name />`, `<Module.Name />`, et `<module::Name />` quand il faut lever une ambiguïté.
- Une balise capitalisée que le fichier **déclare** (`fn Name`) reste un composant rustx : la règle existante ne bouge pas.
- Versions npm exactes : `react@18.3.1`, `react-dom@18.3.1`, `recharts@2.12.7`, `@radix-ui/react-dialog@1.1.1`, `tailwindcss@3.4.10`, `esbuild@0.23.1`.
- Ce que le framework génère ne va jamais dans le dépôt : `.rustx/` est ignoré par git, comme `node_modules/`.

## Review Focus

1. Deux bibliothèques qui exportent le même nom : la page doit apprendre laquelle a été prise et comment choisir, pas obtenir silencieusement l'une des deux.
2. Une application sans `node_modules` : le développeur doit lire « lance `npm install` », pas une erreur d'esbuild introuvable.
3. Une balise capitalisée qui n'est ni déclarée dans le fichier ni exportée par une bibliothèque : la plainte doit nommer les deux endroits où elle a été cherchée.
4. Une application sans `package.json` du tout : rien ne doit être généré, rien ne doit échouer — c'est le cas de quatre des cinq applications.
5. Une classe Tailwind écrite dans un `.rsx` : elle doit se retrouver dans la feuille, donc le scan de Tailwind doit lire les `.rsx`, pas seulement des `.js`.

---

### Task 1 : Une balise capitalisée inconnue est un composant de bibliothèque

**Files:**
- Modify: `crates/rustx-compiler/src/emit.rs`, `crates/rustx-compiler/src/parse.rs`
- Test: `crates/rustx-compiler/tests/foreign.rs`

**Interfaces:**
- Produces: `<LineChart />` → `Element::Foreign { node: ForeignNode { component: "LineChart", … } }` ; `<Dialog.Trigger />` → `component: "Dialog.Trigger"` ; `<recharts::LineChart />` inchangé ; `<Counter />` reste un appel rustx quand le fichier déclare `fn Counter`.

- [x] **Step 1** : `a_capitalised_tag_the_file_does_not_declare_is_a_library_component`, `a_dotted_tag_is_a_path_into_a_library`, `a_tag_the_file_declares_is_still_a_rustx_component`, `a_library_tag_may_hold_children_and_a_rust_tag_inside_it_is_refused`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : l'aiguillage dans `expression`, et le parseur qui accepte `.` dans un nom de balise capitalisée.
- [x] **Step 4** : vert, plus `cargo test -p rustx-compiler`.
- [x] **Step 5** : commit `feat(compiler): a capitalised tag is a library component`.

---

### Task 2 : Le runtime résout un nom parmi les bibliothèques installées

**Files:**
- Modify: `crates/rustx/assets/runtime.js`
- Test: `crates/rustx/tests/runtime.rs`

**Interfaces:**
- Produces: `resolve(name)` cherche, dans l'ordre : les composants enregistrés, puis chaque bibliothèque déclarée, pour un export de ce nom ; `A.B` descend dans les propriétés ; `module::Name` reste explicite. Deux bibliothèques qui exportent le même nom : la première gagne et `console.warn` nomme les deux et la forme explicite.

- [x] **Step 1** : `a_bare_name_is_found_in_a_declared_library`, `a_dotted_name_walks_into_an_export`, `a_name_two_libraries_export_warns_and_names_both`, `a_name_nobody_exports_names_both_places_it_was_looked_for`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : la résolution.
- [x] **Step 4** : vert.
- [x] **Step 5** : commit `feat(rustx): a library component is found by its name`.

---

### Task 3 : Une page voit les fonctions publiques du projet

**Files:**
- Modify: `crates/rustx-macros/src/lib.rs`
- Test: `apps/capture` (dont les pages appellent `Stream::open`), plus `crates/rustx-macros` si un test unitaire s'y prête

**Interfaces:**
- Produces: le module généré ouvre, en plus de `use super::*`, un `use super::<module>::*;` par `mod <module>;` déclaré dans le fichier où `#[rustx::main]` est écrit. Un `mod` privé reste privé au crate, ce qui suffit : la page est dans le même crate.

- [x] **Step 1** : `apps/capture/app/device/[id]/page.rsx` appelle `Stream::open` sans que `main.rs` l'importe ; `apps/capture/src/main.rs` perd sa ligne `use devices::Stream;`.
- [x] **Step 2** : rouge (`cannot find type Stream`).
- [x] **Step 3** : le scan des `mod` et les `use` générés.
- [x] **Step 4** : vert (`cargo test -p capture`).
- [x] **Step 5** : commit `feat(macros): a page sees the modules of its own crate`.

---

### Task 4 : Le framework écrit et construit le JavaScript

**Files:**
- Create: `crates/rustx-macros/src/bundle.rs`
- Modify: `crates/rustx-macros/src/lib.rs`, `crates/rustx/src/lib.rs`, `.gitignore`
- Delete: `apps/hybrid/web/` (le `.jsx` écrit à la main)
- Create: `apps/hybrid/package.json`
- Test: `crates/rustx-macros/tests/bundle.rs`, `cargo test -p hybrid`

**Interfaces:**
- Produces: `pub fn entry_for(dependencies: &[String]) -> String`, le point d'entrée généré : un `import * as <nom> from "<paquet>"` par dépendance, les trois entrées de React, et `window.rustxLibraries = { … }` ; `pub fn build(app: &Path) -> Result<Option<PathBuf>, String>` qui écrit `<app>/.rustx/entry.js`, lance `<app>/node_modules/.bin/esbuild` et rend le chemin du bundle. Pas de `package.json` : `Ok(None)`, et rien n'est généré. `node_modules` absent : `Err` dont le message dit de lancer `npm install`.
- `App::new` injecte le bundle avec `rustx_engine::inject("app-bundle", …)`, donc une page n'a plus de `<script src>` à écrire. Le chemin du bundle est passé par `#[rustx::main]`, qui l'a construit.
- Un nom de paquet n'est pas un identifiant JavaScript (`@radix-ui/react-dialog`) : il est déclaré sous un nom lisible (`radixDialog`), obtenu en camelCase des segments.

- [x] **Step 1** : `an_entry_imports_every_dependency_under_a_usable_name`, `a_scoped_package_gets_a_camel_case_name`, `an_application_with_no_package_json_generates_nothing`, `a_missing_node_modules_says_to_install`.
- [x] **Step 2** : rouge.
- [x] **Step 3** : la génération, l'appel d'esbuild, l'injection.
- [x] **Step 4** : vert, plus `cargo test -p hybrid` avec `apps/hybrid/web` supprimé.
- [x] **Step 5** : commit `feat(rustx): the framework writes the JavaScript it needs`.

---

### Task 5 : Tailwind lit les `.rsx`

**Files:**
- Modify: `crates/rustx-macros/src/bundle.rs`, `crates/rustx/src/lib.rs`
- Create: `apps/hybrid/assets/tailwind.css` (une ligne : les trois `@tailwind`)
- Test: `cargo test -p hybrid`

**Interfaces:**
- Produces: quand `tailwindcss` est une dépendance de l'application, `build` lance aussi `tailwindcss` sur `assets/tailwind.css` avec `--content "app/**/*.rsx"` et écrit `.rustx/tailwind.css` ; `App::new` injecte la feuille dans la page (un `<style>` ajouté au document, par le runtime, à partir du texte compilé dans le binaire).

- [x] **Step 1** : le scénario lit la couleur calculée d'un élément qui porte `className="bg-violet-500"` écrit dans la page ; `a_tailwind_class_written_in_a_page_reaches_the_screen`.
- [x] **Step 2** : rouge (la classe n'existe pas, la couleur est transparente).
- [x] **Step 3** : l'appel à Tailwind et l'injection de la feuille.
- [x] **Step 4** : vert.
- [x] **Step 5** : commit `feat(rustx): Tailwind reads the pages it styles`.

---

### Task 6 : Un composant shadcn, écrit en `.rsx`

**Files:**
- Modify: `apps/hybrid/app/page.rsx`, `apps/hybrid/package.json`, `apps/hybrid/assets/index.html`, `apps/hybrid/tests/app.rs`
- Test: `cargo test -p hybrid`

Ce que shadcn/ui est, c'est Radix plus des classes Tailwind. Le `.rsx` écrit les deux :

```rsx
<radixDialog::Root>
    <radixDialog::Trigger className="rounded-md bg-violet-500 px-3 py-2">ouvrir</radixDialog::Trigger>
    <radixDialog::Portal>
        <radixDialog::Overlay className="fixed inset-0 bg-black/50" />
        <radixDialog::Content className="fixed left-1/2 top-1/2 rounded-lg bg-zinc-900 p-6">
            <radixDialog::Title>Un dialogue</radixDialog::Title>
        </radixDialog::Content>
    </radixDialog::Portal>
</radixDialog::Root>
```

- [x] **Step 1** : `a_radix_dialog_written_in_the_page_opens` — le scénario clique le déclencheur et lit `[role="dialog"]` avec son titre, hors du conteneur (Radix le met dans un portail, donc ailleurs dans le document) ; `the_dialog_is_styled_by_tailwind` lit la couleur de fond du déclencheur.
- [x] **Step 2** : rouge.
- [x] **Step 3** : la page, la dépendance, le bundle.
- [x] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [x] **Step 5** : commit `feat(hybrid): a shadcn component, written in rsx`.

---

## Ce que ce plan livre

`npm install recharts @radix-ui/react-dialog tailwindcss`, puis des balises dans un `.rsx`. Aucun fichier JavaScript dans l'application, aucun préfixe, et les fonctions du projet appelables depuis les pages.
