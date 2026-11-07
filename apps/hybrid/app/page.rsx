// The landing page of the hybrid application.
//
// Everything on screen is written here, in Rust. What it is made of is
// not: `Button`, `Card`, `Dialog`, `Badge`, `Tabs` and `Separator` were
// installed with `npx shadcn add`, so they are TypeScript in
// `components/ui/` — the application's own components, which the
// framework puts in the bundle and this page writes as tags. The chart
// is recharts, from npm. The state is a `let mut` at the top of this
// file.

let mut points = 0;


fn Page() -> Element {
    // The application's own data, for a component it did not write.
    let data: Vec<Point> = (0..=points)
        .map(|at| Point { at: at as f64, value: (at * at) as f64 })
        .collect();

    return <div className="min-h-screen bg-zinc-950 text-zinc-100 antialiased">
        {Header()}

        <main className="mx-auto max-w-5xl px-6 pb-24">
            {Hero()}

            <section className="mt-16 grid gap-6 md:grid-cols-3">
                {Stat("un".to_string(), "processus".to_string(), "Rust et React dans le même, sans IPC.".to_string())}
                {Stat("60".to_string(), "images/seconde".to_string(), "Une seule boucle de rendu pour les deux arbres.".to_string())}
                {Stat("0".to_string(), "ligne de JavaScript".to_string(), "Le framework écrit le pont et le bundle.".to_string())}
            </section>

            <section className="mt-16 grid gap-6 lg:grid-cols-5">
                <Card className="lg:col-span-3 border-zinc-800 bg-zinc-900/40">
                    <CardHeader>
                        <CardTitle className="text-sm font-semibold uppercase tracking-wider text-zinc-400">
                            recharts, depuis la page
                        </CardTitle>
                        <CardDescription className="text-zinc-400">
                            Les points sont un <code className="text-violet-300">let mut</code> de ce
                            fichier. La courbe est une bibliothèque npm.
                        </CardDescription>
                    </CardHeader>
                    <CardContent>
                        <div id="chart">
                            <recharts::LineChart width={520} height={180} data={data}>
                                <recharts::XAxis dataKey="at" stroke="#52525b" />
                                <recharts::YAxis stroke="#52525b" />
                                <recharts::Line type="monotone" dataKey="value" stroke="#a78bfa" strokeWidth={2} isAnimationActive={false} />
                            </recharts::LineChart>
                        </div>

                        <div className="mt-6 flex items-center gap-3">
                            <Button id="add" onClick={move |_| points += 1}>
                                ajouter un point
                            </Button>
                            <span className="text-sm text-zinc-500">{points} déjà posés</span>
                        </div>
                    </CardContent>
                </Card>

                <div className="lg:col-span-2 rounded-xl border border-zinc-800 bg-zinc-900/40 p-6 shadow">
                    <h2 className="text-sm font-semibold uppercase tracking-wider text-zinc-400">
                        React tient ce panneau
                    </h2>
                    <p className="mt-1.5 text-sm text-zinc-400">
                        Cette carte-ci est à rustx — des balises de ce fichier, pas un
                        composant installé. Ce qui est écrit dans un composant React est
                        rendu par React, et ce panneau doit rester un nœud que rustx
                        possède : le compteur dedans est à rustx, avec ses propres
                        cellules, relâché avec le nœud.
                    </p>
                    <div id="panel-host" className="mt-6 min-h-24 rounded-lg border border-dashed border-zinc-800 p-4"></div>
                </div>
            </section>

            <section className="mt-16">
                <h2 className="text-2xl font-semibold tracking-tight">Ce que ça change</h2>

                <Tabs defaultValue="pont" className="mt-6">
                    <TabsList className="bg-zinc-900">
                        <TabsTrigger value="pont">Le pont</TabsTrigger>
                        <TabsTrigger value="dom">Le DOM</TabsTrigger>
                        <TabsTrigger value="npm">L'écosystème</TabsTrigger>
                    </TabsList>
                    <TabsContent value="pont" className="mt-4 text-sm leading-relaxed text-zinc-400">
                        Les props traversent comme valeurs V8, dans le processus de rendu.
                        Rien n'est encodé en texte.
                    </TabsContent>
                    <TabsContent value="dom" className="mt-4 text-sm leading-relaxed text-zinc-400">
                        rustx ne regarde pas dans un nœud de React, React ne touche pas à un
                        nœud de rustx.
                    </TabsContent>
                    <TabsContent value="npm" className="mt-4 text-sm leading-relaxed text-zinc-400">
                        shadcn, Radix, recharts, Tailwind : installés avec npm, écrits comme
                        des balises.
                    </TabsContent>

                </Tabs>

                <div  className="mt-6 grid gap-6 md:grid-cols-3">
                    {Feature(
                        "Pas de pont sérialisé".to_string(),
                        "Les props traversent comme valeurs V8, dans le processus de rendu.".to_string(),
                    )}
                    {Feature(
                        "Deux propriétaires, un DOM".to_string(),
                        "Ce qui est à React est à React, et rustx n'y touche pas.".to_string(),
                    )}
                    {Feature(
                        "L'écosystème entier".to_string(),
                        "`shadcn add` pose du TypeScript ; la page l'écrit comme une balise.".to_string(),
                    )}
                </div>
            </section>
        </main>

        {Footer()}
    </div>;
}

