// The landing page of the hybrid application.
//
// Everything on screen is written here: the layout and the look in
// Tailwind classes, the dialog in Radix, the chart in recharts, and the
// pieces that repeat as components of this page — which is what
// shadcn/ui is, with Rust where the TSX would be.

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
                {Stat("one".to_string(), "processus".to_string(), "Rust et React dans le même, sans IPC.".to_string())}
                {Stat("60".to_string(), "images/seconde".to_string(), "Une seule boucle de rendu pour les deux arbres.".to_string())}
                {Stat("0".to_string(), "ligne de JavaScript".to_string(), "Le framework écrit le pont et le bundle.".to_string())}
            </section>

            <section className="mt-16 grid gap-6 lg:grid-cols-5">
                <Card>
                    <div className="lg:col-span-3">
                        <h2 className="text-sm font-semibold uppercase tracking-wider text-zinc-400">
                            recharts, depuis la page
                        </h2>
                        <p className="mt-2 text-sm text-zinc-400">
                            Les points sont un <code className="text-violet-300">let mut</code> de ce
                            fichier. La courbe est une bibliothèque npm.
                        </p>

                        <div id="chart" className="mt-6">
                            <LineChart width={520} height={180} data={data}>
                                <XAxis dataKey="at" stroke="#52525b" />
                                <YAxis stroke="#52525b" />
                                <Line type="monotone" dataKey="value" stroke="#a78bfa" strokeWidth={2} isAnimationActive={false} />
                            </LineChart>
                        </div>

                        <div className="mt-6 flex items-center gap-3">
                            <button id="add" className="inline-flex items-center justify-center gap-2 rounded-md bg-violet-500 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-violet-400" onClick={move |_| points += 1}>
                                ajouter un point
                            </button>
                            <span className="text-sm text-zinc-500">{points} déjà posés</span>
                        </div>
                    </div>
                </Card>

                <Card>
                    <div className="lg:col-span-2">
                        <h2 className="text-sm font-semibold uppercase tracking-wider text-zinc-400">
                            React tient ce panneau
                        </h2>
                        <p className="mt-2 text-sm text-zinc-400">
                            Le cadre est à React, et le compteur dedans est à rustx — avec ses
                            propres cellules, relâché avec le nœud.
                        </p>
                        <div id="panel-host" className="mt-6 min-h-24 rounded-lg border border-dashed border-zinc-800 p-4"></div>
                    </div>
                </Card>
            </section>

            <section className="mt-16">
                <h2 className="text-2xl font-semibold tracking-tight">Ce que ça change</h2>
                <div className="mt-6 grid gap-6 md:grid-cols-3">
                    {Feature(
                        "Pas de pont sérialisé".to_string(),
                        "Les props traversent comme valeurs V8, dans le processus de rendu. Rien n'est encodé en texte.".to_string(),
                    )}
                    {Feature(
                        "Deux propriétaires, un DOM".to_string(),
                        "rustx ne regarde pas dans un nœud de React, React ne touche pas à un nœud de rustx.".to_string(),
                    )}
                    {Feature(
                        "L'écosystème entier".to_string(),
                        "Radix, recharts, Tailwind : installés avec npm, écrits comme des balises.".to_string(),
                    )}
                </div>
            </section>
        </main>

        {Footer()}
    </div>;
}

/// The bar above the page.
fn Header() -> Element {
    return <header className="sticky top-0 z-10 border-b border-zinc-800/80 bg-zinc-950/80 backdrop-blur">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-4">
            <div className="flex items-center gap-3">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-500 text-sm font-black text-white">
                    R
                </div>
                <span id="title" className="text-lg font-semibold tracking-tight">
                    rustx
                </span>
                {Badge("v0.1".to_string())}
            </div>

            <nav className="hidden items-center gap-6 text-sm text-zinc-400 md:flex">
                <span className="hover:text-zinc-100">Le graphique</span>
                <span className="hover:text-zinc-100">Le panneau</span>
                <span className="hover:text-zinc-100">Ce que ça change</span>
            </nav>
        </div>
    </header>;
}

