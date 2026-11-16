// HELP CENTRE — the index. Every card goes to `/help/<topic>`, which
// is a real page with the real text of that topic.

fn Page() -> Element {
    return <div key="help" className="px-5 py-4">
        <h1 className="text-[13px] font-semibold text-foreground">Help Centre</h1>
        <p className="mt-0.5 max-w-[560px] text-[11px] leading-[1.7] text-muted">
            Four topics, written against what this application actually does. Where an action leaves
            the app and touches the machine, the page that explains it says which call it makes.
        </p>

        <div id="help-index" className="mt-4 grid grid-cols-2 gap-2.5">
            {help_topics().iter().map(|topic| Card(topic.clone())).collect::<Vec<_>>()}
        </div>

        <div className="mt-5 flex gap-2">
            <button
                className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint"
                onClick={move |_| rustx::router::navigate("/support".to_string())}
            >
                Support
            </button>
            <button
                className="rounded-md border border-border bg-[#11151b] px-3 py-[6px] text-[11px] text-foreground hover:border-faint"
                onClick={move |_| rustx::router::navigate("/feedback".to_string())}
            >
                Send feedback
            </button>
        </div>
    </div>;
}

fn Card(topic: HelpTopic) -> Element {
    let going = format!("/help/{}", topic.slug);
    return <button
        key={topic.slug.clone()}
        className="rounded-md border border-border bg-[#0f131a] p-3.5 text-left hover:border-faint"
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span className="flex items-center gap-1.5 text-[12px] font-semibold text-foreground">
            {topic.title.clone()}
            <ChevronRight size={11} color="#5a6572" />
        </span>
        <span className="mt-1.5 block text-[11px] leading-[1.6] text-muted">{topic.blurb.clone()}</span>
        <span className="mt-2 block text-[9px] tracking-[0.1em] text-faint">
            {format!("{} SECTION(S)", topic.body.len())}
        </span>
    </button>;
}