/// The bar above the page.
fn Header() -> Element {
    return <header className="sticky top-0 z-40 border-b border-zinc-800/80 bg-zinc-950/80 backdrop-blur">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-4">
            <div className="flex items-center gap-3">
                <div className="grid h-8 w-8 place-items-center rounded-md bg-violet-500 font-black text-white">
                    R
                </div>
                <span className="text-lg font-semibold tracking-tight">rustx</span>
                <Badge className="bg-violet-500/10 text-violet-300 hover:bg-violet-500/10">
                    hybride
                </Badge>
            </div>

            <div className="flex items-center gap-4 text-sm text-zinc-400">
                <span>shadcn/ui</span>
                <Separator orientation="vertical" className="h-4 bg-zinc-800" />
                <span>Radix</span>
                <Separator orientation="vertical" className="h-4 bg-zinc-800" />
                <span>recharts</span>
            </div>
        </div>
    </header>;
}

/// The first screenful.
fn Hero() -> Element {
    return <section className="pt-16">
        <Badge className="bg-violet-500/10 text-violet-300 hover:bg-violet-500/10">
            un seul processus
        </Badge>

        <h1 className="mt-6 text-5xl font-semibold tracking-tight">
            Une page, deux propriétaires.
        </h1>
        <p className="mt-4 max-w-2xl text-lg leading-relaxed text-zinc-400">
            Les composants de cette page viennent de shadcn/ui — du TypeScript posé
            dans le projet par <code className="text-violet-300">shadcn add</code> — et
            tout ce qui les assemble est écrit en Rust, dans ce fichier.
        </p>

        <div className="mt-8 flex flex-wrap items-center gap-3">
            {About()}
            <Button variant="outline" className="border-zinc-700" id="to-chart">
                voir la courbe
            </Button>
        </div>
    </section>;
}

/// The dialog, which is shadcn's — Radix underneath, Tailwind on top,
/// and a trigger that is this page's own `Button` handed to it with
/// `asChild`.
fn About() -> Element {
    return <Dialog>
        <DialogTrigger asChild={true}>
            <Button id="open-dialog" className="px-5">
                Voir ce que fait cette page
            </Button>
        </DialogTrigger>
        <DialogContent className="border-zinc-800 p-6 sm:max-w-md">
            <DialogHeader>
                <DialogTitle>One DOM, two owners</DialogTitle>
                <DialogDescription className="text-zinc-400">
                    Ce dialogue est un composant shadcn/ui installé dans ce projet :
                    Radix pour le comportement, Tailwind pour le reste. Radix le met
                    dans un portail — ailleurs dans le document — et rustx ne le
                    cherche pas : ce qui est à React est à React.
                </DialogDescription>
            </DialogHeader>
            <DialogFooter>
                <DialogClose id="close-dialog" className="inline-flex items-center justify-center rounded-md border border-zinc-700 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-800">
                    fermer
                </DialogClose>
            </DialogFooter>
        </DialogContent>
    </Dialog>;
}

/// The line under everything.
fn Footer() -> Element {
    return <footer className="border-t border-zinc-800/80">
        <div className="mx-auto flex max-w-5xl flex-wrap items-center justify-between gap-4 px-6 py-8 text-sm text-zinc-500">
            <span>Écrit en .rsx — aucune ligne de JavaScript dans cette application.</span>
            <button id="leave" className="rounded-md px-3 py-1.5 text-zinc-400 underline-offset-4 hover:text-zinc-100 hover:underline" onClick={move |_| rustx::router::navigate("/merci".to_string())}>
                quitter la page
            </button>
        </div>
    </footer>;
}

/// One number, in a card the application installed rather than wrote.
fn Stat(number: String, unit: String, note: String) -> Element {
    return <Card className="border-zinc-800 bg-zinc-900/40">
        <CardHeader>
            <CardTitle className="text-3xl font-semibold tracking-tight text-violet-300">
                {number}
            </CardTitle>
            <CardDescription className="uppercase tracking-wider text-zinc-500">
                {unit}
            </CardDescription>
        </CardHeader>
        <CardContent className="text-sm leading-relaxed text-zinc-400">
            {note}
        </CardContent>
    </Card>;
}

/// One of the three things this architecture changes.
fn Feature(title: String, body: String) -> Element {
    return <Card className="mt-4 border-zinc-800 bg-zinc-900/40">
        <CardHeader>
            <CardTitle className="text-base font-semibold">{title}</CardTitle>
        </CardHeader>
        <CardContent className="text-sm leading-relaxed text-zinc-400">
            {body}
        </CardContent>
    </Card>;
}

/// A rustx component, mounted inside the panel React holds.
fn Counter() -> Element {
    return <span id="counter-in-panel" className="font-mono text-violet-300">{points}</span>;
}
