# La couche OS : les périphériques, la caméra, l'écran

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `apps/capture` énumère les périphériques réellement branchés, ouvre une caméra, affiche son flux à l'écran sans qu'une image ne traverse une sérialisation, et enregistre dans un fichier — le critère d'acceptation de la spec §8, mesuré par un test.

**Architecture:** Un crate `rustx-os`, sans CEF et sans DOM, qui ne connaît que le matériel : l'énumération par Media Foundation, un `Camera` qui pousse ses images depuis son propre thread, un `Screen` qui fait la même chose avec un écran, un `Recording` qui écrit un `.mp4`. Puis une seule pièce dans le moteur, `pixels`, qui donne les octets d'une image à V8 **sans copie** (`cef_v8_value_create_array_buffer` avec son callback de libération) pour qu'un `<canvas>` les affiche. C'est là que se joue la thèse de la spec §7.1 : le thread de rendu et le code de l'application sont dans le même processus, donc une image de 1080p est un pointeur, pas un message.

**Tech Stack:** Rust. Une dépendance nouvelle, `windows` (les liaisons officielles de Microsoft), confinée à `rustx-os` : la spec §7.6 demande explicitement de s'appuyer sur un crate existant quand il en existe un bon, et écrire les vtables COM de Media Foundation à la main est exactement le genre de travail que ce projet n'a aucune raison de refaire.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.6 (la couche OS), §8 (le logiciel de preuve et ses chiffres), §7.1 (pourquoi une image ne traverse rien), anti-objectifs n°1 et n°2.

## Global Constraints

- Windows d'abord. Les autres plateformes compilent et refusent en une phrase qui nomme la plateforme — pas un `unimplemented!()`, pas un silence.
- `windows` est la seule dépendance nouvelle, et elle ne figure que dans le `Cargo.toml` de `rustx-os`. Le compilateur, le DOM et la façade n'en entendent jamais parler.
- Aucune image ne traverse une frontière sérialisée. Une image qui arrive à l'écran est passée à V8 en tant que `ArrayBuffer` sur la mémoire du frame, libérée par son callback.
- Une machine sans caméra ne fait échouer aucun test. Ce qui a besoin de matériel le dit et s'arrête ; ce qui prouve le chemin complet utilise la source synthétique de la Task 2.
- Les chiffres de la spec §8 — 1080p à 30 images/s sans image perdue, latence capteur→écran sous 100 ms, interface à 60 images/s pendant la capture — sont mesurés par un test qui les imprime, pas affirmés dans un commentaire.
- `Drop` reste le seul nettoyage : fermer une caméra, arrêter un thread, finaliser un fichier sont des `Drop`, jamais un `close()` que l'application doit penser à appeler.

## Review Focus

1. **Aucune caméra, ou une caméra déjà prise par une autre application** (Teams tourne) : la liste la montre et dit qu'elle est occupée, l'ouverture refuse en une phrase. Test dans la Task 1 (`a_device_that_is_busy_is_still_listed`) et la Task 2 (`a_camera_that_is_busy_is_refused_in_words`).
2. **Une caméra débranchée pendant le flux** : le flux se termine, l'application l'apprend par la fin du flux et non par un plantage, et la page affiche la dernière image. Test dans la Task 2 (`a_camera_that_goes_away_ends_its_stream`).
3. **Un format que la caméra n'offre pas** (1080p60 demandé à une webcam 720p30) : le plus proche de ce qu'elle offre, dit à voix haute, jamais un échec silencieux ni une image étirée. Test dans la Task 2 (`a_format_nobody_offers_becomes_the_nearest_one_offered`).
4. **Un enregistrement vers un chemin qu'on ne peut pas écrire, ou un disque plein** : il s'arrête, nomme le fichier, et ce qui a déjà été écrit reste un fichier lisible. Test dans la Task 5 (`a_recording_that_cannot_be_written_says_which_file`, `what_was_written_before_the_trouble_still_plays`).
5. **Quitter l'aperçu pendant que des images sont en vol** : aucune image n'est posée sur un élément qui n'existe plus, et le thread s'arrête au `Drop` du jeton. Test dans la Task 3 (`a_frame_in_flight_when_the_screen_leaves_is_dropped`) et la Task 6 (`leaving_the_preview_closes_the_camera`).

