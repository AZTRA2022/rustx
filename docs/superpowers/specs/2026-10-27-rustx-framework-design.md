# rustx — framework hybride Rust + React pour logiciels natifs

Document de conception — 2026-10-27

## 1. Intention

Construire des logiciels desktop aujourd'hui impose un choix frustrant.

Avec Tauri ou Electron, on écrit une interface web agréable, mais le code
qui touche la machine vit dans un autre processus. Pour lister les
périphériques USB, ouvrir une webcam ou enregistrer l'écran, il faut
déclarer une commande côté backend, l'invoquer depuis le frontend, et
sérialiser les données qui transitent. Cette frontière coûte du temps
d'écriture, du temps d'exécution, et surtout elle casse le fil de la
pensée : on ne code plus un logiciel, on code deux moitiés de logiciel
qui se parlent.

Avec egui ou les toolkits natifs, la frontière disparaît mais
l'écosystème aussi. Plus de React, plus de shadcn, plus de tiptap, et une
expérience d'écriture d'interface très loin de ce qu'un développeur front
connaît.

`rustx` supprime le choix. On écrit du `.rsx` : la syntaxe, les balises,
les props et les événements du TSX, mais le langage à l'intérieur des
accolades est du Rust compilé, qui s'exécute dans le même processus que
le rendu. Pas de commande à déclarer, pas d'`invoke()`, pas de
sérialisation. Un `onClick` appelle du Rust comme une fonction appelle
une fonction.

La thèse du projet : **Rust est le langage hôte, et l'écosystème web est
invité chez lui** — pas l'inverse.

### À qui ça s'adresse

`rustx` est un framework public, distribué et versionné comme React,
Next.js ou Svelte le sont. Le public visé en premier est le développeur
TSX qui veut écrire un vrai logiciel natif sans réapprendre un métier. Il
doit pouvoir ouvrir un projet `rustx`, reconnaître ce qu'il voit, et être
productif le premier jour.

## 2. Objectifs

1. **Une syntaxe `.rsx` indiscernable du TSX à l'œil.** JSX nu, sans
   macro enveloppante, sans `rsx! { }`. L'ensemble des balises HTML, des
   attributs et des gestionnaires d'événements du DOM est disponible,
   avec les mêmes noms qu'en React (`onClick`, `onChange`, `className`,
   …).
2. **Zéro frontière.** Le Rust écrit dans un `.rsx` s'exécute dans le
   processus qui porte le moteur de rendu. Un appel d'événement est un
   appel de fonction, pas un message.
3. **L'écosystème npm reste utilisable.** React, tiptap, lucide, shadcn.
   Les fichiers `.tsx` et `.rsx` cohabitent dans la même arborescence et
   s'appellent mutuellement comme de simples composants, dans les deux
   sens.
4. **L'écosystème Cargo reste utilisable.** `git2`, `windows`, `rodio`,
   n'importe quel crate. Le framework fournit des raccourcis, jamais une
   prison.
5. **Les capacités de l'OS sont dans la boîte.** Énumérer les
   périphériques USB, ouvrir une webcam, lister les écrans, enregistrer
   l'écran, notifier, mettre une icône dans la barre système : une ligne,
   pas une expédition dans les crates.
6. **Un seul artefact, une seule commande.** `rustx build` produit un
   `.exe`. Pas deux chaînes de compilation à orchestrer à la main.
7. **L'outillage éditeur est un composant du framework**, pas un bonus :
   autocomplétion, erreurs, survol, aller-à-la-définition dans les
   `.rsx`.
8. **Le plafond est au niveau d'OBS.** Capture d'écran et de caméra,
   audio, encodage, composition temps réel doivent être atteignables.

## 3. Non-objectifs

- **Le runtime Next.js.** Décision explicite du commanditaire : on
  reprend le *confort* de Next.js — routing par fichiers, layouts
  imbriqués, hot reload, conventions de projet — et `rustx` le fournit
  nativement. On ne reprend pas le serveur Node, ni le SSR, ni les server
  components, ni les API routes. Il n'y a pas de serveur dans un logiciel
  desktop.
