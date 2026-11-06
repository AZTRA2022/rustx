# Chasse aux bugs — 2026-11-06

Un projet écrit **du dehors**, comme un développeur l'écrirait, et ce
qu'il a trouvé.

La méthode : `rustx new labo-rustx` dans un dossier voisin du dépôt,
puis les bibliothèques installées par les commandes que donnent leurs
propres pages de démarrage — Tailwind 4 (`npm install tailwindcss
@tailwindcss/cli`), TipTap (`npm install @tiptap/react @tiptap/pm
@tiptap/starter-kit`), shadcn (`npx shadcn init`, puis `npx shadcn add
button card dialog input label`) — et rien d'autre. Aucun contournement,
aucune version choisie pour faire passer la construction.

Le labo pointe sur les sources du dépôt par un `[patch.crates-io]`, donc
chaque correction se vérifie dans la seconde.

Ce qu'il reste à la fin : un `tests/app.rs` dans le labo qui ouvre une
vraie fenêtre et lit ce qu'elle dessine. Onze assertions, toutes vertes :
une `Card` de shadcn montée par React, sa classe Tailwind appliquée, un
TipTap vivant dans un composant React que le projet a écrit lui-même, un
`onClick` en Rust appelé depuis l'intérieur de React qui fait monter un
`let mut` partagé de la page, un bloc `async` qui écrit un fichier avec
`tokio::fs` sans bloquer la fenêtre — relu depuis le processus de test,
hors de la fenêtre — et une valeur attendue avec `.await` dans un
composant, affichée par la frame qui suit son arrivée.

Une deuxième vague est partie d'un seul message : « pourquoi est-on
obligé d'utiliser `task(...)` », « je remarque des `rustx::tokio::...` »,
« certaines opérations effectuées avant le retour du code UI bloquent le
rendu ». Les trois étaient justes, et les numéros 14 à 17 en viennent.

---

## Ce qui a été trouvé

| # | Gravité | Où | Ce qui n'allait pas | État |
|---|---|---|---|---|
| 1 | bloquant | `rustx-cli`, `rustx`, `rustx-macro` | un projet neuf ne compilait pas | corrigé |
| 2 | important | documentation | `npx shadcn init` ne connaît pas rustx | documenté |
| 3 | bloquant | `rustx-compiler` | `@tailwindcss/cli` partait dans le paquet du navigateur | corrigé |
| 4 | bloquant | `rustx-compiler` | un paquet sans export racine faisait tomber toute la construction | corrigé |
| 5 | important | `rustx-compiler` | esbuild appelé avec `--platform=node` pour une page | corrigé |
| 6 | bloquant | `rustx-compiler` | un `let mut` écrit et jamais lu engendrait du Rust invalide | corrigé |
| 7 | mineur | `rustx-cli` | le `.gitignore` n'ignorait pas ce que la construction écrit | corrigé |
| 8 | important | `rustx-compiler` | Tailwind sans sa ligne de commande : zéro style, zéro mot | corrigé |
| 9 | important | `rustx-compiler` | `components/` hors de `components/ui` n'était jamais dans le paquet | corrigé |
| 10 | mineur | `rustx-lsp` | l'éditeur ne proposait pas les composants du projet hors `ui` | corrigé |
| 11 | important | `rustx-compiler` | deux bibliothèques pour un seul nom de balise, en silence | corrigé |
| 12 | important | les harnais de test | un échec qui accuse le framework à la place du pilote 3D | corrigé |
| 13 | bloquant | `rustx-lsp` | la complétion des balises npm n'a jamais marché | corrigé |
| 14 | important | `rustx-compiler` | `task(…)` autour d'un bloc `async` ne disait rien | corrigé |
| 15 | important | `rustx-macro` | `rustx::tokio::` est un chemin que personne n'écrit | corrigé |
| 16 | bloquant | `rustx-compiler`, `rustx-engine` | attendre avant le `return` bloquait le rendu | corrigé |
| 17 | bloquant | `rustx-compiler` | un commentaire français faisait paniquer le compilateur | corrigé |
| 18 | important | `rustx-lsp` | l'éditeur n'offrait aucune prop des balises npm | corrigé |
| 19 | important | extension VS Code | taper `>` ne fermait pas la balise | corrigé |
| 20 | mineur | `rustx-compiler` | trois refus arrivaient avec des rafales d'espaces | corrigé |
| 21 | important | `rustx-dom` | la boîte aux lettres d'un thread parti grossissait sans fin | corrigé |

