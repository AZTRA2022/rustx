# L'asynchrone dans un composant

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Écrire `task(async { … })` dans un `.rsx`, y utiliser `.await` et n'importe quelle bibliothèque async de l'écosystème (`tokio`, `reqwest`, `sqlx`, un `Future` de `std`), écrire l'état depuis ce bloc comme on l'écrit depuis un `onClick`, et voir le travail s'arrêter quand le composant quitte l'écran.

**Architecture:** Un composant reste synchrone — on lui demande l'écran tel qu'il est maintenant, il ne peut pas attendre. Ce qui attend, c'est une tâche : `task(…)` existe déjà et rend un jeton gardé dans une cellule du composant, dont le `Drop` annule le travail. Ce plan lui donne une seconde forme, un `Future` au lieu d'une closure, exécuté par un runtime `tokio` qui vit sur ses propres threads et jamais sur celui du rendu. Et il ferme la boucle du côté de l'état : une mutation écrite **dans un bloc `async`** promeut la variable, exactement comme une mutation écrite dans un gestionnaire, et l'écriture repart vers le thread de l'UI par la file de frames — donc elle est coalescée avec les autres, donc mille `await` qui écrivent ne font pas mille rendus.

**Tech Stack:** Rust ; `tokio` (`rt-multi-thread`, `time`) comme runtime, parce que c'est le réacteur dont les bibliothèques de l'écosystème ont besoin et qu'un exécuteur écrit à la main ne le remplace pas ; la file de frames et `Poster` déjà en place pour le retour vers l'UI.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.4 (« Effets et tâches asynchrones », et la règle de promotion, qui nomme déjà le bloc asynchrone), §4 (le modèle de threads).

## Global Constraints

- L'état n'appartient qu'au thread du rendu. Un `Future` ne le touche jamais directement : il écrit par un handle `Send` qui poste sur la file de frames, et c'est la frame qui applique.
- Le runtime ne démarre pas tant que personne ne l'a demandé : une application sans `async` ne paie pas un thread de plus.
- `tokio` n'est une dépendance que de `rustx-engine` et de la façade `rustx`. Le compilateur et `rustx-dom` n'en dépendent pas.
- Un composant ne devient jamais `async` : `async fn Page()` reste refusé en toutes lettres (déjà le cas), et le refus nomme `task(…)`.
- Aucun `block_on` sur le thread du rendu, jamais : ce serait une fenêtre gelée, qui est précisément le défaut que ce plan existe pour éviter.
- Le jeton annule : laisser tomber le `Task` abandonne le `Future` et refuse ce qu'il avait encore à dire, parce que le composant qui l'aurait lu n'est plus là.

## Review Focus

1. Une fenêtre qu'on ferme pendant qu'un `await` est en vol : ce que la tâche poste ensuite doit être refusé, pas appliqué à une page démontée ni perdu dans un panic.
2. Un `Future` qui panique : la tâche meurt, le runtime et la fenêtre vivent, et quelque chose le dit sur `stderr`.
3. Mille écritures d'état depuis une boucle `async` : une frame, pas mille rendus — le même contrat que pour un thread de `task` aujourd'hui.
4. Un composant démonté pendant que la tâche tient encore une écriture : la cellule n'existe plus, l'écriture doit être jetée sans panic.
5. Un `await` sur un timer `tokio` alors que la fenêtre ne produit aucune frame (le GPU de cette machine) : le réveil vient du runtime, pas du compositeur, donc il doit arriver quand même.

---

### Task 1 : Un runtime dans le processus de rendu

**Files:**
- Create: `crates/rustx-engine/src/runtime.rs`
- Modify: `crates/rustx-engine/src/lib.rs` (le module), `crates/rustx-engine/Cargo.toml` (`tokio`), `crates/rustx-engine/src/task.rs` (`spawn_future`)
- Test: `crates/rustx-engine/tests/runtime.rs`

**Interfaces:**
- Produces: `runtime::handle() -> tokio::runtime::Handle` (démarre le runtime au premier appel, jamais avant) ; `runtime::started() -> bool` ; `runtime::stop()` appelé par l'arrêt de la fenêtre ; `task::spawn_future<F: Future<Output = ()> + Send + 'static>(future: F) -> Task`, qui rend le même `Task` que `task::spawn` — donc le même `Drop` annule.
- Consumes: `frame::poster`, `frame::PostError` (déjà en place).

- [ ] **Step 1** : les tests — `a_future_runs_and_says_so`, `dropping_the_token_abandons_the_future_before_it_finishes` (un `Future` qui attend 5 s puis écrit un drapeau ; le jeton est lâché, le drapeau reste faux), `nothing_starts_a_runtime_until_a_future_asks_for_one` (`runtime::started()` est faux après un `spawn` de closure, vrai après un `spawn_future`), `a_future_that_panics_leaves_the_runtime_alive` (un second `Future` tourne après).
- [ ] **Step 2** : rouge (`cargo test -p rustx-engine --test runtime`).
- [ ] **Step 3** : le runtime (`tokio::runtime::Builder::new_multi_thread`, `enable_time`, un `OnceLock`), et `spawn_future` qui garde l'`AbortHandle` dans le `Shared` de `Task` pour que `Drop` l'abandonne en plus de baisser le drapeau `alive`.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(engine): a future runs beside the window, and stops with its component`.

---