- **Le mobile et le web.** Hors périmètre à ce stade.
- **macOS et Linux dans la première version.** Voir §5.
- **Compatibilité avec les projets Tauri ou Electron existants.** Pas de
  couche de migration.

## 4. Anti-objectifs

Ce qui ferait de `rustx` un échec même si tout fonctionne techniquement.
Ces cinq points viennent du commanditaire et priment sur les détails
d'implémentation.

1. **Un logiciel amputé de ses meubles.** Si le `.exe` sort sans icône,
   sans métadonnées de version, sans notifications, sans accès
   périphérique — tout ce que n'importe quel logiciel du commerce a — le
   framework a échoué.
2. **Un plafond en dessous d'OBS.** Si on ne peut pas écrire un logiciel
   de capture et de diffusion sérieux avec, c'est un jouet.
3. **Lent ou instable.** Un framework qui rame ou qui crashe ne sera pas
   adopté, quelle que soit l'élégance de sa syntaxe.
4. **La perte de la sûreté mémoire de Rust.** C'est l'anti-objectif le
   plus structurant. Tout le sucre syntaxique — l'état implicite, les
   closures sans `move`, l'absence de `Rc<RefCell<…>>` visible — doit se
   compiler vers du Rust **sûr**. Le code généré ne doit pas reposer sur
   `unsafe` pour faire fonctionner la réactivité. Un développeur doit
   pouvoir écrire `unsafe` s'il le veut ; il ne doit jamais en hériter
   sans l'avoir demandé.
5. **Une expérience réservée aux initiés.** Si écrire un `rustx` demande
   plus de cérémonie que faire une simple page web, l'objectif principal
   est manqué. Le critère de réussite : un développeur TSX ouvre le
   projet et reconnaît ce qu'il voit.

## 5. Contraintes

- **Plateforme de départ : Windows.** Décision du concepteur, non
  contestée par le commanditaire. Il développe sous Windows 10, et le
  premier logiciel de démonstration touche au matériel — les API de
  capture sont spécifiques à chaque OS, viser les trois d'emblée
  diluerait l'effort. L'architecture impose une séparation nette entre le
  cœur du framework et la couche OS, pour que macOS et Linux arrivent
  sans réécriture du cœur.
- **Équipe : deux personnes.** D'où l'importance du point 4 des objectifs
  : on s'appuie sur les crates existants partout où c'est possible, on
  n'écrit soi-même que ce qui n'existe pas.
- **Le projet part de zéro.** Le dépôt `rustx` ne contient qu'un
  `Cargo.toml` (edition 2024) et un `main.rs` vide.
- **Aucune obligation GPL propagée aux utilisateurs du framework.**
  `rustx` étant public, toute obligation de licence qu'il embarque se
  propage à chaque logiciel écrit avec lui. Le commanditaire a d'abord
  demandé des licences strictement permissives, puis a tranché en faveur
  de ffmpeg pour l'encodage. Les deux se concilient : ffmpeg compilé
  **sans** `--enable-gpl` — donc sans x264 ni x265 — est sous LGPL, et
  lié dynamiquement il n'impose rien aux logiciels des utilisateurs.
  C'est la configuration retenue pour `rustx`. Le reste des dépendances
  du framework reste permissif (MIT, Apache-2.0, BSD, ou API système).
  Un développeur qui veut x264 ou une variante GPL de ffmpeg dans *son*
  logiciel l'ajoute lui-même et assume la licence correspondante ;
  `rustx` ne la lui impose pas.
- **Pas de contenu web distant.** Le bac à sable du processus de rendu
  est désactivé (§7.1), ce qui est défendable tant que le seul code qui
  y tourne est celui du développeur. Charger une URL distante, une
  iframe ou un script tiers depuis ce processus ouvrirait une exécution
  de code arbitraire. Décision du concepteur : en v1, un logiciel
  `rustx` ne charge que du contenu local empaqueté dans le binaire.
  C'est une contrainte documentée et appliquée, pas un oubli. Le jour où
  le contenu distant entrera dans le périmètre, il faudra un second type
  de processus de rendu, sous bac à sable, pour les origines non fiables.

