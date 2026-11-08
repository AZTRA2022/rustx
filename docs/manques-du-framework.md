# Ce qui manque, et ce qui est cassé

Registre tenu en écrivant une vraie application sur rustx — un
navigateur de dépôts GitHub : routes imbriquées, paramètres, palette de
commandes, recherche avec *debounce* sur une vraie API HTTPS, tâches
concurrentes, graphes recharts, éditeur TipTap, presse-papiers et
dialogues natifs. Rien n'a été écrit pour plaire au framework : chaque
ligne est ce qu'un développeur écrit d'abord, et ce registre dit ce qui
s'est passé quand il l'a écrit.

Trois colonnes : ce qui est **cassé** (le framework émet du code faux ou
refuse du Rust correct), ce qui **manque** (une fonctionnalité annoncée
ailleurs, ou attendue de quiconque vient de Next), et ce qui **coûte
trop cher** (ça marche, mais le développeur paie une cérémonie que le
framework promettait de tuer).

## Résumé

| # | nature | pièce | ce que c'est | état |
|---|--------|-------|--------------|------|
| 1 | cassé | `rustx-compiler` | une écriture dans un bras de `match` avale les bras suivants | corrigé |
| 2 | cassé | `rustx-compiler` | un nom dans un commentaire d'un bloc `async` est réécrit | corrigé |
| 3 | cassé | `rustx-compiler` | `autre.dit()` pris pour une lecture de l'état `dit` | corrigé |
| 4 | cassé | `rustx-compiler` | une cellule de composant lue dans le corps n'est pas réécrite | corrigé |
| 5 | cassé | `rustx-compiler` | un `let mut` **dans** un bloc `async` est pris pour une cellule | **corrigé** |
| 6 | cassé | `rustx-compiler` | une capture en ligne `format!("{x}")` n'est pas réécrite | **corrigé** |
| 7 | cassé | `rustx-compiler` | un `let mut` d'une fonction utilitaire d'une page est promu | **corrigé** |
| 8 | cassé | `rustx-compiler` | un `async fn` à côté de la page ne peut pas toucher l'état | **refus explicite** (voir 29) |
| 9 | manque | `rustx-dom` | un `Element` ne peut pas être enfant d'un composant React | **corrigé** |
| 10 | manque | `rustx-dom` | `Event` ne porte que `kind` : le texte tapé est hors d'atteinte | **corrigé** |
| 11 | manque | `rustx-dom` | `Value` n'a aucun accesseur (`as_str`, `as_f64`, `get`) | **corrigé** |
| 12 | manque | `rustx-compiler` | un callback ne reçoit que le **premier** argument | **corrigé** |
| 13 | manque | routeur | pas de *query string* : `/depots?q=rust` ne correspond à rien | **corrigé** |
| 14 | manque | routeur | pas de segment attrape-tout `[...chemin]` | **corrigé** |
| 15 | manque | routeur | pas de groupes de routes `(section)` | **corrigé** |
| 16 | manque | routeur | pas de `forward`, pas de `replace` | **corrigé** |
| 17 | manque | routeur | un `Layout` ne reçoit pas les paramètres de la route | **corrigé** |
| 18 | manque | routeur | pas d'écran de chargement ni de frontière d'erreur par segment | **corrigé** |
| 19 | manque | routeur | la pile de navigation n'est pas lisible | **corrigé** |
| 20 | coûte | `rustx-engine` | un `.await` de composant n'est posé qu'une fois : `with_key` obligatoire | **corrigé** |
| 21 | coûte | `rustx-macro` | un refus de page entraîne un second message sur `async main` | **corrigé** |
| 22 | coûte | `rustx-lsp` | la complétion exige le chemin écrit en entier | **corrigé** |
| 23 | coûte | documentation | le README dit encore qu'une lecture dans un bloc est refusée | **corrigé** |
| 24 | cassé | `rustx-compiler` | l'annotation de type d'une valeur de page était perdue | **corrigé** |
| 25 | cassé | `rustx-compiler` | une mutation par méthode qui rend une valeur ne compilait pas | **corrigé** |
| 26 | cassé | `rustx-compiler` | une valeur qu'un futur écrit et que l'écran ne capture qu'en ligne n'était pas liée | **corrigé** |
| 27 | cassé | `rustx-compiler` | une écriture dont la partie droite lit une autre valeur de page était éditée deux fois | **corrigé** |
| 28 | cassé | `rustx-compiler` | une fonction ordinaire à côté d'une page n'atteignait pas ses valeurs | **corrigé** |
| 29 | manque | `rustx-compiler` | une `async fn` ne peut pas atteindre une valeur de page (refus explicite) | ouvert |
| 30 | coûte | `rustx-compiler` | un `.await` de composant qui emprunte un local est refusé par rustc dans un fichier généré | documenté, pas refusé |
| 35 | cassé | `rustx-compiler` | un commentaire avant une écriture annulait l'écriture | **corrigé** |
| 36 | cassé | `rustx-dom` / `rustx-engine` | `style="…"` traversait la frontière React en chaîne, pas en objet | **corrigé** |
| 37 | manque | `rustx-engine` | une collision de nom avec une bibliothèque JS faisait tomber tout l'écran sans un mot | **le silence est corrigé** ; la collision reste un piège à documenter |
| 38 | cassé | `rustx-engine` | `router::replace`/`navigate` depuis un `async` sur un autre thread ne faisaient rien | **corrigé** |
| 39 | coûte | `rustx-compiler` | `rustx dev` reconstruisait le projet sans qu'aucun fichier n'ait changé | **corrigé** |
| 40 | cassé | `rustx-cli` / `rustx-engine` | une fenêtre encore ouverte faisait échouer la construction (os error 5) | **corrigé** |
| 41 | cassé | `rustx-cli` | `rustx new` écrivait `version = "0.1"`, obsolète dès 0.2.0 | **corrigé** |
| 42 | manque | `rustx-cli` | `rustx new` ne laissait rien à la première `npm install` | **corrigé** |
| 43 | coûte | `rustx-lsp` | une mort de `rust-analyzer` rendait le LSP aveugle jusqu'au rechargement | **corrigé** |
| 44 | manque | installation | pas de panneaux d'installation ni de métadonnées (auteur, version…) par OS | ouvert |
| 45 | manque | `rustx-os` | raccourcis globaux, barre système et contrôle de fenêtre non écrits sur Linux et macOS | ouvert — le reste (presse-papiers, notifications, démarrage auto, instance unique, caméras, dialogue de fichiers) est réel sur les deux |
| 46 | manque | `rustx-lsp` | pas d'autocomplétion ni d'auto-import des crates installées dans un `.rsx` | ouvert |

