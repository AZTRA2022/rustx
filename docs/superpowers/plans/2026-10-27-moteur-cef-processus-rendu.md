# rustx — Plan 1 : le moteur, Rust dans le processus de rendu

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produire un `.exe` Windows où un `onClick` du DOM exécute une fonction Rust **dans le processus de rendu**, qui modifie le DOM en retour, sans IPC ni sérialisation.

**Architecture:** Le binaire se relance lui-même comme sous-processus de rendu CEF. Dans ce processus, du code Rust s'accroche au contexte V8 à sa création et y installe des fonctions natives. Les mutations du DOM partent du même thread, par les liaisons V8 du processus. Les threads de travail ne touchent pas au DOM : ils postent dans une file que la boucle de frame vide une fois par frame.

**Tech Stack:** Rust edition 2024, crate `cef` + `cef-dll-sys` (tauri-apps/cef-rs), binaires CEF préconstruits, Windows 10 x64.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md`

**Portée :** ce plan ne couvre que la couche `rustx-engine` du §7.2 du document. Le compilateur `.rsx`, le runtime de réactivité, la couche OS, la CLI et le LSP font l'objet de plans séparés. C'est délibéré : le document désigne ce montage comme le risque n°1, celui dont dépend toute l'architecture. Rien d'autre ne doit être construit avant qu'il tienne.

## Global Constraints

- Plateforme cible : Windows 10 x64 uniquement. Aucun code spécifique à macOS ou Linux.
- Rust edition 2024.
- Aucune dépendance sous GPL. Les dépendances du framework sont MIT, Apache-2.0, BSD, LGPL en liaison dynamique, ou API système.
- Aucun contenu web distant : le moteur ne charge que des ressources locales. Le bac à sable du processus de rendu est désactivé, cette contrainte est ce qui rend ce choix défendable.
- `unsafe` est autorisé et inévitable dans la couche de liaison FFI à CEF, qui est du C. L'anti-objectif n°4 du document porte sur le code **généré** par `rustx` et sur le modèle de réactivité : ceux-là doivent être sûrs. Tout `unsafe` de ce plan est confiné à `rustx-engine` et encapsulé derrière une API sûre.
- L'état n'est ni `Send` ni `Sync` et vit sur le thread d'interface du processus de rendu.
- Pas d'extension éditeur dans ce plan : l'outillage vient en dernier.

## Review Focus

Cinq conditions que le document implique mais qu'aucune tâche n'exerce spontanément. Chacune a son test rattaché à la tâche qui possède le code.

1. **Le sous-processus de rendu reçoit les arguments de ligne de commande de l'utilisateur.** Si `bootstrap()` consomme ou confond les arguments CEF et ceux de l'application, un logiciel `rustx` lancé avec ses propres options se comporte de travers ou lance des fenêtres fantômes. → Tâche 2.
2. **Une fonction native Rust qui panique.** Un panic traversant la frontière FFI vers du C est un comportement indéfini. Le logiciel de preuve manipule du matériel, les panics arriveront. → Tâche 3.
3. **JavaScript appelle une fonction native avec le mauvais nombre ou le mauvais type d'arguments.** Le JS n'est pas typé ; le code de l'application ne doit jamais recevoir des arguments mal formés, et l'appelant JS doit recevoir une exception lisible. → Tâche 3.
4. **Une référence DOM utilisée après la destruction de son contexte V8** (navigation, rechargement, fermeture de fenêtre). C'est le cas d'usage-après-libération que le §7.5 du document interdit de résoudre par `unsafe`. → Tâche 4.
5. **Un thread de travail poste des mutations après la fermeture de la fenêtre.** Le logiciel de preuve a un thread de capture qui tourne ; il ne doit pas faire tomber l'application à l'extinction. → Tâche 6.

---

### Task 1: Espace de travail et fenêtre affichant une page locale

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `crates/rustx-engine/Cargo.toml`
- Create: `crates/rustx-engine/src/lib.rs`
- Create: `crates/rustx-engine/src/app.rs`
- Create: `crates/rustx-engine/src/error.rs`
- Create: `crates/rustx-engine/docs/cef-api.md`
- Create: `examples/hello/main.rs`
- Create: `examples/hello/index.html`
- Modify: `Cargo.toml` racine existant (devient le manifeste du workspace), `src/main.rs` supprimé
- Test: `crates/rustx-engine/tests/window.rs`

**Interfaces:**
- Consumes: rien.
- Produces:
  - `rustx_engine::AppConfig { pub title: String, pub width: u32, pub height: u32, pub index: std::path::PathBuf }`
  - `rustx_engine::App::new(config: AppConfig) -> Result<App, EngineError>`
  - `rustx_engine::App::run(self) -> Result<(), EngineError>`
  - `rustx_engine::EngineError` (enum, `std::error::Error`)

- [ ] **Step 1: Épingler la version du crate `cef` et relever l'API réelle**

Consulter docs.rs pour la dernière version publiée de `cef` et de `cef-dll-sys`. Noter dans `crates/rustx-engine/docs/cef-api.md` la version exacte retenue et les noms réels des éléments que ce plan utilise : point d'entrée d'exécution de sous-processus, trait de gestionnaire d'application, trait de gestionnaire de processus de rendu, rappel de création de contexte V8, création de valeur et de fonction V8, trait de gestionnaire V8.

Ce plan nomme ces éléments par leur rôle, pas par leur identifiant exact : les bindings suivent les versions de Chromium et renomment. Ce fichier est la table de correspondance, et les tâches suivantes s'y réfèrent.

- [ ] **Step 2: Écrire le test qui échoue**

```rust
// crates/rustx-engine/tests/window.rs
#[test]
fn app_rejects_a_missing_index() {
    let config = AppConfig {
        title: "t".into(), width: 800, height: 600,
        index: "n-existe-pas.html".into(),
    };
    assert!(matches!(App::new(config), Err(EngineError::IndexNotFound(_))));
}

