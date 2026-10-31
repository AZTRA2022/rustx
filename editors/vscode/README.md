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
- **La complétion, le survol et « aller à la définition »** — ils viennent
  de `rust-analyzer`, que le serveur lance lui-même et interroge sur le
  `.rs` que la construction génère déjà pour la page. Donc le vocabulaire
  est celui de la caisse de l.application : ses propres modules, `rustx`,
  toutes ses dépendances. Si `rust-analyzer` n.est pas sur le `PATH`,
  `rustx.rustAnalyzer.path` dit où il est ; s.il n.est pas là du tout, il
  n.y a pas de complétion et le soulignement continue.

  À l.intérieur d.une balise, la réponse est vide, et c.est assumé : une
  région JSX tient plusieurs lignes du `.rsx` et une seule expression
  Rust, donc la colonne n.a pas d.équivalent — et une réponse fausse
  serait pire que pas de réponse.

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
