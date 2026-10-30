# rustx — Plan 5 : le routing par fichiers et les layouts

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Qu'une application soit un dossier de fichiers — `app/page.rsx`, `app/device/[id]/page.rsx`, `app/layout.rsx` — et qu'aller d'un écran à l'autre garde ce que les écrans qui restent tenaient, sans qu'on écrive un routeur.

```rust
// app/layout.rsx
fn Layout(children: Element) -> Element {
    return <main><Nav /><section>{children}</section></main>;
}

// app/device/[id]/page.rsx
fn Page(id: String) -> Element {
    let camera = keep(Camera::open(&id));
    return <Preview source={camera} />;
}
```

**Architecture:** Trois morceaux, tous posés sur le plan 4. Le compilateur apprend à voir une balise majuscule comme un appel de composant, et chaque emplacement d'appel devient une instance enfant — ce qui donne la composition et, du même coup, les layouts. Le compilateur dépose à côté des pages une table des routes lue depuis l'arborescence de `app/`. Le routeur est un magasin : changer de route marque les lecteurs, la frame suivante re-rend, et un composant qui reste atteint garde ses cellules tandis qu'un composant qui cesse de l'être est libéré — c'est déjà la règle du plan 4, appliquée à un sous-arbre entier.

**Tech Stack:** Rust edition 2024, `rustx-dom`, `rustx-compiler`, `rustx-engine` (CEF 156.2.0), Windows 10 x64.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.7, dernier point (« Routing par fichiers et layouts imbriqués, à la manière de Next.js, fournis par le runtime ») ; §7.2, la couche `rustx-runtime` ; §7.3 pour la déterminisme de la transpilation.

**Portée :** la composition de composants (`<Preview />`, props et `children`), la table des routes engendrée depuis `app/`, l'appariement d'un chemin (segments statiques, segment dynamique `[id]`, page introuvable), les layouts imbriqués qui survivent à une navigation, `navigate`, le retour en arrière, et un logiciel de preuve à deux écrans.

**Hors périmètre, explicitement :** le style objet (`style={{…}}`), `className` et les attributs camelCase de React — ce sont les conventions de props de React, et elles appartiennent au plan 6 (§7.5). Également hors périmètre : les segments attrape-tout (`[...rest]`), les routes parallèles, le chargement paresseux d'une route, et l'historique du système (boutons précédent/suivant de la fenêtre), qui appartient à la couche OS.

## Global Constraints

- Plateforme cible : Windows 10 x64 uniquement. Rust edition 2024.
- Aucune dépendance sous GPL ; `unsafe` reste confiné à `rustx-engine`.
- **Une navigation ne touche jamais au `location` du document.** Charger une URL détruirait le contexte V8, donc l'état de l'application, et le refus du plan 1 (§5, contenu local seulement) l'interdit de toute façon. Une route est une valeur dans un magasin, et rien d'autre.
- L'état vit sur le thread d'interface du processus de rendu. Une navigation postée depuis un thread de fond est une mutation de magasin comme une autre : elle marque, elle ne dessine pas.
- Le nettoyage reste celui de Rust. Une page quittée est libérée parce qu'elle cesse d'être atteinte, pas parce qu'un routeur l'a démontée à la main.
- La transpilation reste déterministe : même arborescence, même table des routes, même ordre.
- Aucune syntaxe nouvelle à apprendre : une balise majuscule, un dossier, deux noms de fichiers (`page.rsx`, `layout.rsx`).

## Review Focus

1. **Une navigation pendant qu'une page tient une tâche.** Les layouts qui restent à l'écran gardent leurs cellules ; la page quittée est libérée, et le thread qu'elle avait lancé s'arrête. → Tâche 1 (la libération) et tâche 5 (la preuve).
2. **Le même composant écrit deux fois dans un arbre.** `<Counter /><Counter />` est une ligne de code et deux états, comme `<Counter />` monté deux fois l'était au plan 3. → Tâche 1.
3. **Un segment dynamique qui change de valeur.** `/device/1` → `/device/2` est la même page avec une autre prop : elle se re-rend, et l'effet qui surveille l'identifiant se relance en fermant avant d'ouvrir. → Tâche 4.
4. **Un chemin que personne n'a prévu, et un chemin que deux routes revendiquent.** Un chemin inconnu affiche la page introuvable plutôt qu'une fenêtre blanche ; `/device/new` face à `/device/[id]` choisit le segment statique. → Tâche 3.
5. **Un `<Preview />` dont le composant n'est pas déclaré dans ce fichier, ou dont un attribut ne nomme aucun paramètre.** Refusé à la compilation, à la ligne du `.rsx`, plutôt que rendu comme une balise HTML inconnue. → Tâche 1.

