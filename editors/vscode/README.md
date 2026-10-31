# rustx for VS Code

Ouvrir un `.rsx` et voir ce qu'on écrit : du Rust coloré, des balises qui
sont des balises, et les erreurs du compilateur soulignées à la ligne où
elles ont été écrites, pendant la frappe.

## Ce qu'elle contient

- **Le langage `rsx`** — `.rsx`, ses commentaires (`//`, `/* */`), ses
  paires, et l'indentation après une balise ouverte.
- **La grammaire** — Rust, plus les balises : un nom de balise est un nom
  de balise, un attribut est un attribut, et ce qui est entre `{}`
  redevient du Rust. Quand la grammaire Rust de l'éditeur est là, elle
  est utilisée ; sinon la grammaire se suffit à elle-même.
- **Le serveur** (`rustx-lsp`) — il compile le fichier à chaque frappe et
  publie ce que le compilateur a refusé, aux coordonnées du `.rsx`.
  Ouvrir un fichier ne lance jamais `cargo`.

## Développement

```sh
npm install
npm test          # la grammaire, tokenisée par vscode-textmate
```

Le serveur se construit avec le reste du projet :

```sh
cargo build -p rustx-lsp
```

La complétion, le survol et « aller à la définition » viennent du plan
suivant, qui met `rust-analyzer` derrière le même serveur.