/// The first screenful.
fn Hero() -> Element {
    return <section className="pt-20">
        {Badge("Rust + React, un seul processus".to_string())}

        <h1 className="mt-6 max-w-3xl text-5xl font-semibold leading-tight tracking-tight">
            Du logiciel natif, écrit comme une page —
            <span className="text-violet-400"> et rapide comme du Rust</span>
        </h1>

        <p className="mt-6 max-w-2xl text-lg text-zinc-400">
            Une page est un <code className="rounded bg-zinc-900 px-1.5 py-0.5 text-violet-300">.rsx</code> :
            du Rust, du JSX, et les bibliothèques de npm comme des balises. Pas de pont,
            pas de sérialisation, pas de second runtime.
        </p>

        <div className="mt-10 flex flex-wrap items-center gap-4">
            {About()}
            <span className="rounded-md border border-zinc-700 px-5 py-2.5 text-sm font-medium text-zinc-200">
                cargo add rustx
            </span>
        </div>
    </section>;
}

/// The dialog of the hero, which is Radix's and nobody else's.
///
/// One tree: the trigger and what it opens are under the same `Root`,
/// because that is where Radix keeps whether it is open. The content
/// goes in a portal — somewhere else in the document — and rustx does
/// not go looking for it.
fn About() -> Element {
    return <Root>
        <Trigger id="open-dialog" className="inline-flex items-center justify-center gap-2 rounded-md bg-violet-500 px-5 py-2.5 text-sm font-medium text-white transition-colors hover:bg-violet-400">
            Voir ce que fait cette page
        </Trigger>
        <Portal>
            <Overlay className="fixed inset-0 bg-black/70 backdrop-blur-sm" />
            <Content id="about" className="fixed left-1/2 top-1/2 w-[28rem] -translate-x-1/2 -translate-y-1/2 rounded-xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl">
                <Title className="text-lg font-semibold tracking-tight">
                    One DOM, two owners
                </Title>
                <Description className="mt-2 text-sm leading-relaxed text-zinc-400">
                    Ce dialogue est @radix-ui/react-dialog, écrit dans le .rsx et stylé par
                    Tailwind. Radix le met dans un portail — ailleurs dans le document — et
                    rustx ne le cherche pas : ce qui est à React est à React.
                </Description>
                <div className="mt-6 flex justify-end">
                    <Close id="close-dialog" className="inline-flex items-center justify-center rounded-md border border-zinc-700 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-800">
                        fermer
                    </Close>
                </div>
            </Content>
        </Portal>
    </Root>;
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

/// The pieces that repeat. This is what shadcn/ui is: a component, and
/// the classes it carries.
fn Card(children: Element) -> Element {
    return <div className="rounded-xl border border-zinc-800 bg-zinc-900/40 p-6 shadow-lg shadow-black/20">
        {children}
    </div>;
}

fn Badge(label: String) -> Element {
    return <span className="inline-flex items-center rounded-full border border-violet-500/30 bg-violet-500/10 px-2.5 py-0.5 text-xs font-medium text-violet-300">
        {label}
    </span>;
}

fn Stat(number: String, unit: String,note: String) -> Element {
    return <div className="rounded-xl border border-zinc-800 bg-zinc-900/40 p-6">
        <div className="flex items-baseline gap-2">
            <span className="text-3xl font-semibold tracking-tight text-violet-300">{number}</span>
            <span className="text-sm text-zinc-400">{unit}</span>
        </div>
        <p className="mt-3 text-sm leading-relaxed text-zinc-400">{note}</p>
    </div>;
}

fn Feature(title: String, body: String) -> Element {
    return <div className="rounded-xl border border-zinc-800 bg-zinc-900/40 p-6">
        <h3 className="font-medium tracking-tight">{title}</h3>
        <p className="mt-2 text-sm leading-relaxed text-zinc-400">{body}</p>
    </div>;
}

fn Counter() -> Element {
    return <span id="counter-in-panel" className="font-mono text-violet-300">{points}</span>;
}