---

### Task 1: La composition — une balise majuscule est un appel

**Files:**
- Modify: `crates/rustx-dom/src/state.rs`
- Modify: `crates/rustx-compiler/src/promote.rs`, `crates/rustx-compiler/src/rewrite.rs`, `crates/rustx-compiler/src/tags.rs`
- Test: `crates/rustx-dom/tests/children.rs`, `crates/rustx-compiler/tests/composition.rs`

Sans ça il n'y a pas de layout : un layout est un composant qui en appelle un autre. Le plan 4 l'avait renvoyé ici.

**Interfaces:**
- Consumes: plan 3 (`Slot`, `Scope`, `InstanceId`, `drop_instance`, `end_render`, `with_key`), plan 4 (`keep_at`, `effect_at`).
- Produces:
  - `rustx_dom::state::component_at<R>(slot: Slot, body: impl FnOnce() -> R) -> R` — rend `body` dans une instance enfant à elle, stable d'un rendu à l'autre, et déclarée enfant de l'instance en cours
  - `rustx_dom::state::children_of(instance: InstanceId) -> Vec<InstanceId>` — pour les tests et le diagnostic
  - `end_render` libère les instances enfants qui ont cessé d'être atteintes ; `drop_instance` libère les enfants avant le parent

- [ ] **Step 1: Écrire les tests qui échouent — `crates/rustx-dom/tests/children.rs`**

```rust
#[test]
fn the_same_component_written_twice_is_two_instances() {
    // Review Focus 2.
    let _scope = Scope::enter(InstanceId(1));
    let first = component_at(here!(), || current().expect("inside a child"));
    let second = component_at(here!(), || current().expect("inside a child"));
    assert_ne!(first, second);
}

#[test]
fn a_child_keeps_its_cells_across_renders_of_its_parent() {
    assert_eq!(counts_over_two_renders_of_the_same_parent(), [1, 2]);
}

#[test]
fn a_child_that_stops_being_reached_is_released() {
    // Review Focus 1, the half that lives in `rustx-dom`: a page the
    // router no longer renders is released because nothing reached it.
    let witness = Rc::new(Cell::new(false));
    render_parent_with_child(Some(witness.clone()));
    render_parent_with_child(None);
    end_render().expect("the parent still reaches everything it declares");
    assert!(witness.get());
}

#[test]
fn releasing_a_parent_releases_the_child_it_declared() {
    let witness = Rc::new(Cell::new(false));
    render_parent_with_child(Some(witness.clone()));
    drop_instance(InstanceId(1));
    assert!(witness.get());
}

#[test]
fn a_childs_cells_are_not_its_parents() {
    let _scope = Scope::enter(InstanceId(1));
    cell(here!(), || 1_u8);
    let inside = component_at(here!(), || live_instances());
    assert_eq!(inside, 2, "the child should have cells of its own");
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent** — `cargo test -p rustx-dom --test children` — FAIL, `component_at` non défini.

- [ ] **Step 3: Implémenter `component_at` et la libération**

L'identité d'une instance enfant est `(instance parente, slot, chemin de clés)`, résolue dans une table `thread_local` et allouée au premier rendu — la même règle que pour une cellule, pour que `with_key` du plan 3 serve aux listes de composants sans rien ajouter. Chaque instance tient la liste de ses enfants ; `end_render` compare les enfants atteints à ceux du rendu précédent et libère la différence, et `drop_instance` descend d'abord.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent** — `cargo test -p rustx-dom --test children` — PASS

- [ ] **Step 5: Écrire les tests du compilateur qui échouent — `crates/rustx-compiler/tests/composition.rs`**

```rust
#[test]
fn a_capitalised_tag_becomes_a_call_to_the_component() {
    let rust = rewrite_one("fn Nav() -> Element {\n    return <p>nav</p>;\n}\nfn Page() -> Element {\n    return <div><Nav /></div>;\n}");
    assert!(rust.contains("component_at"), "got {rust}");
    assert!(rust.contains("Nav()"), "got {rust}");
}

#[test]
fn a_lowercase_tag_is_still_html() {
    let rust = rewrite_one("fn Page() -> Element {\n    return <section><p>hi</p></section>;\n}");
    assert!(!rust.contains("component_at"), "got {rust}");
}