## 6. Décisions prises

### 6.1 Prises par le commanditaire

| Décision | Contenu |
|---|---|
| Extension | Le fichier source s'appelle `.rsx`. On n'essaie pas de rester en `.rs`. |
| Outillage | L'analyseur / extension éditeur fait partie du produit, on l'écrit. |
| Syntaxe des événements | La valeur d'un gestionnaire est une **vraie closure Rust** : `onClick={ \|\| n += 1 }`. Raison : `rust-analyzer` doit pouvoir lire ce qu'il y a entre les accolades, sinon l'autocomplétion disparaît — inacceptable. |
| Déclaration d'état | Pas de `use_state`, pas de `signal`. On écrit `let mut n = 0;`, le compilateur fait le reste. |
| Débit des mises à jour | Le framework protège automatiquement le développeur contre les rafales de mutations, sans restreindre ce qu'il a le droit d'écrire. |
| Mixité | Les `.tsx` et les `.rsx` cohabitent et s'appellent mutuellement. Certaines bibliothèques JS en ont besoin. |
| Crates externes | Autorisés et encouragés. |
| Portée | Framework public, pas outil interne. |
| Définition de « fini » pour la v1 | La v1 est atteinte quand le logiciel de preuve (§8) fonctionne bien. La couche OS s'étoffe ensuite, elle n'a pas à être complète avant. Pas de date imposée. |
| Éditeur | VS Code d'abord. Mais l'extension vient **en dernier** : la stabilité du moteur passe avant l'outillage. |
| Licences | Permissives uniquement. Voir §5. |
| Hot reload | À la React : l'état de l'application survit à une modification du code Rust. Un redémarrage rapide ne suffit pas. |

### 6.2 Prises par le concepteur

Ces décisions sont des appels du concepteur, pas des choix validés par le
commanditaire. Elles sont renversables.

**Moteur de rendu : Chromium embarqué (CEF) pour la v1, architecture
ouverte à Servo.** Trois voies existaient :

- *WebView2 / WKWebView* (la voie Tauri) : léger, mais multi-processus
  par conception et sans moyen d'injecter du natif dans le contexte JS du
  moteur. Écarté : il réintroduit exactement la frontière qu'on tue.
- *Servo* : moteur web écrit en Rust, embarquable, cohérent avec la thèse
  du projet. Mais sa compatibilité CSS et JS reste incomplète, et des
  bibliothèques npm casseront. Écarté pour la v1, gardé en ligne de mire.
