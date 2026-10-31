# La couche OS : ce que le web ne sait pas faire

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Presse-papiers, dialogues de fichiers natifs, notifications, icône et menu de barre système, raccourcis globaux, glisser-déposer depuis l'explorateur, multi-fenêtres, instance unique, démarrage automatique — appelés depuis un `.rsx` en Rust, et refusés en une phrase nommant la plateforme là où ils n'existent pas encore.

**Architecture:** Ce plan est le complément exact de `2026-10-31-les-medias-par-chromium.md` : là, Chromium savait faire et le framework n'a écrit que la politique et l'API ; ici, le web **ne sait pas** faire — une notification web exige un service worker et n'a pas l'air native, il n'y a pas d'API d'icône de barre système, pas de raccourci global, pas de seconde fenêtre native. Donc c'est du natif, dans `rustx-os`, une capacité par module, chacune avec son `supported()`, son chemin Windows, et une couture nommée pour macOS et Linux.

**Tech Stack:** Rust, le crate `windows` (déjà là, confiné à `rustx-os`), et CEF pour ce que CEF fait mieux que Win32 : `ImplBrowserHost::run_file_dialog` est un dialogue natif sur les trois plateformes, donc les dialogues passent par lui plutôt que par `GetOpenFileNameW`.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.6 (la liste des capacités, et l'unification comme apport du framework), §7.4 (le magasin, pour le partage entre fenêtres), anti-objectif n°1 (un logiciel du commerce).

## Global Constraints

- **Une capacité, un module, un `supported()`.** `clipboard`, `dialog`, `notify`, `tray`, `shortcut`, `windows`, `instance`, `autostart`. Chacun expose `supported() -> bool` et refuse en nommant la plateforme là où il n'est pas implémenté — jamais un `unimplemented!()`, jamais un silence.
- **Rien ne panique.** `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` sur `rustx-os`, et chaque erreur nomme la chose : le format du presse-papiers, le fichier, la touche, la fenêtre.
- **Chaque chose enregistrée auprès du système est un jeton.** Une icône de barre système, un raccourci global, une fenêtre : leur `Drop` les retire. Un raccourci qui survit à l'application est un raccourci que l'utilisateur ne peut plus récupérer sans redémarrer.
- **Le thread compte.** Une icône de barre système et un raccourci global ont besoin d'une boucle de messages : ils vivent sur le thread de l'interface du processus navigateur, et l'API les y poste. Un test qui les crée ailleurs doit échouer avec une phrase, pas se taire.
- **Aucun test n'exige une main humaine.** Ce qui ouvre une fenêtre modale (les dialogues) est prouvé par son refus et par un exemple à lancer à la main ; tout le reste — presse-papiers, icône, raccourci, instance unique — est vérifiable sans personne devant l'écran.
- Windows d'abord. macOS et Linux compilent, et chaque module dit en une phrase ce qui manque chez eux.
- **Un répertoire par système, un fichier par capacité dedans.** `src/<capacité>.rs` est l'API — les types, les mots, aucune plateforme — et `src/windows/<capacité>.rs`, `src/linux/<capacité>.rs`, `src/other/mod.rs` sont les implémentations, avec les mêmes noms de fonctions et une constante `AVAILABLE`. Rien au-dessus de `platform` ne nomme un système. Les tests compilent aussi `linux/` et `other/` sur la machine qui les lance, pour qu'un changement d'API casse ici et non chez quelqu'un d'autre. (Posé à la tâche 1 ; les tâches suivantes s'y conforment.)

## Review Focus

