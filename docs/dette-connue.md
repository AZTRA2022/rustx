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