- *CEF* : compatibilité web totale dès le premier jour, tiptap et shadcn
  fonctionnent. Coût : un moteur C++ volumineux que Rust pilote. Retenu,
  parce que l'anti-objectif n°2 (le plafond OBS) et l'objectif n°3
  (l'écosystème npm) pèsent plus lourd que la pureté linguistique.

La couche d'adaptation au moteur est une frontière interne explicite du
projet, pour qu'un remplacement par Servo plus tard soit un travail
circonscrit.

**Type de retour d'un composant : `Element`.** Le commanditaire avait
laissé ce nom au brainstorming. `fn Counter() -> Element`.

**Nom du binaire et de la CLI : `rustx`.**

**Pas de fork de Chromium.** On reste sur des binaires CEF publiés. Les
mutations du DOM passent par les liaisons V8 du processus de rendu, et
non par des appels directs à Blink — l'API publique de CEF n'expose pas
le DOM de Blink en écriture, et l'exposer demanderait de maintenir un
fork de Chromium à deux personnes. Ce n'est pas une concession sur la
thèse : les appels restent dans le processus, en pile, sans
sérialisation ni IPC. Seul le coût d'une transition Rust → V8 → DOM
subsiste, et il se compte en nanosecondes là où un aller-retour IPC se
compte en microsecondes.

**Pas de contenu distant en v1.** Voir §5.

**Bar de performance** (§8) : fixée par le concepteur faute d'exigence
chiffrée, et renversable.

## 7. Architecture

### 7.1 Le modèle de processus — le cœur de la thèse

C'est le point où `rustx` se distingue de tout ce qui existe, et il faut
le comprendre avant le reste.

Chromium est multi-processus : un processus navigateur, un ou plusieurs
processus de rendu qui portent Blink (le DOM) et V8 (le JS). Dans Tauri
ou Electron, le code applicatif vit dans le processus navigateur et parle
au rendu par messages — c'est la frontière.

Dans `rustx`, **le code applicatif Rust vit dans le processus de
rendu**, aux côtés de Blink et V8. CEF permet ce montage : le même
binaire est relancé comme sous-processus de rendu, et le code Rust s'y
accroche. Conséquence directe :

- Un `onClick` appelle une fonction Rust, en pile, dans le thread où
  tourne le JS. Zéro sérialisation.
- Les composants `.rsx` produisent des mutations DOM par appels directs
  aux liaisons V8 du même processus, pas par envoi de patches à
  sérialiser.
- Un composant Rust peut monter un composant React et lui passer des
  données sans les copier ni les encoder.
- Le bac à sable du processus de rendu est désactivé par défaut : c'est
  un logiciel desktop, pas un navigateur, et le code qui y tourne est
  celui du développeur.

Le travail lourd (capture, encodage, I/O) part sur des threads du même
processus, et les images remontent vers le canvas sans copie.

Cette décision a un revers qu'il faut assumer : un panic ou une
corruption dans le code applicatif abat le processus de rendu. C'est le
prix de l'absence de frontière, et c'est précisément pourquoi
l'anti-objectif n°4 — rester dans du Rust sûr — n'est pas négociable.

### 7.2 Les couches

```
┌──────────────────────────────────────────────┐
│  Application : .rsx  +  .tsx  +  crates      │
├──────────────────────────────────────────────┤
│  rustx-std : OS, périphériques, capture,     │
│  notifications, fenêtres, fichiers           │
├──────────────────────────────────────────────┤
│  rustx-runtime : état, réactivité, rendu,    │
│  routing, pont JS ↔ Rust                     │
├──────────────────────────────────────────────┤
│  rustx-engine : adaptation au moteur web     │
│  (CEF aujourd'hui, Servo visé)               │
├──────────────────────────────────────────────┤
│  rustx-compiler : .rsx → .rs                 │
│  rustx-cli / rustx-lsp                       │
└──────────────────────────────────────────────┘
```

### 7.3 Le compilateur `.rsx`

Un `.rsx` n'est pas du Rust valide ; c'est du Rust *étendu* par le JSX.
Le compilateur le transpile en `.rs` ordinaire, déposé hors de
l'arborescence source, avec une **carte de correspondance position à
position** vers le `.rsx` d'origine. Cette carte sert à deux choses : les
diagnostics `rustc` renvoyés aux bonnes lignes, et le serveur de langage
(§7.7).

La transpilation est déterministe : même entrée, même sortie, pour que
les builds soient reproductibles et que la carte reste fiable.

### 7.4 L'état et la réactivité

C'est le morceau le plus exigeant du projet, et c'est le cœur de
l'expérience d'écriture. Le code que l'on veut rendre possible :

```rust
fn Counter() -> Element {
    let mut n = 0;
    return <button onClick={|| n += 1}>{n}</button>;
}
```

Aucune annotation. Pas de `use_state`, pas de `signal`, pas de `move`,
pas de `Rc<RefCell<…>>`.

**La règle de promotion**, telle qu'énoncée au commanditaire et
approuvée : le compilateur analyse le corps de chaque composant. Une
variable locale qui est à la fois **lue dans le JSX retourné** et **mutée
depuis une closure de gestionnaire d'événement** (ou depuis un bloc
asynchrone) est promue en cellule d'état persistante, identifiée par son
emplacement dans le code, et survit aux re-rendus. Toute autre variable
reste un `let` Rust ordinaire, sans coût.

