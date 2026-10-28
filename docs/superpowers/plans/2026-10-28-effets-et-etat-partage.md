# rustx — Plan 4 : les effets et l'état partagé

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Qu'un composant puisse ouvrir quelque chose au montage et que ce soit fermé au démontage, sans qu'on ait à y penser — et que deux composants éloignés voient la même donnée sans se la passer de parent en parent.

```rust
fn Preview() -> Element {
    let camera = keep(Camera::open("default"));
    let mut frames = 0;

    task(move |alive| while alive() { frames_received.send(camera.grab()); });

    return <p>{frames} images</p>;
}
```

**Architecture:** Trois mécanismes, tous construits sur ce que le plan 3 a déjà : une cellule vit aussi longtemps que son instance et son `Drop` s'exécute au démontage. `keep` est une cellule qu'on ne mute pas. Une tâche de fond est une cellule qui tient un jeton d'annulation, donc elle s'arrête quand le composant s'en va. Un magasin est une cellule qui n'appartient à personne, avec la liste de qui l'a lue.

**Tech Stack:** Rust edition 2024, `rustx-dom`, `rustx-compiler`, `rustx-engine` (CEF 156.2.0), Windows 10 x64.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.4 bis, les deux derniers points.

**Portée :** `keep`, les tâches de fond avec annulation au démontage, les effets qui se relancent quand une valeur change, et un magasin partagé entre composants d'un même processus de rendu.

**Hors périmètre, explicitement :** le magasin partagé **entre fenêtres**, hébergé dans le processus navigateur (§7.4, multi-fenêtres). C'est le seul endroit du système où une frontière subsiste, et elle appartient au plan de la couche OS, qui est ce qui donnera une raison d'ouvrir une deuxième fenêtre. Également hors périmètre : la composition de composants (`<Preview />`), qui vient avec le routing au plan 5.

## Global Constraints

- Plateforme cible : Windows 10 x64 uniquement. Rust edition 2024.
- Aucune dépendance sous GPL ; `unsafe` reste confiné à `rustx-engine`.
- L'état vit sur le thread d'interface du processus de rendu. Un magasin n'est **pas** une exception : il est partagé entre composants, pas entre threads.
- Une tâche de fond, elle, tourne sur un autre thread, et ne touche à rien : elle poste, comme au plan 1.
- Le nettoyage reste celui de Rust. Rien dans ce plan n'ajoute un second mécanisme à côté de `Drop`.
- Aucune syntaxe nouvelle à apprendre au-delà de trois noms : `keep`, `task`, `store`. Pas de tableau de dépendances à tenir à jour à la main.
- La transpilation reste déterministe.

## Review Focus

1. **Un composant démonté pendant que sa tâche tourne encore.** La tâche doit s'arrêter, et ce qu'elle poste après coup doit être refusé plutôt que d'arriver dans un composant qui n'existe plus. → Tâche 2.
2. **Une tâche qui panique.** Un thread de capture touche du matériel ; il paniquera. L'application doit rester à l'écran et le dire. → Tâche 2.
3. **Deux composants qui lisent le même magasin, dont un seul est à l'écran.** Muter le magasin ne doit re-rendre que ceux qui existent, et ne doit pas garder en vie l'instance d'un composant démonté. → Tâche 5.
4. **Un effet dont la valeur surveillée change deux fois dans la même frame.** Il doit se relancer une fois, pas deux, et son nettoyage doit passer avant sa relance. → Tâche 3.
5. **`keep` appelé dans une branche conditionnelle.** L'identité par emplacement suppose que le même code est exécuté à chaque rendu ; un `keep` sous un `if` ne l'est pas. Le framework doit le dire plutôt que de rendre une valeur qui n'est pas celle qu'on croit. → Tâche 1.

---

### Task 1: `keep` — une valeur construite une fois

**Files:**
- Modify: `crates/rustx-dom/src/state.rs`
- Test: `crates/rustx-dom/tests/keep.rs`

**Interfaces:**
- Consumes: plan 3 (`Slot`, `Scope`, `cell`, `drop_instance`).
- Produces:
  - `rustx_dom::state::Kept<T>` — un handle en lecture seule sur une valeur construite une fois, `Clone`
  - `Kept::<T>::with<R>(&self, read: impl FnOnce(&T) -> R) -> R` et, pour `T: Clone`, `get()`
  - `rustx_dom::state::keep_at<T: 'static>(slot: Slot, build: impl FnOnce() -> T) -> Kept<T>`
  - `rustx_dom::state::end_render() -> Result<(), String>` — appelée par le moteur à la fin de chaque rendu ; rend une `Err` nommant un `keep` qui a cessé d'être atteint

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_kept_value_is_built_once_however_many_renders() {
    let built = Rc::new(Cell::new(0));
    for _ in 0..3 {
        let _scope = Scope::enter(InstanceId(1));
        let counter = built.clone();
        keep_at(here!(), move || { counter.set(counter.get() + 1); 7_i32 });
    }
    assert_eq!(built.get(), 1);
}

