# Les médias par Chromium

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Une caméra, un écran, un enregistrement et, quand Rust doit voir les pixels, les images elles-mêmes — appelés depuis un `.rsx` en Rust, servis par le moteur que le framework embarque déjà, et prouvés sans matériel par la caméra factice de Chromium.

**Architecture:** Chromium sait capturer une caméra et un écran, les encoder avec le matériel, et il est testé par Google. Le framework n'écrit donc pas de Media Foundation : il écrit la **politique** (qui a le droit, décidé en Rust, jamais une boîte de dialogue surprise), l'**API Rust** qu'un `.rsx` appelle sans une ligne de JavaScript, et le **chemin des pixels vers Rust** pour les cas où l'application doit les traiter — par `cef_v8_value_array_buffer_data`, donc sans copie et sans sérialisation, ce qui est exactement la thèse de la §7.1 tenue par l'autre bout. Ce que Chromium ne voit pas — cartes d'acquisition, NDI, encodeurs maison — reste du natif, écrit le jour où un besoin réel et du matériel sont là en même temps.

**Tech Stack:** Rust, CEF 156 (`ImplPermissionHandler::on_request_media_access_permission`, `MediaAccessCallback`, `v8_value_create_array_buffer`, `array_buffer_data`), `getUserMedia` / `getDisplayMedia` / `MediaRecorder` / WebCodecs dans le processus de rendu, `rustx-os::devices` pour les noms de périphériques sans permission. Aucune dépendance nouvelle.

