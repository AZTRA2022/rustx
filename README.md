# rustx

Des logiciels natifs, écrits en Rust, dessinés par un moteur de
navigateur.

Les composants s'écrivent dans des fichiers `.rsx` — la syntaxe de JSX,
le langage de Rust — et un `let mut` y est un état, sans annotation, sans
macro, sans `store::`. Les bibliothèques npm (React, Recharts, Tailwind,
shadcn) s'utilisent depuis le `.rsx` lui-même. Le `main.rs` d'un projet
fait quatre lignes, parce que la cérémonie est le travail du framework.

Rust et React vivent dans **le même processus** : un `onClick` est un
appel de fonction, pas un message sérialisé, et les props traversent la
frontière comme des valeurs V8.

> État : Windows est écrit et mesuré. macOS et Linux compilent et
> refusent en nommant le fichier qui leur manque. La version est `0.1.0`
> — l'API peut encore bouger.

## Démarrer

```sh
cargo install rustx-cli
rustx new mon-app
cd mon-app
rustx dev
```

Trois commandes, et une fenêtre. Ce que `rustx new` écrit tient en cinq
fichiers : un `Cargo.toml`, un `src/main.rs`, une page, un `.gitignore`
et un dossier `assets/`. Pas de `build.rs`, rien à configurer.

Le `Cargo.toml` nomme le paquet `rustx-app`, parce que `rustx` est pris
sur crates.io par un autre projet ; la bibliothèque, elle, s'appelle
`rustx`, donc tout ce qui suit s'écrit `rustx::` :

```toml
[dependencies]
rustx = { package = "rustx-app", version = "0.1" }
```

Le `main.rs`, en entier :

```rust
use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(Config::new("Mon app").size(900, 640))?;

    app.run()
}
```

Et `app/page.rsx` :

```rust
let mut clicks = 0;

fn Page() -> Element {
    return <main>
        <h1>Bonjour.</h1>
        <button onClick={move |_| clicks += 1}>
            Cliqué {clicks} fois
        </button>
    </main>;
}
```

## L'état : un `let mut`, et sa place dit sa portée

Il n'y a rien d'autre à apprendre. Un `let mut` **au sommet du fichier**
est partagé par l'application : deux composants qui le lisent lisent la
même valeur, et celui qui l'écrit redessine les deux. Le même `let mut`
**dans un composant** est à lui seul — une cellule par emplacement dans
l'arbre.

```rust
let mut points = 0;              // partagé : toute l'application le voit

fn Page() -> Element {
    let mut ouvert = false;      // privé : cette instance, et elle seule

    return <div>
        <p>{points} points</p>
        <button onClick={move |_| points += 1}>un de plus</button>
        <button onClick={move |_| ouvert = !ouvert}>{ouvert}</button>
    </div>;
}
```

Écrire, c'est marquer l'écran pour la frame suivante. Dix mille écritures
dans la même frame dessinent une fois : le débit est celui de la boucle
de rendu, pas celui de la boucle `for` du développeur.

## Les composants, les props, les routes

Un composant est une fonction qui rend un `Element`, et on l'appelle
comme une fonction de Rust :

```rust
fn Page() -> Element {
    return <main>{Carte("Rust".to_string(), 3)}</main>;
}

fn Carte(titre: String, combien: u32) -> Element {
    return <article className="carte">
        <h2>{titre}</h2>
        <p>{combien} fois</p>
    </article>;
}
```

Les balises prennent **les props de HTML et de JSX** : `className`,
`onClick`, `onInput`, `value`, `disabled`, `style={{ … }}` en objet,
`aria-*`, `data-*`. Ce qui n'existe pas est refusé à la compilation, avec
le nom le plus proche.

Les routes sont le dossier `app/` : `app/page.rsx` est `/`,
`app/reglages/page.rsx` est `/reglages`, `app/device/[id]/page.rsx` est
une route à paramètre. On navigue avec
`rustx::router::navigate("/reglages")`, et quitter un écran libère ce
qu'il tenait — y compris les fils qu'il avait démarrés :

```rust
task(move |alive| {
    while alive.get() {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = alive.post(|| ticks += 1);
    }
});
```

Un fil de travail ne touche jamais l'état : il **poste** un travail, et
la frame le drainera. C'est la seule règle du modèle de threads, et elle
est dans le type.

## Les bibliothèques JavaScript

Un `npm install` suffit : le framework lit les `dependencies` du
`package.json`, les met dans le paquet, et une balise majuscule est
résolue à l'exécution.