#[test]
fn a_kept_value_is_dropped_when_its_component_goes() {
    let witness = Rc::new(Cell::new(false));
    {
        let _scope = Scope::enter(InstanceId(2));
        keep_at(here!(), || Noisy(witness.clone()));
    }
    drop_instance(InstanceId(2));
    assert!(witness.get());
}

#[test]
fn a_kept_value_need_not_be_clone() {
    let _scope = Scope::enter(InstanceId(3));
    let kept = keep_at(here!(), || std::cell::RefCell::new(vec![1_u8]));
    assert_eq!(kept.with(|value| value.borrow().len()), 1);
}

#[test]
fn a_keep_that_does_not_run_every_render_says_so() {
    // Review Focus 5: identity by place assumes the same code runs each
    // time. A `keep` behind an `if` does not, and the value a later
    // render gets back is not the one that line appears to ask for.
    let _scope = Scope::enter(InstanceId(4));
    keep_at(here!(), || 1_i32);
    keep_at(here!(), || 2_i32);
    end_render();

    let _scope = Scope::enter(InstanceId(4));
    keep_at(here!(), || 1_i32); // the second one is skipped this time
    let complaint = end_render().expect_err("a skipped keep should be reported");
    assert!(complaint.contains("every render"), "got {complaint}");
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p rustx-dom --test keep`
Expected: FAIL, `keep_at` non défini.

- [ ] **Step 3: Implémenter `keep_at`, `Kept` et la vérification**

`keep_at` est `cell` sans `set` ni `update`. La vérification de la Review Focus 5 : chaque rendu note les slots touchés ; `end_render` compare au rendu précédent et rend une `Err` nommant le fichier et la ligne du slot manquant. Le message dit quoi faire — sortir le `keep` du `if`, ou donner une clé à la branche avec `with_key`.

`end_render()` est appelée par le moteur à la fin d'un rendu (tâche 4 du plan 3, `render.rs`) ; ici elle est publique pour que le test puisse la déclencher.

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent** — `cargo test -p rustx-dom --test keep` — PASS

- [ ] **Step 5: Commit**

```bash
git add crates/rustx-dom
git commit -m "feat(dom): keep a value for as long as its component lives"
```

---

### Task 2: Les tâches de fond

**Files:**
- Create: `crates/rustx-engine/src/task.rs`
- Modify: `crates/rustx-engine/src/lib.rs`
- Test: `crates/rustx-engine/tests/task.rs`

Ce que le logiciel de preuve demande dès le premier jour : on ouvre un flux caméra, il faut le fermer.

**Interfaces:**
- Consumes: plan 1 (`frame::poster`, `PostError`), tâche 1 (`Kept`).
- Produces:
  - `rustx_engine::task::Task` — un jeton ; le laisser tomber annule la tâche
  - `rustx_engine::task::spawn(work: impl FnOnce(Alive) + Send + 'static) -> Task`
  - `rustx_engine::task::Alive` — `Clone + Send`, `alive()` rend `false` dès que la tâche est annulée
  - `Task::is_running(&self) -> bool`

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_task_stops_when_its_token_is_dropped() {
    // Review Focus 1.
    assert_eq!(report().get("task_stopped"), "yes");
}

#[test]
fn a_task_that_panics_does_not_take_the_window_with_it() {
    // Review Focus 2: a capture thread touches hardware; it will panic.
    assert_eq!(report().get("alive_after_task_panic"), "yes");
    assert!(report().raw().contains("a background task panicked"));
}

#[test]
fn what_a_cancelled_task_posts_afterwards_is_refused() {
    assert_eq!(report().get("post_after_cancel"), "refused");
}

#[test]
fn a_task_kept_by_a_component_stops_when_the_component_goes() {
    assert_eq!(report().get("task_after_unmount"), "stopped");
}
```

- [ ] **Step 2 à 6** : scénario `task` dans `hello`, `cargo test -p rustx-engine --test task`, puis commit `feat(engine): background work that stops when its component does`.

**Step 3, la décision :** `Alive` enveloppe un `Arc<AtomicBool>`. `Task` tient l'autre bout et le met à `false` dans son `Drop` ; il ne joint pas le thread, parce qu'attendre un thread de capture bloquerait le thread d'interface — l'annulation est coopérative et c'est à la tâche de regarder `alive()`. Le corps de la tâche tourne dans un `catch_unwind` qui rapporte « a background task panicked » sur stderr et laisse le reste tourner.

---

### Task 3: Les effets

**Files:**
- Modify: `crates/rustx-dom/src/state.rs`
- Test: `crates/rustx-dom/tests/effect.rs`

**Interfaces:**
- Consumes: tâche 1.
- Produces:
  - `rustx_dom::state::effect_at<D, G>(slot: Slot, watched: D, run: impl FnOnce() -> G)` où `D: PartialEq + 'static` et `G: 'static`
  - La valeur rendue par `run` est un garde : elle est laissée tomber avant la relance suivante, et au démontage.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn an_effect_runs_once_while_what_it_watches_does_not_change() {
    assert_eq!(runs_over_three_renders_of(&[1, 1, 1]), 1);
}

#[test]
fn an_effect_runs_again_when_what_it_watches_changes() {
    assert_eq!(runs_over_three_renders_of(&[1, 2, 2]), 2);
}

#[test]
fn the_previous_guard_is_dropped_before_the_effect_runs_again() {
    // Order matters: a camera has to be closed before the next is opened.
    assert_eq!(order_over_renders_of(&[1, 2]), ["open 1", "close 1", "open 2"]);
}

#[test]
fn two_changes_in_one_frame_run_the_effect_once() {
    // Review Focus 4. The render is what runs effects, and a frame
    // renders once however many times the state moved.
    assert_eq!(runs_for_two_mutations_in_one_frame(), 1);
}

#[test]
fn an_effects_guard_is_dropped_when_the_component_goes() {
    assert_eq!(closed_after_unmount(), 1);
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-dom --test effect`, puis commit `feat(dom): effects that re-run only when what they watch changes`.

**Step 3, la décision :** la cellule d'un effet tient `(D, Option<G>)`. Au rendu, si `D` est égal à celui d'avant, rien ne se passe. Sinon le `G` précédent est laissé tomber **avant** l'appel de `run` — fermer avant d'ouvrir, parce qu'une caméra ne s'ouvre pas deux fois.

---

### Task 4: Le compilateur fournit les emplacements

**Files:**
- Modify: `crates/rustx-compiler/src/rewrite.rs`, `crates/rustx-compiler/src/promote.rs`
- Test: `crates/rustx-compiler/tests/effects.rs`

Le développeur écrit `keep(Camera::open())`, pas `keep_at(Slot { … }, || Camera::open())`. C'est la même mécanique que la promotion du plan 3 : le compilateur sait où se trouve l'appel, donc c'est lui qui le dit.

**Interfaces:**
- Consumes: tâches 1 à 3, plan 3 (`rewrite`).
- Produces: rien de public ; `promote_state` reconnaît trois appels de plus.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn keep_gets_its_slot_from_where_it_is_written() {
    let rust = rewrite_one("fn C() -> Element {\n    let c = keep(Camera::open());\n    return <p>{c.get()}</p>;\n}");
    assert!(rust.contains("keep_at"), "got {rust}");
    assert!(rust.contains("line: 2u32"), "got {rust}");
}

#[test]
fn effect_gets_its_slot_too() {
    let rust = rewrite_one("fn C(id: u8) -> Element {\n    effect(id, || Camera::open(id));\n    return <p>ok</p>;\n}");
    assert!(rust.contains("effect_at"), "got {rust}");
}

#[test]
fn a_keep_outside_a_component_is_left_alone() {
    // `keep` is an ordinary name outside a `fn … -> Element`.
    let source = "fn helper() -> u8 { let keep = 1; keep }";
    assert_eq!(rewrite_one(source), source);
}

#[test]
fn the_rewrite_still_keeps_the_file_the_same_length() {
    let source = "fn C() -> Element {\n    let c = keep(Camera::open());\n    return <p>{c.get()}</p>;\n}\n";
    assert_eq!(transpile(source).unwrap().rust.lines().count(), source.lines().count());
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-compiler --test effects`, puis commit `feat(compiler): give keep, effect and task their call sites`.

**Step 3, la décision :** `keep(EXPR)` → `rustx_dom::state::keep_at(Slot { … }, || EXPR)` ; `effect(WATCHED, CLOSURE)` → `effect_at(Slot { … }, WATCHED, CLOSURE)` ; `task(CLOSURE)` → `keep_at(Slot { … }, || rustx_engine::task::spawn(CLOSURE))`, ce qui est **tout** ce qui lie une tâche à la vie d'un composant : le jeton est une valeur gardée, donc il est laissé tomber au démontage, donc la tâche s'arrête. Seuls les appels à l'intérieur d'un `fn … -> Element` sont touchés.

---

### Task 5: Le magasin partagé

**Files:**
- Create: `crates/rustx-dom/src/store.rs`
- Modify: `crates/rustx-dom/src/lib.rs`
- Test: `crates/rustx-dom/tests/store.rs`

Pour que la liste des périphériques et le panneau d'aperçu voient la même donnée sans qu'un parent la leur passe.

**Interfaces:**
- Consumes: tâche 1, plan 3 (`InstanceId`, `take_dirty`).
- Produces:
  - `rustx_dom::store::Store<T>` — `Clone`, comme `State<T>` mais sans propriétaire
  - `rustx_dom::store::store<T: Default + 'static>() -> Store<T>` — le magasin de ce type, créé au premier appel
  - `Store::<T>::get/set/update/with`, de même forme que `State`
  - `rustx_dom::store::reset()` — vide les magasins, pour les tests

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn two_components_asking_for_the_same_type_get_the_same_store() {
    let _a = Scope::enter(InstanceId(1));
    store::<u32>().set(5);
    drop(_a);
    let _b = Scope::enter(InstanceId(2));
    assert_eq!(store::<u32>().get(), 5);
}

#[test]
fn mutating_a_store_marks_everyone_who_read_it() {
    // Review Focus 3, the first half.
    read_from(InstanceId(1));
    read_from(InstanceId(2));
    store::<u32>().set(9);
    let mut dirty = take_dirty();
    dirty.sort();
    assert_eq!(dirty, vec![InstanceId(1), InstanceId(2)]);
}

#[test]
fn a_component_that_never_read_the_store_is_left_alone() {
    read_from(InstanceId(1));
    let _ = Scope::enter(InstanceId(2)); // renders, reads nothing
    store::<u32>().set(9);
    assert_eq!(take_dirty(), vec![InstanceId(1)]);
}

#[test]
fn an_unmounted_reader_is_forgotten_rather_than_kept_alive() {
    // Review Focus 3, the second half: a store must not be a leak of
    // every component that ever looked at it.
    read_from(InstanceId(1));
    read_from(InstanceId(2));
    drop_instance(InstanceId(2));
    store::<u32>().set(9);
    assert_eq!(take_dirty(), vec![InstanceId(1)]);
}
```

- [ ] **Step 2 à 5** : `cargo test -p rustx-dom --test store`, puis commit `feat(dom): a store two distant components can share`.

**Step 3, la décision :** les magasins vivent dans un `thread_local` indexé par `TypeId`. Chaque magasin garde la liste des `InstanceId` qui l'ont lu ; `drop_instance` la nettoie, de sorte qu'un magasin n'est jamais une raison pour une instance morte de rester référencée. Lire pendant un rendu inscrit le lecteur ; lire hors d'un rendu n'inscrit personne.

---

### Task 6: Le logiciel de preuve

**Files:**
- Create: `apps/monitor/` (`Cargo.toml`, `build.rs`, `app/page.rsx`, `src/main.rs`, `assets/index.html`, `tests/`)
- Modify: `Cargo.toml`

Une horloge alimentée par un thread de fond, lue par deux composants éloignés, et qui s'arrête quand on la démonte. C'est la forme exacte du panneau de capture du logiciel de preuve, sans le matériel — celui-ci arrive avec la couche OS.

- [ ] **Step 1: Écrire les tests qui échouent**

```rust
#[test]
fn a_background_thread_drives_what_is_on_screen() {
    assert!(report().get("ticks").parse::<u32>().unwrap() >= 3);
}

#[test]
fn two_distant_components_show_the_same_value() {
    assert_eq!(report().get("panel"), report().get("header"));
}

#[test]
fn unmounting_the_panel_stops_the_thread_it_started() {
    assert_eq!(report().get("ticking_after_unmount"), "no");
}
```

- [ ] **Step 2 à 7** : comme le plan 3 — `cargo test -p monitor`, puis `cargo test --workspace -- --test-threads=1`, puis commit `feat(apps): a monitor fed by a background thread`.

**Step 3, la page :**

```rust
fn Panel() -> Element {
    let ticks = store::<Ticks>();

    task(move |alive| {
        let post = poster();
        while alive() {
            std::thread::sleep(Duration::from_millis(50));
            let _ = post.post(|| store::<Ticks>().update(|t| t.0 += 1));
        }
    });

    return <p id="panel">{ticks.get().0}</p>;
}
```

Aucun `use_effect`, aucun tableau de dépendances, aucun `move` écrit à la main autour de l'état : la tâche est une valeur gardée par le composant, donc elle s'arrête quand il part.

---

## Ce que ce plan livre, et ce qui vient après

À la fin, le §7.4 bis est entier : props, listes à clé, montage et démontage, effets et tâches asynchrones, état partagé. Un composant peut ouvrir une ressource et la refermer sans qu'on y pense, et deux composants éloignés voient la même donnée.

Ensuite : le routing par fichiers et les layouts — le confort de Next.js, pas son runtime — puis l'interopérabilité React et npm, la couche OS, la CLI et le hot reload, et l'extension VS Code en dernier.
