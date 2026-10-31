# Le CLI et le rechargement

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rustx new mon-app`, `rustx dev`, `rustx build` — créer un projet qui tient debout du premier coup, écrire une page et la voir à l'écran sans fermer la fenêtre, et produire un `.exe` qui porte son icône et son nom dans l'explorateur.

**Architecture:** Un binaire, `rustx`, qui ne réinvente rien : la construction est `cargo` et la compilation des pages est déjà celle du macro d'attribut. Ce que le CLI ajoute, c'est ce que cargo ne sait pas : un squelette de projet juste, une boucle qui regarde les fichiers et relance, l'état de l'application qui traverse ce relancement, et les ressources Windows tamponnées dans le `.exe` après le lien — par `UpdateResourceW`, donc sans `build.rs` et sans dépendance, parce qu'un `build.rs` dans le projet d'un développeur est précisément ce que ce framework refuse.

**Tech Stack:** Rust. Aucune dépendance nouvelle : les arguments sont lus à la main comme le reste du projet lit ses tokens, la surveillance des fichiers est une comparaison de dates de modification, et les ressources passent par les mêmes liaisons Windows que l'icône de fenêtre.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.7 (`rustx-cli`), §1 (le nom du binaire), et la ligne de risque sur le rechargement à chaud.

## Global Constraints

- Le projet créé ne contient ni `build.rs`, ni plomberie dans `main.rs` : `App::new(Config::new(…))?` et `app.run()`, rien de plus.
- Le CLI n'a aucune dépendance externe, comme le reste du projet.
- `rustx build` ne remplace pas cargo : il l'appelle, puis tamponne le `.exe` qu'il a produit. Un développeur qui préfère `cargo build` obtient un binaire qui marche, juste sans icône de fichier.
- **Ce que ce plan ne fait pas** : recharger du code natif en gardant l'état vivant en mémoire. C'est le sous-système que la spec signale comme tel. Ce qu'il fait : relancer vite et faire traverser l'état *partagé* et la page courante, de sorte que l'écran revienne où il était. La limite est dite, dans le `--help` et dans la documentation.
- L'état qui traverse est celui que la page déclare en haut avec un `let mut` : il porte un nom, et un nom est ce qu'on retrouve après une recompilation. Les cellules d'un composant repartent à neuf, parce que leur identité est un emplacement dans un code qui vient de changer.

## Review Focus

1. Un `rustx dev` lancé dans un projet qui ne compile pas : la fenêtre précédente doit rester à l'écran avec l'erreur lisible, pas disparaître à chaque faute de frappe.
2. Dix sauvegardes en deux secondes (ce que fait un éditeur qui formate en enregistrant) : une seule reconstruction, pas dix.
3. Un `rustx new` dans un répertoire qui existe déjà, ou avec un nom que cargo refuse (`mon app`, `rustx`) : refusé en une phrase, avant d'avoir écrit un seul fichier.
4. Un `rustx build` sur un projet sans icône : le `.exe` est produit quand même, sans ressource et sans avertissement effrayant.
5. Un état partagé dont le *type* a changé entre deux compilations (`let mut n = 0;` devenu `let mut n = "0";`) : la valeur conservée ne doit pas être réinjectée de force — elle est jetée, et la page repart de son initialisateur.

---

### Task 1 : `rustx new` écrit un projet qui compile

**Files:**
- Create: `crates/rustx-cli/Cargo.toml`, `crates/rustx-cli/src/main.rs`, `crates/rustx-cli/src/arguments.rs`, `crates/rustx-cli/src/new.rs`
- Modify: `Cargo.toml` (le membre)
- Test: `crates/rustx-cli/tests/new.rs`

**Interfaces:**
- Produces: `arguments::read(&[String]) -> Command` où `Command` est `New { name: String }`, `Dev`, `Build { release: bool }`, `Run`, `Help`, `Unknown(String)` ; `new::write(name: &str, into: &Path) -> Result<Vec<PathBuf>, String>` qui écrit `Cargo.toml`, `src/main.rs`, `app/page.rsx`, `assets/` et `.gitignore`, et rend ce qu'il a écrit.
- Le `main.rs` écrit est celui du document de conception : l'attribut, `App::new(Config::new("Nom").size(…))?`, `app.run()`.

- [ ] **Step 1** : les tests — `a_new_project_has_a_page_and_no_build_script`, `the_main_it_writes_is_four_lines_long`, `a_name_cargo_would_refuse_is_refused_first` (Review Focus 3 : rien n'est écrit), `a_directory_that_exists_is_not_overwritten`, `the_project_it_writes_resolves` (`cargo metadata` sur le projet écrit).
- [ ] **Step 2** : rouge (`cargo test -p rustx-cli`).
- [ ] **Step 3** : la lecture des arguments, le squelette.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(cli): a new project that compiles the first time`.

---

### Task 2 : `rustx build` donne au `.exe` son icône et son nom

**Files:**
- Create: `crates/rustx-cli/src/build.rs`, `crates/rustx-cli/src/resources.rs`
- Test: `crates/rustx-cli/tests/build.rs`

