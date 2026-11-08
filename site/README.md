# Le site de rustx

Une application Next.js autonome, en npm, sans monorepo. Elle ne dépend
que de paquets npm publiés.

## Lancer

```sh
cd site
npm install
npm run dev      # http://localhost:3000
```

Et pour la version à livrer :

```sh
npm run build
npm start        # http://localhost:3000
```

`npm install` lance `fumadocs-mdx` par son `postinstall` : c'est lui qui
écrit `.source/`, le module que `lib/source.ts` importe. Sans cette
étape, la construction ne trouve pas `@/.source/server`.

## Ce qu'il y a dedans

```text
app/
  layout.tsx            la racine : polices, thème, en-tête, pied de page
  page.tsx              l'accueil
  showcase/page.tsx     la vitrine : les applications du dépôt
  docs/layout.tsx       le menu latéral de la documentation
  docs/[[...slug]]/     une page de documentation, et sa table des matières
  api/search/route.ts   la recherche (fumadocs)
  og/route.tsx          l'image OpenGraph
  globals.css           les jetons de thème et les utilitaires
  typeset.css           la feuille de style de prose
components/             l'en-tête, le menu ⌘K, le menu latéral, …
components/ui/          les composants shadcn
content/docs/           la documentation, en MDX
lib/ hooks/             ce que ces composants tirent derrière eux
```

La documentation est le `README.md` du dépôt, découpé en sections : son
texte est le nôtre, et il est repris tel quel. L'arbre de navigation est
dans les `meta.json` de `content/docs/`.

## Ce qui vient de shadcn

Le châssis — la mise en page, les composants d'interface, la feuille de
style, la configuration de fumadocs. C'est sous licence MIT, et le
détail de ce qui a été repris, modifié ou laissé est dans `NOTICE`, avec
une copie de la licence dans `LICENSE.shadcn.md`.

`npx shadcn add <composant>` continue de marcher : `components.json`
pose les fichiers dans `components/ui/`, là où ils sont déjà.

## Les décisions prises sans demander

- **Le lien « les paquets » de l'en-tête** va à crates.io, pas à GitHub :
  le dépôt n'est pas public. Poser `NEXT_PUBLIC_REPO_URL` le remplace, et
  `lib/config.ts` est le seul endroit à changer. Le compteur d'étoiles de
  shadcn a été retiré avec le lien.
- **L'URL du site** est `https://rustx.dev` par défaut, et
  `NEXT_PUBLIC_APP_URL` la remplace. Elle sert aux métadonnées, au
  `sitemap.xml` et aux liens canoniques — rien ne la contacte.
- **Les composants `ui/` qui demandaient autre chose que des paquets npm
  publiés n'ont pas été repris** : `message-scroller` (un paquet
  `workspace:*` de shadcn), `_registry.ts`, et ceux dont la dépendance ne
  servirait à rien ici — `calendar`, `carousel`, `chart`, `combobox`,
  `drawer`, `form`, `input-otp`, `resizable`, `sonner`, `message`,
  `bubble`, `attachment`. 48 des 62 sont là. `npx shadcn add` les ramène
  à l'unité si le besoin arrive.
- **`useConfig` est un état de React et `localStorage`**, et non un atome
  `jotai` : une seule valeur ne valait pas une dépendance de plus.
- **`scroll-fade` et `scroll-fade-x` sont statiques.** Chez shadcn elles
  s'allument au défilement avec `animation-timeline: scroll()`, dans une
  feuille que le paquet `shadcn` publie ; elles sont réécrites à la fin de
  `globals.css` sans l'animation.
- **Le menu ⌘K est réécrit** : celui de shadcn cherche aussi ses blocs,
  ses couleurs et ses thèmes. Celui-ci cherche les pages de la
  documentation, les trois pages du site, et change le thème.
- **`docs-copy-page` ne copie que la page.** Les entrées « ouvrir dans
  v0 / ChatGPT / Claude » ont été retirées avec les marques.

## Ce qui reste à faire

- **Pas de capture d'écran.** La vitrine est typographique, faute
  d'images : il n'y en a aucune dans le dépôt, et inventer une capture
  aurait été pire.
- **Pas de page `.md` par document.** Le site de shadcn sert chaque page
  en Markdown brut sous `/docs/…​.md` pour les agents ; ça demandait une
  route de plus, et le `README.md` du dépôt fait déjà ce travail.
- **Pas de favicon ni d'image OG fixe.** L'image OG est engendrée par
  `/og`, mais `app/favicon.ico` et `app/opengraph-image.png` restent à
  dessiner — ceux de shadcn ont été retirés.
- **Pas de page « ce qui reste » tenue automatiquement.**
  `content/docs/ce-qui-reste.mdx` est écrite à la main depuis
  `docs/manques-du-framework.md` ; les deux peuvent diverger.