---

### 1. Un projet neuf ne compilait pas

`rustx new mon-app` puis `cargo build` :

```text
error[E0433]: cannot find module or crate `rustx_dom` in this scope
  --> target/rustx-pages/pages.rs:9:36
```

Le `Cargo.toml` engendré ne nomme qu'une caisse — `rustx-app`, comme la
documentation le promet — mais le Rust que le compilateur `.rsx` écrit
nomme `rustx_dom::Element`, `rustx_dom::route::Params` et
`rustx_engine::task::spawn`. Les applications du dépôt masquaient le
défaut en déclarant les trois à la main, avec un commentaire qui le
disait presque : « Named by the generated pages, never by a line a
developer writes. »

**Correction.** La façade réexporte les deux caisses
(`pub use rustx_dom; pub use rustx_engine;`, `#[doc(hidden)]`), et le
prélude que `#[rustx::main]` pose dans `mod __rustx_pages` les importe à
côté de `rustx::Element`. Une application en nomme toujours une seule,
et c'est vrai maintenant au lieu d'être une intention.

Un test le garde : `a_new_project_names_one_dependency_and_one_only`.

### 2. `npx shadcn init` ne connaît pas rustx

```text
We could not detect a supported framework at …
```

shadcn cherche Next, Vite ou Remix. Rien à corriger dans le code : ce
qu'il aurait écrit, ce sont deux fichiers, et le README les donne
maintenant en entier (`components.json` et `tsconfig.json`, dont les
`paths` sont ce qui apprend à esbuild ce que `@/` veut dire). `npx
shadcn add` marche ensuite sans rien d'autre.

### 3. `@tailwindcss/cli` partait dans le paquet du navigateur

```text
error: esbuild refused this application's libraries:
  X [ERROR] Could not resolve "@tailwindcss/cli"
      7 │ import * as tailwindcssCli from "@tailwindcss/cli";
```

La liste `TOOLS` ne contenait que des noms nus. `npm install tailwindcss
@tailwindcss/cli` — la première ligne de la page de Tailwind 4 — met la
ligne de commande sous `dependencies`, et le point d'entrée l'importait,
donc le navigateur se faisait demander de rendre un compilateur.

**Correction.** `TOOLS` s'est agrandi (`rollup`, `parcel`, `prettier`,
`eslint`, `@biomejs/biome`) et, surtout, `TOOL_SCOPES` refuse par
portée : `@tailwindcss/`, `@esbuild/`, `@swc/`, `@parcel/`, `@rollup/`.

### 4. Un paquet sans export racine faisait tomber toute la construction

```text
  X [ERROR] Could not resolve "@tiptap/pm"
    The path "." is not exported by package "@tiptap/pm"
```

`@tiptap/pm` est un paquet-parapluie : il exporte `./state`, `./view`,
`./model`, et pas de racine. Une page écrit `@tiptap/pm/state` ou rien.
Il est dans les `dependencies` parce que la page de TipTap dit de
l'installer, et une seule ligne de `package.json` suffisait à ce que
plus rien ne compile.

**Correction.** `bundle::installed(app, package)` lit le manifeste
installé et répond `Importable`, `NothingToImport` ou `Absent`. Les
`NothingToImport` sortent du point d'entrée sans un mot — ce n'est pas
une faute du développeur. Deux formes y tombent : les sous-chemins sans
racine, et les paquets dont le point d'entrée est un `.css`
(`tw-animate-css`, que shadcn installe). Les `Absent`, eux, sont dits
par leur nom, parce que la réparation est une commande.

Ce qui ne devait pas disparaître dans l'affaire : « this application
declares JavaScript libraries … and has no `node_modules` ». Le test
`a_missing_node_modules_says_to_install` a attrapé la régression au
premier essai.