> **La forme retenue, au 2026-11-06.** La moitié « lue dans le JSX » a
> été abandonnée : une **mutation** depuis un gestionnaire ou depuis un
> bloc `async` suffit à promouvoir. La règle à deux moitiés engendrait du
> Rust qui ne compile pas pour une variable écrite et montrée nulle part
> — un gestionnaire est rangé dans un `Rc<dyn Fn>`, et une fermeture qui
> affecte une variable capturée ne peut pas en être une. La moitié « lue »
> ne choisissait donc pas entre une cellule et un `let` ordinaire : elle
> choisissait entre une cellule et une erreur d'emprunt sur un fichier
> engendré.
>
> Et pour le bloc `async`, que ce paragraphe nommait déjà : la capture est
> un **handle** qui porte l'identité de la valeur et rien d'autre
> (`Remote`), l'écriture part sur la file de la frame, et la frame
> l'applique sur le thread qui possède la cellule. Une **lecture** dans le
> bloc est refusée en toutes lettres — la valeur vit sur le thread qui
> dessine — et ce qu'il faut lire se lit avant le bloc. Mille écritures
> depuis une boucle `async` coûtent un rendu, par le même marquage que
> tout le reste.

Les captures sont réécrites pour passer par des handles plutôt que par
des emprunts, ce qui rend les conflits d'emprunt structurellement
impossibles — sans `unsafe`.

**Le cadencement**, également approuvé : muter l'état ne déclenche pas un
rendu. Cela marque le composant comme sale. Le rendu a lieu une fois par
frame, calé sur le vsync. Un thread de capture qui pousse 60 images par
seconde obtient 60 rendus ; une boucle qui pousse 10 000 mises à jour par
seconde obtient toujours 60 rendus. Le développeur écrit sa boucle aussi
vite qu'il veut et ne peut pas se tirer dans le pied. Pas de debounce à
configurer, pas d'API de freinage à apprendre.

**Le modèle de threads** (décision du concepteur). L'état appartient au
thread d'interface du processus de rendu, et à lui seul. Il n'est ni
`Send` ni `Sync`, il n'y a pas de verrou sur le chemin chaud, et un
`let mut n = 0;` n'impose aucune contrainte de thread à ce que la
closure capture. Un thread de travail — capture, encodage, I/O — ne
mute pas l'état directement : il poste sa mutation dans une file que le
thread d'interface vide une fois par frame. Du point de vue du
développeur c'est transparent, le code s'écrit pareil ; ce qui change,
c'est qu'il n'y a jamais de course possible.

**Le multi-fenêtres** (décision du concepteur). Une fenêtre correspond à
un processus de rendu, donc à son propre arbre d'état. Pour les données
partagées entre fenêtres, le framework fournit un magasin explicite,
hébergé dans le processus navigateur. C'est le seul endroit du système
où une frontière subsiste — elle est nommée, visible, et on ne la
traverse que si l'on écrit une application multi-fenêtres qui partage un
modèle.

### 7.4 bis — Le contrat d'un composant

Le compteur de §7.4 ne montre qu'un cas. Tout ce qui suit fait partie de
la v1, parce que chacun de ces points est une fonctionnalité du
*compilateur* — l'analyse de promotion doit les voir — et non une
bibliothèque qu'on ajoutera plus tard.

- **Props** : paramètres typés de la fonction composant. Un changement
  de prop re-rend le composant.
- **Listes** : l'identité d'une cellule d'état par emplacement dans le
  code ne suffit pas pour `n` éléments issus d'un `Vec`. Un attribut
  `key`, de même sémantique qu'en React, identifie l'état à travers les
  réordonnancements.
- **Montage et démontage** : une cellule promue est libérée au
  démontage du composant, avec exécution du `Drop` de Rust. C'est le
  mécanisme de nettoyage naturel du langage, on n'en invente pas
  d'autre.
- **Effets et tâches asynchrones** : lancer un travail au montage,
  l'annuler au démontage. Le logiciel de preuve en a besoin dès le
  premier jour — on ouvre un flux caméra, il faut le fermer.