### Task 2 : `task(async { … })` depuis un `.rsx`

**Files:**
- Modify: `crates/rustx-compiler/src/rewrite.rs` (l'émission de `CallKind::Task`)
- Test: `crates/rustx-compiler/tests/effects.rs`

**Interfaces:**
- Consumes: `task::spawn_future` (Task 1).
- Produces: pour `task(async move { … })`, le compilateur écrit `keep_at(slot, || rustx_engine::task::spawn_future(async move { … }))` ; pour `task(|alive| …)`, il écrit `spawn` comme aujourd'hui. Le choix se lit sur le premier mot de l'argument : `async`.

- [ ] **Step 1** : les tests — `a_task_whose_argument_is_an_async_block_spawns_a_future` (le Rust émis contient `spawn_future`), `a_task_whose_argument_is_a_closure_still_spawns_a_thread` (il contient `spawn (` et pas `spawn_future`), `an_async_task_keeps_the_line_count_of_what_it_replaced` (la carte de positions du plan 2 : autant de `\n` qu'avant).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la lecture du premier mot de l'argument et la seconde forme d'émission.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(compiler): a task may be a future`.

---

### Task 3 : L'état s'écrit depuis un bloc `async`

**Files:**
- Modify: `crates/rustx-dom/src/state.rs` (le handle `Send`), `crates/rustx-compiler/src/promote.rs` (les blocs `async` comptent comme des mutations), `crates/rustx-compiler/src/rewrite.rs` (la réécriture d'un bloc `async`)
- Test: `crates/rustx-dom/tests/remote.rs`, `crates/rustx-compiler/tests/rewrite.rs`

**Interfaces:**
- Produces: `State<T>::remote() -> Remote<T>` où `Remote<T>` est `Send + Clone` et ne porte que l'identité de la cellule (`InstanceId` + `Slot`) ; `Remote<T>::set(value)` et `Remote<T>::update(impl FnOnce(&mut T) + Send + 'static)` postent sur la file de frames, trouvent la cellule à l'arrivée et **jettent l'écriture sans panic** si l'instance est partie. `rustx_dom::state::apply_remote(job)` est le point d'entrée que l'hôte appelle pour les appliquer.
- Consumes: `frame::poster` (Task 1 ne le change pas), la promotion existante.
- Un bloc `async` est traité comme un gestionnaire : une variable lue dans le JSX et mutée dans un bloc `async` est promue ; dans le bloc, la capture est un `Remote` et `n += 1` devient `n.update(|n| *n += 1)`.

- [ ] **Step 1** : les tests — `a_remote_write_lands_on_the_next_frame`, `a_remote_write_to_a_dropped_instance_is_thrown_away`, `a_thousand_remote_writes_cost_one_render` (Review Focus 3 : `renders()` augmente de 1), `a_write_in_an_async_block_promotes_the_local` et `a_write_in_an_async_block_goes_through_the_remote` (le Rust émis contient `remote` et pas `+= 1` nu), `a_remote_write_after_the_window_closed_is_refused` (Review Focus 1).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : `Remote`, l'application à la frame, et l'extension de la promotion aux blocs `async`.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(dom): state written from a future lands on a frame`.

---

### Task 4 : La preuve dans une application

**Files:**
- Create: `apps/hybrid/app/async/page.rsx` (la page qui attend), `apps/hybrid/tests/async.rs`
- Modify: `apps/hybrid/assets/index.html` (le scénario), `crates/rustx/Cargo.toml` et `crates/rustx/src/lib.rs` (la réexportation de `tokio` et de `spawn_future`, pour qu'une page n'ait jamais à nommer `rustx-engine`)
- Test: `apps/hybrid/tests/async.rs`

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: une page qui écrit `let mut rows = Vec::new();` en haut, lance `task(async move { … })` au montage, attend trois fois `rustx::time::sleep`, écrit l'état à chaque réveil, et affiche la liste. La fenêtre de cette machine ne produit aucune frame : le réveil doit venir du runtime (Review Focus 5).

- [ ] **Step 1** : le scénario rapporte `rows_after_three_awaits=3`, `renders_for_a_thousand_writes=1`, et `alive_after_leaving=false` après une navigation qui démonte la page (Review Focus 4).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la page, le scénario, les réexportations.
- [ ] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [ ] **Step 5** : commit `feat(hybrid): a page that waits, and a window that does not`.

---

### Task 5 : Ce qu'un développeur lit

**Files:**
- Modify: `README.md`, `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` (§7.4 : la forme retenue)
- Test: `crates/rustx/tests/documented.rs`

**Interfaces:**
- Produces: la page de documentation de l'asynchrone — `task(async { … })`, l'écriture d'état depuis un `await`, l'annulation au démontage, et la phrase qui dit pourquoi un composant n'est pas `async`. Le test lit les exemples du README et les compile par `rustx_compiler::transpile::transpile` : un exemple faux dans la documentation est un bug.

- [ ] **Step 1** : `every_async_example_in_the_readme_compiles`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la documentation.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `docs: what await looks like in a .rsx`.

---

## Ce que ce plan livre

`task(async { … })` dans un `.rsx`, `.await` sur n'importe quelle bibliothèque de l'écosystème, l'état écrit depuis un `Future` comme depuis un `onClick`, et le travail qui s'arrête quand le composant s'en va. Un composant reste synchrone, et c'est ce qui garde la fenêtre vivante.