#[test]
fn an_attribute_is_passed_as_the_parameter_it_names() {
    // Written in the other order than declared: what matters is the
    // name, not the position.
    let rust = rewrite_one("fn Row(label: String, count: u8) -> Element {\n    return <li>{label}</li>;\n}\nfn Page() -> Element {\n    return <ul><Row count={2} label={\"a\".to_string()} /></ul>;\n}");
    assert!(rust.contains("Row(\"a\".to_string(), 2)"), "got {rust}");
}

#[test]
fn what_is_written_between_the_tags_arrives_as_children() {
    let rust = rewrite_one("fn Layout(children: Element) -> Element {\n    return <main>{children}</main>;\n}\nfn Page() -> Element {\n    return <Layout><p>hi</p></Layout>;\n}");
    assert!(rust.contains("Layout(rustx_dom :: Element :: tag (\"p\")"), "got {rust}");
}

#[test]
fn a_component_that_is_not_declared_in_this_file_is_refused() {
    // Review Focus 5: a silent wrong call is worse than a refusal.
    let error = transpile("fn Page() -> Element {\n    return <Preview />;\n}").expect_err("should be refused");
    assert_eq!(error.line, 2);
    assert!(error.message.contains("Preview"), "got {}", error.message);
}

#[test]
fn an_attribute_that_names_no_parameter_is_refused() {
    let error = transpile("fn Row(label: String) -> Element {\n    return <li>{label}</li>;\n}\nfn Page() -> Element {\n    return <Row labell={1} />;\n}").expect_err("should be refused");
    assert!(error.message.contains("labell"), "got {}", error.message);
}

#[test]
fn composing_keeps_the_file_the_same_length() {
    let source = "fn Nav() -> Element {\n    return <p>nav</p>;\n}\nfn Page() -> Element {\n    return <div><Nav /></div>;\n}\n";
    assert_eq!(transpile(source).unwrap().rust.lines().count(), source.lines().count());
}
```

- [ ] **Step 6: Lancer, implémenter, relancer** — `cargo test -p rustx-compiler --test composition`

`<Comp a={x}>…</Comp>` devient `rustx_dom::state::component_at(Slot { … }, || Comp(args…))`, les arguments rangés dans l'ordre des paramètres déclarés, `children` recevant l'arbre écrit entre les balises. Les paramètres sont lus dans le même fichier : un composant venu d'ailleurs est refusé, parce que sans la liste des paramètres, l'ordre des arguments ne peut qu'être deviné. Une balise dont la première lettre est majuscule et qui n'est pas un composant connu est une erreur, jamais une balise HTML.

- [ ] **Step 7: Commit** — `feat(dom): a component written inside another is an instance of its own` puis `feat(compiler): a capitalised tag is a call to a component`

---

### Task 2: La table des routes, engendrée depuis `app/`

**Files:**
- Modify: `crates/rustx-compiler/src/build.rs`
- Create: `crates/rustx-compiler/src/routes.rs`
- Test: `crates/rustx-compiler/tests/routes.rs`

**Interfaces:**
- Consumes: tâche 1, plan 2 (`compile_directory`, `transpile`).
- Produces:
  - `rustx_compiler::routes::table_for(from: &Path) -> Result<String, CompileError>` — le texte du `routes.rs` engendré
  - `compile_directory` écrit `routes.rs` à côté des pages
  - Le fichier engendré expose `pub fn route_table() -> &'static [rustx_dom::route::Route]` et inclut chaque page dans son propre module

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_page_at_the_root_is_the_root_route() {
    let generated = table_for(&tree(&[("page.rsx", PAGE)])).unwrap();
    assert!(generated.contains("pattern: \"/\""), "got {generated}");
}

#[test]
fn a_folder_is_a_segment_and_a_bracketed_folder_is_a_parameter() {
    let generated = table_for(&tree(&[("device/[id]/page.rsx", PAGE_WITH_ID)])).unwrap();
    assert!(generated.contains("pattern: \"/device/[id]\""), "got {generated}");
    assert!(generated.contains("params.get(\"id\")"), "got {generated}");
}

#[test]
fn a_layout_encloses_every_page_below_it_outermost_first() {
    let generated = table_for(&tree(&[
        ("layout.rsx", LAYOUT),
        ("device/layout.rsx", LAYOUT),
        ("device/page.rsx", PAGE),
    ])).unwrap();
    assert!(generated.contains("layouts: &[layout_0, layout_1]"), "got {generated}");
}

#[test]
fn each_page_is_included_in_a_module_of_its_own() {
    // Two pages both declaring `fn Page` must not collide.
    let generated = table_for(&tree(&[("page.rsx", PAGE), ("device/page.rsx", PAGE)])).unwrap();
    assert_eq!(generated.matches("include!").count(), 2, "got {generated}");
}

