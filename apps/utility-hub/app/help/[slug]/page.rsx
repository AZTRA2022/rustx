// HELP CENTRE / one topic.
//
// A slug nobody wrote is not an error screen: it is the index again
// with a line saying so, because a production office should never
// show a page a reader cannot get out of.

fn Page(slug: String) -> Element {
    let found = help_topic_named(&slug);

    return <div key={format!("help-{slug}")} className="px-5 py-4">
        <div className="flex items-center gap-1.5 text-[11px] text-muted">
            <button className="hover:text-foreground" onClick={move |_| rustx::router::navigate("/help".to_string())}>
                Help Centre
            </button>
            <span className="text-faint">→</span>
            <span className="text-foreground">
                {found.as_ref().map(|topic| topic.title.clone()).unwrap_or_else(|| "Not written".to_string())}
            </span>
        </div>

        {match found {
            Some(topic) => Article(topic),
            None => Missing(slug.clone()),
        }}
    </div>;
}

fn Article(topic: HelpTopic) -> Element {
    let others: Vec<HelpTopic> = help_topics().into_iter().filter(|other| other.slug != topic.slug).collect();

    return <div key={topic.slug.clone()} className="mt-3 grid grid-cols-[1fr_200px] gap-6">
        <article className="max-w-[560px]">
            <h1 className="text-[16px] font-semibold text-foreground">{topic.title.clone()}</h1>
            <p className="mt-1.5 text-[11px] leading-[1.7] text-muted">{topic.blurb.clone()}</p>
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
        <h2 className="text-[9px] tracking-[0.14em] text-faint">{heading}</h2>
        <p className="mt-1.5 text-[12px] leading-[1.75] text-foreground">{text}</p>
    </div>;
}

fn Link(slug: String, title: String) -> Element {
    let going = format!("/help/{slug}");
    return <button
        key={slug.clone()}
        className="rounded-md px-2 py-[6px] text-left text-[11px] text-muted hover:bg-[#141922] hover:text-foreground"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
    </button>;
}

fn Missing(slug: String) -> Element {
    return <div className="mt-4 max-w-[520px]">
        <h1 className="text-[15px] font-semibold text-foreground">Nothing is written under that name</h1>
        <p className="mt-1.5 text-[11px] leading-[1.7] text-muted">
            {format!("There is no help topic called “{slug}”. The four that exist are below.")}
        </p>
        <div className="mt-3 flex flex-col gap-1">
            {help_topics().iter().map(|topic| Link(topic.slug.clone(), topic.title.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}