## Ce qui est cassé

### 1. Une écriture dans un bras de `match` avale les bras suivants

```rust
<Button onClick={move |_| {
    match rustx::clipboard::text() {
        Ok(texte) => titre = texte,
        Err(pourquoi) => dit = format!("refusé : {pourquoi}"),
    };
}}>
```

`expression_after` (`rewrite.rs`) cherchait la fin de la valeur écrite
et s'arrêtait sur `;` ou sur une accolade fermante — **pas sur une
virgule**. L'écriture du premier bras emportait donc tout le reste :

```rust
titre.set(texte, Err(pourquoi) => dit = format!(…))
```

Et le développeur lisait « this Rust does not parse » sur une ligne
qu'il avait écrite juste. Sur plusieurs lignes, le dégât remontait
jusqu'au scanner et devenait « unclosed tag \<Button\> », qui désigne
une balise sans rapport.

**Corrigé** : une virgule de profondeur zéro termine la valeur. Une
virgule *dans* la valeur — `vec![1, 2]`, `(a, b)`, `Foo { a: 1, b: 2 }` —
est derrière un délimiteur, donc à une profondeur qui lui appartient.

### 2. Un nom dans un commentaire d'un bloc `async` est réécrit

Partout ailleurs le compilateur cherche un nom à travers `push_code`,
qui écarte les chaînes et les commentaires. `uses_in` — la fonction qui
sert aux blocs `async` — regardait la **région brute**. Un commentaire
qui mentionnait la valeur sortait donc réécrit :