#[test]
fn the_table_is_the_same_whatever_order_the_files_come_in() {
    // §7.3: the transpilation is deterministic, and so is this.
    let one = table_for(&tree(&[("page.rsx", PAGE), ("device/page.rsx", PAGE)])).unwrap();
    let two = table_for(&tree(&[("device/page.rsx", PAGE), ("page.rsx", PAGE)])).unwrap();
    assert_eq!(one, two);
}

#[test]
fn a_page_without_a_page_component_is_refused_by_name() {
    let error = table_for(&tree(&[("page.rsx", "fn Other() -> Element { return <p>x</p>; }")]))
        .expect_err("should be refused");
    assert!(error.message.contains("fn Page"), "got {}", error.message);
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-compiler --test routes`, puis commit `feat(compiler): a route table read from the app directory`.

**Step 3, la décision :** un `page.rsx` doit déclarer `fn Page(…)`, un `layout.rsx` doit déclarer `fn Layout(children: Element)` ; les paramètres de `Page` sont lus dans le fichier et appariés aux segments entre crochets du chemin, par nom. Le fichier engendré utilise `include!("device/page.rs")` — `include!` résout relativement au fichier qui l'écrit, donc un seul `include!` dans `main.rs` suffit à l'application. Les routes sortent triées par chemin.

---

### Task 3: L'appariement d'un chemin

**Files:**
- Create: `crates/rustx-dom/src/route.rs`
- Modify: `crates/rustx-dom/src/lib.rs`
- Test: `crates/rustx-dom/tests/route.rs`

**Interfaces:**
- Consumes: rien.
- Produces:
  - `rustx_dom::route::Params` — les valeurs capturées, `get(&self, name: &str) -> &str`
  - `rustx_dom::route::Route { pattern: &'static str, render: fn(&Params) -> Element, layouts: &'static [fn(Element) -> Element] }`
  - `rustx_dom::route::match_route<'a>(table: &'a [Route], path: &str) -> Option<(&'a Route, Params)>`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_static_path_matches_its_own_route() {
    assert_eq!(matched("/device", &["/", "/device"]), Some("/device"));
}

#[test]
fn a_bracketed_segment_captures_what_is_there() {
    let (_, params) = match_route(&table(&["/device/[id]"]), "/device/7").expect("a match");
    assert_eq!(params.get("id"), "7");
}

#[test]
fn a_static_segment_wins_over_a_parameter() {
    // Review Focus 4, the second half.
    assert_eq!(matched("/device/new", &["/device/[id]", "/device/new"]), Some("/device/new"));
}

#[test]
fn a_path_nobody_declared_matches_nothing() {
    // Review Focus 4, the first half: the router turns this into the
    // not-found page rather than a blank window.
    assert_eq!(matched("/nowhere", &["/", "/device"]), None);
}

#[test]
fn a_trailing_slash_is_the_same_path() {
    assert_eq!(matched("/device/", &["/device"]), Some("/device"));
}

