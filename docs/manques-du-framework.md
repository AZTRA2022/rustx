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
| 5 | cassé | `rustx-compiler` | un `let mut` **dans** un bloc `async` est pris pour une cellule | ouvert |
| 6 | cassé | `rustx-compiler` | une capture en ligne `format!("{x}")` n'est pas réécrite | ouvert |
| 7 | cassé | `rustx-compiler` | un `let mut` d'une fonction utilitaire d'une page est promu | ouvert |
| 8 | cassé | `rustx-compiler` | un `async fn` à côté de la page ne peut pas toucher l'état | ouvert |
| 9 | manque | `rustx-dom` | un `Element` ne peut pas être enfant d'un composant React | ouvert |
| 10 | manque | `rustx-dom` | `Event` ne porte que `kind` : le texte tapé est hors d'atteinte | ouvert |
| 11 | manque | `rustx-dom` | `Value` n'a aucun accesseur (`as_str`, `as_f64`, `get`) | ouvert |
| 12 | manque | `rustx-dom` | un callback ne reçoit que le **premier** argument | ouvert |
| 13 | manque | routeur | pas de *query string* : `/depots?q=rust` ne correspond à rien | ouvert |
| 14 | manque | routeur | pas de segment attrape-tout `[...chemin]` | ouvert |
| 15 | manque | routeur | pas de groupes de routes `(section)` | ouvert |
| 16 | manque | routeur | pas de `forward`, pas de `replace` | ouvert |
| 17 | manque | routeur | un `Layout` ne reçoit pas les paramètres de la route | ouvert |
| 18 | manque | routeur | pas d'écran de chargement ni de frontière d'erreur par segment | ouvert |
| 19 | manque | routeur | la pile de navigation n'est pas lisible | ouvert |
| 20 | coûte | `rustx-compiler` | un `.await` de composant n'est posé qu'une fois : `with_key` obligatoire | ouvert |
| 21 | coûte | `rustx-compiler` | un refus de page entraîne un second message sur `async main` | ouvert |
| 22 | coûte | `rustx-lsp` | la complétion exige le chemin écrit en entier | ouvert |
| 23 | coûte | documentation | le README dit encore qu'une lecture dans un bloc est refusée | ouvert |

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
