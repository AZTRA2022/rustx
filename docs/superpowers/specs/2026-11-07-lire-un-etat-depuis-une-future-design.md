# Lire un état depuis une future

*Spec — 7 novembre 2026*

## Le problème, tel qu'il se lit dans une page

Voici la sauvegarde automatique du labo, aujourd'hui :

```rust
let tampon = keep(std::sync::Arc::new(std::sync::Mutex::new(String::new())));
let a_ecrire = tampon.with(std::sync::Arc::clone);

async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let texte = a_ecrire.lock().map(|tenu| tenu.clone()).unwrap_or_default();
        if texte.is_empty() { continue; }
        tokio::fs::write(crate::autosave(), texte.as_bytes()).await.ok();
        enregistrements += 1;
    }
};
```

et le gestionnaire du bouton, qui doit alimenter ce tampon :

```rust
onClick={move |_| {
    clicks += 1;
    tampon.with(|tenu| {
        if let Ok(mut tenu) = tenu.lock() {
            *tenu = format!("clics: {}", clicks);
        }
    });
}}
```

Cinq concepts — `keep`, `Arc`, `Mutex`, un tampon, une seconde poignée —
pour dire une seule chose : **la tâche veut lire `clicks`**. L'écriture
(`enregistrements += 1`) marche déjà ; c'est la lecture qui est refusée
par le compilateur :

> `clicks` is read inside a `task(async ...)`. A value lives on the
> thread that draws, and this block runs beside it, so there is nothing
> there to read.

Le refus est exact — une cellule est un `Rc<RefCell<T>>` qui n'appartient
qu'au thread du rendu — et c'est précisément pour cela qu'il n'est pas
une réponse : il décrit la contrainte et laisse le développeur
réimplémenter à la main, mal, ce que le framework pourrait faire une
fois. L'objectif de rustx est de tuer la complexité du code en gardant
la performance de Rust ; ce refus fait l'inverse.

## Ce que la page écrira

```rust
async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        tokio::fs::write(crate::autosave(), format!("clics: {clicks}")).await.ok();
        enregistrements += 1;
    }
};
```

et le gestionnaire redevient `onClick={move |_| clicks += 1}`.

`keep`, `Arc`, `Mutex`, `tampon` et `a_ecrire` disparaissent de la page.
Douze lignes et cinq concepts en moins, pour la même chose faite mieux :
la tâche voit la valeur, au lieu d'une copie qu'un gestionnaire devait
penser à tenir à jour.

## Sémantique

**Une lecture dans un bloc `async` voit la valeur de la page.** Pas une
copie prise à l'entrée du bloc, pas une valeur d'il y a dix frames : la
valeur, lue là où elle est.

**Une écriture depuis un bloc `async` continue de passer par la boîte aux
lettres**, et c'est voulu. C'est elle qui garde le marquage « sale » sur
le thread du rendu, une seule frame pour mille écritures, et le refus
d'une écriture dont le composant est parti. Conséquence, qui est à
documenter en une phrase dans le README : *une future qui écrit puis
relit voit l'ancienne valeur jusqu'à la frame qui applique l'écriture.*

Ce qui reste refusé, et garde ses mots : une lecture dans la **queue**
d'une expression attendue — `let x = f().await.max(n);`. Cette queue-là
part vraiment avec la future, sur un thread du runtime, et n'a aucune
cellule à sa portée.

## Mécanisme : une cellule à deux formes, choisie à la compilation

Le compilateur sait, en compilant la page, **quelles cellules une future
lit** : il voit la lecture dans le bloc. Ce n'est donc pas une
information à transporter à l'exécution, c'est un choix de
représentation, pris cellule par cellule :

| cellule | représentation | coût |
|---|---|---|
| ordinaire | `Rc<RefCell<T>>` — celle d'aujourd'hui | aucun changement |
| lue par une future | `Arc<RwLock<T>>` | un compteur atomique, un verrou non contendu par accès |

Il n'y a pas de troisième cas, et **aucun clone n'est ajouté nulle part**.
La future ne lit pas une copie : elle lit la valeur, sous un verrou de
lecture. Le thread du rendu la lit comme aujourd'hui — un `.get()` qui ne
clone que si l'expression de la page a besoin d'une valeur possédée, ce
qui est déjà le cas.

### Ce qui est émis

```rust
// `clicks` est lu dans un bloc async -> ouvert en cellule traversante
let clicks = rustx_dom::state::cell_across(SLOT, || 0);
…
// dans le preambule du bloc, sur le thread du rendu
let clicks = clicks.across_until(&__cancel);
rustx_engine::task::spawn_future_until(__cancel, async move {
    loop { … format!("clics: {}", clicks.get()) … }
})
```

Une valeur que la page a déclarée (`let mut clicks = 0;` au sommet du
fichier) suit le même chemin à travers `store`, par son nom.

Une valeur à la fois lue **et** écrite dans le bloc ne demande qu'une
poignée : `Across<T>` porte `get`, et `update`/`set` qui postent comme
`Remote<T>` le fait déjà.

### La contrainte, et où vit son message

Le compilateur n'a aucune information de type : il ne peut pas vérifier
`Send + Sync`. La contrainte est donc portée par le code émis, et son
explication avec elle — c'est rustc qui parlera, mais avec nos mots et
sur la ligne de la page :