#[test]
fn a_parameter_does_not_match_across_a_slash() {
    assert_eq!(matched("/device/7/edit", &["/device/[id]"]), None);
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-dom --test route`, puis commit `feat(dom): matching a path against the route table`.

**Step 3, la décision :** l'appariement est segment à segment sur des `&str` empruntés au chemin, sans allocation hors des valeurs capturées. Les routes candidates sont essayées en ordonnant d'abord celles qui ont le plus de segments statiques, de sorte que `/device/new` ne soit jamais mangé par `/device/[id]`.

---

### Task 4: Le routeur

**Files:**
- Create: `crates/rustx-engine/src/router.rs`
- Modify: `crates/rustx-engine/src/lib.rs`
- Test: `crates/rustx-engine/tests/router.rs`

**Interfaces:**
- Consumes: tâches 1 à 3, plan 3 (`render::mount_app`), plan 4 (`store`).
- Produces:
  - `rustx_engine::router::start(table: &'static [Route], host: &str, not_found: fn() -> Element) -> Result<(), DomError>`
  - `rustx_engine::router::navigate(path: impl Into<String>)`
  - `rustx_engine::router::back() -> bool`
  - `rustx_engine::router::current() -> String`

- [ ] **Step 1: Écrire les tests qui échouent — scénario `router` dans `hello`**

```rust
#[test]
fn the_first_screen_is_the_route_the_application_started_on() {
    assert_eq!(report().get("first_screen"), "home");
}

#[test]
fn navigating_shows_the_other_screen() {
    assert_eq!(report().get("after_navigate"), "device 7");
}

#[test]
fn a_layout_that_stays_on_screen_keeps_its_state() {
    // Review Focus 1: the comfort of Next.js is exactly this.
    assert_eq!(report().get("layout_state_after_navigate"), "3");
}

#[test]
fn the_page_that_was_left_is_released_and_its_task_stops() {
    assert_eq!(report().get("left_page_released"), "yes");
    assert_eq!(report().get("left_page_task"), "stopped");
}

#[test]
fn changing_a_parameter_re_renders_the_same_page_and_re_runs_its_effect() {
    // Review Focus 3.
    assert_eq!(report().get("after_param_change"), "device 8");
    assert_eq!(report().get("effect_order"), "open 7,close 7,open 8");
}

#[test]
fn a_path_nobody_declared_shows_the_not_found_page() {
    assert_eq!(report().get("unknown_path"), "not found");
}

#[test]
fn going_back_returns_to_the_previous_screen() {
    assert_eq!(report().get("after_back"), "device 8");
}

#[test]
fn a_navigation_posted_from_a_worker_thread_lands_on_the_next_frame() {
    assert_eq!(report().get("after_posted_navigate"), "home");
}
```

- [ ] **Step 2 à 6** : scénario `router` dans `hello`, `cargo test -p rustx-engine --test router`, puis commit `feat(engine): file routes, nested layouts and navigation`.

**Step 3, la décision :** la route courante est un magasin (`store::<Route>` du plan 4), donc `navigate` depuis un thread de fond est un `post` comme un autre et la frame suivante dessine une fois. Le routeur monte **un** composant sous `host` : il apparie, puis appelle les layouts de l'extérieur vers l'intérieur par `component_at`, la page au centre. Les layouts qui n'ont pas changé sont atteints au même emplacement, donc gardent leurs cellules ; la page quittée cesse d'être atteinte, donc est libérée avec ses tâches — aucun démontage à la main. L'historique est une pile de chemins dans le routeur, et non celle du navigateur.

---

### Task 5: Le logiciel de preuve, à deux écrans

**Files:**
- Create: `apps/console/` (`Cargo.toml`, `build.rs`, `app/layout.rsx`, `app/page.rsx`, `app/device/[id]/page.rsx`, `src/main.rs`, `assets/index.html`, `tests/`)
- Modify: `Cargo.toml`

La forme du logiciel de preuve du §8 : une liste de périphériques, et l'aperçu d'un périphérique. L'aperçu tient une tâche ; la barre de navigation tient un compteur qui ne doit pas repartir de zéro quand on change d'écran.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn clicking_a_device_opens_its_preview() {
    assert_eq!(report().get("preview_title"), "Webcam");
}

#[test]
fn the_navigation_bar_keeps_counting_across_screens() {
    assert!(report().get("ticks_after_navigate").parse::<u32>().unwrap() >= 3);
}

#[test]
fn leaving_a_preview_closes_the_stream_it_opened() {
    assert_eq!(report().get("closed_streams"), "1");
}

#[test]
fn the_two_screens_share_the_selected_device() {
    assert_eq!(report().get("selected_in_list"), report().get("selected_in_preview"));
}
```

- [ ] **Step 2 à 7** : comme au plan 4 — `cargo test -p console`, puis `cargo test --workspace --exclude counter -- --test-threads=1`, puis commit `feat(apps): a two-screen console with a layout that outlives its pages`.

**Step 3, les pages :** `app/layout.rsx` tient la barre (un `task` qui compte, de sorte que sa survie soit visible) ; `app/page.rsx` liste trois périphériques et navigue vers `/device/[id]` ; `app/device/[id]/page.rsx` prend `id: String`, ouvre un flux factice avec `keep`, et l'affiche. Aucun `useRouter`, aucun `<Link>` à configurer : un `onClick` appelle `navigate`.

---

## Ce que ce plan livre, et ce qui vient après

À la fin, une application `rustx` est un dossier : les écrans sont des fichiers, les cadres qui les entourent sont des fichiers, et passer d'un écran à l'autre garde ce qui reste à l'écran sans qu'on l'ait demandé. Le §7.7, dernier point, est tenu.

Ensuite : l'interopérabilité React et npm — et avec elle le dialecte de props de React, `style={{…}}`, `className` et les attributs camelCase, qui est ce qui manque aujourd'hui à une page écrite comme on écrit du TSX — puis la couche OS, puis la CLI et le hot reload.