---

### Task 1 : Ce qui est branché

**Files:**
- Create: `crates/rustx-os/Cargo.toml`, `crates/rustx-os/src/lib.rs`, `crates/rustx-os/src/devices.rs`, `crates/rustx-os/src/error.rs`
- Modify: `Cargo.toml` (le membre)
- Test: `crates/rustx-os/tests/devices.rs`

**Interfaces:**
- Produces: `enum Kind { Camera, Microphone, Screen }` ; `struct Device { pub id: String, pub name: String, pub kind: Kind, pub busy: bool }` ; `fn devices() -> Vec<Device>` et `fn devices_of(kind: Kind) -> Vec<Device>` ; `enum OsError { NoSuchDevice(String), Busy(String), Unsupported(&'static str), Platform(String) }` avec un `Display` qui dit quoi faire, et `impl std::error::Error`.
- L'`id` d'une caméra est le lien symbolique Media Foundation (`\\?\usb#vid_…`), celui d'un écran son nom de périphérique : stable d'une énumération à l'autre, ce qu'un `/device/[id]` dans une URL demande.

- [ ] **Step 1** : les tests — `every_device_has_an_id_and_a_name` (ce qui est là est nommé, et aucun nom n'est vide), `a_machine_without_a_camera_lists_none_and_does_not_fail` (Global Constraint : aucun matériel requis), `a_device_that_is_busy_is_still_listed` (Review Focus 1 : `busy` est un état, pas une absence), `the_screens_are_devices_too` (au moins un écran sur une machine qui a un écran), `an_id_survives_two_enumerations`.
- [ ] **Step 2** : rouge (`cargo test -p rustx-os`).
- [ ] **Step 3** : l'énumération — `MFEnumDeviceSources` pour les caméras et les micros, `EnumDisplayDevicesW` pour les écrans ; `busy` est ce que dit une ouverture-fermeture immédiate.
- [ ] **Step 4** : vert, et `cargo test -p rustx-os -- --nocapture` imprime ce qu'il a trouvé sur cette machine.
- [ ] **Step 5** : commit `feat(os): what is plugged in`.

---

### Task 2 : Une caméra qui pousse ses images

**Files:**
- Create: `crates/rustx-os/src/camera.rs`, `crates/rustx-os/src/frame.rs`
- Test: `crates/rustx-os/tests/camera.rs`

**Interfaces:**
- Produces: `struct Frame { pub width: u32, pub height: u32, pub bytes: Vec<u8>, pub taken: std::time::Instant }` — les octets sont du BGRA, dans cet ordre, parce que c'est ce qu'un `<canvas>` prend ; `struct Wanted { pub width: u32, pub height: u32, pub fps: u32 }` et `Wanted::hd()` = 1920×1080 à 30 ; `struct Format { pub width: u32, pub height: u32, pub fps: u32 }` ; `Camera::open(id: &str, wanted: Wanted) -> Result<Camera, OsError>` ; `Camera::format(&self) -> Format` ; `Camera::frames(self, each: impl FnMut(Frame) -> Keep + Send + 'static) -> Streaming` où `enum Keep { Going, Enough }` ; `struct Streaming` dont le `Drop` arrête le thread et ferme la caméra ; `Camera::synthetic(wanted: Wanted) -> Camera` — une mire qui bouge, pour que le chemin complet soit prouvable sur une machine sans caméra.
- Consumes: `OsError`, `Device` (Task 1).

- [ ] **Step 1** : les tests — `a_synthetic_camera_delivers_thirty_frames_in_a_second` (±2, et chaque image fait `width * height * 4` octets), `a_camera_that_is_busy_is_refused_in_words` (Review Focus 1), `a_format_nobody_offers_becomes_the_nearest_one_offered` (Review Focus 3, sur la synthétique : 4000×3000 demandés deviennent ce qui existe, et `format()` le dit), `a_camera_that_goes_away_ends_its_stream` (Review Focus 2 : la source synthétique sait disparaître sur demande), `dropping_the_stream_stops_the_thread_within_a_frame` (le thread est mort en moins de 100 ms après le `Drop`), `nothing_is_delivered_after_enough` (`Keep::Enough` est le dernier mot).
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : `IMFSourceReader` sur le thread du flux, négociation du format le plus proche, conversion NV12/YUY2 → BGRA dans `frame.rs`.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): a camera that pushes its frames`.

---

### Task 3 : Une image arrive à l'écran sans traverser quoi que ce soit

**Files:**
- Create: `crates/rustx-engine/src/pixels.rs`
- Modify: `crates/rustx-engine/src/lib.rs` (le module), `crates/rustx-engine/Cargo.toml` (la dépendance `rustx-os`, pour le type `Frame` et rien d'autre), `crates/rustx/src/lib.rs` (`rustx::pixels::show` et `rustx::os`)
- Test: `crates/rustx-engine/tests/pixels.rs`, `apps/capture/tests/app.rs`

**Interfaces:**
- Produces: `pixels::show(canvas_id: &str, frame: &Frame) -> Result<(), DomError>` — trouve le `<canvas>`, et lui passe les octets en `ArrayBuffer` construit par `cef_v8_value_create_array_buffer` sur la mémoire du frame, avec un callback de libération qui garde le `Frame` en vie jusqu'à ce que V8 n'en veuille plus ; `pixels::showing(canvas_id: &str) -> Showing`, un jeton dont le `Drop` oublie le canvas, pour qu'une image en vol ne soit pas posée sur un élément disparu.
- Consumes: `Frame` (Task 2), `crate::dom` (les poignées), `Alive::post` (le passage au thread de l'interface).

- [ ] **Step 1** : les tests — `the_bytes_reach_the_canvas_without_a_copy` (l'adresse que V8 voit est celle du `Frame` : le test compare le pointeur), `the_frame_lives_until_v8_lets_go` (le callback de libération est appelé, et pas avant), `a_frame_in_flight_when_the_screen_leaves_is_dropped` (Review Focus 5 : après le `Drop` du jeton, `show` refuse en une phrase au lieu d'écrire), `a_canvas_that_is_not_there_is_a_sentence_not_a_crash`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le `ArrayBuffer`, le callback, le `putImageData` du côté script.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(engine): a frame reaches the screen without being copied`.

