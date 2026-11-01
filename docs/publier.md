# Publier rustx sur crates.io

Huit paquets, un ordre, et une seule commande qui le respecte.

```sh
cargo publish --workspace --dry-run    # ce qui est vérifié ici
cargo publish --workspace              # ce qui part là
```

Mesuré le 2026-11-01 : les huit passent la vérification, y compris
`rustx-engine`, qui compile CEF depuis le paquet.

## Les noms

`rustx` et `rustx_macros` sont pris sur crates.io par d'autres projets
(`rustx` 0.0.1, « Run self-contained Rust scripts » ; `rustx_macros`
0.5.0). Les paquets partent donc sous :

| le paquet | la bibliothèque | ce qu'on écrit |
|---|---|---|
| `rustx-app` | `rustx` | `use rustx::{App, Config};` |
| `rustx-macro` | `rustx_macros` | `#[rustx::main]` |

Le nom de la **bibliothèque** n'est pas celui du paquet, et c'est ce qui
fait que rien d'autre ne bouge : une application écrit
`rustx = { package = "rustx-app", version = "0.1" }` dans son
`Cargo.toml` — ce que `rustx new` écrit pour elle — et tout le reste,
jusqu'au dernier `.rsx`, dit `rustx::`.

Les six autres portent leur propre nom : `rustx-dom`, `rustx-engine`,
`rustx-os`, `rustx-compiler`, `rustx-cli`, `rustx-lsp`.

## L'ordre

`cargo publish --workspace` le calcule, et c'est celui-là :

1. `rustx-dom`, `rustx-compiler` — rien en dessous d'eux ;
2. `rustx-macro` (compiler), `rustx-engine` (dom), `rustx-os` ;
3. `rustx-app` — la façade, qui dépend des quatre ;
4. `rustx-cli`, `rustx-lsp` — les outils.

## Ce qui est vérifié avant

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo doc --no-deps --workspace          # zéro avertissement
cargo publish --workspace --dry-run
```

## Ce qu'une version 0.1.0 dit

Que l'API peut encore bouger. `0.1` est la première version publiée, pas
une promesse de stabilité ; une `1.0.0` dit « ceci ne cassera plus », et
ce qui reste à écrire — macOS, Linux, les médias, l'asynchrone — touchera
encore à des signatures. La version est dans `[workspace.package]`, à un
seul endroit.

## Ce que l'utilisateur installe

```sh
cargo install rustx-cli      # rustx new, dev, build, run
cargo install rustx-lsp      # ce que l'éditeur interroge
```

Et une application n'installe rien d'autre : `rustx new` écrit un
`Cargo.toml` qui nomme `rustx-app`, et `cargo build` fait le reste —
CEF compris, par le paquet `cef`.
