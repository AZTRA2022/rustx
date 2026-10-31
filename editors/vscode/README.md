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
- **La complétion dans le balisage** — elle vient du serveur lui-même et
  n'attend personne : les balises HTML et SVG, les attributs, les
  événements (insérés comme `onClick={move |_| }`), les clés d'un
  `style={{ … }}`, les composants déclarés dans le fichier avec leurs
  paramètres, et ceux que l'application a installés — `npx shadcn add
  button` donne `<Button>`. Les listes sont celles du compilateur, donc
  ce qui est proposé est ce qui est accepté. Rien à construire, rien à
  installer.

- **La complétion, le survol et « aller à la définition » dans le Rust** —
  ils viennent de `rust-analyzer`, que le serveur lance lui-même et
  interroge sur le `.rs` que la construction génère déjà pour la page.
  Le vocabulaire est donc celui de la caisse de l'application : ses
  propres modules, `rustx`, toutes ses dépendances. Si `rust-analyzer`
  n'est pas sur le `PATH`, `rustx.rustAnalyzer.path` dit où il est ;
  s'il n'est pas là du tout, le balisage se complète quand même et le
  soulignement continue.

- **Et dans un `{…}` du balisage** — les vraies complétions, avec les
  types. Une région JSX est émise sur une seule ligne de Rust, donc une
  colonne dedans n'a pas d'équivalent : le serveur fabrique, pour cette
  question-là, un fichier où l'expression sous le curseur est posée à sa
  ligne et à sa colonne, entourée du vrai code de la page. Donc
  `onClick={move |_| points.` propose ce que `points` a vraiment.
  Demande que l'application ait été construite une fois, parce que
  `rust-analyzer` lit un projet construit.

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