#[test]
fn app_rejects_a_remote_url() {
    let config = AppConfig {
        title: "t".into(), width: 800, height: 600,
        index: "https://example.com".into(),
    };
    assert!(matches!(App::new(config), Err(EngineError::RemoteContentForbidden)));
}
```

Le second test applique la contrainte globale « pas de contenu distant » dès la première tâche, pour qu'elle ne puisse pas s'éroder plus tard.

- [ ] **Step 3: Lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p rustx-engine --test window`
Expected: FAIL, le crate ne compile pas (`App` non défini).

- [ ] **Step 4: Créer le workspace et le crate**

Transformer le `Cargo.toml` racine en manifeste de workspace avec un seul membre, `crates/rustx-engine`, et supprimer `src/main.rs`. Déclarer `cef` en dépendance à la version épinglée au step 1.

- [ ] **Step 5: Implémenter `AppConfig`, `EngineError` et `App::new`**

`App::new` valide seulement : `index` doit exister sur le disque et ne doit pas être une URL à schéma distant. Aucune initialisation de CEF à ce stade.

- [ ] **Step 6: Lancer le test pour vérifier qu'il passe**

Run: `cargo test -p rustx-engine --test window`
Expected: PASS

- [ ] **Step 7: Implémenter `App::run` et l'exemple**

`App::run` initialise CEF, ouvre une fenêtre native aux dimensions données et charge `index` par une URL `file://`. `examples/hello/index.html` contient un `<h1>Hello world</h1>` et un `<button id="inc">`, qui servira aux tâches suivantes.

