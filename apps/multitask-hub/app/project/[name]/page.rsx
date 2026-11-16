// A second, real screen: `/project/<name>`, with the project's name
// captured from the URL by the `[name]` segment of
// `app/project/[name]/page.rsx` — the same mechanism `apps/counter`
// and `apps/capture` use. It does not reach into the board's live
// `Vec<Project>`, so a direct link to it always renders the same
// thing for the same name, rather than guessing at state that may
// have moved on.

fn Page(name: String) -> Element {
    return <div style={{ padding: "24px", maxWidth: "640px" }}>
        <button onClick={move |_| rustx::router::navigate("/".to_string())} style={{ background: "none", border: "none", color: "#9a9db3", cursor: "pointer", padding: "0", fontSize: "13px" }}>
            ← back to the board
        </button>

        <h1 style={{ margin: "16px 0 6px", fontSize: "22px" }}>{name.clone()}</h1>
        <p style={{ margin: "0", fontSize: "13px", color: "#9a9db3" }}>
            reached at <code>/project/{name.clone()}</code>, with the name captured by the
            <code> [name]</code> route segment rather than a query string.
        </p>
    </div>;
}