### 5. esbuild appelé avec `--platform=node` pour une page

Le commit `de7261a` dit ce qu'il est : « adding `--platform=node` to
bundle to check if node error is gone ». C'était un pansement sur le
défaut n° 4, et il coûte cher : `--platform=node` marque les modules de
node comme externes — donc un `require("fs")` reste dans une IIFE qu'un
navigateur exécute — et choisit la variante serveur d'une bibliothèque
qui en a deux.

**Correction.** `--platform=browser`, explicitement, avec la raison
écrite à côté. La page est un navigateur, quelle que soit la machine qui
construit.

### 6. Un `let mut` écrit et jamais lu engendrait du Rust invalide

```rust
let mut ouvert = false;
…
<Button onClick={move |_| ouvert = !ouvert}>Ouvrir</Button>
```

```text
error[E0596]: cannot borrow `__called` as mutable, as it is a captured
              variable in a `Fn` closure
  --> target/rustx-pages/page.rs:70:23
```

La règle de `promote.rs` demandait deux moitiés : lu dans le JSX **et**
écrit depuis un gestionnaire. Un état écrit et montré nulle part restait
un `let` ordinaire — et un gestionnaire est rangé dans un
`Rc<dyn Fn>`, donc une fermeture qui affecte une variable capturée ne
compile pas du tout. Le développeur lisait une erreur d'emprunt sur un
fichier qu'il n'a jamais ouvert, dans un dossier temporaire.

**Correction.** Une écriture depuis un gestionnaire suffit à promouvoir.
Il n'existe aucun cas où la laisser plate compile, donc la moitié
« lue » ne décidait rien : elle choisissait entre une cellule et une
erreur. Le `let mut __called` du chemin des props React a reçu son `mut`
dans la même passe.

Le test qui gardait l'ancienne règle dit maintenant la nouvelle, avec la
raison : `a_variable_only_mutated_in_a_handler_is_promoted_anyway`.

### 7. Le `.gitignore` n'ignorait pas ce que la construction écrit

Une ligne, `/target`. Le premier `git add .` d'un projet avec des
bibliothèques npm emportait `node_modules`, le `.rustx/` que le
framework engendre, et `assets/rustx-bundle.js` — deux mégaoctets de
JavaScript minifié. Le commentaire de `bundle.rs` affirmait pourtant
l'inverse : « Ignored by git, like `node_modules` ».

**Correction.** Quatre lignes, et un test qui les lit.

### 8. Tailwind sans sa ligne de commande : zéro style, zéro mot

`tool_in(app, "tailwindcss")` ne trouve rien, et `styles()` répondait
`Ok(None)`. Une application qui a installé `tailwindcss` seul — ce qui
arrive en 4, où la commande vit dans `@tailwindcss/cli` — ouvrait une
fenêtre avec tous ses `className` et pas une règle, sans une ligne dans
la console.

**Correction.** `node_modules/tailwindcss` présent et aucune commande
pour le faire tourner est un refus qui nomme le `npm install` à écrire.
Un projet qui n'a pas Tailwind du tout, lui, continue à ne rien dire.

### 9. `components/` hors de `components/ui` n'était jamais dans le paquet

`index_for` lisait `components/ui` et rien d'autre. Or les globs de
Tailwind, trois fonctions plus haut dans le même fichier, lisent
`components/**` — et ce qui tombait dans l'écart est exactement le
fichier dont on a le plus besoin : un composant React que le projet
écrit lui-même, celui qui appelle un hook, ce qu'une page `.rsx` ne sait
pas faire. `components/Editor.tsx` avec `useEditor` de TipTap compilait,
et la page ne trouvait rien à l'exécution.

Deux défauts en un, en fait : `export * from "…"` ne porte pas le
`default`, et un composant à soi est presque toujours
`export default function Editor()`.