- **État partagé** : un mécanisme de contexte ou de magasin, pour que
  deux composants éloignés voient la même donnée sans la faire
  descendre de parent en parent. Le logiciel de preuve en a besoin : la
  liste des périphériques et le panneau d'aperçu partagent un état.

### 7.5 Interopérabilité avec JavaScript

Dans les deux sens, et sans sérialisation JSON :

- Un `.rsx` monte un composant React importé d'un `.tsx` ou d'un package
  npm, en lui passant des props.
- Un `.tsx` monte un composant `.rsx` comme n'importe quel composant
  React.
- Les deux arbres partagent le même DOM et le même cycle de rendu.

Deux règles d'appropriation (décisions du concepteur) évitent que les
deux réconciliateurs se marchent dessus :

- **React possède son sous-arbre.** Un composant Rust qui monte du React
  lui donne un nœud conteneur dont React a la pleine propriété, et le
  réconciliateur de `rustx` n'y touche plus. Les deux algorithmes ne
  s'entrelacent jamais dans le même sous-arbre ; ils se passent le
  relais à une frontière de nœud. Symétriquement pour un composant
  `.rsx` monté depuis du React.
- **Rien n'est emprunté à travers la frontière.** Une valeur Rust
  transmise en prop à du JavaScript est déplacée ou clonée dans une
  cellule gérée par le ramasse-miettes de V8. Aucune référence Rust ne
  survit dans V8 : c'est V8 qui décide quand le JavaScript lâche son
  handle, et il peut le faire après la mort du propriétaire Rust. Pour
  les gros volumes — une image de caméra — ce n'est pas une copie mais
  un tampon partagé dont la propriété est transférée, pas prêtée.
  L'anti-objectif n°4 interdit de régler ce problème avec `unsafe`.

Les dépendances npm sont gérées par le gestionnaire de paquets habituel
du développeur et empaquetées dans le binaire au build.

### 7.6 La couche OS

C'est ce qui sépare `rustx` d'un Tauri plus rapide. Le périmètre visé est
dicté par les anti-objectifs n°1 et n°2 : tout ce qu'il faut pour écrire
un logiciel du commerce, et tout ce qu'il faut pour écrire un OBS.

Énumération et lecture des périphériques (USB, caméras, micros, écrans) ;
capture caméra et capture d'écran ; audio en entrée et en sortie ;
encodage ; notifications système ; icône et menu de barre système ; icône
d'application et métadonnées de version dans le binaire ; dialogues de
fichiers ; presse-papiers ; glisser-déposer ; raccourcis globaux ; gestion
multi-fenêtres.

Chaque capacité s'appuie sur un crate existant quand il en existe un bon.
L'apport de `rustx` est l'unification : une API cohérente, documentée au
même endroit, utilisable sans chercher.

### 7.7 L'outillage

**Ordre imposé par le commanditaire : l'outillage vient en dernier.** La
stabilité du moteur passe avant l'extension éditeur. Concrètement, on
développe le framework en écrivant du `.rsx` sans autocomplétion le
temps que le moteur tienne debout, et l'extension arrive une fois le
logiciel de preuve fonctionnel. L'objectif n°7 reste un objectif de
v1 — c'est son rang dans la file qui est fixé, pas son existence.

- **`rustx-cli`** : création de projet, `dev` avec hot reload, `build`
  vers un `.exe` unique. Le hot reload préserve l'état de l'application
  à travers une recompilation du code Rust (exigence du commanditaire,
  pas un redémarrage rapide) : c'est un sous-système à part entière, et
  il contraint la façon dont le compilateur dispose les cellules d'état,
  qui doivent pouvoir être retrouvées et réinjectées après rechargement.
- **`rustx-lsp`** : serveur de langage qui se place devant
  `rust-analyzer`. Il lui transmet le `.rs` généré, récupère complétions,
  diagnostics, survols et définitions, et les retraduit aux positions du
  `.rsx` via la carte de correspondance. C'est ce qui rend la syntaxe nue
  viable.