```rust
// Le numéro est noté à côté : `verifies` sera écrit par la frame.
// devenait
// Le numéro est noté à côté : `verifies.get()` sera écrit par la frame.
```

**Corrigé** : `uses_in` passe par `push_code` comme le reste.

### 3. `autre.dit()` pris pour une lecture de l'état `dit`

Une page déclare `let mut dit`, et l'écosystème a le droit d'avoir une
méthode `.dit()` — `Echec::dit()` dans l'application qui a trouvé ça.
Le nom était reconnu sans regarder ce qui le précédait :

```rust
dit = pourquoi.dit();
// devenait
dit.set(pourquoi.dit.get()())et()();
```

Deux fautes en une : le nom de méthode lu comme une lecture, puis
l'édition appliquée **par-dessus elle-même**, parce que cette fausse
lecture tombait dans l'étendue de l'écriture.

**Corrigé** : un nom précédé d'un `.` seul ou d'un `::` appartient à
quelqu'un d'autre. Deux points sont une plage — `0..combien` reste une
lecture — et c'est toute la règle. Le byte **immédiatement** avant, pas
le dernier non-espace : un commentaire finit par un point, et
`// … pendant qu'on dessine.` suivi d'`effect(…)` à la ligne suivante
faisait passer l'appel pour un champ.

### 4. Une cellule de composant lue dans le corps n'est pas réécrite

```rust
fn Page() -> Element {
    let mut n = 0;
    let double = n * 2;          // ← laissé tel quel
    return <button onClick={move |_| n += 1}>{double}</button>;
}
```

`n` est devenu une cellule, et cette ligne nommait la cellule : rustc
refusait, sur une ligne correcte, avec un message sur un type que le
développeur n'a jamais écrit. Les lectures dans les balises et dans les
gestionnaires marchaient, celles du corps non — c'est-à-dire la forme la
plus naturelle.

La cause est une ligne : le code après la déclaration était choisi
**par spans entiers** (`span.start >= declaration.end`), et le span qui
couvre le corps commence avant la déclaration. Il était donc jeté en
entier, avec toutes les lectures qu'il contenait.

**Corrigé** : le span est **coupé** à la déclaration au lieu d'être
jeté.

### 5. Un `let mut` écrit dans un bloc `async` est pris pour une cellule

```rust
async move {
    let mut dernier_vu = String::new();   // la mémoire de la boucle
    loop { … if terme != dernier_vu { … } }
};
```

`dernier_vu` est une variable locale de la future. Personne d'autre ne
la voit, elle ne survit à rien, et elle n'a aucune raison d'être un
état. Elle est pourtant promue en cellule de composant, puis **refusée à
la lecture** par la règle qui protège les cellules — refus juste pour
une cellule, absurde pour une locale.

Le contournement est de la déclarer au sommet de la page, ce qui en fait
un état partagé qui n'en est pas un. Deux pages de l'application de test
le portent, chacune avec un commentaire qui s'en excuse.

### 6. Une capture en ligne `format!("{x}")` n'est pas réécrite

```rust
let texte = format!("clics: {clics} — titre: {titre}");
```

Les captures en ligne de Rust 2021 vivent **dans** le littéral, et le
littéral est de la prose pour le compilateur. Les noms ne sont donc pas
réécrits, `format!` capture la poignée elle-même, et rustc dit
`Shared<u64> doesn't implement Display`.

La forme positionnelle marche — `format!("clics: {}", clics)` — et c'est
celle que le README utilise. Le framework exige donc en silence le style
d'avant 2021, sur la fonction la plus écrite de Rust.

### 7. Un `let mut` d'une fonction utilitaire d'une page est promu

```rust
// dans un `.rsx`, à côté des composants
fn deux_segments(chemin: &str) -> (String, String) {
    let mut segments = chemin.split('/').filter(|part| !part.is_empty());
    …
}
```

`deux_segments` ne rend pas un `Element` et n'est pas un composant. Son
`let mut` est quand même traité comme de l'état, et le code émis ne
compile plus : `cannot find value segments`.

### 8. Un `async fn` à côté de la page ne peut pas toucher l'état