**Correction.** Une seule marche récursive, `bundle::own_components`,
que les deux moitiés du framework lisent — le paquet et l'éditeur. Les
`.d.ts` sont sautés. Et pour chaque fichier qui a un `export default`,
l'index ajoute `export { default as Editor }`, sous le nom du fichier en
PascalCase, ce qu'un `index.ts` ne reçoit pas parce que `<Index>` n'est
rien que personne veut dire.

### 10. L'éditeur ne proposait pas les composants du projet hors `ui`

Le même écart, de l'autre côté : `rustx-lsp` lisait lui aussi
`components/ui` seul, et son horodatage de cache regardait ce dossier.
Il lit maintenant `bundle::own_components` et `bundle::own_directory`,
et ajoute le nom du fichier pour chaque `export default`. L'éditeur et
le paquet ne peuvent plus être en désaccord : il n'y a plus qu'une
marche.

### 11. Deux bibliothèques pour un seul nom de balise, en silence

`name_for` fait d'un nom de paquet un identifiant : `@tiptap/react`
devient `tiptapReact`, et `tiptap-react` aussi. Le point d'entrée
écrivait les deux comme deux clés du même objet littéral
`window.rustxLibraries`, ce que JavaScript accepte : la seconde
remplace la première, et une page reçoit les composants d'une
bibliothèque qu'elle n'a pas demandée, sans un mot.

**Correction.** `bundle::a_collision_in` est consulté avant d'écrire le
point d'entrée, et la construction refuse en nommant les deux paquets
et le nom qu'ils se disputent. Trouvé en lisant le code après le
défaut n° 3, pas dans le labo : aucune paire installée ne collisionnait.

### 12. Un échec qui accuse le framework à la place du pilote 3D

Sur la machine de la chasse, `cargo test --workspace` disait :

```text
a_panic_in_a_posted_job_does_not_stop_the_frame_loop ... FAILED
  assertion `left == right` failed: a panicking job killed the frame loop
```

La phrase est fausse. Le journal, trente lignes plus haut, disait la
vérité : `Failed to create shared context for virtualization`. Cette
machine ne donne à Chromium aucun contexte 3D, la fenêtre n'est pas
composée, elle ne produit aucune frame — et la moitié des scénarios
attendent une frame. Avec `RUSTX_SOFTWARE=1`, les six tests passent.

Le README connaissait le remède ; les tests, non. Un développeur qui lit
« a panicking job killed the frame loop » passe son après-midi dans
`panics.rs`.

**Correction.** Les sept harnais de scénario — celui du moteur, un par
application de preuve, et celui du labo — regardent ce qu'ils ont
capturé : si le journal contient « Failed to create shared context »,
chaque échec se termine par la phrase qui nomme la machine, la cause et
`RUSTX_SOFTWARE=1`. Le test échoue toujours — il doit — mais il n'accuse
plus le mauvais coupable.

C'est le seul défaut de la liste que le labo n'a pas trouvé en
construisant : il est sorti de la suite du dépôt, lancée pour vérifier
les onze autres.

### 13. La complétion des balises npm n'a jamais marché

Rapporté en ouvrant le labo dans l'éditeur : aucune complétion pour les
balises des bibliothèques. `suggest.rs` :

```rust
let Some(application) = application_of(page) else { ... };
let Some(root) = application.parent() else { ... };   // <-- ici
libraries::exports_for(root)
```

`application_of` rend le dossier de l'application — celui qui a le
`Cargo.toml` et le `app/`. Son **parent** n'a ni `package.json`, ni
`node_modules`, ni `components/`. Donc
`labo-rustx/app/page.rsx` était répondu depuis `projects/`, et
`apps/zoo/app/page.rsx` depuis `apps/` : la liste revenait vide, et
l'éditeur n'offrait que les balises HTML et les fonctions du fichier.
Tous les autres lecteurs du serveur utilisaient déjà l'application
elle-même.

Ce que la suite ne pouvait pas voir : `libraries::read` était testé avec
un chemin de dossier d'application, et la complétion avec une `Page`
construite à la main. Rien ne partait d'un chemin de `.rsx` sur le
disque, qui est la seule chose que l'éditeur envoie.

