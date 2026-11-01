# Dette connue

Ce qu'on sait être imparfait, pourquoi on l'a laissé, et ce qui nous
ferait changer d'avis. Une ligne ici est une décision, pas un oubli : si
une entrée n'a plus de « on s'en occupe quand », elle doit être corrigée
ou supprimée.

---

## Moteur (plan 1, revue de branche du 2026-10-28)

La revue a classé seize points. Les huit premiers — deux critiques, six
importants — sont corrigés dans les commits `3a04e63`, `cd2ec2f` et
`e1ee60e`. Les huit suivants sont ici.

### 1. `__rustx_frame` est une fonction globale publique

`renderer::install` l'enregistre via le `register_native` public. Elle
atterrit donc dans l'espace de noms des natives de l'application (qui
peut l'écraser) et sur le global JavaScript (le script de la page peut
l'appeler, ce qui ré-entre dans `tick()`, ou la masquer, ce qui arrête
la boucle).

Ce n'est pas de la corruption : le prendre-puis-rendre de `tick()`
survit à la ré-entrance sans perdre de callbacks. C'est de la
robustesse.

**On s'en occupe quand** on fera le hot reload, qui a de toute façon
besoin d'un canal interne entre le moteur et la page. Le bon correctif
est un emplacement interne séparé et une propriété non énumérable, pas
un renommage.

### 2. `shutdown()` jette les jobs déjà en file

`*guard = None` laisse tomber le `Vec<Job>` en attente. La
documentation ne promet que le refus des posts *suivants*, donc le
comportement n'est pas un mensonge — mais il n'est pas non plus un
choix qu'on a fait exprès.

**On s'en occupe quand** on saura ce qu'un logiciel de capture veut :
vider la file avant de fermer (les dernières frames arrivent) ou la
jeter (la fenêtre est partie, plus personne ne les verra). La réponse
vient du logiciel de preuve, pas d'ici.

### 3. `Poster::post` depuis le processus navigateur réussit dans le vide

`tick()` ne tourne que dans le processus de rendu. Un `post` fait
depuis le processus navigateur est accepté et n'est jamais drainé : la
file grossit sans borne et sans un mot.

**On s'en occupe quand** la couche OS existera, parce que c'est elle
qui donnera aux développeurs une raison d'avoir du code dans les deux
processus. `poster()` refusera alors hors de `Role::Renderer`.

### 4. Les hooks enregistrés après le premier contexte ne tournent jamais

`HOOKS_RUN` est volontaire : les hooks tournent une fois par processus
et non une fois par page, pour que l'état capturé par une closure
survive à une navigation. Mais la documentation de
`on_context_created` ne le dit pas.

**On s'en occupe** à la prochaine passe de documentation publique de
l'API. C'est un contrat à écrire, pas un code à changer.

### 5. Aucun garde-fou à la compilation contre `panic = "abort"`

Toute la tolérance aux panics du moteur repose sur `catch_unwind`. Une
application compilée avec `panic = "abort"` transforme chacun de ces
`catch_unwind` en un abandon du processus, silencieusement, et tout ce
que la revue vient de faire corriger revient.

**On s'en occupe quand** on écrira l'outillage de build (`rustx build`),
qui est de toute façon le seul endroit d'où on peut refuser un profil.

### 6. `Document::current()` n'affirme pas son thread

Elle est `pub` et appelable depuis un thread de travail. En pratique
`CEF_V8_REQUIRE_ISOLATE_RETURN` de libcef renvoie null plutôt que de
mal se comporter — ce n'est donc pas un comportement indéfini, juste un
`None` qu'on ne s'explique pas. Un `cef::currently_on(RENDERER)`
explicite rendrait le contrat lisible.

**On s'en occupe quand** on touchera à `dom.rs` pour le réconciliateur.

### 7. Aucun test ne couvre un appel DOM qui lève

`dom.rs` vérifie `has_exception()` après `execute_function`, et ça
marche. Mais rien ne prouve qu'une exception levée par le DOM ressort
en `DomError::Failed` plutôt qu'en `None` silencieux.

**On s'en occupe quand** le réconciliateur écrira dans le DOM pour de
vrai : c'est là qu'un appel qui lève cesse d'être hypothétique.

### 8. `App::run` ne fait pas `cef::shutdown()` sur le chemin d'erreur

Si `run_message_loop` panique, l'arrêt propre de CEF n'a pas lieu.
Cosmétique pour un binaire de démonstration ; à revoir quand `App` sera
l'API publique que des logiciels tiers utilisent.

---

## Compilateur `.rsx` (plan 2, revue de branche du 2026-10-28)

La revue a classé seize points. Les deux critiques et les six importants
sont corrigés. Les huit mineurs sont ici, plus une limite que la passe de
correction a laissée volontairement en place.

### 9. Une erreur `rustc` nomme le bon numéro de ligne, pas le bon fichier

