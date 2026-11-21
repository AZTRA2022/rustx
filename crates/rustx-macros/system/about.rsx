// The default About screen: what the application says it is. Write
// system/about.rsx in your project to replace it. Open it from Rust with
// rustx::setup::open("about").

fn Page() -> Element {
    let about = rustx::window::about();
    let license = rustx::setup::info().license;
    let button = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:14px;";
    return <div style={{ display: "flex", flexDirection: "column", position: "fixed", top: "0", left: "0", right: "0", bottom: "0", zIndex: "2147483000", fontFamily: "inherit", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        <main style={{ flex: "1", padding: "32px 36px 0", overflow: "auto" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>{about.name.clone()}</h1>
            <p style={{ color: "var(--rx-muted)", margin: "0 0 20px" }}>{format!("Version {}", about.version)}</p>
            <p>{about.description.clone()}</p>
            <p style={{ color: "var(--rx-muted)", fontSize: "13px" }}>{about.publisher.clone()}</p>
            <pre style={{ maxHeight: "200px", overflow: "auto", whiteSpace: "pre-wrap", fontSize: "12px", color: "var(--rx-muted)" }}>{license}</pre>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