**Correction.** Le dossier de l'application, sans `.parent()`. Et un
test qui part de là où l'éditeur part : un projet écrit sur le disque,
un `node_modules`, un `components/ui`, un `components/Editor.tsx`, et
`page_of(source, &root.join("app/page.rsx"))` —
`a_page_is_offered_what_the_application_around_it_installed`.

### 14. `task(…)` autour d'un bloc `async` ne disait rien

Rapporté en lisant le labo : « pourquoi est-on obligé d'utiliser
`task(...)`, alors qu'en JSX on n'utilise rien de tout ça ». La question
était juste. Le premier mot de l'argument de `task` était déjà ce qui
décidait entre un fil et une future :

```rust
CallKind::Task if is_a_future(arguments) => future_task(…),
CallKind::Task => spawn(…),
```

Donc `task` ne portait aucune information que le `async` ne portait pas.
C'était une cérémonie, et une cérémonie dans un framework dont l'argument
de vente est qu'il n'y en a pas.

**Correction.** Un bloc `async` écrit comme **instruction** d'un composant
est reconnu tel quel, avec la même émission. Un bloc qui est une *valeur*
— `let tenu = async { … };`, ou un argument — est laissé tranquille :
quatre tests le disent. `task(async { … })` marche toujours, et veut dire
la même chose.

Ce que ça a révélé au passage : lire « est-ce une instruction ? » ne
remontait que les espaces, donc **tout bloc précédé d'un commentaire**
passait pour le milieu d'une expression — et presque tous le sont.

### 15. `rustx::tokio::` est un chemin que personne n'écrit

Rapporté sur le même message : « je remarque des `rustx::tokio::...`,
non ». La façade réexportait tokio en entier, et le réexport se lisait
dans le chemin. Un développeur qui copie une ligne de la documentation de
`tokio` devait la réécrire, et un extrait collé de `sqlx` ou de `reqwest`
ne compilait pas.

**Correction.** Le nom, pas le chemin : `pub(crate) use rustx::tokio;`
dans le module des pages, et un `use rustx::tokio;` **à côté** de `main`
plutôt que dedans — parce que le code d'une application qui attend n'est
pas tout dans `main`, et une `async fn` écrite à côté en a autant besoin.
Une page écrit `tokio::fs::write(…)` et `tokio::select! { … }` comme
n'importe quel code Rust, sans déclarer tokio.

Pourquoi ce n'est pas un conflit pour une application qui déclare tokio
elle-même : cargo n'en résout qu'un seul en 1.x, donc c'est le même
crate, et un `use` nommé bat de toute façon le prélude externe.

### 16. Attendre avant le `return` bloquait le rendu

Rapporté sur le même message : « certaines opérations effectuées avant le
retour du code UI bloquent le rendu ». Un composant est interrogé sur
l'écran tel qu'il est maintenant, donc il ne peut pas attendre — et la
seule réponse du framework était une phrase qui renvoyait vers
`task(…)`. Ce qui veut dire : pour afficher une valeur qui vient d'un
fichier ou d'un serveur, il fallait écrire soi-même la cellule
`Option<T>`, le handle, le `match`, et le rendu vide.

**Correction.** `let x = travail().await;` dans un composant s'écrit tel
quel. Ce que le compilateur en fait, en deux éditions aux deux bouts de
l'instruction et rien entre les deux :

```rust
let Some(x) = rustx_engine::task::awaited(
    rustx_dom::state::cell(SLOT_A, || None), SLOT_B,
    || { let __work = travail(); async move { __work.await } },
) else { return Element::Empty; };
```

La future est **construite sur le thread qui dessine** — c'est ce qui
permet à l'expression de lire une cellule — et **attendue sur ceux du
runtime**. La question n'est posée qu'une fois, le type écrit est gardé,
et l'expression reste exactement là où elle a été écrite, donc une
lecture d'état dedans est réécrite par la passe qui réécrit toutes les
autres.

Ce que ça a révélé : `.await.unwrap_or_else(…)` et `.await.ok()` sont la
façon normale d'écrire ça, et la première règle — « le `.await` doit être
la dernière chose de la valeur » — les refusait. Ce qui suit l'`await`
travaille sur la réponse, qui n'existe qu'à côté de la fenêtre : il part
donc avec la future.

