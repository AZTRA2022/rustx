// DOCS — the index. Four pages, each one a real route with the real
// text of that topic.

fn Page() -> Element {
    return <div key="docs" id="screen-docs" className="plain">
        <div className="label">DOCS</div>
        <h1 className="h1" style={{ marginTop: "8px" }}>Documentation</h1>
        <p className="lead" style={{ marginTop: "10px", maxWidth: "600px" }}>
            Written against what the application does, not against what the landing page says it
            does. Where the two differ, these pages are the ones that are right.
        </p>

        <div id="doc-index" className="grid2" style={{ marginTop: "24px" }}>
            {docs().iter().map(|doc| Card(doc.clone())).collect::<Vec<_>>()}
        </div>

        <div className="panel" style={{ marginTop: "22px" }}>
            <div className="label">THE WHOLE APPLICATION, BY ADDRESS</div>
            <div className="readout" style={{ marginTop: "10px", fontSize: "11px" }}>
                <div>/ — the landing page</div>
                <div>/product — what it does</div>
                <div>/security — what it refuses to claim</div>
                <div>/pricing — three plans</div>
                <div>/docs and /docs/&lt;topic&gt; — this, and one page each</div>
                <div>/how-it-works — the four steps, at length</div>
                <div>/contact — what a custody officer would ask for</div>
                <div>/vault and /vault/&lt;label&gt; — the vault, and one item's proof</div>
            </div>
        </div>
    </div>;
}

fn Card(doc: Doc) -> Element {
    let going = format!("/docs/{}", doc.slug);
    return <button
        key={doc.slug.clone()}
        className="panel"
        style={{ textAlign: "left", cursor: "pointer" }}
        onClick={move |_| rustx::router::navigate(going.clone())}
    >
        <span style={{ display: "block", fontSize: "14px" }}>{doc.title.clone()}</span>
        <span style={{ display: "block", fontSize: "12px", lineHeight: "1.65", color: "#3a3a3a", marginTop: "7px" }}>
            {doc.blurb.clone()}
        </span>
        <span style={{ display: "block", fontSize: "9px", letterSpacing: "0.12em", color: "#9a9a9a", marginTop: "10px" }}>
            {format!("{} SECTIONS", doc.body.len())}
        </span>
    </button>;
}