```rust
let mut avancement: Vec<u32> = Vec::new();

fn Page() -> Element {
    async move { tokio::join!(ouvrier(0, 60), ouvrier(1, 95)); };
    …
}

async fn ouvrier(qui: usize, pas: u64) {
    … avancement[qui] = etape * 10;   // cannot find value `avancement`
}
```

Le travail d'une page sérieuse ne tient pas dans le corps du composant.
Sortir une `async fn` est ce que tout le monde fait, et l'état n'y est
pas atteignable : tout doit être écrit en ligne dans le bloc.

## Ce qui manque

### 9. Un `Element` ne peut pas être enfant d'un composant React

`ForeignChild` a `Node`, `Text` et `Value`. Il n'a pas `Element`. Donc

```rust
<CardContent>
    {if ouvert { Panneau() } else { Element::Empty }}
</CardContent>
```

ne compile pas : `the trait bound rustx::Element: ToValue is not
satisfied`. Un composant rustx peut être enfant d'une balise native, pas
d'un composant de bibliothèque — alors que mélanger les deux est
l'argument de vente du framework, et que shadcn met des `Card` partout.

### 10. `Event` ne porte que `kind` : le texte tapé est hors d'atteinte

```rust
<input onInput={move |event| /* … et maintenant ? */} />
```

`Event { kind: String }` dit *ce qui s'est passé* et jamais *ce qui est
écrit*. Il n'y a donc, depuis le Rust seul, aucun moyen de lire un champ
de texte : pas de valeur, pas de touche, pas de modificateurs, pas de
cible. `Node::attribute("value")` ne sert à rien — l'attribut garde la
valeur initiale, le texte vit dans la propriété.

La porte de sortie existe et elle marche : une prop de fonction d'un
composant React reçoit la première valeur que le JavaScript lui passe,
donc trois lignes de TypeScript referment la chaîne. L'application de
test porte ce composant, et il s'appelle `Champ.tsx`. Mais **un
formulaire est la moitié des applications**, et il commence par quitter
Rust.

### 11. `Value` n'a aucun accesseur

Pas d'`as_str`, pas d'`as_f64`, pas de `get("champ")`. Chaque rappel
venu d'une bibliothèque se démonte donc à la main, par un `match` sur
six variantes, dans chaque application. L'application de test a écrit
`src/js.rs` pour ne le faire qu'une fois.

### 12. Un callback ne reçoit que le premier argument

`Callback::first(__arguments)` : ce qu'une bibliothèque passe en
deuxième position est perdu. `onChange(valeur, index)`,
`onDrop(resultat, fournisseur)`, `onResize(largeur, hauteur)` sont des
signatures ordinaires de l'écosystème React.

### 13 à 19. Le routeur, mesuré contre Next

Ce qu'il a : les segments statiques, `[param]`, « le statique bat le
paramètre », les layouts imbriqués par profondeur, `not_found`,
`navigate`, `back`, `current`. Vérifié dans une vraie fenêtre, trois
layouts empilés, deux paramètres, cinq segments de profondeur : tout
cela marche.

Ce qu'il n'a pas :

- **13. la *query string***. `navigate("/depots?q=rust")` ne correspond
  à aucune route, donc l'écran « introuvable » s'ouvre. Un filtre, un
  tri, une page de résultats partageable n'ont pas d'endroit où vivre.
- **14. l'attrape-tout** `[...chemin]`. Un dossier nommé `[...chemin]`
  déclare un paramètre qui s'appelle `...chemin` et ne prend qu'un seul
  segment. Toute documentation, tout arbre de fichiers, tout chemin de
  profondeur inconnue est hors de portée.
- **15. les groupes de routes** `(section)`, qui chez Next permettent de
  partager un layout sans ajouter un segment à l'URL.
- **16. `forward` et `replace`**. `back()` existe seul : on ne peut pas
  refaire le pas en avant, ni remplacer l'entrée courante — donc toute
  navigation après une validation laisse le formulaire dans l'historique.
- **17. les paramètres dans un `Layout`**. `fn Layout(children: Element)`
  et rien d'autre, alors que chez Next un layout reçoit `params`. Le
  cadre d'un dépôt doit donc relire `current()` et le découper lui-même,
  en redisant ce que le routeur savait déjà.