```rust
#[diagnostic::on_unimplemented(
    message = "`{Self}` ne peut pas etre lu depuis un bloc `async`",
    note = "une valeur lue a cote de la fenetre est lue sous un verrou : \
            il lui faut `Send + Sync`. Lisez-la avant le bloc et \
            deplacez la copie dedans."
)]
pub trait Crossing: Send + Sync + 'static {}
```

### Invariants à ne pas casser

- **Le compte de lignes est conservé.** Le préambule tient sur la ligne
  de l'appel, les lectures sont remplacées sur place. La carte des
  positions du plan 2 repose là-dessus.
- **Le chemin chaud de `state.rs` n'est pas touché.** Les cellules
  traversantes vivent dans un second registre ; `CELLS` et son code
  restent tels quels.
- **Les éditions ne se chevauchent pas.** Une lecture réécrite dans un
  bloc est à l'intérieur du texte que `future_task` produit, comme les
  écritures le sont déjà.

## Le runtime suit le crate de l'application

Deux choses imposent tokio plus fort que nécessaire :

```toml
# crates/rustx-engine — plancher de version trop haut
tokio = { version = "1.47", default-features = false, features = [...] }
# crates/rustx — `full` impose a toute application
tokio = { version = "1.47", features = ["full"] }
```

Les *features* ne sont pas le problème : cargo les unit par addition,
donc une application qui déclare `full` gagne déjà. Les deux vrais
problèmes :

1. **le plancher** `^1.47` fait échouer une application restée en 1.40 →
   devient `"1"` partout, et c'est l'application qui fixe la version ;
2. **`full` imposé** → devient une feature **par défaut** de la façade
   (`tokio-complet`). Par défaut, rien à déclarer et `tokio::fs::write`
   marche dans une page ; une application qui veut choisir coupe la
   feature par défaut et déclare son tokio. Le `use rustx::tokio;` de la
   macro reste correct : c'est le même crate.

Hors périmètre, décidé le 7 novembre : rustx ne se charge pas de piloter
une bibliothèque externe. Le registre de shadcn, ses éléments absents et
ses compositions ne sont pas son affaire.

## Ce qui est déjà vrai, et qui doit être prouvé plutôt que construit

Deux moitiés de la demande marchent aujourd'hui et n'ont aucun test qui
le dise :

- **une `async fn` déclarée à côté et appelée depuis une page** — le
  `use rustx::tokio;` est posé à côté de `main` et non dedans, donc une
  `async fn` du module en bénéficie ;
- **`.await` directement dans `Page`**, suivi de balises qui lisent *et*
  écrivent l'état de la page — l'émission est un `return` anticipé, donc
  la suite de la fonction tourne bien sur le thread du rendu.

Chacune reçoit un test nommé d'après ce qu'elle promet.

## L'éditeur

**L'icône.** `Kind::Attribute` et `Kind::Property` envoient le même
`CompletionItemKind` (`10`) à l'éditeur : une prop propre au composant et
un attribut HTML ont la même icône. `Property` devient `5` (*Field*), et
une **loi** interdit à deux `Kind` de partager un numéro.

**Les entrées manquantes.** La cause n'est pas connue : `exports_for`
inclut bien les composants propres de l'application, et les listes
`export { … }` multi-lignes sont bien lues. Le premier pas est donc un
test qui dresse deux listes à partir du vrai labo — ce qui est installé
sur le disque, et ce que le serveur offre — et qui échoue en nommant la
différence. Le correctif suivra ce que ce test dira, pas ce qu'on
suppose.

## Les lois

Aucun test ne lisait les pages des applications : c'est pour cela qu'une
ligne tronquée dans `labo-rustx/app/page.rsx` a survécu à un rapport
vert. Un test parcourt chaque `.rsx` du dépôt — et ceux du labo quand le
dépôt voisin est là — et vérifie :

| loi | ce qu'elle attrape |
|---|---|
| la page est acceptée par le compilateur | une page qu'un refus casse sans que personne le sache |
| le résultat s'analyse comme du Rust (`syn`) | une troncature, une émission qui ne ferme pas |
| le compte de lignes est conservé | la carte des positions cassée en silence |
| le résultat n'est pas vide | une édition qui avale tout |

Le labo vit hors du dépôt : il est inclus quand `../labo-rustx` existe,
et le test **dit** quand il ne l'inclut pas. Un filet qui se tait quand
il ne couvre rien est le défaut que cette spec vient réparer.

## Test, par morceau

Chaque morceau est écrit en TDD, avec un RED regardé.

| morceau | le test qui le prouve |
|---|---|
| `cell_across` / `Across<T>` | une future lit une valeur écrite par le thread du rendu, sans clone |
| le choix de représentation | une cellule qu'aucune future ne lit reste un `Rc<RefCell<T>>` |
| l'émission | une lecture dans un bloc devient `.get()`, le compte de lignes tient |
| le refus qui tombe | la page du labo, sans `Mutex`, est acceptée |
| le refus qui reste | `f().await.max(n)` est toujours refusé, avec ses mots |
| le runtime | une application qui déclare son propre tokio compile |
| l'icône | deux `Kind` ne partagent jamais un numéro |
| les lois | chaque `.rsx` du dépôt, et le labo quand il est là |

Et la preuve finale est celle qui compte : **le labo passe ses onze
assertions dans une vraie fenêtre, avec une page qui ne contient plus ni
`keep`, ni `Arc`, ni `Mutex`.**