- **Extensions éditeur** : coloration syntaxique et branchement du LSP.
- **Routing par fichiers et layouts imbriqués**, à la manière de Next.js,
  fournis par le runtime.

## 8. Le logiciel de preuve

Un logiciel de capture et d'affichage de périphériques : il énumère ce
qui est branché, ouvre un flux caméra, affiche les données reçues en
temps réel, et enregistre.

Il a été choisi par le commanditaire parce qu'il prouve les deux moitiés
de la thèse d'un seul coup : la nature bas niveau du framework, et
l'interface. C'est aussi le cas où une architecture à IPC s'effondre
visiblement — pousser 60 images par seconde à travers une frontière
sérialisée ne tient pas. Si ce logiciel est fluide, `rustx` a démontré ce
qu'il annonce.

Il sert de critère d'acceptation, pas de simple démo — et il définit la
v1 : **`rustx` sort quand ce logiciel fonctionne bien.** La couche OS
s'étoffe après, elle n'a pas besoin d'être complète pour la première
version.

« Fonctionne bien » demande des chiffres, sinon l'anti-objectif n°3 est
intestable. Faute d'exigence chiffrée de la part du commanditaire, le
concepteur pose cette barre, qui reste renversable :

- aperçu caméra 1080p à 30 images/seconde, sans image perdue ;
- latence de bout en bout (capteur → pixel à l'écran) sous 100 ms ;
- interface à 60 images/seconde pendant la capture ;
- moins de 15 % de CPU sur un portable milieu de gamme de 2020 ;
- `.exe` et ressources sous 200 Mo installés.

## 9. Ce qui est laissé au constructeur

- Le découpage exact en crates et l'ordre d'implémentation.
- Les bindings CEF retenus et la façon de s'accrocher au processus de
  rendu.
- La représentation interne de l'arbre d'éléments et l'algorithme de
  réconciliation.
- La grammaire formelle du `.rsx` et la technologie de parsing.
- Le format de la carte de correspondance.
- La stratégie de tests, y compris la façon de tester du code qui a
  besoin d'un moteur web vivant.
- Le choix des crates pour chaque capacité OS.
- Le mécanisme de hot reload.
- La stratégie d'empaquetage des assets JS dans le binaire.

## 10. Risques connus

| Risque | Nature |
|---|---|
| Le montage CEF en processus de rendu | C'est le pari central. Il doit être validé par un spike avant toute autre chose : un `.exe` qui affiche une page et appelle une fonction Rust depuis un `onClick` du DOM, dans le même processus. Si ce montage ne tient pas, l'architecture entière est à revoir. |
| L'analyse de promotion d'état | La règle est claire sur les cas simples. Les cas limites (variables capturées en cascade, mutations conditionnelles, closures imbriquées) demanderont des règles précises, et un message d'erreur compréhensible quand le compilateur ne peut pas trancher. |
| Le LSP par procuration | Faire transiter les diagnostics de `rust-analyzer` à travers une carte de positions est faisable mais délicat ; la qualité perçue du framework en dépend directement. |
| Le hot reload avec préservation d'état | Recharger du code natif en gardant l'état vivant est un problème dur : rechargement de bibliothèque dynamique, stabilité de l'ABI, réinjection des cellules après un changement de disposition. C'est un sous-système, pas une option de CLI, et il contraint le compilateur. |
| Le poids du binaire | Chromium embarqué pèse lourd. Barre fixée à 200 Mo installés (§8), à mesurer tôt. |
| L'encodage via ffmpeg LGPL | Il faut une chaîne de compilation ffmpeg sous Windows produisant des DLL LGPL, et veiller à ce qu'aucune dépendance GPL n'y entre par inadvertance. La liaison dynamique est une obligation de licence, pas un détail de build : une liaison statique changerait les obligations. |
| La taille du périmètre | Les objectifs couvrent un compilateur, un runtime, une couche OS, une CLI et un serveur de langage, à deux personnes. L'ordre d'attaque compte plus que tout. |
