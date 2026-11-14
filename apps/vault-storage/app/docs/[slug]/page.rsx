// DOCS / one topic. A slug nobody wrote shows the index again with a
// line saying so, rather than an error screen.

fn Page(slug: String) -> Element {
    let found = doc_named(&slug);

    return <div key={format!("doc-{slug}")} id="screen-doc" className="plain">
        <div className="row" style={{ fontSize: "11px", color: "#767676" }}>
            <button className="nav-item" onClick={move |_| rustx::router::navigate("/docs".to_string())}>Docs</button>
            <span style={{ color: "#c4c4c4" }}>/</span>
            <span style={{ color: "#0a0a0a" }}>
                {found.as_ref().map(|doc| doc.title.clone()).unwrap_or_else(|| "Not written".to_string())}
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

    return <div key={doc.slug.clone()} style={{ display: "grid", gridTemplateColumns: "1fr 220px", gap: "34px", marginTop: "18px" }}>
        <article style={{ maxWidth: "620px" }}>
            <h1 className="h1">{doc.title.clone()}</h1>
            <p className="lead" style={{ marginTop: "8px" }}>{doc.blurb.clone()}</p>
            {doc.body.iter().map(|part| Section(part.0.clone(), part.1.clone())).collect::<Vec<_>>()}
        </article>
        <aside>
            <div className="label">OTHER PAGES</div>
            <div style={{ marginTop: "10px", display: "flex", flexDirection: "column", gap: "4px" }}>
                {others.iter().map(|other| Link(other.slug.clone(), other.title.clone())).collect::<Vec<_>>()}
            </div>
            <button className="cta small" style={{ marginTop: "16px", width: "100%" }} onClick={move |_| rustx::router::navigate("/vault".to_string())}>
                OPEN THE VAULT
            </button>
        </aside>
    </div>;
}

fn Section(heading: String, text: String) -> Element {
    return <div key={heading.clone()} style={{ marginTop: "22px" }}>
        <div className="label">{heading}</div>
        <p style={{ fontSize: "13px", lineHeight: "1.8", margin: "7px 0 0" }}>{text}</p>
    </div>;
}

fn Link(slug: String, title: String) -> Element {
    let going = format!("/docs/{slug}");
    return <button
        key={slug.clone()}
        style={{ background: "none", border: "none", padding: "5px 0", textAlign: "left", fontSize: "12px", color: "#767676" }}
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        {title}
    </button>;
}

fn Missing(slug: String) -> Element {
    return <div style={{ marginTop: "18px", maxWidth: "560px" }}>
        <h1 className="h1">Nothing is written under that name</h1>
        <p className="lead" style={{ marginTop: "8px" }}>
            {format!("There is no documentation page called “{slug}”. The four that exist are below.")}
        </p>
        <div style={{ marginTop: "14px", display: "flex", flexDirection: "column", gap: "4px" }}>
            {docs().iter().map(|doc| Link(doc.slug.clone(), doc.title.clone())).collect::<Vec<_>>()}
        </div>
    </div>;
}
