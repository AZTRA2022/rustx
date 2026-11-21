// The default licence screen. Write system/license.rsx in your project to
// replace it. Open it with rustx::setup::open("license").

fn Page() -> Element {
    let license = rustx::setup::info().license;
    let text = if license.is_empty() { "This application ships no licence file.".to_string() } else { license };
    let button = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:14px;";
    return <div style={{ display: "flex", flexDirection: "column", position: "fixed", top: "0", left: "0", right: "0", bottom: "0", zIndex: "2147483000", fontFamily: "inherit", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        <main style={{ flex: "1", padding: "32px 36px 0", overflow: "auto" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 12px" }}>Licence</h1>
            <pre style={{ whiteSpace: "pre-wrap", fontSize: "12px" }}>{text}</pre>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::close(); }}>Close</button>
        </footer>
    </div>;
}