- **18. `loading` et `error` par segment**. Chez Next, un fichier par
  dossier donne l'écran d'attente et la frontière d'erreur. Ici chaque
  page refait à la main ses quatre états — vide, en vol, trouvé, échoué —
  et une panique dans un écran n'a pas de frontière à elle.
- **19. la pile n'est pas lisible**. `back()` rend `false` quand il n'y a
  rien derrière : c'est la seule façon de le savoir, donc un bouton
  « retour » ne peut pas être grisé avant d'avoir été cliqué.

## Ce qui coûte trop cher

### 20. Un `.await` de composant n'est posé qu'une fois

`let x = charger().await;` dans un composant ne pose sa question
**qu'une fois par cellule**. Deux dépôts à la même route sont le même
composant au même endroit de l'arbre, donc la même cellule : sans clé,
le second affiche la réponse du premier. Il faut écrire

```rust
{with_key(clef.clone(), || Resume(proprietaire, nom))}
```

ce qui est juste, mais c'est au développeur de savoir qu'une route à
paramètre réutilise la cellule — et de le savoir **avant** le bug, pas
après.

### 21. Un refus de page entraîne un second message sans rapport

Toute page refusée produit aussi `error[E0752]: main function is not
allowed to be async`, parce que la macro abandonne avant d'avoir enlevé
le `async`. Le développeur reçoit deux erreurs dont une est fausse, et
la fausse désigne `src/main.rs`.

### 22. La complétion exige le chemin écrit en entier

Dans un `.rsx`, il faut avoir tapé `nomducrate::` avant que la méthode
voulue soit proposée. rust-analyzer propose sur le préfixe seul.

### 23. Le README se contredit

La section « Attendre une valeur sans bloquer le rendu » dit encore
qu'**une lecture dans le bloc est refusée**, et donne un exemple où la
valeur lue est une valeur de page — c'est-à-dire précisément ce qui
marche depuis que lire une valeur de page depuis une future a été
implémenté. Deux sections du même fichier disent le contraire l'une de
l'autre.

### 24 à 28. Cinq réécritures fausses, corrigées

Relevées en construisant le labo, chacune avec sa loi écrite avant le
correctif.

