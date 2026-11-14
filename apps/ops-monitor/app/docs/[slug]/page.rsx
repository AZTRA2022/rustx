// DOCS / one topic — the page behind every card of the index, and
// behind the `[slug]` of the address bar.
//
// A slug nobody wrote is not an error page: it is the index again,
// with a line saying so, because a console should never show a screen
// a reader cannot get out of.

fn Page(slug: String) -> Element {
    let found = topic_named(&slug);

    return <div key={format!("doc-{slug}")} id="screen-doc" className="pt-5">
        <div className="flex items-center gap-2 py-1 text-[11px] text-muted">
            <FileText size={11} color="#6a6a66" />
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/docs".to_string())}>
                Docs
            </button>
            <span className="text-faint">/</span>
            <span className="text-foreground">{found.as_ref().map(|topic| topic.title.clone()).unwrap_or_else(|| "Not written".to_string())}</span>
        </div>

        {match found {
            Some(topic) => Article(topic),
            None => Missing(slug.clone()),
        }}
    </div>;
}

fn Article(topic: Topic) -> Element {
    let others: Vec<Topic> = topics().into_iter().filter(|other| other.slug != topic.slug).collect();

    return <div key={topic.slug.clone()} className="mt-3 grid grid-cols-[1fr_230px] gap-7">
        <article className="max-w-[640px]">
            <h1 className="text-[17px] font-semibold tracking-tight">{topic.title.clone()}</h1>
            <p className="mt-1.5 text-[12px] leading-[1.7] text-muted">{topic.blurb.clone()}</p>
            {topic.body.iter().map(|part| Section(part.0.clone(), part.1.clone())).collect::<Vec<_>>()}
        </article>

        <aside>
            <div className="text-[9px] tracking-[0.12em] text-faint">OTHER TOPICS</div>
            <div className="mt-2.5 flex flex-col gap-1">
                {others.iter().map(|other| Link(other.slug.clone(), other.title.clone())).collect::<Vec<_>>()}
            </div>
        </aside>
    </div>;
}

fn Section(heading: String, text: String) -> Element {
    return <div key={heading.clone()} className="mt-5">
        <h2 className="text-[10px] tracking-[0.14em] text-muted">{heading}</h2>
        <p className="mt-2 text-[12px] leading-[1.75]">{text}</p>
    </div>;
}

fn Link(slug: String, title: String) -> Element {
    let going = format!("/docs/{slug}");
    return <button
        key={slug.clone()}
        className="rounded-[5px] px-2 py-[6px] text-left text-[11px] text-muted hover:bg-panel2 hover:text-foreground"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
    </button>;
}

fn Missing(slug: String) -> Element {
    return <div className="mt-3 max-w-[560px]">
        <h1 className="text-[17px] font-semibold tracking-tight">Nothing is written under that name</h1>
        <p className="mt-2 text-[12px] leading-[1.7] text-muted">
            {format!("There is no topic called “{slug}”. The six that exist are below.")}
        </p>
        <div className="mt-4 flex flex-col gap-1">
            {topics().iter().map(|topic| Link(topic.slug.clone(), topic.title.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}