---

### Task 4 : L'écran est une source comme une autre

**Files:**
- Create: `crates/rustx-os/src/screen.rs`
- Test: `crates/rustx-os/tests/screen.rs`

**Interfaces:**
- Produces: `Screen::open(id: &str, wanted: Wanted) -> Result<Screen, OsError>` et `Screen::frames(self, each: impl FnMut(Frame) -> Keep + Send + 'static) -> Streaming` — la même forme que `Camera`, parce qu'un aperçu ne doit pas savoir ce qu'il affiche ; `Screen::primary() -> Result<Screen, OsError>`.
- Consumes: `Frame`, `Keep`, `Streaming`, `Wanted` (Task 2).

- [ ] **Step 1** : les tests — `the_primary_screen_delivers_a_frame_the_size_of_the_screen`, `a_screen_frame_is_bgra_like_a_camera_frame` (la même forme : un aperçu branché sur l'un marche sur l'autre), `a_screen_that_was_unplugged_is_refused_in_words`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : `Windows.Graphics.Capture` quand il est là, `BitBlt` sinon, et la ligne qui dit lequel.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): a screen is a source like any other`.

---

### Task 5 : Enregistrer

**Files:**
- Create: `crates/rustx-os/src/record.rs`
- Test: `crates/rustx-os/tests/record.rs`

**Interfaces:**
- Produces: `Recording::start(path: &Path, format: Format) -> Result<Recording, OsError>` ; `Recording::write(&mut self, frame: &Frame) -> Result<(), OsError>` ; `Recording::written(&self) -> u64` (le nombre d'images) ; le `Drop` finalise le fichier, et un fichier non finalisé est le seul vrai défaut d'un enregistreur.
- Consumes: `Frame`, `Format` (Task 2).

- [ ] **Step 1** : les tests — `a_minute_of_frames_becomes_a_file_that_plays` (sur 60 images synthétiques : le fichier existe, son en-tête est un MP4, et `MFCreateSourceReaderFromURL` le relit avec le bon nombre d'images), `a_recording_that_cannot_be_written_says_which_file` (Review Focus 4 : un dossier qui n'existe pas), `what_was_written_before_the_trouble_still_plays` (Review Focus 4 : le `Drop` finalise même après une écriture refusée), `a_frame_of_the_wrong_size_is_refused_rather_than_written_crooked`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : `IMFSinkWriter`, H.264, horodatage à partir du `taken` du frame.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(os): recording`.

