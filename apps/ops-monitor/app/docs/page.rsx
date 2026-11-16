// DOCS — the index. Every card here goes to `/docs/<slug>`, which is
// a real page with the real text of that topic, not a placeholder.

fn Page() -> Element {
    return <div key="docs" id="screen-docs" className="pt-5">
        <div className="flex items-center gap-2 py-1 text-[11px] text-muted">
            <FileText size={11} color="#6a6a66" />
            <span className="text-foreground">Docs</span>
        </div>

        <h1 className="mt-3 text-[15px] font-semibold tracking-tight">How this console works</h1>
        <p className="mt-1 max-w-[620px] text-[12px] leading-[1.7] text-muted">
            Six topics, written against what the application actually does. Where a number comes
            from the machine, the page that explains it says which call read it.
        </p>

        <div id="doc-index" className="mt-5 grid grid-cols-2 gap-3">
            {topics().iter().map(|topic| Card(topic.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}

fn Card(topic: Topic) -> Element {
    let going = format!("/docs/{}", topic.slug);
    return <button
        key={topic.slug.clone()}
        className="rounded-[7px] border border-line bg-panel p-4 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="flex items-center gap-1.5 text-[12px] font-semibold">
            {topic.title.clone()}
            <ChevronRight size={11} color="#6a6a66" />
        </span>
        <span className="mt-1.5 block text-[11px] leading-[1.6] text-muted">{topic.blurb.clone()}</span>
        <span className="mt-2.5 block text-[9px] tracking-[0.1em] text-faint">
            {format!("{} SECTION(S)", topic.body.len())}
        </span>
    </button>;
}