Et deux refus, parce qu'ils ne peuvent pas être autre chose : un `.await`
enterré dans un argument (`f(g().await)` attend avant que `f` soit même
appelée), et une lecture d'une valeur de la page **après** l'`await`, pour
la même raison qu'une lecture dans un bloc `async`.

### 17. Un commentaire français faisait paniquer le compilateur

Trouvé en reconstruisant le labo, par le code du n° 14 : lire en arrière
par-dessus un commentaire comparait deux octets à `*/` sans vérifier que
la frontière en était une.

```rust
if index >= 2 && &source[index - 2..index] == "*/" {
```

Un commentaire fini par « déjà. » met un `à` de deux octets juste avant le
point, donc `index - 2` tombait **au milieu du caractère** et Rust
paniquait. Le message que voyait le développeur :
`custom attribute panicked … start byte index 832 is not a char
boundary`, sans un mot sur quel fichier ni quelle ligne.

**Correction.** `source.is_char_boundary(index - 2)` avant la tranche, et
un test dont le commentaire finit par « déjà. » — parce que mon premier
test disait « fenêtre » et que l'accent n'y était pas dans les deux
derniers octets, donc il passait.

### 18. L'éditeur n'offrait aucune prop des balises npm

Rapporté avec le n° 13, et seule la moitié avait été corrigée : les
**noms** des balises arrivaient, pas ce qu'elles prennent. Sur
`<Button `, la liste était celle des attributs HTML — `accept`,
`accesskey`, `action` — et `variant`, `size`, `asChild` n'y étaient
nulle part. `suggest::attributes` ne lisait les paramètres que pour les
composants déclarés dans le fichier même.