- **L'annotation de type était perdue.** `let mut visites: Vec<String> =
  Vec::new();` devenait `shared("visites", || Vec::new())` : le `T` du
  magasin n'avait plus rien pour s'inférer. La closure porte maintenant
  le type.
- **Une mutation par méthode qui rend une valeur.**
  `battements.remove(0)` devenait `update(move |__v| __v.remove(__a0))`,
  et `remove` rend l'élément retiré. Le corps de la closure est
  désormais une instruction.
- **Une valeur qu'un futur écrit et que l'écran ne lit qu'en capture en
  ligne** n'était pas liée : la capture est écrite en prose, donc aucune
  occurrence du nom n'existait dans le code.
- **Une écriture dont la partie droite lit une autre valeur de page**
  (`vus = lancements;` dans un bloc `async`) sortait en
  `vus.set(lancements).get()` : la lecture appartenait au texte de
  l'écriture et était appliquée après elle.
- **Une fonction ordinaire à côté d'une page** n'atteignait pas ses
  valeurs : `fn combien() -> usize { journal.len() }` recevait « cannot
  find value `journal` in this scope », avec la suggestion d'appeler le
  composant `Journal`. Elle est maintenant réécrite comme un composant
  sans balisage.

### 29. Une `async fn` ne peut pas atteindre une valeur de page

Une valeur de page vit sur le fil qui dessine ; une `async fn` est
lancée par le runtime, donc ailleurs. La lier là-bas construirait une
seconde valeur, vide, et la lirait à la place — faux en silence. C'est
désormais un refus explicite qui nomme les deux sorties : écrire le
travail dans le bloc `async` du composant, ou passer la valeur en
paramètre. Ce qui manque est la troisième sortie, celle qu'un
développeur attend : que le compilateur passe lui-même les poignées à la
fonction et corrige ses appels.

### 30. Un `.await` de composant qui emprunte un local

`let lu = api::depot(&proprietaire, &nom).await;` dans un composant est
refusé — le futur part sur le runtime et ne peut pas emprunter un local
du fil qui dessine. C'est du Rust ordinaire, mais le message désigne la
colonne 259 d'un fichier généré que le développeur n'a jamais ouvert, et
ne dit pas que prendre les arguments en propre suffit.

### 10, encore : une barre de titre entière ne peut pas être traînée

Depuis que la fenêtre se commande, ce manque a une deuxième
conséquence. Une barre de titre à soi veut être traînable sur toute sa
largeur ; mais un `onMouseDown` posé sur la barre reçoit aussi les
appuis sur ses propres boutons, et `Event` ne dit pas sur quoi on a
appuyé — donc le déplacement commence et le clic du bouton est avalé.
La poignée doit être posée sur un espace vide, ce que fait
`app/layout.rsx` du labo. Ce qui le lèvera : `Event` porte sa cible.

---

## Ce que la deuxième nuit a fermé

### 10 à 12. Ce que la frontière JavaScript ne faisait pas traverser

- **`Event` ne portait que `kind`.** Un `onInput` ne pouvait donc pas
  lire ce qui venait d'être tapé — la seule chose pour laquelle un
  `onInput` existe — et le labo tenait un composant React de trois
  lignes de TypeScript pour faire traverser la chaîne. L'événement porte
  maintenant `value`, `number()`, `id`, `key`, `checked` et les quatre
  modificateurs, lus sur l'événement du DOM là où ils existent. Le
  composant de contournement a disparu du labo.
- **`Value` n'avait aucun accesseur.** Lire ce qu'une bibliothèque passe
  à un callback demandait un `match` sur cinq variantes à chaque fois.
  Il y a maintenant `as_str`, `as_f64`, `as_bool`, `as_list`,
  `as_record`, `get`, `at`, `text`, `number` et `truth` — dont `truth`,
  qui dit oui comme JavaScript dit oui.
- **Un callback ne recevait que le premier argument.** Le compilateur
  appelle maintenant la closure avec autant d'arguments qu'elle en
  déclare, comptés dans le texte : une bibliothèque qui appelle
  `onChange(valeur, index)` — les listes virtuelles, les tableaux, les
  sélecteurs de plage le font toutes — marche, et `onClose={move || …}`
  aussi.

### 13 à 19. Le routeur

Les sept manques sont fermés : query string lue et écrite (`params.query`,
`router::query`, `router::with_query`, avec le décodage d'un navigateur),
attrape-tout `[...chemin]` et son optionnel `[[...chemin]]` avec l'ordre
de précision de Next.js, groupes de routes `(section)`, `replace` et
`forward` sur un historique qui est une liste et un doigt dessus,
`can_go_back` / `can_go_forward` / `history()`, les paramètres du dossier
reçus par le layout, et `loading.rsx` / `error.rsx` par segment.

Ce qui reste hors d'atteinte, et qui n'est pas un manque mais une autre
nature : pas de rendu serveur, pas de `generateStaticParams`, pas de
préchargement d'une route avant le clic — il n'y a pas de serveur, et
une page n'est pas un document à aller chercher.

### 20. Un `.await` de composant n'était posé qu'une fois

Il l'est maintenant une fois **par adresse**. `awaited` pose ses deux
emplacements sous une clé qui est l'adresse de l'écran, donc passer de
`/depots/rust-lang/rust` à `/depots/tokio-rs/tokio` repose la question
et oublie la réponse d'avant, sans que la page ait à écrire un
`with_key` autour de chaque appel. Et la clé est posée **là** et non par
le routeur, pour que le composant ne soit pas reclé : il garde ses
cellules, et un `effect` qui surveille le paramètre ferme toujours avant
d'ouvrir.

### 21. Un refus de page entraînait un second message

`#[rustx::main]` réémettait la signature exactement comme elle était
écrite quand une page était refusée, donc le développeur recevait sa
vraie erreur **et** `error[E0752]: main function is not allowed to be
async`, qui désigne `src/main.rs` et ne parle de rien qu'il ait fait.
L'`async` est maintenant retiré avant que quoi que ce soit puisse mal
tourner.