```rust
return <Card>
    <CardHeader><CardTitle>Les ventes</CardTitle></CardHeader>
    <LineChart width={520} height={240} data={points}>
        <XAxis dataKey="at" />
        <Line dataKey="value" stroke="#8b5cf6" />
    </LineChart>
</Card>;
```

`Card` et `CardHeader` viennent de `components/ui/`, où `npx shadcn add`
les a laissés ; `LineChart` vient de recharts. **Aucun préfixe** : les
composants de l'application passent devant les bibliothèques dont ils
sont construits, donc `<Dialog>` est celui du projet et non la primitive
Radix. Pour demander l'autre explicitement :
`<radixUiReactDialog::Dialog>`.

Aucune ligne du framework ne nomme une bibliothèque ni une version. La
seule exception est un outil que la construction **exécute**, parce qu'un
programme a une ligne de commande et qu'une ligne de commande a une
version : Tailwind. Le framework lit celle que `node_modules` contient et
parle le contrat correspondant — `--content` et `@tailwind utilities`
pour la 3, `@source` et `@import "tailwindcss"` pour la 4. Les deux
marchent ; passer d'une à l'autre, c'est le `package.json` d'une
application, rien du framework.

## Les fenêtres

`Config` porte ce qu'une fenêtre est : son titre, sa taille, son icône,
et ce que l'application dit d'elle-même.

```rust
let app = App::new(
    Config::new("Mon app")
        .size(1100, 720)
        .icon("assets/icon.ico")
        .version("1.4.0")
        .description("Ce que ce logiciel fait")
        .publisher("Moi"),
)?;
```

Une page peut en ouvrir une deuxième, sur la route qu'elle choisit :

```rust
<button onClick={move |_| {
    let _ = rustx::window::open(
        rustx::window::What::new("reglages")
            .titled("Réglages")
            .at("/reglages"),
    );
}}>Réglages</button>
```

Elle se referme par son nom, et fermer une fenêtre n'arrête pas
l'application tant qu'il en reste une. Chaque fenêtre a son propre
processus de rendu, donc son propre état : ce qu'elles partagent, elles
le partagent par un message, pas par une variable.

## Ce que le web ne sait pas faire

Une page ne peut pas lire le presse-papiers quand elle veut, ni ouvrir un
dialogue qui rend un chemin, ni poser une icône à côté de l'horloge.
C'est ce que `rustx-os` fait, et ça s'écrit dans une page comme le reste :

```rust
use rustx::{autostart, clipboard, dialog, dropped, instance, notify, shortcut, tray};

// Le presse-papiers : du texte, une image, ou un format à soi.
clipboard::set_text("copié depuis Rust")?;

// Un dialogue du système, qui rend un PathBuf et non un File.
let Ok(paths) = dialog::open(
    dialog::Ask::new("Choisir une image").filter("Images", ["png", "jpg"])
) else { return; };

// Une icône de barre système, retirée quand son jeton est lâché.
let icon = tray::Icon::new(Path::new("assets/icon.ico"), "Mon logiciel")?;
icon.menu(&[tray::Item::new("quitter", "Quitter")])?;

notify::show(notify::What::new("Fini", "Le rendu est terminé."))?;

// Un raccourci qui marche quand la fenêtre n'a pas le focus.
let capture = shortcut::Shortcut::listen("Ctrl+Shift+R")?;
if capture.pressed() > 0 { commencer(); }

// Les fichiers lâchés sur la fenêtre, avec leur vrai chemin — ce
// qu'une page web ne peut pas savoir.
dropped::on_files(|paths| for path in paths { ouvrir(&path); });

// Une seule instance : la deuxième passe ses arguments et s'en va.
let Ok(only) = instance::Only::one("mon-logiciel") else {
    instance::hand_over("mon-logiciel", &arguments)?;
    return Ok(());
};
autostart::set(true, "mon-logiciel")?;
```

Chaque chose enregistrée auprès du système est un **jeton** : l'icône, le
raccourci, le nom de l'instance. Lâché, il la retire — une icône morte
dans la barre ou un `Ctrl+Shift+R` qu'aucune application ne rend sont des
défauts que l'utilisateur ne peut réparer qu'en redémarrant.

La caméra, le micro et l'écran passent par Chromium — `getUserMedia`,
`getDisplayMedia` — avec les permissions du système : c'est le même code
que sur le web, et il n'y a pas de couche à apprendre par-dessus.