1. **Le presse-papiers tenu par une autre application** (Excel l'a ouvert) : la lecture et l'écriture réessaient brièvement puis refusent en une phrase, au lieu d'échouer sur la première tentative comme le fait `OpenClipboard`. Test Task 1 (`a_clipboard_somebody_else_holds_is_retried_then_refused`).
2. **Deux fenêtres qui lisent le même magasin** : un `let mut` partagé écrit dans l'une se voit dans l'autre, et la fermeture de l'une ne touche pas à ce que l'autre tient. Test Task 6 (`two_windows_share_one_store`, `closing_one_window_leaves_the_other_alone`).
3. **Un raccourci global que le système refuse** (déjà pris par une autre application) : refusé en nommant la touche, et l'application peut en proposer un autre — jamais un enregistrement qui a l'air d'avoir marché. Test Task 5 (`a_shortcut_somebody_else_has_is_refused_by_name`).
4. **Une seconde instance lancée** (double-clic sur le `.exe` alors qu'il tourne) : la première fenêtre revient au premier plan avec les arguments de la seconde, et la seconde s'arrête sans rien dire d'effrayant. Test Task 6 (`a_second_instance_hands_over_and_leaves`).
5. **Un fichier glissé depuis l'explorateur** : le chemin arrive à la page, même pour un chemin accentué ou un dossier, et un glisser-déposer de texte n'est pas confondu avec un fichier. Test Task 4 (`a_dropped_file_arrives_with_its_real_path`).

---

### Task 1 : Le presse-papiers

**Files:**
- Create: `crates/rustx-os/src/clipboard.rs`
- Modify: `crates/rustx-os/src/lib.rs`
- Test: `crates/rustx-os/tests/clipboard.rs`

**Interfaces:**
- Produces: `clipboard::text() -> Result<String, OsError>` ; `clipboard::set_text(&str) -> Result<(), OsError>` ; `clipboard::image() -> Result<Image, OsError>` et `clipboard::set_image(&Image)` où `struct Image { pub width: u32, pub height: u32, pub bgra: Vec<u8> }` ; `clipboard::set_custom(format: &str, bytes: &[u8])` et `clipboard::custom(format: &str) -> Result<Vec<u8>, OsError>` — un format nommé, enregistré par `RegisterClipboardFormatW`, pour qu'un gros projet puisse copier ses propres objets entre deux de ses fenêtres ; `clipboard::supported() -> bool`.
- L'ouverture réessaie pendant 200 ms par pas de 10 ms avant de refuser (Review Focus 1).

- [ ] **Step 1** : les tests — `text_written_is_text_read` (aller-retour, avec des accents et un emoji), `an_image_survives_the_round_trip` (une image 2×2 BGRA), `a_custom_format_is_read_back_by_the_same_name`, `a_clipboard_somebody_else_holds_is_retried_then_refused` (Review Focus 1 : le test tient le presse-papiers depuis un autre thread et mesure que le refus arrive après ~200 ms, pas tout de suite), `an_empty_clipboard_is_not_an_error_it_is_an_empty_string`.
- [ ] **Step 2** : rouge (`cargo test -p rustx-os --test clipboard`).
- [ ] **Step 3** : `OpenClipboard` et ses amis, la boucle de réessai, les trois formats.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): the clipboard, including a project's own format`.

---

### Task 2 : Les dialogues de fichiers

**Files:**
- Create: `crates/rustx-engine/src/dialog.rs`
- Modify: `crates/rustx/src/lib.rs` (`rustx::dialog`)
- Test: `crates/rustx-engine/tests/dialog.rs`, `apps/capture/tests/app.rs`

**Interfaces:**
- Produces: `dialog::open(Ask) -> Result<Vec<PathBuf>, DialogError>` et `dialog::save(Ask) -> Result<PathBuf, DialogError>` où `struct Ask { pub title: String, pub filters: Vec<(String, Vec<String>)>, pub start_in: Option<PathBuf>, pub many: bool }` ; `enum DialogError { Cancelled, NoWindow, Platform(String) }` — l'annulation est un cas normal, nommé, pas une erreur à journaliser.
- L'implémentation passe par `ImplBrowserHost::run_file_dialog` (natif sur les trois plateformes) et poste le résultat au thread de l'interface.
- `Cancelled` est ce que rend un dialogue fermé : une application qui écrit `let Ok(paths) = dialog::open(…) else { return; }` se comporte juste.

- [ ] **Step 1** : les tests — `a_dialog_without_a_window_is_refused_in_words` (appelé avant que la fenêtre existe), `a_cancelled_dialog_is_cancelled_and_not_an_error`, `the_filters_become_what_cef_expects` (la conversion `("Images", ["png","jpg"])` → la chaîne de CEF, testée sans ouvrir quoi que ce soit), plus un exemple `cargo run -p capture --example choose_a_file` à lancer à la main, qui est dit dans le README.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la conversion, l'appel, le retour.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): native file dialogs`.

---

### Task 3 : Les notifications et la barre système

**Files:**
- Create: `crates/rustx-os/src/tray.rs`, `crates/rustx-os/src/notify.rs`
- Test: `crates/rustx-os/tests/tray.rs`

**Interfaces:**
- Produces: `tray::Icon::new(icon: &Path, tooltip: &str) -> Result<Icon, OsError>` dont le `Drop` retire l'icône ; `Icon::menu(items: &[Item])` où `struct Item { pub id: String, pub label: String, pub checked: Option<bool> }` ; `Icon::clicked() -> Option<String>` (l'identifiant choisi, lu depuis la file du thread de l'interface) ; `notify::show(What) -> Result<(), OsError>` où `struct What { pub title: String, pub body: String, pub icon: Option<PathBuf> }`.
- La notification passe par `Shell_NotifyIconW` avec `NIF_INFO` plutôt que par les toasts WinRT : un toast exige un AppUserModelID enregistré dans le menu Démarrer, ce qu'un `rustx dev` n'a pas et ne doit pas exiger. La limite est dite dans la documentation, avec la ligne qui la lèvera (un `.exe` installé peut s'enregistrer).
- Les deux modules vivent sur le thread de l'interface et refusent ailleurs en une phrase.

- [ ] **Step 1** : les tests — `an_icon_appears_and_its_drop_removes_it` (l'icône est créée, `Shell_NotifyIconW` répond vrai, et la suppression aussi), `a_menu_item_that_was_chosen_is_reported_once`, `a_notification_does_not_block` (l'appel rend la main en moins de 50 ms), `an_icon_file_that_is_not_an_ico_is_refused_in_words` (le même lecteur d'en-tête que la fenêtre), `a_tray_icon_from_the_wrong_thread_is_refused_in_words`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : `Shell_NotifyIconW`, le menu, la file des clics.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): a tray icon, a menu, and a notification`.

---

### Task 4 : Le glisser-déposer depuis l'explorateur

**Files:**
- Create: `crates/rustx-engine/src/dropped.rs`
- Modify: `crates/rustx-engine/src/client.rs` (le gestionnaire de glisser-déposer), `crates/rustx/src/lib.rs`
- Test: `crates/rustx-engine/tests/dropped.rs`, `apps/catalogue/tests/app.rs`

**Interfaces:**
- Produces: `dropped::on_files(hook: impl Fn(Vec<PathBuf>) + 'static)` — appelé sur le thread de l'interface quand des fichiers sont lâchés sur la fenêtre ; `dropped::files() -> Vec<PathBuf>` pour une page qui préfère lire que s'abonner.
- Ce que le web donne (`DataTransfer`) ne contient pas le chemin réel d'un fichier ; c'est tout l'intérêt de passer par le gestionnaire natif de CEF.

- [ ] **Step 1** : les tests — `a_dropped_file_arrives_with_its_real_path` (Review Focus 5 : un chemin accentué, par le gestionnaire appelé directement avec un `DragData` fabriqué), `a_dropped_folder_is_a_path_too`, `dropped_text_is_not_mistaken_for_a_file`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le gestionnaire, le crochet.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): files dropped on the window, with their real paths`.

---

### Task 5 : Les raccourcis globaux

**Files:**
- Create: `crates/rustx-os/src/shortcut.rs`
- Test: `crates/rustx-os/tests/shortcut.rs`

**Interfaces:**
- Produces: `shortcut::listen(keys: &str) -> Result<Shortcut, OsError>` où `keys` s'écrit `"Ctrl+Shift+R"` ; `Shortcut::pressed(&self) -> u64` (combien de fois depuis la dernière lecture) ; le `Drop` désenregistre ; `shortcut::parse(keys: &str) -> Result<(u32, u32), OsError>` — modificateurs et touche virtuelle, séparés pour être testables sans rien enregistrer.
- Review Focus 3 : `RegisterHotKey` qui échoue devient `OsError::Busy("Ctrl+Shift+R")`, avec la phrase qui dit qu'une autre application l'a.

- [ ] **Step 1** : les tests — `a_shortcut_is_read_from_how_it_is_written` (`parse` sur `"Ctrl+Shift+R"`, `"Alt+F4"`, `"Win+Space"`, et `"Ctrl+"` refusé en une phrase), `a_shortcut_somebody_else_has_is_refused_by_name` (Review Focus 3 : le test enregistre la même combinaison deux fois, le second refus nomme la touche), `dropping_a_shortcut_gives_the_keys_back` (le second enregistrement réussit après le `Drop` du premier), `a_shortcut_nobody_pressed_counts_zero`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la lecture des touches, l'enregistrement, le compte.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): global shortcuts, given back when dropped`.

---

### Task 6 : Plusieurs fenêtres, une seule instance

**Files:**
- Create: `crates/rustx-engine/src/second_window.rs`, `crates/rustx-os/src/instance.rs`, `crates/rustx-os/src/autostart.rs`
- Modify: `crates/rustx/src/lib.rs` (`rustx::window::open`, `Config::single_instance`, `Config::autostart`)
- Test: `crates/rustx-engine/tests/second_window.rs`, `crates/rustx-os/tests/instance.rs`, `apps/monitor/tests/app.rs`

**Interfaces:**
- Produces: `window::open(What) -> Result<Window, EngineError>` où `struct What { pub title: String, pub width: u32, pub height: u32, pub at: String }` (`at` est la route sur laquelle la fenêtre s'ouvre) ; `Window::close(self)` et un `Drop` qui ferme ; `instance::only_one(name: &str) -> Result<Only, OsError>` dont l'échec porte les arguments de la première instance ; `instance::handed_over() -> Vec<String>` ; `autostart::set(on: bool, name: &str) -> Result<(), OsError>` et `autostart::is_on(name: &str) -> bool`.
- Les deux fenêtres partagent le magasin : elles vivent dans le même processus de rendu, donc un `let mut` partagé est le même objet — ce que la §7.4 annonçait comme le travail de la couche OS se trouve être, dans cette architecture, déjà vrai. Le test le prouve plutôt que la documentation ne le promette.

- [ ] **Step 1** : les tests — `a_second_window_opens_on_the_route_it_was_given`, `two_windows_share_one_store` (Review Focus 2), `closing_one_window_leaves_the_other_alone` (Review Focus 2), `a_second_instance_hands_over_and_leaves` (Review Focus 4 : deux processus, un mutex nommé, les arguments qui traversent), `autostart_is_set_and_unset_and_nothing_is_left_behind` (la clé de registre est retirée).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : la seconde fenêtre, le mutex nommé, la clé `Run`.
- [ ] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [ ] **Step 5** : commit `feat(os): a second window, and only one instance`.

---

## Ce que ce plan livre

Les capacités qu'un logiciel du commerce a et qu'une page web n'a pas, sous une API Rust unique, documentée au même endroit — ce que la §7.6 appelle l'apport du framework — avec un jeton par chose enregistrée auprès du système, de sorte qu'une application qui s'arrête ne laisse derrière elle ni icône, ni raccourci, ni clé de registre.

## Ce que ce plan ne fait pas

- macOS et Linux : chaque module compile et nomme ce qui lui manque. Les coutures sont là ; les remplir est un plan par plateforme, et il demande la plateforme sous la main.
- L'impression, le registre au-delà du démarrage automatique, les services Windows, et l'USB/HID brut au-delà de ce que `WebUSB` et `WebHID` donnent déjà par Chromium.
