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

> **État.** Windows est écrit et mesuré. macOS et Linux compilent et
> refusent en nommant le fichier qui leur manque. La version est
> `0.1.0` : l'API peut encore bouger.

Ce fichier est le mode d'emploi complet. Dans l'ordre :
[installer](#installer) · [un projet](#un-projet) ·
[le fichier .rsx](#le-fichier-rsx) · [l'état](#létat) ·
[les routes](#les-routes) · [les bibliothèques JavaScript](#les-bibliothèques-javascript) ·
[la fenêtre](#la-fenêtre) · [les fils](#les-fils-de-travail) ·
[ce que le web ne sait pas faire](#ce-que-le-web-ne-sait-pas-faire) ·
[les commandes](#les-commandes) · [l'éditeur](#léditeur) ·
[quand ça ne marche pas](#quand-ça-ne-marche-pas) ·
[les paquets](#les-paquets)

---

## Installer

```sh
cargo install rustx-cli      # rustx new, dev, build, run
```

Il faut Rust 1.88 ou plus. Rien d'autre : CEF — le moteur de navigateur
— arrive avec le paquet `cef` à la première construction, et une
application qui n'utilise aucune bibliothèque npm n'a besoin ni de Node
ni de npm.

Pour les bibliothèques JavaScript, Node et npm, comme n'importe quel
projet qui en utilise.

## Un projet

```sh
rustx new mon-app
cd mon-app
rustx dev
```

Cinq fichiers, et une fenêtre :

```text
mon-app/
  Cargo.toml        rustx = { package = "rustx-app", version = "0.1" }
  src/main.rs       la fenêtre, en quatre lignes
  app/page.rsx      la page `/`
  assets/           ce que la page charge (une icône, une image, un index.html)
  .gitignore        /target
```

Le paquet s'appelle `rustx-app` sur crates.io, parce que `rustx` y est
pris par un autre projet ; la **bibliothèque** s'appelle `rustx`, donc
tout le code s'écrit `rustx::` :

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

`#[rustx::main]` fait tout le reste : il compile les `.rsx` de `app/` en
Rust, écrit la table des routes, construit le paquet JavaScript et la
feuille de style s'il y a un `package.json`, trouve ou écrit
`assets/index.html`, et fait sortir les sous-processus de Chromium avant
qu'ils touchent à quoi que ce soit. Aucune de ces lignes ne varie d'une
application à l'autre, donc aucune n'est à écrire.

Il n'y a **pas de `build.rs`**, et rien à configurer.

## Le fichier `.rsx`

Un `.rsx` est du Rust, avec du balisage dans les `return`. Un composant
est une fonction qui rend un `Element` :

```rust
fn Page() -> Element {
    return <main>
        <h1>Bonjour.</h1>
    </main>;
}
```

### Le texte et les expressions

Entre deux balises, le texte est du texte ; entre accolades, c'est du
Rust, et la valeur est affichée :

```rust
fn Page() -> Element {
    let nom = "monde";
    let combien = 3;

    return <p>Bonjour {nom}, {combien} fois, {combien * 2} en tout.</p>;
}
```

Tout ce qui sait devenir un `Element` ou du texte peut aller là :
nombres, chaînes, booléens, `Option`, `Vec<Element>`.

Un `.rsx` est du Rust, et un commentaire s'y écrit comme en Rust —
y compris **dans** les balises :

```rust
return <div>
    // Ce qui suit est la barre de titre, et ceci ne sera pas affiché.
    <header
        // Un commentaire entre deux attributs est un commentaire aussi.
        className="flex gap-2"
    >
        {/* Et la forme de JSX, pour qui vient de là. */}
        <b>ok</b>
    </header>
    /* Sur plusieurs lignes, également. */
</div>;
```

Un `//` est un commentaire quand il **commence** une ligne : `voir
https://exemple.fr` reste une phrase, adresse comprise. Ce qui entoure
un commentaire reste du texte, et ce qui est dedans n'arrive jamais à
l'écran.

### Les attributs

Les balises prennent **les props de HTML et de JSX**, et le compilateur
refuse ce qui n'existe pas en proposant le nom le plus proche :

```rust
return <button
    id="go"
    className="primary large"
    disabled={occupe}
    aria-label="Lancer le rendu"
    data-kind="action"
    onClick={move |_| lancer()}
>Lancer</button>;
```

- `className` et non `class`, `onClick` et non `onclick` : la graphie de
  JSX ;
- `aria-*` et `data-*` sont acceptés tels quels, avec leurs tirets ;
- un attribut booléen prend une expression (`disabled={true}`) ;
- un gestionnaire prend une fermeture : `onClick={move |_| …}`,
  `onInput={move |event| …}`.

Ce que l'événement porte — une liste courte et fixe, parce qu'un nœud du
DOM ne traverse pas :

| | |
|---|---|
| `event.kind` | `click`, `input`, `keydown` : le nom du DOM |
| `event.value` | `target.value` : ce qui est dans le champ, en texte |
| `event.number()` | le même, en nombre, quand c'en est un |
| `event.id` | `target.id`, pour qu'un gestionnaire serve plusieurs éléments |
| `event.key` | `a`, `Enter`, `Escape`, `ArrowUp` |
| `event.checked` | pour une case à cocher |
| `event.shift` `event.control` `event.alt` `event.meta` | les modificateurs |

```rust
let mut terme = String::new();

fn Chercher() -> Element {
    return <div>
        <input value={terme.clone()} onInput={move |event| terme = event.value} />
        <input onKeyDown={move |event| if event.key == "Enter" { lancer(&terme) }} />
    </div>;
}
```

Le `style` s'écrit en objet, comme en JSX, avec les clés en
`camelCase` :

```rust
return <div style={{
    display: "grid",
    placeItems: "center",
    height: "100vh",
    backgroundColor: "#0b0b10",
}}>…</div>;
```

### Les composants et leurs props

Un composant s'appelle **comme une fonction de Rust**, entre accolades.
Pas de balise pour les composants du projet : ses props sont ses
arguments, avec les types de Rust.

```rust
fn Page() -> Element {
    return <main>
        {Carte("Rust".to_string(), 3)}
        {Carte("React".to_string(), 1)}
    </main>;
}

fn Carte(titre: String, combien: u32) -> Element {
    return <article className="carte">
        <h2>{titre}</h2>
        <p>{combien} fois</p>
    </article>;
}
```

Une balise **majuscule** est réservée aux composants des bibliothèques
JavaScript — voir [plus bas](#les-bibliothèques-javascript).

### Les listes et les conditions

Du Rust ordinaire : `map` et `collect` pour une liste, `if` ou `match`
pour un choix.

```rust
fn Liste(noms: Vec<String>) -> Element {
    let items: Vec<Element> = noms
        .iter()
        .map(|nom| Element::tag("li").child(Element::Text(nom.clone())).build())
        .collect();

    return <ul>{items}</ul>;
}
```

Et plus souvent, un composant par ligne :

```rust
let lignes: Vec<Element> = appareils.iter().map(|each| Ligne(each.clone())).collect();

return <ul>{lignes}</ul>;
```

Une condition dans l'arbre :

```rust
return <div>
    {if occupe { Attente() } else { Resultat(valeur) }}
</div>;
```

> Une boucle qui ne finit pas, écrite **avant** le `return`, bloque la
> frame : le rendu est synchrone, et une page qui ne rend pas son
> `Element` ne dessine rien. Un travail qui dure va dans un
> [`task`](#les-fils-de-travail).

## L'état

Un `let mut`, et **sa place dit sa portée**. Il n'y a rien d'autre à
apprendre.

```rust
let mut points = 0;              // au sommet du fichier : partagé

fn Page() -> Element {
    let mut ouvert = false;      // dans un composant : privé

    return <div>
        <p>{points} points</p>
        <button onClick={move |_| points += 1}>un de plus</button>
        <button onClick={move |_| ouvert = !ouvert}>{ouvert}</button>
    </div>;
}

fn Ailleurs() -> Element {
    // Le même `points`, la même valeur : deux composants qui lisent un
    // état partagé lisent le même objet, sans que rien ne descende de
    // l'un à l'autre.
    return <small>{points}</small>;
}
```

- **au sommet du fichier** : partagé par l'application. Il porte un nom,
  donc il survit à un `rustx dev` ;
- **dans un composant** : une cellule par emplacement dans l'arbre. Deux
  `{Carte(…)}` ont chacune la leur, et quitter l'écran les libère.

Lire, c'est lire. Écrire, c'est marquer l'écran pour la frame suivante :
dix mille écritures dans la même frame dessinent **une** fois. Le débit
est celui de la boucle de rendu, pas celui de la boucle `for` du
développeur — il n'y a ni *debounce* à régler ni limitation à apprendre.

Un état ne s'écrit pas **pendant** un rendu : un composant qui calcule
son écran ne le change pas en même temps. L'écriture va dans un
gestionnaire (`onClick={…}`), dans un `effect`, ou dans un `task`, et le
compilateur le dit avec ces trois mots quand on l'oublie.

### `keep`, `effect`, `task`

Trois noms, dans un composant, et rien d'autre à retenir :

```rust
fn Panneau() -> Element {
    // Construit une fois, pas à chaque rendu.
    let client = keep(Client::connect());

    // Rejoué quand ce qui est surveillé change. `()` : une seule fois.
    effect(filtre.clone(), move || recharger(&filtre));

    // Un fil qui s'arrête quand ce composant quitte l'arbre.
    task(move |alive| {
        while alive.get() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = alive.post(|| ticks += 1);
        }
    });

    return <p>{ticks}</p>;
}
```

`keep(EXPR)` garde une valeur. `effect(SURVEILLÉ, TRAVAIL)` refait le
travail quand la valeur surveillée change. `task(TRAVAIL)` démarre un
fil dont le jeton est une cellule du composant : quitter l'écran
l'arrête, sans un seul nettoyage écrit à la main.

## Les routes

Les routes sont le dossier `app/` :

```text
app/page.rsx                      →  /
app/reglages/page.rsx             →  /reglages
app/device/[id]/page.rsx          →  /device/<quelque chose>
app/fichiers/[...chemin]/page.rsx →  /fichiers/<tout le reste>
app/aide/[[...chemin]]/page.rsx   →  /aide, et /aide/<tout le reste>
app/(boutique)/panier/page.rsx    →  /panier  (le groupe n'est pas dans l'URL)
app/layout.rsx                    →  un cadre autour de ce qui est en dessous
app/depots/loading.rsx            →  ce qu'on montre tant que la page n'a rien
app/depots/error.rsx              →  ce qu'on montre si la page tombe
```

Un segment entre crochets arrive en argument du composant, dans l'ordre
où il est écrit. Quand plusieurs routes décrivent le même chemin, le
plus précis gagne : un segment statique bat un segment nommé, qui bat un
attrape-tout.

```rust
// aller ailleurs
rustx::router::navigate("/reglages");

// y aller *à la place* de l'écran courant : revenir ne ramènera pas ici
rustx::router::replace("/merci");

// revenir, et repartir en avant
rustx::router::back();            // false s'il n'y a pas d'avant
rustx::router::forward();         // false si on n'est pas revenu

// ce qu'un bouton demande avant de se dessiner gris
rustx::router::can_go_back();
rustx::router::can_go_forward();

// où on est
let ici = rustx::router::current();        // avec la query string
let route = rustx::router::route();        // le chemin seul
let (visites, rang) = rustx::router::history();

// ce que la query string dit
let terme = rustx::router::query("q");     // "" si personne ne l'a écrit
rustx::router::has_query("brut");          // `?brut` ne dit rien et est là
// l'URL d'à côté, sans encoder quoi que ce soit à la main
rustx::router::navigate(rustx::router::with_query("tri", Some("étoiles")));
```

L'historique est une liste et un doigt dessus : revenir déplace le
doigt et garde ce qui est devant, aller ailleurs efface ce futur-là.
C'est la règle d'un navigateur.

Une page peut aussi lire ses paramètres sans les déclarer, et un
**layout** prend ceux de son propre dossier :

```rust
// app/depots/[proprietaire]/layout.rsx
fn Layout(children: Element, proprietaire: String) -> Element {
    return <section><h1>{proprietaire}</h1>{children}</section>;
}
```

Il ne peut demander que les segments de son dossier à lui : les écrans
en dessous peuvent en avoir plus, et il est le cadre de tous. Un layout
qui demande autre chose est refusé en disant pourquoi.

Un layout qui entoure encore le nouvel écran garde ses cellules ; l'écran
qu'on quitte est libéré, et les fils qu'il tenait s'arrêtent.

### Attendre, et tomber

Une page qui attend un `.await` ne dessine rien jusqu'à la réponse — ce
qui est juste, et est aussi un écran blanc. Le `loading.rsx` le plus
proche va là, **dans les mêmes layouts** : la barre du haut reste, et
seul le milieu dit « un instant ».

```rust
// app/depots/loading.rsx
fn Loading() -> Element {
    return <p className="animate-pulse">un instant…</p>;
}
```

Et si la page tombe — un `unwrap` sur rien, un index hors liste — le
`error.rsx` le plus proche prend sa place, avec ce que la panique a dit,
au lieu que la fenêtre s'en aille :

```rust
// app/depots/error.rsx
fn Error(pourquoi: String) -> Element {
    return <p>ça n'a pas marché : {pourquoi}</p>;
}
```

Sans `error.rsx` au-dessus d'elle, une page qui tombe se comporte comme
avant : rien n'est attrapé, parce qu'attraper partout transformerait un
bogue en écran que personne n'a demandé.

`Config` dit par où ça commence et ce qu'on montre pour une route que
personne n'a déclarée :

```rust
Config::new("Mon app")
    .start("/reglages")           // `/` par défaut
    .not_found(Introuvable)       // sinon : « This page does not exist. »
```

## Les bibliothèques JavaScript

Un `package.json` et un `npm install`, et c'est tout : le framework lit
les `dependencies`, écrit le point d'entrée, construit le paquet avec
esbuild, et une balise majuscule est résolue à l'exécution.

```sh
cd mon-app
npm install react react-dom recharts
npx shadcn add button card dialog
```

```rust
return <Card>
    <CardHeader><CardTitle>Les ventes</CardTitle></CardHeader>
    <CardContent>
        <LineChart width={520} height={240} data={points}>
            <XAxis dataKey="at" />
            <Line dataKey="value" stroke="#8b5cf6" />
        </LineChart>
    </CardContent>
</Card>;
```

`Card` et `CardHeader` viennent de `components/ui/`, où `npx shadcn add`
les a laissés ; `LineChart` vient de recharts.

**Aucun préfixe à écrire.** Les composants de l'application passent
devant les bibliothèques dont ils sont construits : `<Dialog>` est celui
de `components/ui/dialog.tsx`, et non la primitive de
`@radix-ui/react-dialog` sur laquelle il est bâti. Pour demander l'autre,
explicitement :

```rust
return <radixUiReactDialog::Dialog>…</radixUiReactDialog::Dialog>;
```

Le nom devant `::` est celui du paquet, sans ce qu'une ligne de Rust
n'accepte pas : `recharts` reste `recharts`, `@radix-ui/react-dialog`
devient `radixUiReactDialog`. Deux bibliothèques qui exportent le même
nom, et c'est dit dans la console avec la forme explicite à écrire.

### shadcn

`npx shadcn add` marche. `npx shadcn init` **non** : il cherche un
framework qu'il connaît — Next, Vite, Remix — et s'arrête en disant
qu'il n'en trouve aucun. Ce qu'il aurait écrit s'écrit à la main, une
fois, et ce sont deux fichiers.

`components.json`, qui dit à `shadcn add` où poser ce qu'il télécharge :

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "assets/tailwind.css",
    "baseColor": "neutral",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib",
    "hooks": "@/hooks"
  },
  "iconLibrary": "lucide"
}
```

`tsconfig.json`, qui dit à esbuild ce que `@/` veut dire — les
composants de shadcn s'importent entre eux par cet alias :

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "skipLibCheck": true,
    "baseUrl": ".",
    "paths": { "@/*": ["./*"] }
  },
  "include": ["components/**/*", "lib/**/*", "hooks/**/*"]
}
```

Ensuite, `npx shadcn add button card dialog` pose ses fichiers dans
`components/ui/`, et une balise majuscule les trouve. Les paires que
shadcn n'installe pas toujours lui-même se demandent comme le reste :
`npm install class-variance-authority lucide-react`.

### Les composants React que le projet écrit lui-même

`components/` en entier, pas seulement `components/ui/`. Un composant à
soi — celui qui appelle un hook, ce qu'une page `.rsx` ne sait pas
faire — se range là, et son `export default` est une balise sous le nom
de son fichier :

```tsx
// components/Editor.tsx
import { useEditor, EditorContent } from "@tiptap/react"
import StarterKit from "@tiptap/starter-kit"

export default function Editor({ contenu }: { contenu?: string }) {
  const editor = useEditor({ extensions: [StarterKit], content: contenu })
  return <EditorContent editor={editor} />
}
```

```rust
return <Editor contenu="<p>Écrire ici.</p>" />;
```

C'est la porte de sortie pour tout ce qu'un hook fait et qu'une page ne
peut pas : TipTap, une carte, un éditeur de code. La page garde l'état
et les props ; le hook reste en TypeScript, là où il est chez lui.

### Les paquets que le paquet n'embarque pas

Trois sortes de `dependencies` ne vont pas dans le paquet du
navigateur, et le framework les laisse de côté sans rien demander :

- **les outils de la construction** — `tailwindcss`,
  `@tailwindcss/cli`, `esbuild`, `typescript`, `vite`. Un navigateur ne
  rend pas un compilateur ;
- **ceux qui n'exportent que des sous-chemins** — `@tiptap/pm` exporte
  `./state` et `./view` et pas de racine : une page écrit
  `@tiptap/pm/state` ou rien ;
- **ceux dont le point d'entrée est une feuille de style** —
  `tw-animate-css` appartient à la feuille, pas au paquet.

Un paquet déclaré dans `dependencies` et absent de `node_modules`, lui,
est dit par son nom, parce que la réparation est une commande.

### Les props qui traversent

Ce qu'on passe à un composant JavaScript est une valeur V8, dans le même
processus : pas de JSON, pas de pont sérialisé.

```rust
#[derive(ToValue)]
struct Point { at: f64, value: f64 }

let data: Vec<Point> = …;

return <LineChart data={data} width={520} height={240}>…</LineChart>;
```

`#[derive(ToValue)]` sur une structure, et elle passe. Pour un objet
écrit sur place : `record([("at", 1.0.to_value()), ("value", 4.0.to_value())])`.

### Tailwind

Il marche, en version 3 comme en 4, et le framework lit laquelle
`node_modules` contient : `--content` et `@tailwind utilities` pour la 3,
`@source` et `@import "tailwindcss"` pour la 4. Les classes sont
cherchées dans les `.rsx`, et la feuille de style est posée dans la page
sans qu'on l'écrive nulle part.

En 4, la ligne de commande vit dans un paquet à elle : il faut les deux,
`npm install tailwindcss @tailwindcss/cli`, comme la page de Tailwind le
dit. Avec la bibliothèque seule, la construction refuse en nommant la
commande qui manque — elle se taisait, et la fenêtre s'ouvrait avec
tous ses `className` et pas une règle.

Les classes sont cherchées dans `app/**/*.rsx`, `src/**/*.rs`,
`components/**` et `lib/**` : les pages, le Rust — un nom de classe peut
vivre dans un `const` — et les composants React du projet, où shadcn
garde les siens.

C'est la **seule** exception à la règle « aucune ligne du framework ne
nomme une bibliothèque ni une version », parce qu'un programme que la
construction *exécute* a une ligne de commande, et qu'une ligne de
commande a une version.

#### Un thème : les jetons ne suffisent pas

Une feuille de style qui déclare `--background` et `--foreground` ne
donne **pas** `bg-background` ni `text-foreground` : une variable CSS
n'est pas une utilitaire Tailwind. Sans le pont, les classes sont des
mots dans des attributs, la fenêtre est blanche, et rien ne se plaint.
Ce pont est `@theme inline`, dans `assets/tailwind.css` :

```css
@import "tailwindcss";
@custom-variant dark (&:is(.dark *));   /* si le thème est une classe */

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --radius-lg: var(--radius);
  /* La police d'une application de bureau est celle du système :
     elle est déjà installée, et c'est celle qu'on reconnaît. */
  --font-sans: "Segoe UI Variable Text", "Segoe UI", ui-sans-serif, system-ui;
}

@layer base {
  html, body {
    background-color: var(--background);
    color: var(--foreground);
    font-family: var(--font-sans);
  }
}
```

`npx shadcn@latest init` écrit ce bloc ; une feuille copiée à la main
l'oublie, et c'est ce qui se voit en premier.

### Dans l'autre sens : un composant rustx dans du React

```rust
// un composant de la page, mis à la disposition de React
Config::new("Mon app").mount("panneau", Panneau)
```

```jsx
// et en JavaScript, là où rustx garde la main
<div id="panneau" />
```

## La fenêtre

`Config` porte ce qu'une fenêtre est :

| | |
|---|---|
| `Config::new(titre)` | le titre, et le nom de l'application |
| `.size(largeur, hauteur)` | en pixels |
| `.icon("assets/icon.ico")` | la barre de titre et la barre des tâches |
| `.version("1.4.0")` | lu par la page, et par `rustx build` |
| `.description("…")` | une ligne |
| `.publisher("…")` | qui l'édite |
| `.index("assets/index.html")` | un document à soi, sinon le framework l'écrit |
| `.host("app")` | l'élément où les écrans sont montés |
| `.start("/")` | l'écran d'ouverture |
| `.not_found(Composant)` | l'écran d'une route que personne n'a déclarée |
| `.mount("id", Composant)` | un composant que React pourra monter |
| `.on_context_created(…)` | du travail au démarrage de la page |
| `.frameless()` | sans le cadre du système : la page dessine tout |

### Une fenêtre sans cadre, et ce qu'il faut alors rendre

Beaucoup de logiciels n'ont pas la fenêtre que le système dessine.
`Config::frameless()` enlève la barre de titre et les trois boutons —
les bords qu'on attrape pour redimensionner restent, ainsi que l'ombre
portée.

Ce que le cadre faisait, la page doit le rendre, et `rustx::window` le
donne :

| | |
|---|---|
| `window::drag()` | commencer un déplacement à la souris |
| `window::minimize()` | réduire dans la barre des tâches |
| `window::maximize()` | agrandir à toute la place du bureau |
| `window::restore()` | revenir à sa taille |
| `window::fullscreen()` | tout l'écran, celui où la fenêtre se trouve |
| `window::toggle_maximize()` | agrandir, ou rendre sa taille si elle l'est déjà |
| `window::toggle_fullscreen()` | prendre tout l'écran, ou en revenir |
| `window::on_top(true)` | la garder devant les autres |
| `window::frameless()` / `window::framed()` | enlever ou remettre le cadre, en cours de route |
| `window::shut()` | la fermer |

Une barre de titre à soi, en entier :

```rust
<header className="flex items-center gap-2 border-b px-3 py-2">
    <span className="font-semibold">Mon logiciel</span>
    // La poignée : appuyer dessus et le système déplace la fenêtre.
    <span className="h-6 flex-1" onMouseDown={|_| { rustx::window::drag(); }} />
    <button onClick={|_| { rustx::window::minimize(); }}>–</button>
    <button onClick={|_| { rustx::window::maximize(); }}>□</button>
    <button onClick={|_| { rustx::window::shut(); }}>×</button>
</header>
```

La poignée est posée sur un espace vide et non sur toute la barre :
un événement rustx ne dit pas encore sur quoi on a appuyé, donc une
barre entière traînable avalerait les clics de ses propres boutons.

Le bouton « agrandir » d'une barre à soi **bascule** : c'est
`toggle_maximize()` et non `maximize()`. Dire « agrandis-toi » à une
fenêtre déjà agrandie ne fait rien, et un bouton qui n'agit qu'une fois
est ce qu'on obtient avec l'autre. La bascule interroge le système
avant d'agir, dans le processus qui tient la poignée — la page n'a rien
à retenir. `toggle_fullscreen()` de même, et sortir du plein écran rend
à la fenêtre **la place qu'elle avait**, parce que le plein écran est
une taille qu'on a posée et que personne d'autre ne s'en souvient.

Et ce que la fenêtre sans cadre rend aussi : **le haut**. Enlever la
barre de titre laisse une bande de quelques pixels que le système peint
lui-même, et qui se voit comme un trait clair au-dessus de ce que la
page a dessiné. `frameless()` la reprend, de sorte que le contenu
commence au bord ; les bords de gauche, de droite et du bas restent ceux
du système, car ce sont eux qu'on attrape pour retailler.

Une deuxième fenêtre se demande sans cadre de la même façon :
`What::new("outil").frameless()`.

Tout ceci est Windows aujourd'hui. Sur les autres plateformes, chaque
appel refuse en nommant la plateforme et la chose demandée, plutôt que
de ne rien faire en silence — ce qui sera écrit est dit dans
`crates/rustx-os/src/linux/window.rs` et dans `other/mod.rs`.

Une page peut ouvrir une **deuxième fenêtre**, sur la route qu'elle
choisit :

```rust
<button onClick={move |_| {
    let _ = rustx::window::open(
        rustx::window::What::new("reglages")   // son nom, pour la refermer
            .titled("Réglages")
            .size(520, 400)
            .at("/reglages"),                  // la route où elle s'ouvre
    );
}}>Réglages</button>
```

La route voyage dans le fragment de l'URL (`…/index.html#/reglages`), ce
qui se lit dans les traces. Ouvrir deux fois le même nom ramène la
fenêtre devant au lieu d'en faire une deuxième. `Window::close()` la
referme, et le `Drop` du jeton **ne** la referme pas — une page ouvre une
fenêtre depuis un gestionnaire d'événement, et une fenêtre qui mourrait
avec le jeton ne serait jamais vue.

Fermer une fenêtre n'arrête pas l'application tant qu'il en reste une.
Chaque fenêtre a son propre processus de rendu, donc son propre état :
ce qu'elles partagent, elles le partagent par un message, pas par une
variable.

## Les fils de travail

Un fil ne touche **jamais** l'état. Il poste un travail, et la frame le
drainera :

```rust
task(move |alive| {
    let flux = ouvrir_la_camera();
    while alive.get() {
        let image = flux.lire();
        if alive.post(move || derniere = image).is_err() {
            break;          // la fenêtre est partie, ou ce composant
        }
    }
});
```

- `alive.get()` : ce composant est-il encore là ;
- `alive.post(travail)` : à exécuter sur le fil de l'interface, au début
  de la prochaine frame ;
- `Err(PostError::WindowClosed)` : la fenêtre est fermée, il faut
  arrêter ; `Err(PostError::Cancelled)` : le composant qui a démarré ce
  fil s'en va.

Hors d'un composant, `rustx::poster()` donne le même jeton, et
`rustx::task::spawn(travail)` un fil dont on garde le `Task`.

### L'asynchrone : un bloc `async`

Un bloc `async` écrit comme une instruction d'un composant est du
travail qui tourne à côté du rendu, porté par un runtime tokio qui vit
sur ses propres threads. Il n'y a rien à mettre autour : en JSX on
n'enveloppe rien non plus, et le mot `async` disait déjà tout ce que
`task(…)` ajoutait.

```rust
let mut lignes = Vec::<String>::new();

fn Panneau() -> Element {
    async move {
        let reponse = charger().await;
        lignes.push(reponse);
    };

    return <ul>{lignes.len()} lignes</ul>;
}
```

Un bloc qui est une **valeur** est laissé tranquille : `let tenu = async
{ … };` est une future que le composant garde, et ce qu'il en fait lui
appartient. `task(async { … })` marche toujours, et veut dire la même
chose — la forme explicite, pour qui préfère la nommer.

#### Lire une valeur de la page depuis le bloc

Un bloc lit les valeurs que la page a déclarées, là où elles sont :

```rust
let mut clics = 0;
let mut enregistrements = 0;

fn Page() -> Element {
    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let texte = format!("clics: {}", clics);
            if tokio::fs::write("sauvegarde.txt", texte).await.is_ok() {
                enregistrements += 1;
            }
        }
    };

    return <button onClick={move |_| clics += 1}>{clics}</button>;
}
```

Il n'y a rien à déclarer à côté : ni `keep`, ni `Arc`, ni `Mutex`, ni
tampon qu'un gestionnaire devrait penser à tenir à jour. Une valeur que
le compilateur voit lue depuis une future vit dans un seul endroit que
les deux côtés atteignent, donc la boucle voit le `clics` d'aujourd'hui
et non une copie prise au premier tour.

Lire et écrire ne sont pas symétriques, et c'est voulu. Une **lecture**
est directe. Une **écriture** repart par la frame — c'est elle qui
marque la page, qui fait coûter un seul rendu à mille écritures, et qui
refuse l'écriture d'un composant déjà parti. Ce qui en découle, dit une
fois : *une future qui écrit puis relit voit l'ancienne valeur jusqu'à
la frame qui applique l'écriture.*

Deux limites à connaître. La valeur doit être `Send + Sync` — c'est ce
qu'être lue depuis un autre thread demande, et rustc le dit sur la ligne
de la page. Et c'est une valeur **de la page**, déclarée au sommet du
fichier : une `let mut` interne à un composant appartient à l'instance
qui dessine, donc elle est refusée dans un bloc, et le refus dit où la
déclarer.

`.await` marche sur tout ce que l'écosystème écrit — `reqwest`, `sqlx`,
un `Future` de `std`, un timer. Le runtime démarre à la **première**
future du processus et jamais avant : une application qui n'écrit aucun
`async` ne paie pas un thread de plus.

`tokio` vient de la façade, donc une application ne le nomme ni dans
son `Cargo.toml` ni dans ses `use`, et l'écrit du nom que tout le monde
lui donne :

```rust
let mut battements = 0;

fn Horloge() -> Element {
    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            battements += 1;
        }
    };

    return <p>{battements}</p>;
}
```

C'est tokio en entier, avec `full` : `tokio::fs`, `::net`, `::io`,
`::sync`, `::process`, `select!`, `spawn` — ce qu'un code async
ordinaire écrit, écrit pareil, et pas un sous-ensemble choisi par le
framework. Une application qui déclare tokio elle-même obtient le même :
cargo n'en résout qu'un seul en 1.x.

### `async fn main`

Le `main` peut attendre, comme n'importe où ailleurs :

```rust
use rustx::{App, Config};

#[rustx::main]
async fn main() -> Result<(), rustx::Error> {
    let reglages = tokio::fs::read_to_string("reglages.toml").await?;

    let app = App::new(Config::new(&titre_de(&reglages)).size(900, 640))?;

    app.run()
}
```

`main` lui-même ne peut pas être `async` — c'est Rust qui le dit, pas le
framework — donc l'attribut lui retire son `async` et exécute le corps
sur le runtime. Il n'y a rien d'autre à savoir : pas de `#[tokio::main]`
à écrire à côté, et pas de `block_on` à la main.

Ce qui reste vrai : **un composant** ne s'arrête jamais pour attendre.
Il peut demander, et c'est la section suivante.

### Attendre une valeur sans bloquer le rendu

`let x = travail().await;` dans un composant s'écrit tel quel :

```rust
fn Reglages() -> Element {
    let texte = tokio::fs::read_to_string("reglages.toml")
        .await
        .unwrap_or_default();

    return <pre>{texte}</pre>;
}
```

Un composant est interrogé sur l'écran **tel qu'il est maintenant**,
donc il ne s'arrête pas : la première frame demande et ne dessine rien,
la frame qui suit l'arrivée de la réponse la dessine. La question n'est
posée qu'une fois, le type écrit est gardé, et le reste de la page est
dessiné pendant ce temps — l'attente appartient au composant qui l'a
écrite.

Ce que la question **emprunte** doit lui survivre. `api::depot(&nom,
&proprietaire).await` dans un composant ne compile pas : la future part
sur le runtime, et un emprunt d'un local du fil qui dessine ne la suit
pas. C'est du Rust ordinaire, pas une règle du framework, et la sortie
est de prendre les arguments en propre — `depot(nom.clone(),
proprietaire.clone())`. Le message, lui, vient de `rustc` et désigne une
colonne d'un fichier engendré : c'est le défaut connu de ce coin-là.

Ce qui suit le `.await` part avec la future, parce que la réponse
n'existe que là-bas : `.unwrap_or_default()`, `.ok()`, un autre `.await`
y sont à leur place. Un `.await` enterré dans un argument —
`f(g().await)` — attend avant que `f` soit même appelée, et obtient une
phrase au lieu d'une compilation. Lire une valeur de la page **après**
le `.await` en obtient une aussi : ce code tourne à côté de la fenêtre,
où il n'y a pas de cellule à lire.

#### Écrire l'état depuis un `await`

Comme depuis un `onClick`, et rien de plus à écrire. Ce qui se passe
dessous : l'état appartient au thread qui dessine, donc la future ne le
touche pas. Elle tient un **handle** qui dit où est la valeur, l'écriture
part sur la file de la frame, et c'est la frame qui l'applique.

Les conséquences se lisent, et ce sont les mêmes que pour un fil :

- **mille écritures dans une boucle `async` font un rendu**, parce
  qu'elles salissent la même valeur une fois. Il n'y a ni *debounce* à
  régler ni limitation à apprendre ;
- une écriture postée **après la fermeture de la fenêtre** est refusée,
  et une écriture qu'une tâche avait postée **avant de s'en aller** l'est
  aussi — le jeton que `task` rend la refuse, ce qui compte surtout pour
  un `let mut` du haut de la page : un nom survit à l'écran, donc sans ça
  l'écriture d'une page partie atterrissait sur la suivante. Rien ne
  panique, et aucun des deux cas n'est une faute du développeur ;
- un `let mut` écrit depuis un bloc `async` est un état, exactement comme
  un `let mut` écrit depuis un gestionnaire. La règle de promotion ne
  change pas.

**Une valeur de la page se lit dans le bloc, là où elle est.** Pas de
copie prise au premier tour : une boucle qui surveille un filtre voit
celui que l'utilisateur vient de taper, et un bouton « pause » agit
vraiment sur la boucle.

```rust
let mut resultats = Vec::<String>::new();
let mut filtre = String::new();

fn Recharger() -> Element {
    async move {
        let mut dernier = String::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if filtre == dernier {
                continue;
            }
            dernier = filtre.clone();
            resultats = chercher(&dernier).await;
        }
    };

    return <p>{resultats.len()} trouvés</p>;
}
```

Deux choses restent refusées, et le refus dit où déclarer la valeur :

- **une cellule de composant** — un `let mut` écrit *dans* le composant —
  appartient à l'instance qui dessine, et le bloc tourne à côté. Ce
  qu'il faut lire se lit avant le bloc, et la copie y entre ;
- **une lecture après un `.await`** dans un `let … .await` de composant :
  ce qui suit le `.await` part avec la future, où il n'y a pas de cellule
  à lire.

#### Quitter l'écran arrête le travail

Le jeton que `task` rend est une cellule du composant. Le composant
s'en va, la cellule est lâchée, et la future est **abandonnée là où elle
attend** — un `await` n'a pas de drapeau à regarder, contrairement au
`alive.get()` d'un fil. Rien à nettoyer à la main, et rien à annuler par
son nom.

> **Un composant n'est jamais `async`.** `async fn Page()` est refusé en
> toutes lettres, et le refus nomme `task(…)`. On demande à un composant
> l'écran tel qu'il est **maintenant** : s'il pouvait attendre, la frame
> attendrait avec lui, et une fenêtre qui attend est une fenêtre gelée.
> C'est la seule raison, et elle suffit.

## Ce que le web ne sait pas faire

Une page ne peut pas lire le presse-papiers quand elle veut, ni ouvrir un
dialogue qui rend un chemin, ni poser une icône à côté de l'horloge.
C'est ce que `rustx-os` fait, et ça s'écrit dans une page comme le reste.

Chaque module répond **`supported()`** avant qu'on lui demande quoi que
ce soit, pour qu'une fonctionnalité absente soit un bouton qu'on ne
dessine pas plutôt qu'une erreur à lire. Et chaque chose enregistrée
auprès du système est un **jeton** : lâché, il la retire — une icône
morte dans la barre ou un `Ctrl+Shift+R` qu'aucune application ne rend
sont des défauts que l'utilisateur ne peut réparer qu'en redémarrant.

### Le presse-papiers

```rust
use rustx::clipboard;

clipboard::set_text("copié depuis Rust")?;
let colle = clipboard::text()?;                  // Result<String, OsError>

let image = clipboard::image()?;                 // { width, height, bgra }
clipboard::set_image(&image)?;

clipboard::set_custom("mon-app/noeuds", &octets)?;
let octets = clipboard::custom("mon-app/noeuds")?;
```

### Les dialogues de fichiers

Ceux du système, et ils rendent un `PathBuf` — pas un `File` de
navigateur.

```rust
use rustx::dialog;

let fichiers = dialog::open(
    dialog::Ask::new("Choisir une image")
        .filter("Images", ["png", "jpg"])
        .starting_in("C:/Users/moi/Pictures")
        .many(),
)?;                                              // Vec<PathBuf>

let ou = dialog::save(dialog::Ask::new("Enregistrer").named("sans-titre.png"))?;
let dossier = dialog::folder(dialog::Ask::new("Choisir un dossier"))?;
```

Une annulation est un cas normal, pas une panne :

```rust
let Ok(fichiers) = dialog::open(dialog::Ask::new("Ouvrir")) else { return; };
```

Un dialogue modal fait tourner sa propre boucle de messages : la fenêtre
reste dessinée pendant qu'il est ouvert.

### La barre système et les notifications

```rust
use rustx::{notify, tray};

let icone = tray::Icon::new(Path::new("assets/icon.ico"), "Mon logiciel")?;
icone.menu(&[
    tray::Item::new("rendre", "Lancer le rendu"),
    tray::Item::new("silence", "Silencieux").checked(false),
    tray::Item::separator(),
    tray::Item::new("quitter", "Quitter"),
])?;

if let Some(choisi) = icone.clicked() {          // lu quand on veut
    match choisi.as_str() {
        "rendre" => lancer(),
        tray::CLICK => montrer_la_fenetre(),     // l'icône elle-même
        _ => {}
    }
}

notify::show(notify::What::new("Fini", "Le rendu est terminé."))?;
```

L'icône est retirée quand le jeton est lâché. La notification passe par
`Shell_NotifyIconW` et non par un toast WinRT : un toast exige un
identifiant enregistré dans le menu Démarrer, ce qu'un `rustx dev` n'a
pas et ne doit pas exiger.

### Les raccourcis globaux

Ils marchent quand la fenêtre n'a pas le focus.

```rust
use rustx::shortcut;

let capture = shortcut::Shortcut::listen("Ctrl+Shift+R")?;
if capture.pressed() > 0 { commencer(); }        // combien de fois depuis la dernière fois
```

`Ctrl`, `Alt`, `Shift`, `Win`, les lettres et les chiffres, `F1`–`F24`,
`Space`, `Tab`, `Enter`, `Esc`, `Backspace`, `Delete`, `Insert`, `Home`,
`End`, `PageUp`, `PageDown` et les flèches. Ce qui n'est pas reconnu est
refusé en nommant ce qui a été écrit.

### Les fichiers lâchés sur la fenêtre

Avec leur **vrai chemin**, ce qu'un `DataTransfer` de navigateur ne donne
pas.

```rust
use rustx::dropped;

effect((), || {
    dropped::on_files(move |chemins| {
        for chemin in chemins { ouvrir(&chemin); }
    })
});
```

Le moteur reprend aussi le lâcher à Chromium, qui sinon *navigue* vers le
fichier et remplace l'application par son contenu.

### Une seule instance, et le démarrage avec la session

```rust
use rustx::{autostart, instance};

// Dans le `main`, avant d'ouvrir la fenêtre.
let Ok(_seule) = instance::Only::one("mon-logiciel") else {
    // Déjà lancée : lui passer nos arguments, et s'en aller.
    instance::hand_over("mon-logiciel", &std::env::args().skip(1).collect::<Vec<_>>())?;
    return Ok(());
};

// Et dans la page, ce que la deuxième instance a transmis.
for argument in instance::handed_over() { ouvrir(&argument); }

autostart::set(true, "mon-logiciel")?;           // HKCU\…\Run, sans administrateur
let coche = autostart::is_on("mon-logiciel");
```

### Les appareils

```rust
use rustx::devices;

for appareil in devices::devices() {
    println!("{} — {} ({})", appareil.name, appareil.kind, appareil.busy);
}

let cameras = devices::devices_of(devices::Kind::Camera);
devices::explain();                              // ce que le système a répondu
```

La caméra, le micro et l'écran se **lisent** ensuite par Chromium —
`getUserMedia`, `getDisplayMedia` — avec les permissions du système :
c'est le même code que sur le web, et il n'y a pas de couche à apprendre
par-dessus. `rustx-os` sert à savoir ce qui est branché, et à le nommer.

### Les plateformes

Un répertoire par système à l'intérieur — `windows/`, `linux/`, le reste
— et une capacité est un fichier par système, avec les mêmes noms de
fonctions. Windows est écrit. Ailleurs, le refus nomme la plateforme
**et le fichier à écrire**, et les tests compilent les trois sur
n'importe quelle machine.

Ce qui demande un œil se regarde à la main :

```sh
cargo run -p rustx-os --example choose_a_file
cargo run -p rustx-os --example tray_icon
cargo run -p rustx-os --example a_global_shortcut
cargo run -p rustx-os --example what_is_plugged_in
```

## Les commandes

| | |
|---|---|
| `rustx new <nom>` | écrit un projet qui compile du premier coup |
| `rustx dev` | regarde les fichiers, reconstruit, ramène la fenêtre |
| `rustx build` | construit, et donne au `.exe` son nom et son icône |
| `rustx build --release` | la version à livrer |
| `rustx run` | construit et lance une fois, sans surveiller |
| `rustx --help` | les quatre lignes, et ce qu'un rechargement conserve |

`cargo build` marche toujours : le binaire produit fonctionne, il a
seulement l'air anonyme dans l'explorateur — `rustx build` n'ajoute que
l'icône et la version dans les propriétés du fichier.

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

Ce qui tourne pendant un `rustx dev` est une **copie** du binaire, dans
`target/rustx-dev/` : un `.exe` en cours d'exécution est tenu par
Windows, et le build suivant doit pouvoir réécrire celui que cargo a
produit — sinon chaque sauvegarde finit en accès refusé. La copie garde
le nom de l'application, et le dossier disparaît avec `target/`.

## L'éditeur

L'extension VS Code est dans `editors/vscode` :

```sh
rustup component add rust-analyzer
cargo install rustx-lsp
code --install-extension editors/vscode/rustx-0.1.0.vsix
```

Ce qu'elle donne :

- la coloration du `.rsx` ;
- les erreurs du compilateur soulignées **à leurs vraies coordonnées**
  dans le `.rsx`, et non dans le Rust généré ;
- la complétion des balises HTML et SVG, des attributs, des événements,
  des clés de `style={{ … }}`, des composants du fichier ;
- la complétion des **composants des bibliothèques installées** —
  `CardHeader`, `TabsTrigger`, `LineChart` sont lus dans `node_modules`
  et dans `components/ui`, avec le nom de la bibliothèque à côté ;
- la complétion de **ce que ces balises prennent** : sur `<Button `,
  `variant`, `size` et `asChild` avant les attributs HTML, lus dans le
  `.d.ts` du paquet ou dans le `components/ui/*.tsx` que `shadcn add` a
  écrit ;
- la **fermeture des balises** : taper le `>` de `<Card>` écrit
  `</Card>` derrière le curseur. `Vec<u32>`, `a < b >`, `=>` et une
  balise qui s'est fermée toute seule sont laissés tranquilles ;
  `rustx.closeTags` le coupe ;
- à l'intérieur d'un `{ … }`, c'est `rust-analyzer` qui répond, derrière
  le serveur : complétion, survol, aller-à-la-définition.

La première liste d'une session peut prendre quelques secondes, le temps
que `rust-analyzer` ait chargé le projet.

Pour savoir ce que l'éditeur offrira dans une application donnée :

```sh
cargo run -p rustx-lsp --example what_is_installed -- <le dossier de l'app>
```

## Quand ça ne marche pas

**« This page does not exist. »** — la route demandée n'est déclarée
nulle part. C'est l'écran par défaut de `not_found` : vérifier le chemin
passé à `navigate`, le nom du dossier dans `app/`, et le `.start(…)` de
`Config`.

**« the page asked for `Button`, and nothing has it »** — le nom n'est ni
une fonction de la page ni un export d'une bibliothèque installée. Le
message liste les bibliothèques trouvées. Souvent : un `npm install`
oublié, ou un composant de la page appelé comme une balise
(`<Carte />` au lieu de `{Carte()}`).

**« this application declares JavaScript libraries … and has no
node_modules »** — `npm install` dans le dossier de l'application.

**« … names `recharts` under `dependencies`, and `node_modules` has
nothing of that name »** — le `package.json` a grandi depuis le dernier
`npm install`. La même commande, dans le même dossier.

**« this application installed tailwindcss and nothing that can run it
»** — Tailwind 4 garde sa ligne de commande dans
`@tailwindcss/cli` : `npm install @tailwindcss/cli`.

**Une balise majuscule que la page ne trouve pas, alors que le fichier
existe dans `components/`** — un composant à soi est pris sous le nom de
son fichier quand il a un `export default`. `components/Editor.tsx`
donne `<Editor>` ; une fonction exportée par son nom donne ce nom.

**La fenêtre est blanche et le journal répète « Failed to create shared
context »** — la machine n'a pas de pilote 3D utilisable (machine
virtuelle, bureau distant). `RUSTX_SOFTWARE=1` compose sur le
processeur : plus lent, et ça marche toujours.

La même chose vaut pour la suite : la moitié des scénarios attendent une
frame, et une fenêtre qui n'est pas composée n'en produit aucune. Sur une
telle machine, `RUSTX_SOFTWARE=1 cargo test --workspace`. Les harnais le
disent eux-mêmes quand ils reconnaissent le journal.

**Un message entre la fenêtre et la page n'arrive pas** —
`RUSTX_TRACE=1` trace les deux bouts : ce qui part, ce qui arrive, et ce
que personne n'écoute.

**« a window can only be opened from a page »** — `window::open` a été
appelé depuis `main` ou depuis un fil ; il s'appelle depuis la page, qui
est le seul endroit qui a la frame pour porter la demande.

**Le compilateur parle d'un `onClick`, d'un `effect` ou d'un `task`** —
un état a été écrit pendant le rendu. L'écriture va dans l'un des trois.

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
Une application qui dépasse la façade peut appeler `rustx_engine`
directement : rien n'est caché, et chaque ligne que l'attribut ajoute est
du code qu'on aurait pu écrire à la main.

## Dans le dépôt

| | |
|---|---|
| `apps/` | les applications qui servent de preuve aux plans |
| `editors/vscode` | l'extension |
| `docs/superpowers/` | la spécification et les plans |
| `docs/dette-connue.md` | ce qu'on sait imparfait, et ce qui nous ferait changer d'avis |
| `docs/chasse-aux-bugs.md` | ce qu'un projet écrit du dehors a cassé, et ce qui l'a réparé |
| `docs/publier.md` | les huit paquets, et sous quels noms ils partent |

## Ce qui reste

- macOS et Linux : chaque module compile et nomme ce qui lui manque ;
- l'enregistrement et la capture au-delà de ce que Chromium donne déjà ;
- un site de documentation, quand il aura à dire autre chose que ceci.

## La licence

MIT ou Apache-2.0, au choix — `LICENSE-MIT` et `LICENSE-APACHE`.