### 30. L'emprunt dans un `.await`

Pas refusé, et c'est un choix. `api::depot(&nom, &proprietaire).await`
dans un composant ne compile pas parce que la future part sur le runtime
— du Rust ordinaire. Mais le code engendré évalue l'expression **avant**
de l'attendre, donc une fonction qui rend une future `'static` à partir
d'une référence marche aujourd'hui : refuser toute référence casserait
le seul cas qui passe. C'est donc écrit dans le README, avec la sortie
(prendre les arguments en propre), et le message reste celui de `rustc`.

---

## 31. Un commentaire dans le markup — **corrigé**

Il était affiché. Les trois formes de Rust et de JSX sont lues et
jetées, dans les enfants comme entre les attributs ; un `//` au milieu
d'une phrase reste la phrase. Voir la chasse aux bugs, #38.

## 32. La bascule d'une fenêtre à soi — **corrigé**

`window::toggle_maximize()` et `window::toggle_fullscreen()`. Le
système est interrogé là où la poignée est tenue, et la place d'avant le
plein écran est retenue pour être rendue. Voir #40.

## 33. Le haut d'une fenêtre sans cadre — **corrigé**

Sept pixels que Windows gardait et peignait. Repris par
`WM_NCCALCSIZE`, les bords de redimensionnement conservés. Voir #39.

## 34. Ce qu'une feuille de style doit porter — **documenté**

Le framework exécute Tailwind ; il n'écrit pas le thème de
l'application, et une feuille sans `@theme inline` donne une fenêtre
blanche sans que rien ne le dise. Le README porte maintenant le bloc
minimal, à l'endroit où l'on installe Tailwind. Ce qui reste ouvert :
`rustx new` pourrait écrire cette feuille, ce qui ferait du framework
l'auteur d'un thème — un choix à faire, pas un oubli.

## 35. Un commentaire avant une écriture — **corrigé**

`is_statement` ne remontait que sur les espaces entre ce qui précède
un appel et ce qui en décide la nature ; un commentaire juste avant
suffisait à faire croire que l'appel suivait un opérateur plutôt qu'un
point-virgule, et l'écriture n'était plus reconnue comme telle.
`back_over_space_and_comments` lit maintenant les trois formes de
commentaire, et un `//` dans une chaîne reste du texte. Voir la chasse
aux bugs, #42.

## 36. `style` en chaîne à la frontière React — **corrigé**

`style_object` traduit maintenant une chaîne CSS en objet avant de la
tendre à React, propriété par propriété, camelCase sauf les
`--variables`. Voir #43.

## 37. Une collision de nom avec une bibliothèque JS — **mitigé**

`LineChart` sans préfixe se résout vers la première bibliothèque
installée qui porte ce nom, qui peut ne pas être celle qu'on voulait —
et ça reste vrai : rien n'empêche encore la collision elle-même. Ce qui
est corrigé, c'est le silence : `draw()` dit maintenant lequel des
composants a jeté et pourquoi, plutôt que de laisser tout l'écran
disparaître sans un mot. **Reste ouvert** : avertir au moment de la
compilation quand un nom de balise existe dans plus d'une bibliothèque
déclarée, pour ne plus avoir à le découvrir à l'écran noir. Voir #44.

## 38. Le routeur sourd à un autre thread — **corrigé**

Un futur de composant ne s'exécute pas forcément sur le thread qui a
ouvert la page, et le routeur était tenu par du thread-local : un
`navigate`/`replace` appelé de là ne faisait rien, sans erreur. Il est
maintenant une copie partagée par tout le processus, et les deux se
reposent eux-mêmes sur le thread de la page quand ils sont appelés
d'ailleurs. Voir #45.

## 39. La reconstruction qui ne s'arrêtait jamais — **corrigé**

Chaque build donnait aux pages engendrées une date neuve, même sans
changement, et cargo les croyait toujours à refaire. Toute écriture du
compilateur compare maintenant le contenu avant d'écrire. Voir #46.

## 40. Une fenêtre tenue bloque l'autre projet — **corrigé**