**Correction.** Les props sont lues là où les noms sont : `interface
ButtonProps { … }` ou `type CardProps = { … }` dans le `.d.ts` du
paquet, le paramètre déstructuré de la fonction pour ce que `shadcn add`
écrit (il n'y a pas de type de props du tout), et sinon le type que la
déclaration nomme, suivi jusqu'à sa propre `interface`. Lues **par
balise** et pas listées : `lucide-react` en exporte près de sept mille,
et lire sept mille déclarations à chaque frappe se sentirait.

### 19. Taper `>` ne fermait pas la balise

Rapporté sur le même message. VS Code apparie des **caractères**
(`autoClosingPairs`), et une balise fermante est un **nom** : il n'y a
rien à déclarer pour ça dans un `language-configuration.json`.

**Correction.** La règle est à nous, et la partie difficile n'est pas le
JSX, c'est le Rust autour : un `.rsx` fini par `Vec<u32>` répondu par
`</u32>` serait pire que rien. Trois choses doivent tenir, toutes sur la
forme de ce qui est écrit plutôt que sur une liste de noms — le `<` n'est
pas collé à un mot (`Vec<` l'est, `<div` ne l'est jamais), ce qui le suit
est un nom sans espace devant (ce qui écarte `a < b > c`), et `::<`
comme `<<` ne sont pas des balises. Quinze tests, dont dix sur ce qui
doit rester intact.

### Une régression, attrapée par la suite

La correction du n° 10 a d'abord mis l'horodatage du cache de l'éditeur
sur `components/` au lieu de `components/ui`. Or un fichier ajouté dans
`components/ui` déplace la date de `components/ui` et laisse celle de
`components` intacte : `npx shadcn add badge` ne se voyait plus jamais.
`an_answer_is_remembered_until_the_application_changes` l'a dit au
premier essai. L'horodatage est maintenant la plus récente date de
l'arbre entier.

C'est l'argument de la suite existante, et il valait d'être noté : sur
onze corrections, deux régressions ont été attrapées par des tests que
personne n'a écrits pour l'occasion.

---

### 20. Trois refus arrivaient avec des rafales d'espaces

Trouvé en relisant les lignes trop longues du n° 16. Un message de refus
est lu par un développeur dans un terminal, et trois des miens étaient
écrits sur une seule ligne de source de trois cents caractères :

```
this `.await` would stop the render. rustx takes `let answer =
work().await;`              off the render thread by itself -- ...
```

Les continuations `\` que j'avais écrites avaient été mangées par
l'outillage, donc l'indentation de la ligne suivante atterrissait **dans
le message**. Les tests existants n'assertaient que de courts fragments
— `error.message.contains("await")` — et ne pouvaient pas le voir.

**Correction.** Les trois chaînes remises en continuations, et un test
qui ne regarde pas un fragment mais la forme : aucun refus ne contient
deux espaces d'affilée. C'est la seule assertion qui tient quel que soit
le texte, donc elle vaut pour les refus qui n'existent pas encore.

### 21. La boîte aux lettres d'un thread parti grossissait sans fin

Trouvé en dépilant les mineurs laissés de côté. Une future n'écrit
jamais dans une cellule : elle poste dans la boîte aux lettres du thread
qui la possède, et la frame vide la boîte. Deux états étaient prévus --
ouverte, et fermée par `close_remote` quand la fenêtre s'en va, qui
refuse les écritures suivantes au lieu de les garder.

Il en manquait un troisième : le thread qui se termine **sans** fermer.
Une panique, un test, une fenêtre dont le démontage n'a pas tourné. La
boîte survit au thread -- chaque handle déjà donné en tient un `Arc` --
donc elle restait *ouverte*, et une future qui produit encore empilait
dans un vecteur que plus rien au monde ne viderait. Ni appliquées, ni
refusées : gardées, aussi longtemps que la future produit.

**Correction.** Le bout que le thread tient est maintenant une valeur à
lui, et son `Drop` ferme la boîte. C'est la seule place qui soit sûre :
elle tourne quand le thread s'en va, quelle que soit la raison, y
compris celle qu'on n'a pas prévue. Avec, un `Remote::is_open()` --
parce que le test avait besoin de la question, et qu'une future qui
s'apprête à calculer cher a autant besoin de la poser.

## Ce que la chasse n'a pas corrigé, et pourquoi

- **`@tiptap/pm/state` depuis une page `.rsx`.** Le point d'entrée
  importe des racines de paquets, donc une bibliothèque qui ne vit que
  dans ses sous-chemins ne s'écrit pas en balise. La porte de sortie
  existe et elle est bonne : un composant React dans `components/`. Un
  `package.json` qui déclarerait `"@tiptap/pm/state"` comme nom de balise
  serait un deuxième langage à apprendre pour un cas que le TypeScript
  règle mieux.

- **`dependencies_in` lit le JSON à la main.** Un `package.json` dont
  une valeur contient une accolade ou une virgule le tromperait. Aucun
  n'en a, ni dans le labo ni dans les six applications du dépôt, et le
  commentaire qui l'assume est honnête. **On s'en occupe quand** un
  `package.json` réel le cassera ; une dépendance sur un analyseur JSON
  dans le compilateur se paie à chaque construction.

- **Les variables de thème de shadcn.** `bg-primary` n'existe pas tant
  que `assets/tailwind.css` ne porte pas le `@theme` que `shadcn init`
  aurait écrit. C'est le défaut n° 2 vu d'en bas, et la réparation est
  dans le README, pas dans le code : une feuille de style est au projet.

- **`a_heavy_page_renders_in_a_time_a_frame_can_hold`, sous charge.** Il
  tombe dans un `cargo test --workspace` où plusieurs scénarios ouvrent
  des fenêtres, et passe seul. Le verrou « une application à la fois »
  existe ; ce qui manque, c'est qu'il couvre aussi les mesures de durée,
  qui ne veulent rien dire sur une machine dont le compositeur boucle.
  **On s'en occupe quand** la mesure sera lue en `--release`, qui est de
  toute façon le seul chiffre qui compte.

- **Les 0,5 Go de disque.** La machine de la chasse en était là, et
  `rustc` comme Node meurent en disant « out of memory » quand il n'y a
  plus de place pour la mémoire virtuelle. Ce n'est pas un défaut du
  framework, mais c'est la deuxième chose à regarder quand une
  construction tombe sans raison.