Récupération des binaires CEF : documenter la procédure dans `crates/rustx-engine/docs/cef-api.md` (l'outil `download-cef` fourni par cef-rs, ou un script de build). Ils ne sont pas versionnés dans le dépôt — ajouter leur emplacement au `.gitignore`.

- [ ] **Step 8: Vérifier à l'œil**

Run: `cargo run --example hello`
Expected: une fenêtre s'ouvre et affiche « Hello world ».

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml crates/ examples/ .gitignore
git rm src/main.rs
git commit -m "feat(engine): workspace et fenetre CEF affichant une page locale"
```

---

### Task 2: Le sous-processus de rendu est notre propre binaire

**Files:**
- Create: `crates/rustx-engine/src/bootstrap.rs`
- Modify: `crates/rustx-engine/src/lib.rs`, `examples/hello/main.rs`
- Test: `crates/rustx-engine/tests/bootstrap.rs`

C'est la tâche charnière du plan. CEF relance l'exécutable pour chacun de ses sous-processus ; en faisant de notre binaire ce sous-processus, le code Rust de l'application se retrouve dans le même processus que Blink et V8. Tout le reste en découle.

**Interfaces:**
- Consumes: Tâche 1 (`App`, `EngineError`).
- Produces:
  - `rustx_engine::Bootstrap` (enum : `Browser`, `Child { exit_code: i32 }`)
  - `rustx_engine::bootstrap() -> Bootstrap` — doit être la première instruction de `main`.
  - `rustx_engine::process::role() -> Role` (enum : `Browser`, `Renderer`, `Other`)

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
// crates/rustx-engine/tests/bootstrap.rs
#[test]
fn parent_process_is_identified_as_browser() {
    // Aucun argument de type sous-processus present.
    assert!(matches!(bootstrap(), Bootstrap::Browser));
    assert_eq!(process::role(), Role::Browser);
}

#[test]
fn application_arguments_survive_bootstrap() {
    // Review Focus 1 : bootstrap() ne doit pas consommer les arguments utilisateur.
    // Lance l'exemple `hello` avec `--mon-option=42` et verifie qu'il les voit.
    let sortie = run_hello_example(&["--mon-option=42"]);
    assert!(sortie.contains("mon-option=42"));
    assert_eq!(sortie.matches("WINDOW_OPENED").count(), 1);
}
```

Le second test couvre le Review Focus n°1 : une seule fenêtre doit s'ouvrir, pas une par sous-processus, et les arguments de l'utilisateur doivent lui parvenir intacts. `examples/hello/main.rs` écrit `WINDOW_OPENED` et ses arguments sur la sortie standard.

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-engine --test bootstrap`
Expected: FAIL (`bootstrap` non défini).

- [ ] **Step 3: Implémenter `bootstrap()` et `process::role()` dans `bootstrap.rs`**

`bootstrap()` appelle le point d'entrée d'exécution de sous-processus de CEF relevé en Tâche 1. Si CEF indique qu'il s'agit d'un sous-processus, renvoyer `Bootstrap::Child { exit_code }` ; l'appelant doit alors quitter immédiatement avec ce code, sans rien exécuter d'autre. Sinon, renvoyer `Bootstrap::Browser`.

`role()` lit le type de processus dans la ligne de commande fournie par CEF.

- [ ] **Step 4: Câbler l'exemple**

`examples/hello/main.rs` commence par `bootstrap()`, quitte si `Child`, puis construit et lance l'`App`.

- [ ] **Step 5: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-engine --test bootstrap`
Expected: PASS, et une seule occurrence de `WINDOW_OPENED`.

- [ ] **Step 6: Commit**

```bash
git add crates/rustx-engine/src/bootstrap.rs crates/rustx-engine/tests/bootstrap.rs crates/rustx-engine/src/lib.rs examples/hello/main.rs
git commit -m "feat(engine): le binaire se relance comme sous-processus de rendu"
```

---

### Task 3: Appeler du Rust depuis le DOM, dans le processus de rendu

**Files:**
- Create: `crates/rustx-engine/src/renderer/mod.rs`
- Create: `crates/rustx-engine/src/renderer/native.rs`
- Create: `crates/rustx-engine/src/value.rs`
- Modify: `examples/hello/index.html`, `examples/hello/main.rs`
- Test: `crates/rustx-engine/tests/native_call.rs`

**Interfaces:**
- Consumes: Tâche 2 (`bootstrap`, `role`).
- Produces:
  - `rustx_engine::Value` (enum : `Null`, `Bool(bool)`, `Number(f64)`, `String(String)`)
  - `rustx_engine::renderer::NativeError { pub message: String }`
  - `rustx_engine::renderer::register_native(name: &str, f: impl Fn(&[Value]) -> Result<Value, NativeError> + 'static)`
  - `rustx_engine::renderer::on_context_created(f: impl Fn() + 'static)` — point d'accroche où l'application enregistre ses fonctions natives.

`register_native` ne demande ni `Send` ni `Sync` : la contrainte globale veut que ce code vive sur le thread d'interface du processus de rendu, et V8 l'exige de toute façon.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
// crates/rustx-engine/tests/native_call.rs
#[test]
fn a_dom_click_runs_rust_and_returns_its_value() {
    let app = driver();
    app.click("#inc");
    assert_eq!(app.text("#out"), "1");
    app.click("#inc");
    assert_eq!(app.text("#out"), "2");
}

#[test]
fn a_panic_in_a_native_function_does_not_kill_the_process() {
    // Review Focus 2.
    let app = driver();
    app.click("#panic");
    assert!(app.text("#error").contains("internal error"));
    app.click("#inc");
    assert_eq!(app.text("#out"), "1"); // toujours vivant
}

#[test]
fn badly_typed_arguments_produce_a_readable_js_exception() {
    // Review Focus 3.
    let app = driver();
    app.click("#bad_args");
    assert!(app.text("#error").contains("expected 1 argument"));
}
```

`driver()` est un utilitaire de test : il lance l'exemple `hello`, attend que la page soit chargée, et expose `click(selector)` et `text(selector)` en injectant du JavaScript. Le choix du mécanisme de pilotage est laissé à l'implémenteur ; le plus simple est de piloter depuis la page elle-même via un script de test et de lire le résultat sur la sortie standard.

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-engine --test native_call`
Expected: FAIL (`register_native` non défini).

- [ ] **Step 3: Implémenter `Value` et la conversion depuis et vers V8 dans `value.rs`**

Conversion dans les deux sens entre `Value` et les valeurs V8 de CEF. Une valeur V8 d'un type non couvert par `Value` devient `NativeError`, elle n'est jamais silencieusement convertie.

- [ ] **Step 4: Implémenter `register_native` et le gestionnaire de processus de rendu dans `renderer/`**

À la création du contexte V8, installer sur l'objet global une fonction par entrée enregistrée. Le gestionnaire V8 appelle la closure Rust. Trois exigences :

- Le corps de la closure est enveloppé dans `std::panic::catch_unwind` ; un panic devient une exception JavaScript (`« internal error : … »`) et ne traverse jamais la frontière FFI. C'est une obligation de correction, pas une politesse : un panic qui se propage dans du C est un comportement indéfini.
- Une `NativeError` renvoyée devient une exception JavaScript portant son message.
- Le compte d'arguments et leurs types sont vérifiés par le code appelant la closure ; le message d'erreur nomme ce qui était attendu.

- [ ] **Step 5: Étoffer l'exemple**

`index.html` reçoit `#out`, `#error`, et les boutons `#inc`, `#panic`, `#bad_args`. `main.rs` enregistre trois fonctions natives via `on_context_created` : un compteur en Rust, une qui panique, une qui exige exactement un argument.

- [ ] **Step 6: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-engine --test native_call`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/rustx-engine/src/renderer crates/rustx-engine/src/value.rs crates/rustx-engine/tests/native_call.rs examples/
git commit -m "feat(engine): fonctions natives Rust appelables depuis le DOM"
```

---

### Task 4: Modifier le DOM depuis Rust

**Files:**
- Create: `crates/rustx-engine/src/dom.rs`
- Modify: `examples/hello/main.rs`
- Test: `crates/rustx-engine/tests/dom.rs`

C'est ce qui rend un réconciliateur écrit en Rust possible : sans écriture dans le DOM depuis Rust, les plans suivants n'ont nulle part où rendre.

**Interfaces:**
- Consumes: Tâche 3 (`Value`, `register_native`, `on_context_created`).
- Produces:
  - `rustx_engine::dom::Document::current() -> Option<Document>`
  - `Document::get_element_by_id(&self, id: &str) -> Option<Node>`
  - `Document::create_element(&self, tag: &str) -> Result<Node, DomError>`
  - `Node::set_text(&self, text: &str) -> Result<(), DomError>`
  - `Node::set_attribute(&self, name: &str, value: &str) -> Result<(), DomError>`
  - `Node::append_child(&self, child: &Node) -> Result<(), DomError>`
  - `Node::remove(&self) -> Result<(), DomError>`
  - `rustx_engine::dom::DomError` (variante obligatoire : `ContextDestroyed`)

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
// crates/rustx-engine/tests/dom.rs
#[test]
fn rust_creates_and_mutates_nodes() {
    let app = driver();
    app.click("#add"); // une fonction native qui construit un <li> en Rust
    assert_eq!(app.text("#list li:last-child"), "item 1");
    app.click("#add");
    assert_eq!(app.count("#list li"), 2);
}

#[test]
fn a_node_outliving_its_context_returns_an_error() {
    // Review Focus 4 : pas d'usage-apres-liberation, pas de panic, une erreur.
    let app = driver();
    app.click("#keep_node"); // Rust conserve un Node
    app.reload();
    app.click("#use_kept_node");
    assert!(app.text("#error").contains("context destroyed"));
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-engine --test dom`
Expected: FAIL (`dom::Document` non défini).

- [ ] **Step 3: Implémenter `Document`, `Node` et `DomError` dans `dom.rs`**

Les opérations passent par les liaisons V8 du processus, conformément au §6.2 du document — pas d'appel direct à Blink, pas de fork de Chromium.

Le point délicat est la durée de vie, et le Review Focus n°4 en dépend. Un `Node` ne détient pas une référence brute : il retient une identité valide pour un contexte V8 donné, accompagnée d'un jeton de génération du contexte. Toute opération vérifie que la génération courante correspond, et renvoie `DomError::ContextDestroyed` sinon. Aucune référence Rust n'est laissée à V8, aucune référence V8 ne survit dans Rust : c'est la règle du §7.5 du document, appliquée ici, et elle se tient sans `unsafe` dans l'API publique.

- [ ] **Step 4: Étoffer l'exemple**

Ajouter `#list`, et les boutons `#add`, `#keep_node`, `#use_kept_node`.

- [ ] **Step 5: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-engine --test dom`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/rustx-engine/src/dom.rs crates/rustx-engine/tests/dom.rs examples/
git commit -m "feat(engine): mutation du DOM depuis Rust avec duree de vie verifiee"
```

---

### Task 5: Prouver l'absence de frontière

**Files:**
- Create: `crates/rustx-engine/tests/no_boundary.rs`
- Modify: `examples/hello/main.rs`

La thèse entière du projet est que le Rust s'exécute dans le processus de rendu. Tant qu'un test ne l'affirme pas, une régression peut la défaire en silence — et personne ne s'en apercevra avant de mesurer des latences décevantes six mois plus tard.

**Interfaces:**
- Consumes: Tâches 3 et 4.
- Produces: aucune API nouvelle.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn rust_called_from_the_dom_runs_in_the_render_process() {
    let app = driver();
    app.click("#pid");
    let rust_pid: u32 = app.text("#out").parse().unwrap();
    let render_pid: u32 = app.eval_js("window.__render_pid").parse().unwrap();
    assert_eq!(rust_pid, render_pid);
    assert_ne!(rust_pid, std::process::id()); // ni dans le processus navigateur
}

#[test]
fn a_dom_rust_dom_round_trip_stays_under_fifty_microseconds() {
    let app = driver();
    let mean_ns = app.measure_round_trip(10_000);
    assert!(mean_ns < 50_000, "round trip too slow : {mean_ns} ns");
}
```

Le seuil de 50 µs est une barrière anti-régression, pas un objectif : un appel en processus doit se compter en centaines de nanosecondes, un aller-retour IPC en dizaines de microsecondes. Franchir ce seuil signifie qu'une frontière est réapparue.

`window.__render_pid` est exposé par une fonction native triviale appelée au chargement ; l'identifiant de processus côté JS n'est pas accessible autrement.

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-engine --test no_boundary`
Expected: FAIL (boutons et fonctions absents de l'exemple).

- [ ] **Step 3: Ajouter à l'exemple la fonction native `#pid` et la boucle de mesure**

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-engine --test no_boundary`
Expected: PASS. Consigner la moyenne observée dans `crates/rustx-engine/docs/cef-api.md` : c'est le chiffre de référence du projet.

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-engine/tests/no_boundary.rs examples/ crates/rustx-engine/docs/cef-api.md
git commit -m "test(engine): prouver que le Rust s'execute dans le processus de rendu"
```

---

### Task 6: Boucle de frame et file de mutations des threads de travail

**Files:**
- Create: `crates/rustx-engine/src/frame.rs`
- Modify: `crates/rustx-engine/src/lib.rs`, `examples/hello/main.rs`
- Test: `crates/rustx-engine/tests/frame.rs`

Implémente le modèle de threads du §7.4 du document. Les plans suivants (réactivité, capture) s'appuient dessus : c'est ici que se décide le fait qu'un thread de capture à 10 000 mises à jour par seconde ne produise que 60 rendus.

**Interfaces:**
- Consumes: Tâche 4 (`dom`).
- Produces:
  - `rustx_engine::frame::Poster` — `Clone + Send`, obtenu depuis le thread d'interface.
  - `Poster::post(&self, f: impl FnOnce() + Send + 'static) -> Result<(), PostError>`
  - `rustx_engine::frame::on_frame(f: impl FnMut() + 'static)` — appelé une fois par frame sur le thread d'interface.
  - `rustx_engine::frame::PostError::WindowClosed`

`Poster` est le seul objet du moteur qui franchit une frontière de thread. Il ne porte aucune donnée du DOM : seulement une fonction qui sera exécutée plus tard sur le thread d'interface.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn posted_mutations_run_once_per_frame_on_the_ui_thread() {
    let app = driver();
    app.click("#start_thread"); // un thread poste 10_000 increments
    app.wait_frames(60);
    assert_eq!(app.text("#out"), "10000");          // tout est applique
    assert!(app.render_count() <= 70);           // mais pas 10 000 rendus
}

#[test]
fn posting_after_window_close_returns_an_error() {
    // Review Focus 5.
    let poster = app_then_close();
    assert!(matches!(poster.post(|| {}), Err(PostError::WindowClosed)));
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-engine --test frame`
Expected: FAIL (`frame::Poster` non défini).

- [ ] **Step 3: Implémenter `Poster`, `on_frame` et la boucle de frame dans `frame.rs`**

Une file de travaux protégée par un verrou, alimentée par n'importe quel thread, vidée intégralement au début de chaque frame sur le thread d'interface, avant les rappels `on_frame`. La cadence suit le rafraîchissement d'écran via le mécanisme de boucle d'animation du moteur.

Après la fermeture de la fenêtre, la file est marquée close : `post` renvoie `PostError::WindowClosed` au lieu d'accumuler des travaux qui ne s'exécuteront jamais, et le thread appelant peut s'arrêter proprement.

- [ ] **Step 4: Étoffer l'exemple**

Bouton `#start_thread` : lance un thread qui poste 10 000 incréments aussi vite qu'il peut. Un compteur de rendus est exposé par une fonction native.

- [ ] **Step 5: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p rustx-engine --test frame`
Expected: PASS

- [ ] **Step 6: Lancer toute la suite**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add crates/rustx-engine/src/frame.rs crates/rustx-engine/tests/frame.rs crates/rustx-engine/src/lib.rs examples/
git commit -m "feat(engine): boucle de frame et file de mutations inter-threads"
```

---

## Ce que ce plan livre, et ce qui vient après

À la fin : un `.exe` Windows où le DOM appelle du Rust en processus, où Rust écrit dans le DOM, où les durées de vie sont vérifiées sans `unsafe` exposé, et où les threads de travail ne peuvent ni courir après le DOM ni noyer le rendu. Le pari central du document est validé ou réfuté — avant qu'une ligne de compilateur `.rsx` ne soit écrite.

Plans suivants, dans cet ordre : le compilateur `.rsx` et sa carte de correspondance ; le runtime de réactivité et le contrat de composant (§7.4 et §7.4 bis) ; le pont React et la cohabitation `.tsx` ; la couche OS pour le logiciel de preuve ; la CLI et le hot reload ; l'extension VS Code en dernier, conformément à ta décision.
