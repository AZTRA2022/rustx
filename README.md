# rustx

Des logiciels natifs, écrits en Rust, dessinés par un moteur de navigateur.

Les composants s'écrivent dans des fichiers `.rsx` — la syntaxe de JSX,
le langage de Rust — et un `let mut` y est un état, sans annotation, sans
macro, sans `store::`. Les bibliothèques npm (React, Recharts, Tailwind,
shadcn) s'utilisent depuis le `.rsx` lui-même. Le `main.rs` d'un projet
fait quatre lignes, parce que la cérémonie est le travail du framework.

## Démarrer

```
cargo install rustx-cli
rustx new mon-app
cd mon-app
rustx dev
```

Trois commandes, et une fenêtre. Ce que `rustx new` écrit tient en cinq
fichiers : un `Cargo.toml`, un `src/main.rs`, une page, un `.gitignore`
et un dossier `assets/`. Pas de `build.rs`, rien à configurer.

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
complétion, survol et aller-à-la-définition par `rust-analyzer` derrière
le serveur.

```
rustup component add rust-analyzer
cargo install --path crates/rustx-lsp
code --install-extension editors/vscode/rustx-0.1.0.vsix
```

## L'organisation du dépôt

| | |
|---|---|
| `crates/rustx` | la façade : `App`, `Config`, `#[rustx::main]` |
| `crates/rustx-compiler` | `.rsx` → `.rs`, et la carte des positions |
| `crates/rustx-dom` | l'arbre, l'état, le diff, les routes |
| `crates/rustx-engine` | l'hôte CEF, la boucle de frames, le routeur |
| `crates/rustx-cli` | `rustx new`, `dev`, `build`, `run` |
| `crates/rustx-lsp` | ce qu'un éditeur demande à propos d'un `.rsx` |
| `apps/` | les applications qui servent de preuve aux plans |
| `docs/superpowers/` | la spécification et les plans |