Un répertoire par système à l'intérieur — `windows/`, `linux/`, le reste
— et une capacité est un fichier par système, avec les mêmes noms de
fonctions. Chaque module répond `supported()` avant qu'on lui demande
quoi que ce soit, pour qu'une fonctionnalité absente soit un bouton qu'on
ne dessine pas plutôt qu'une erreur à lire. Ailleurs que sur Windows, le
refus nomme la plateforme **et le fichier à écrire**.

Ce qui demande un œil se regarde à la main :

```sh
cargo run -p rustx-os --example choose_a_file
cargo run -p rustx-os --example tray_icon
cargo run -p rustx-os --example what_is_plugged_in
```

## Les commandes

| | |
|---|---|
| `rustx new <nom>` | écrit un projet qui compile du premier coup |
| `rustx dev` | regarde les fichiers, reconstruit, ramène la fenêtre |
| `rustx build` | construit, et donne au `.exe` son nom et son icône |
| `rustx run` | construit et lance une fois, sans surveiller |

`rustx build --release` pour la version à livrer. `cargo build` marche
toujours : le binaire produit fonctionne, il a seulement l'air anonyme
dans l'explorateur.

### Ce que `rustx dev` conserve, et ce qu'il ne conserve pas

`rustx dev` **relance** l'application ; il ne recharge pas du code natif
en gardant le processus vivant, et ne prétend pas le faire. Traversent la
relance :

- l'état partagé — le `let mut` écrit **en haut** d'un `.rsx`, qui porte
  un nom, et un nom se retrouve après une recompilation ;
- la page qui était ouverte.

Ne la traversent pas :

- les cellules d'un composant, dont l'identité est un emplacement dans un
  code qui vient de changer ;
- les valeurs qu'on ne sait pas écrire sur une ligne : tout ce qui n'est
  ni un nombre, ni une chaîne, ni un booléen, reparti de son
  initialisateur.

Une valeur dont le *type* a changé entre deux compilations est jetée, pas
réinjectée de force.

## L'éditeur

L'extension VS Code est dans `editors/vscode` : coloration du `.rsx`,
erreurs du compilateur soulignées à leurs vraies coordonnées, et
complétion des balises, des attributs, des événements, des clés de
`style={{ … }}` et **des composants des bibliothèques installées** —
`CardHeader`, `TabsTrigger`, `LineChart` sont lus dans `node_modules` et
dans `components/ui`. À l'intérieur d'un `{ … }`, c'est `rust-analyzer`
qui répond, derrière le serveur.

```sh
rustup component add rust-analyzer
cargo install rustx-lsp
code --install-extension editors/vscode/rustx-0.1.0.vsix
```

## Les paquets

| sur crates.io | ce qu'il fait |
|---|---|
| [`rustx-app`](https://crates.io/crates/rustx-app) | la façade : `App`, `Config`, `#[rustx::main]` — s'écrit `rustx::` |
| [`rustx-cli`](https://crates.io/crates/rustx-cli) | `rustx new`, `dev`, `build`, `run` |
| [`rustx-lsp`](https://crates.io/crates/rustx-lsp) | ce qu'un éditeur demande à propos d'un `.rsx` |
| [`rustx-compiler`](https://crates.io/crates/rustx-compiler) | `.rsx` → `.rs`, et la carte des positions |
| [`rustx-dom`](https://crates.io/crates/rustx-dom) | l'arbre, l'état, le diff, les routes |
| [`rustx-engine`](https://crates.io/crates/rustx-engine) | l'hôte CEF, la boucle de frames, le routeur |
| [`rustx-os`](https://crates.io/crates/rustx-os) | le presse-papiers, les dialogues, la barre système |
| [`rustx-macro`](https://crates.io/crates/rustx-macro) | `#[rustx::main]`, `#[derive(ToValue)]` |

Une application n'en nomme qu'un : `rustx-app`. Les autres viennent avec.

## Dans le dépôt

| | |
|---|---|
| `apps/` | les applications qui servent de preuve aux plans |
| `editors/vscode` | l'extension |
| `docs/superpowers/` | la spécification et les plans |
| `docs/publier.md` | les huit paquets, et sous quels noms ils partent |

## Ce qui reste

- macOS et Linux : chaque module compile et nomme ce qui lui manque ;
- l'asynchrone (`async`, tokio) dans un composant : le plan est écrit ;
- l'enregistrement et la capture au-delà de ce que Chromium donne déjà.

## La licence

MIT ou Apache-2.0, au choix — `LICENSE-MIT` et `LICENSE-APACHE`.
