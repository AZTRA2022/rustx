// Where the footer's button goes.
//
// A real screen, and not a path nobody declared: leaving a page is what
// releases what it held — its cells, its tasks, the node React owned —
// and that is true of any screen. Navigating to a path that does not
// exist proved the same thing while showing "This page does not exist."
// to anyone who opened the application.

fn Page() -> Element {
    return <div className="flex min-h-screen items-center justify-center bg-zinc-950 text-zinc-100">
        <div className="max-w-md px-6 text-center">
            <h1 className="text-3xl font-semibold tracking-tight">Merci d'être passé</h1>
            <p className="mt-4 text-sm leading-relaxed text-zinc-400">
                La page d'accueil est partie, et avec elle son graphique, son panneau et le
                nœud que React tenait — personne n'a écrit de nettoyage.
            </p>
            <button
                id="back"
                className="mt-8 inline-flex items-center justify-center rounded-md border border-zinc-700 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-800"
                onClick={move |_| rustx::router::navigate("/".to_string())}
            >
                revenir
            </button>
        </div>
    </div>;
}