**Spec:** `docs/superpowers/specs/2026-10-27-rustx-framework-design.md` — §7.6 (la couche OS et son « s'appuyer sur ce qui existe »), §8 (le logiciel de preuve et ses chiffres), §7.1 (pourquoi une image ne traverse pas de frontière sérialisée), §7.5 (la frontière avec JavaScript).

## Global Constraints

- **Rien ne panique.** `rustx-engine::media` et `rustx-os` portent `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` ; chaque refus nomme la chose refusée — le périphérique, le fichier, la permission.
- **Chaque poignée est un jeton.** `Stream`, `Recording`, `Frames` : leur `Drop` arrête la piste, finalise le fichier, oublie le rappel. Aucune méthode `close()` que l'application doit penser à appeler, et un test compte les pistes vivantes avant et après.
- **Aucune file sans borne.** Si l'application ne suit pas, les images sont **jetées et comptées** (`Stats::dropped`), jamais empilées : une file qui grossit est une fuite de mémoire qui met vingt minutes à tuer un gros projet.
- **Aucun test n'exige de matériel.** `--use-fake-device-for-media-stream` donne une vraie mire à la résolution demandée ; un test qui aurait besoin d'une webcam imprime `SKIPPED: <raison>` et réussit, et la raison apparaît dans la sortie de la suite.
- **La permission est une politique, pas une boîte de dialogue.** L'application déclare ce qu'elle permet dans son `Config` ; le gestionnaire du moteur applique. Une demande non permise est refusée avec une phrase sur le canal que le développeur lit, jamais en silence.
- **Le JavaScript injecté par le framework vit sous un seul nom** (`window.__rustx_media`), non énumérable, et ne touche à rien d'autre : une application qui charge React, Tailwind et trois bibliothèques ne doit pas pouvoir entrer en collision avec le framework.
- Windows d'abord pour `rustx-os` ; le chemin média est celui de Chromium, donc il marche partout où Chromium marche.

## Review Focus

1. **La permission refusée** (politique absente, ou l'utilisateur a retiré l'accès dans Windows) : `Stream::open` rend une erreur qui dit laquelle et comment la permettre, l'application continue de tourner. Test Task 1 (`a_request_without_a_policy_is_refused_in_words`).
2. **Le périphérique disparaît pendant le flux** (webcam débranchée, écran partagé fermé par l'utilisateur) : `Stream::ended()` devient vrai, l'aperçu garde sa dernière image, rien ne panique. Test Task 2 (`a_stream_that_ends_says_so_and_nothing_panics`).
3. **L'application ne suit pas** (un rappel de traitement qui prend 80 ms à 30 images/s) : les images sont jetées et comptées, la mémoire ne monte pas, l'interface reste vivante. Test Task 5 (`a_slow_callback_drops_frames_instead_of_queueing_them`).
4. **L'enregistrement vers un chemin impossible, ou un disque qui se remplit** : arrêt net, le fichier nommé, et ce qui a déjà été écrit reste lisible parce que le `Drop` finalise. Test Task 4 (`a_recording_that_cannot_be_written_says_which_file`, `what_was_written_before_the_trouble_still_plays`).
5. **Quitter l'écran pendant que des images sont en vol** : aucune image posée sur un élément disparu, aucune piste laissée ouverte, et le compteur de pistes vivantes revient à zéro. Test Task 2 (`leaving_the_screen_stops_every_track`) et Task 6 (`leaving_the_preview_closes_the_camera`).

---

### Task 1 : Qui a le droit, décidé en Rust

**Files:**
- Create: `crates/rustx-engine/src/permission.rs`
- Modify: `crates/rustx-engine/src/client.rs` (`permission_handler`), `crates/rustx-engine/src/cef_app.rs` (les commutateurs de la caméra factice), `crates/rustx/src/lib.rs` (`Config::permit`)
- Test: `crates/rustx-engine/tests/permission.rs`, `apps/capture/tests/app.rs`

**Interfaces:**
- Produces: `enum Permission { Camera, Microphone, Screen, Clipboard, Notifications }` ; `permission::permit(what: Permission)` et `permission::permitted(what: Permission) -> bool` (la politique, lue par le gestionnaire sur le thread de l'interface) ; `permission::refusals() -> Vec<String>` — ce qui a été refusé, pour qu'un test puisse le lire ; `Config::permit(Permission) -> Self` côté façade, qui est la seule chose qu'une application écrit.
- Le gestionnaire répond `MediaAccessCallback::cont(allowed_permissions)` pour ce que la politique permet et `cancel()` pour le reste, et imprime une phrase nommant la permission et la ligne de `Config` qui l'aurait donnée.
- Les commutateurs `--use-fake-device-for-media-stream` et `--use-fake-device-for-media-stream` ne sont ajoutés que si `RUSTX_FAKE_MEDIA=1` : un développeur ne doit jamais recevoir une mire au lieu de sa caméra.

- [ ] **Step 1** : les tests — `a_policy_is_what_the_handler_reads` (permit puis permitted), `a_request_without_a_policy_is_refused_in_words` (Review Focus 1 : la phrase nomme `Config::permit(Permission::Camera)`), `a_permission_asked_twice_is_answered_twice` (pas de mémoire d'un refus qui empêcherait un `permit` plus tard), `the_fake_device_is_only_there_when_asked` (sans la variable, aucun commutateur).
- [ ] **Step 2** : rouge (`cargo test -p rustx-engine --test permission`).
- [ ] **Step 3** : la politique, le gestionnaire, les commutateurs.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(engine): who may use the camera, decided in Rust`.

---

### Task 2 : Un flux, montré, et fermé par son `Drop`

**Files:**
- Create: `crates/rustx-engine/src/media.rs`, `crates/rustx-engine/assets/media.js`
- Modify: `crates/rustx-engine/src/lib.rs`, `crates/rustx/src/lib.rs` (`rustx::media`)
- Test: `crates/rustx-engine/tests/media.rs`, `apps/capture/tests/app.rs`

**Interfaces:**
- Produces: `struct Wanted { pub width: u32, pub height: u32, pub fps: u32 }` avec `Wanted::hd()` (1920×1080 à 30) ; `enum Source { Camera { device: Option<String>, wanted: Wanted }, Screen { display: Option<String>, wanted: Wanted } }` ; `Stream::open(Source) -> Result<Stream, MediaError>` ; `Stream::show(&self, element_id: &str) -> Result<(), MediaError>` (attache le flux à un `<video>` de la page) ; `Stream::ended(&self) -> bool` ; `Stream::stats(&self) -> Stats { delivered, dropped, last_frame_age_ms }` ; `enum MediaError { Refused(Permission), NoSuchDevice(String), Ended, NoElement(String), Platform(String) }` avec un `Display` qui dit quoi faire.
- Le `Drop` de `Stream` arrête chaque piste et oublie l'élément. `media::live_tracks() -> u32` existe pour que les tests comptent (Review Focus 5).
- `media.js` est injecté sous `window.__rustx_media`, non énumérable, et n'expose que ce que Rust appelle.
- Consumes: `permission::permitted` (Task 1), la file de frames du moteur pour poster au thread de l'interface.

- [ ] **Step 1** : les tests — `a_fake_camera_fills_a_video_element` (la page rapporte `videoWidth` et des images qui avancent), `a_stream_that_ends_says_so_and_nothing_panics` (Review Focus 2 : la piste est arrêtée depuis le JS, `ended()` devient vrai), `leaving_the_screen_stops_every_track` (Review Focus 5 : `live_tracks()` revient à 0 après le `Drop`), `a_video_element_that_is_not_there_is_a_sentence` , `two_streams_at_once_each_keep_their_own_element`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le runtime JS, les appels natifs, le jeton.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(media): a stream, shown, and closed by its own Drop`.

---

### Task 3 : La liste des périphériques, nommée sans permission

**Files:**
- Modify: `crates/rustx-engine/src/media.rs`, `crates/rustx-engine/assets/media.js`, `crates/rustx-os/src/devices.rs` (rien de neuf, juste la consommation)
- Test: `crates/rustx-engine/tests/media.rs`, `crates/rustx-os/tests/devices.rs`

**Interfaces:**
- Produces: `media::devices() -> Vec<Device>` où `Device { pub id: String, pub name: String, pub kind: Kind, pub busy: bool, pub web_id: Option<String> }` — les noms viennent de `rustx_os::devices` (Media Foundation les donne **sans** permission, là où `enumerateDevices` rend des étiquettes vides avant l'accord), les `web_id` de `navigator.mediaDevices`, appariés par le nom.
- `Source::Camera { device: Some(id) }` accepte l'`id` natif comme le `web_id` : l'application ne doit pas avoir à savoir d'où vient le nom qu'elle a reçu.

- [ ] **Step 1** : les tests — `the_names_are_there_before_any_permission_is_given` (la liste a des noms non vides sans `permit`), `a_native_id_and_a_web_id_name_the_same_device` (sur la caméra factice), `a_machine_with_nothing_plugged_in_lists_nothing_and_says_so`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : l'appariement.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(media): the device list, named before permission`.

---

### Task 4 : Enregistrer dans un fichier que Rust possède

**Files:**
- Create: `crates/rustx-engine/src/recording.rs`
- Modify: `crates/rustx-engine/assets/media.js`
- Test: `crates/rustx-engine/tests/recording.rs`

**Interfaces:**
- Produces: `Recording::start(stream: &Stream, path: &Path, wanted: Bitrate) -> Result<Recording, MediaError>` ; `struct Bitrate(pub u32)` avec `Bitrate::good()` (8 Mb/s à 1080p) ; `Recording::written(&self) -> u64` (octets) ; `Recording::stop(self) -> Result<PathBuf, MediaError>` et un `Drop` qui finalise de toute façon.
- Les morceaux viennent de `MediaRecorder` et sont écrits **par Rust** : `array_buffer_data` les lit sans copie, et le fichier appartient au processus, pas au bac à sable du navigateur — un gros projet doit pouvoir choisir le chemin, le nom et le moment.
- Un chemin qu'on ne peut pas écrire est refusé **avant** que la première image soit demandée.

- [ ] **Step 1** : les tests — `three_seconds_of_the_fake_camera_becomes_a_file_that_plays` (le fichier existe, son en-tête est un conteneur connu, sa taille est cohérente avec le débit), `a_recording_that_cannot_be_written_says_which_file` (Review Focus 4), `what_was_written_before_the_trouble_still_plays` (Review Focus 4 : le `Drop` finalise après une écriture refusée), `stopping_twice_is_not_a_crash`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : `MediaRecorder`, le pont des morceaux, l'écriture.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(media): recording, into a file Rust owns`.

---

### Task 5 : Quand Rust doit voir les pixels

**Files:**
- Create: `crates/rustx-engine/src/pixels.rs`
- Modify: `crates/rustx-engine/assets/media.js`
- Test: `crates/rustx-engine/tests/pixels.rs`

**Interfaces:**
- Produces: `struct Frame { pub width: u32, pub height: u32, pub taken: std::time::Instant, bytes: *const u8, len: usize }` avec `Frame::bytes(&self) -> &[u8]` (BGRA, emprunté le temps du rappel, jamais possédé) ; `pixels::frames(stream: &Stream, each: impl FnMut(Frame) -> Keep + 'static) -> Frames` ; `enum Keep { Going, Enough }` ; `Frames::stats(&self) -> Stats`.
- Le chemin : WebCodecs (`VideoFrame.copyTo`) écrit dans un `ArrayBuffer` du **bassin** que Rust a fourni par `v8_value_create_array_buffer`, puis le rappel Rust lit cette mémoire. Aucune copie, aucune sérialisation, et le bassin a une taille fixe — trois tampons — donc une application en retard ne fait pas grossir la mémoire.
- Le rappel tourne sur le thread de l'interface : la documentation dit en une phrase que le travail lourd va dans `task(…)`, et `Stats::dropped` est la mesure qui le prouve.

- [ ] **Step 1** : les tests — `the_pixels_of_the_fake_camera_arrive_in_rust` (la mire a des couleurs connues : le test lit un pixel et le reconnaît), `nothing_is_copied_on_the_way_in` (l'adresse vue par Rust est celle du tampon du bassin, deux images d'affilée), `a_slow_callback_drops_frames_instead_of_queueing_them` (Review Focus 3 : un rappel à 80 ms sur 30 images/s livre moins d'images et `dropped` monte, et la mémoire du processus ne bouge pas de plus de 5 Mo), `enough_is_the_last_frame`, `dropping_frames_releases_the_pool`.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : le bassin, le pont WebCodecs, le compte.
- [ ] **Step 4** : vert.
- [ ] **Step 5** : commit `feat(media): when Rust has to see the pixels`.

---

### Task 6 : Le logiciel de preuve, et ses chiffres

**Files:**
- Modify: `apps/capture/src/main.rs`, `apps/capture/src/devices.rs`, `apps/capture/app/page.rsx`, `apps/capture/app/device/[id]/page.rsx`
- Create: `apps/capture/tests/numbers.rs`
- Test: `apps/capture/tests/app.rs`

**Interfaces:**
- Consumes: tout ce qui précède. La page ouvre le flux dans un `keep(…)`, le montre dans un `<video>`, enregistre sur un clic, et ne ferme rien — le `Drop` du jeton le fait.
- Produces: les chiffres de la spec §8, imprimés par `cargo test -p capture --test numbers -- --nocapture` : images livrées, images jetées, latence médiane et au 95e centile de l'horodatage à l'affichage, images par seconde de l'interface pendant la capture.

- [ ] **Step 1** : les tests — `app.rs` : `the_list_shows_what_is_plugged_in`, `opening_a_device_shows_its_stream`, `leaving_the_preview_closes_the_camera` (Review Focus 5), `recording_writes_a_file_and_says_where` ; `numbers.rs` : `a_thousand_frames_arrive_and_none_is_lost`, `the_latency_from_frame_to_pixel_stays_under_a_tenth_of_a_second`, `the_interface_keeps_sixty_frames_a_second_while_capturing` — tous sur la caméra factice à 1080p30.
- [ ] **Step 2** : rouge.
- [ ] **Step 3** : les pages, le câblage, la mesure.
- [ ] **Step 4** : vert, plus `cargo test --workspace` et `cargo clippy --workspace --all-targets`.
- [ ] **Step 5** : commit `feat(capture): the proof software, and its numbers`.

---

## Ce que ce plan livre

Une caméra, un écran, un enregistrement et les pixels eux-mêmes, appelables depuis un `.rsx` en Rust, sans qu'une application écrive du JavaScript ni qu'un développeur installe quoi que ce soit — et le critère d'acceptation de la §8 mesuré sur une machine sans webcam.

## Ce que ce plan ne fait pas

- La capture que Chromium ne voit pas : cartes d'acquisition, NDI, caméras industrielles, composition de plusieurs sources, encodeur maison. Le jour où un besoin réel arrive, l'API de la Task 2 est déjà la bonne forme (`Source` gagne une variante) et le matériel sera là pour la valider.
- L'audio : la piste sonore suit le même chemin (`MediaRecorder` en prend deux), et ce qui manque est une API Rust pour le niveau, le choix du micro et le mélange. C'est le petit plan qui suit.
- Tout ce que le web ne sait pas faire — presse-papiers, notifications, dialogues, barre système, raccourcis globaux, multi-fenêtres — qui est l'autre plan, `2026-10-31-la-couche-os-ce-que-le-web-ne-sait-pas-faire.md`.
