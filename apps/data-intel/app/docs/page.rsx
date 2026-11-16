// DOCS — the index. Four pages, each one a real route.

fn Page() -> Element {
    return <div key="docs" id="screen-docs" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="text-[10px] tracking-[0.16em] text-faint">DOCS</div>
        <h1 className="mt-2 text-[22px] font-semibold tracking-tight">Documentation</h1>
        <p className="mt-2.5 max-w-[600px] text-[12px] leading-[1.8] text-muted-foreground">
            Written against what this application does, not against what the landing page says it
            does. Where the two differ, these pages are the ones that are right.
        </p>

        <div id="doc-index" className="mt-7 grid grid-cols-2 gap-4">
            {docs().iter().map(|doc| Card(doc.clone())).collect::<Vec<_>>()}
        </div>

        <div className="mt-6 rounded-[6px] border border-border bg-card p-5">
            <div className="text-[10px] tracking-[0.16em] text-faint">EVERY SCREEN, BY ADDRESS</div>
            <div className="mt-3 flex flex-col gap-1 text-[11px] leading-[1.9] text-muted-foreground">
                <span>/ — the landing page</span>
                <span>/architecture — four layers, one direction</span>
                <span>/platform — what you get, and what it is</span>
                <span>/solutions — four rooms with the same problem</span>
                <span>/developers — the parts you would have to write</span>
                <span>/docs and /docs/&lt;topic&gt; — this, and one page each</span>
                <span>/company — who is behind it</span>
                <span>/contact — the brief, on your clipboard</span>
                <span>/console and /console/&lt;table&gt; — the product</span>
            </div>
        </div>
    </div>;
}

fn Card(doc: Doc) -> Element {
    let going = format!("/docs/{}", doc.slug);
    return <button
        key={doc.slug.clone()}
        className="rounded-[6px] border border-border bg-card p-5 text-left hover:border-[#3a3a3a]"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="block text-[14px] font-semibold">{doc.title.clone()}</span>
        <span className="mt-2 block text-[11px] leading-[1.7] text-muted-foreground">{doc.blurb.clone()}</span>
        <span className="mt-3 block text-[9px] tracking-[0.14em] text-warm">
            {format!("{} SECTIONS →", doc.body.len())}
        </span>
    </button>;
}