---

### Task 6 : Le logiciel de preuve, pour de vrai

**Files:**
- Modify: `apps/capture/src/devices.rs` (il devient le vrai matériel), `apps/capture/app/page.rsx` (la liste), `apps/capture/app/device/[id]/page.rsx` (l'aperçu, le bouton d'enregistrement), `apps/capture/Cargo.toml`
- Test: `apps/capture/tests/app.rs`
- Create: `apps/capture/tests/numbers.rs`

**Interfaces:**
- Consumes: tout ce qui précède. La page ouvre la caméra dans un `keep(…)`, pousse les images par `pixels::show`, et le `Drop` du jeton ferme le flux — aucune ligne de fermeture dans la page.
- Produces: les chiffres de la spec §8, imprimés par `cargo test -p capture --test numbers -- --nocapture` : images reçues, images perdues, latence médiane et au 95e centile, images par seconde de l'interface pendant la capture.

- [ ] **Step 1** : les tests — dans `app.rs` : `the_list_shows_what_is_plugged_in`, `opening_a_device_shows_its_preview`, `leaving_the_preview_closes_the_camera` (Review Focus 5, par le compteur déjà là), `recording_writes_a_file_and_says_so` ; dans `numbers.rs` : `a_thousand_frames_arrive_without_a_loss`, `the_latency_from_sensor_to_pixel_stays_under_a_tenth_of_a_second`, `the_interface_keeps_sixty_frames_a_second_while_capturing` — tous trois sur la source synthétique à 1080p30, donc sans matériel.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : les pages, le câblage, la mesure.
- [ ] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [ ] **Step 5** : commit `feat(capture): the proof software, with real hardware`.

---

## Ce que ce plan livre

Le critère d'acceptation de la spec §8 : un logiciel qui énumère ce qui est branché, ouvre une caméra, affiche son flux en temps réel et enregistre — avec les chiffres mesurés plutôt qu'annoncés, et une source synthétique pour que la preuve tienne sur une machine sans webcam.

## Ce que ce plan ne fait pas

L'audio — entrée, sortie, et la piste sonore dans le même fichier que la vidéo — est le plan qui suit celui-ci : la §7.6 le demande, et un enregistreur muet est un enregistreur à moitié. Les micros sont énumérés ici (Task 1) ; rien n'est encore lu d'eux.

Le reste de la §7.6 — presse-papiers, dialogues de fichiers, notifications, icône de barre système, raccourcis globaux, glisser-déposer, multi-fenêtres — est de l'intégration au bureau, pas de la capture. C'est un plan à part, et il peut être écrit après celui-ci sans que rien n'attende.