**Interfaces:**
- Produces: `build::run(release: bool, project: &Path) -> Result<PathBuf, String>` — appelle `cargo build`, trouve le `.exe`, puis `resources::stamp(exe, &About)` ; `resources::stamp` écrit le groupe d'icônes (`RT_GROUP_ICON` + `RT_ICON`) et la `VS_VERSIONINFO` par `BeginUpdateResourceW` / `UpdateResourceW` / `EndUpdateResource`. L'icône et les métadonnées sont lues dans le `Config` du projet — le CLI les prend là où le développeur les a déjà écrites, pas dans un second fichier.
- Consumes: le lecteur de `.ico` de `rustx-engine::window` (le même en-tête, déjà validé).

- [ ] **Step 1** : les tests — `an_icon_becomes_a_resource_of_the_exe` (relue par `FindResourceW` sur le fichier produit), `the_version_info_carries_the_name_and_the_version`, `a_project_without_an_icon_still_builds` (Review Focus 4), `a_file_that_is_not_an_ico_is_refused_in_words`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : l'appel à cargo, le tamponnage.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(cli): a built .exe knows its own name`.

---

### Task 3 : `rustx dev` regarde, reconstruit, relance

**Files:**
- Create: `crates/rustx-cli/src/dev.rs`, `crates/rustx-cli/src/watch.rs`
- Test: `crates/rustx-cli/tests/dev.rs`

**Interfaces:**
- Produces: `watch::Watched::new(roots: &[PathBuf], extensions: &[&str])` et `Watched::changed(&mut self) -> bool` — dates de modification, comparées ; `dev::once(project: &Path, build: &mut impl FnMut() -> Result<PathBuf, String>, run: &mut impl FnMut(&Path)) -> Outcome` qui construit, relance, et dit ce qui s'est passé. La boucle est à part de ce qu'elle lance, pour qu'un test n'ait pas à compiler quoi que ce soit.
- La reconstruction est groupée : rien ne part avant 200 ms sans nouveau changement (Review Focus 2).
- Un échec de construction garde le processus précédent en vie et imprime l'erreur du compilateur telle quelle, aux coordonnées du `.rsx` (Review Focus 1).

- [ ] **Step 1** : les tests — `a_saved_page_is_rebuilt`, `ten_saves_in_a_breath_are_one_rebuild`, `a_build_that_fails_leaves_the_window_alone`, `a_file_that_is_not_a_page_or_a_source_is_ignored` (un `.log` dans `app/`).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la surveillance, la boucle, le groupement.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(cli): a saved page comes back on screen`.

---

### Task 4 : L'état partagé traverse le rechargement

**Files:**
- Create: `crates/rustx-dom/src/carried.rs`
- Modify: `crates/rustx-dom/src/store.rs` (le relevé et la réinjection), `crates/rustx-engine/src/lib.rs` (l'écriture à la fermeture, la lecture au démarrage), `crates/rustx-cli/src/dev.rs` (le passage du fichier)
- Test: `crates/rustx-dom/tests/carried.rs`, `apps/counter/tests/app.rs`

**Interfaces:**
- Produces: `carried::written(path: &Path)` et `carried::read(path: &Path)` — les valeurs partagées par nom, et la page courante, en une ligne par valeur (`nom=json`), écrites à la fermeture de la fenêtre et relues au démarrage quand `RUSTX_CARRIED` nomme un fichier. `store::shared` demande d'abord ce qui a été porté, et retombe sur son initialisateur quand le nom est absent **ou quand la valeur n'a pas la forme attendue** (Review Focus 5).
- Consumes: `Value` et `ToValue` (déjà là).

- [ ] **Step 1** : les tests — `a_shared_value_comes_back_after_a_restart`, `a_value_whose_shape_changed_is_thrown_away`, `a_component_cell_starts_fresh` (la limite, écrite noir sur blanc), `the_page_that_was_open_is_the_page_that_opens`, et dans `counter` : `the_count_survives_a_restart` par le scénario.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le relevé, l'écriture, la réinjection.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(cli): the application comes back where it was`.

---

### Task 5 : Ce qu'un développeur tape et lit

**Files:**
- Modify: `crates/rustx-cli/src/main.rs` (`--help`, `rustx run`), `README.md`
- Test: `crates/rustx-cli/tests/help.rs`

**Interfaces:**
- Produces: `rustx` sans argument et `rustx --help` écrivent les quatre commandes, une ligne chacune, et la phrase qui dit ce que `dev` conserve et ce qu'il ne conserve pas ; `rustx run` construit et lance une fois, sans surveiller ; une commande inconnue propose la plus proche.
- Le README gagne la section « démarrer », avec les trois commandes et le `main.rs` de quatre lignes.

- [ ] **Step 1** : les tests — `help_names_every_command`, `help_says_what_a_reload_keeps`, `an_unknown_command_suggests_the_nearest`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : l'aide, `run`, le README.
- [ ] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [ ] **Step 5** : commit `feat(cli): what a developer types, and what it says back`.

---

## Ce que ce plan livre

`rustx new`, `rustx dev`, `rustx build`, `rustx run`. Un projet qui compile du premier coup sans `build.rs` ni plomberie, une page sauvegardée qui revient à l'écran avec l'état partagé de l'application et la page qui était ouverte, et un `.exe` qui porte son icône et son nom. Le rechargement de code natif à état vivant reste un sous-système à part — nommé, pas prétendu.