Le `.rs` engendré a maintenant exactement le même nombre de lignes que le
`.rsx`, et chaque ligne porte le même numéro : une erreur de `rustc` à la
ligne 10 est bien à la ligne 10 du `.rsx`. Mais le chemin affiché reste
celui du fichier d'`OUT_DIR`, que le développeur n'a pas écrit.

Il manque la réécriture du chemin dans la sortie de `cargo`, et
`PositionMap` n'a donc toujours aucun lecteur.

**On s'en occupe au plan 8**, avec la CLI `rustx` : c'est elle qui
enveloppe `cargo` et qui est le seul endroit d'où on peut réécrire ses
messages. D'ici là, le numéro de ligne est juste et c'est l'essentiel.

### 10. Une ligne à l'intérieur d'une région se résout au début de la région

Une région reste une seule expression Rust sur une seule ligne, suivie des
retours à la ligne qu'elle a avalés. Donc une erreur écrite à la ligne 10
d'une région qui commence ligne 6 est rapportée ligne 6 — au début du
`return`, où l'on voit l'ensemble, et non plus à la fin comme avant.

Pour tomber sur la ligne exacte il faudrait poser chaque enfant émis sur
la ligne où commençait sa source, ce qui demande un décalage sur
`Node::Text` et `Node::Expression`. **On s'en occupe quand** le plan 8
aura besoin de colonnes exactes pour l'extension d'éditeur.

### 11. Une balise fermante orpheline après une région est recopiée telle quelle

`return <div></div></span>;` produit `… .build()</span>;`, donc une erreur
de syntaxe de `rustc` dans le fichier engendré au lieu d'un message en
coordonnées du `.rsx`. **On s'en occupe** à la prochaine passe sur
`scan.rs`.

### 12. Un attribut répété est accepté en silence

`<div id="a" id="b"/>` émet les deux ; le dernier gagne dans le DOM. Et
`value={x} defaultValue="y"` se résolvent tous deux en `value` et se
marchent dessus sans un mot. **On s'en occupe quand** on touchera à
`emit.rs` pour les composants.

### 13. `trim_end` mort dans la détection d'une balise auto-fermante

`source[..index].trim_end().ends_with("/>")` ne peut jamais voir
`<div/ >`, parce que `index` est toujours une position après le `>`. Sans
effet aujourd'hui ; le `trim_end` laisse croire qu'un espace est toléré
alors qu'il ne l'est pas.

### 14. `PositionMap::from_text` n'a ni appelant ni test

Mort tant que le point 9 n'est pas traité.

### 15. `mount_in` n'a pas de retour en arrière

Un échec à mi-parcours laisse un demi-arbre attaché. Combiné au montage
réessayé à chaque frame dans `apps/catalogue`, le préfixe réussi serait
rattaché une seconde fois. Peu probable, pas cher à garder : construire
le sous-arbre entier puis l'attacher d'un coup. **On s'en occupe au plan
3**, qui réécrit de toute façon le montage pour le re-rendu.

### 16. Les attributs passent tous par `setAttribute`

`value` et `checked` posent donc la valeur *par défaut* et non la
propriété vivante. Correct pour un montage statique ; faux dès qu'un
champ contrôlé sera re-rendu. **On s'en occupe au plan 3.**

### 17. `tag_name` et `Cursor::name` ne s'accordent pas sur le non-ASCII

`scan.rs` accepte les octets `>= 0x80` dans un nom de balise, `parse.rs`
non. Ne produit qu'une erreur un peu confuse. Les deux devraient partager
une seule fonction.

### 18. Un composant rustx écrit *dans* un composant JS est refusé

`<ui::CardContent>{Feature("…")}</ui::CardContent>` ne compile pas : un
`{…}` à l'intérieur d'une frontière est émis en `ForeignChild::Value`,
donc passé par `ToValue`, et `Element` n'est pas une valeur. Le message
que voit le développeur est `the trait bound rustx::Element: ToValue is
not satisfied` dans un fichier généré — c'est-à-dire incompréhensible.

Ce qui marche aujourd'hui : un nœud que rustx possède (`#panel-host`) et
`rustx_component("Nom", "id")` depuis le JavaScript de la page, ce que
prouve `apps/hybrid`. Ce qui manque est la forme naturelle, et elle
manquera vite : une page faite de composants installés est faite de
composants dans des composants.

Ce qu'il faudrait : une variante `ForeignChild::Ours(Element)`, un
emplacement que le runtime rend à la place (`<span data-rustx-slot>`), et
le montage du sous-arbre rustx dedans sur la même frame — avec la vie du
sous-arbre liée à celle du nœud React, puisque React peut le démonter
quand il veut. Le compilateur ne peut pas choisir la variante sur un type
qu'il ne connaît pas, donc il faudra un trait (`IntoForeignChild`) avec
la spécialisation par autoref, ou une méthode sur `Element` que le
`{…}` appelle quand elle existe.

**À faire** : son propre petit plan, après les médias. En attendant, le
refus doit au moins être une phrase du compilateur qui nomme la forme
qui marche, au lieu d'une erreur de trait dans un fichier généré.
