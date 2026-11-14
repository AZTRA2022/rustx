// DOCS / one topic. A slug nobody wrote shows the index again with a
// line saying so, rather than an error screen.

fn Page(slug: String) -> Element {
    let found = doc_named(&slug);

    return <div key={format!("doc-{slug}")} id="screen-doc" className="mx-auto max-w-[1000px] px-6 py-10">
        <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/docs".to_string())}>
                Docs
            </button>
            <span className="text-faint">/</span>
            <span className="text-foreground">
                {found.as_ref().map(|doc| doc.title.clone()).unwrap_or_else(|| "not written".to_string())}
            </span>
        </div>

        {match found {
            Some(doc) => Article(doc),
            None => Missing(slug.clone()),
        }}
    </div>;
}

fn Article(doc: Doc) -> Element {
    let others: Vec<Doc> = docs().into_iter().filter(|other| other.slug != doc.slug).collect();

    return <div key={doc.slug.clone()} className="mt-4 grid grid-cols-[1fr_220px] gap-8">
        <article className="max-w-[640px]">
            <h1 className="text-[22px] font-semibold tracking-tight">{doc.title.clone()}</h1>
            <p className="mt-2 text-[12px] leading-[1.8] text-muted-foreground">{doc.blurb.clone()}</p>
            {doc.body.iter().map(|part| Section(part.0.clone(), part.1.clone())).collect::<Vec<_>>()}
        </article>
        <aside>
            <div className="text-[9px] tracking-[0.14em] text-faint">OTHER PAGES</div>
            <div className="mt-2.5 flex flex-col gap-1">
                {others.iter().map(|other| Link(other.slug.clone(), other.title.clone())).collect::<Vec<_>>()}
            </div>
            <button className="slab mt-5 w-full rounded-[5px] px-3 py-[7px] text-[10px]" onClick={move |_| rustx::router::navigate("/console".to_string())}>
                OPEN THE CONSOLE
            </button>
        </aside>
    </div>;
}

fn Section(heading: String, text: String) -> Element {
    return <div key={heading.clone()} className="mt-6">
        <div className="text-[9px] tracking-[0.16em] text-faint">{heading}</div>
        <p className="mt-2 text-[13px] leading-[1.85]">{text}</p>
    </div>;
}

fn Link(slug: String, title: String) -> Element {
    let going = format!("/docs/{slug}");
    return <button
        key={slug.clone()}
        className="rounded-[4px] px-2 py-[6px] text-left text-[11px] text-muted-foreground hover:bg-[#141414] hover:text-foreground"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
    </button>;
}

fn Missing(slug: String) -> Element {
    return <div className="mt-5 max-w-[560px]">
        <h1 className="text-[22px] font-semibold tracking-tight">Nothing is written under that name</h1>
        <p className="mt-2 text-[12px] leading-[1.8] text-muted-foreground">
            {format!("There is no documentation page called “{slug}”. The four that exist are below.")}
        </p>
        <div className="mt-4 flex flex-col gap-1">
            {docs().iter().map(|doc| Link(doc.slug.clone(), doc.title.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}
