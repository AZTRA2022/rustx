// A second, real screen: `/table/<name>`. It does not reach into the
// browser's live in-memory tables — a dropped CSV only exists for the
// session that dropped it — so this page shows what a table page
// would always be able to say about any name: what it is, and a way
// back. That keeps a direct link to it honest rather than guessing at
// state that may not exist.

fn Page(name: String) -> Element {
    return <div className="mx-auto max-w-2xl px-6 py-10">
        <button className="text-sm text-muted-foreground hover:text-foreground" onClick={move |_| rustx::router::navigate("/".to_string())}>
            ← back to the tables
        </button>

        <div className="mt-4 flex items-center gap-3">
            <h1 className="text-2xl font-semibold tracking-tight">{name.clone()}</h1>
            <Badge>table</Badge>
        </div>

        <Card className="mt-6 border-border bg-card">
            <CardHeader>
                <CardTitle className="text-sm text-muted-foreground">about this route</CardTitle>
            </CardHeader>
            <CardContent className="text-sm leading-relaxed text-muted-foreground">
                This screen is reached at <code className="text-primary">/table/{name.clone()}</code>, with
                the table's name captured from the URL by the <code className="text-primary">[name]</code> segment
                of <code className="text-primary">app/table/[name]/page.rsx</code> — the same mechanism
                <code className="text-primary"> apps/counter</code> and <code className="text-primary">apps/capture</code>
                use for their own routes.
            </CardContent>
        </Card>
    </div>;
}