`cef-dll-sys` recopie `libcef.dll` à chaque build ; une fenêtre oubliée
ouverte la tient, et comme labo et framework partagent `target-dir`,
l'un bloquait l'autre avec « Access is denied. (os error 5) ».
`build::held()` le dit en clair avant cargo, et `rustx dev` ferme sa
propre fenêtre d'abord. Voir #47.

## 41. La version demandée par un projet neuf — **corrigé**

`rustx new` écrivait `version = "0.1"` à la main, et cargo refuse sous
le 0.x qu'un `0.1` accepte un `0.2.0` : tout projet neuf recevait la
version d'avant, en silence. Lue maintenant dans la version de la
ligne de commande elle-même. Voir #48.

## 42. Rien pour la première `npm install` — **corrigé**

Sans `package.json`, la première bibliothèque qu'un projet installe
écrit un fichier nommé d'après le dossier plutôt que par le projet, un
pas trop tard. `rustx new` en écrit un minimal. Voir #49.

## 43. Le LSP aveugle après la mort de `rust-analyzer` — **corrigé**

Un canal fermé pour de bon (le serveur en aval est mort) ne se
distinguait pas d'un canal simplement lent, et les deux rendaient la
main vide pour le reste de la session. Un canal fermé relance
maintenant le serveur une fois et repose la question. Voir #50.

## 44. Les panneaux d'installation et les métadonnées du logiciel — **ouvert**

`labo-rustx/BUG.md` : « les panneaux d'installation & metadata du
logiciel (author, version, …) pour les différents OS. » Rien n'écrit
encore, pour une application faite avec `rustx`, ce qu'un installateur
Windows (MSI/NSIS), un `.app` macOS ou un `.deb`/`.rpm` Linux montre
avant l'ouverture : nom affiché, auteur, version, icône au bon format
pour chaque système. Un travail de `rustx-cli build --release`, pas
encore commencé.

## 45. Les fonctionnalités système, barre système et raccourcis compris — **en partie ouvert**

Mis à jour deux fois dans la même nuit. Presse-papiers, notifications,
démarrage automatique, instance unique et liste des caméras sont réels
maintenant pour **Linux et macOS**, pas seulement Windows —
`src/linux/` et `src/macos/`, chacun par un outil ou un fichier que le
système porte déjà (`xclip`/`wl-copy`, `notify-send`, un `.desktop`,
`pbcopy`/`pbpaste`, `osascript`, un plist `LaunchAgent`, un socket
Unix, `/sys/class/video4linux`, `system_profiler`), jamais par une
bibliothèque que ce crate devrait ajouter. Le dialogue de fichiers
aussi, des deux côtés : `zenity`/`kdialog` sous Linux, `osascript`
disant `choose file` sous macOS.

Reste ouvert sur les deux : les raccourcis globaux, la barre système et
le contrôle de fenêtre, qui ont chacun besoin soit d'un portail de
bureau sur D-Bus (Linux), soit d'`AppKit`/Carbon (macOS) — un
attachement en processus ou une liaison native, pas un programme à
lancer depuis une ligne de commande. Vérifié par `cargo check --target
x86_64-unknown-linux-gnu` pour les fichiers Linux (zéro machine Linux,
mais un vrai retour du compilateur) ; les fichiers macOS n'ont que la
relecture à la main, faute d'une cible Apple installée ici — voir
l'en-tête « Written, not run » de chacun.

## 46. L'autocomplétion des crates installées dans un `.rsx` — **ouvert**

`labo-rustx/BUG.md` : « on est obligé d'écrire tous les imports de la
sorte… alors qu'on est censé avoir l'autocomplétion & l'auto-import à
toutes les échelles possibles comme Rust-Analyzer Extension. »
`rustx-lsp` relaie déjà `rust-analyzer` pour le fichier engendré ; ce
qui manque est de lister, pour une crate installée, ses fonctions et
composants publics avant que le chemin complet ne soit tapé — le
travail de `libraries.rs`/`suggest.rs`, étendu au-delà de ce qu'ils
couvrent aujourd'hui (les bibliothèques JavaScript déclarées, pas les
crates Rust).

