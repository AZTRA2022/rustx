// The default uninstall page. Write system/uninstall.rsx in your project to
// replace it.

let mut removed: bool = false;
let mut failed: String = String::new();

fn Page() -> Element {
    let info = rustx::setup::info();
    let button = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);cursor:pointer;font-size:14px;";
    let danger = "padding:8px 18px;border-radius:8px;border:1px solid var(--rx-danger);background:var(--rx-danger);color:#fff;cursor:pointer;font-size:14px;";
    let title = if removed { "Removed".to_string() } else { format!("Uninstall {}?", info.name) };
    let line = if !failed.is_empty() {
        failed.clone()
    } else if removed {
        format!("{} is removed from this computer.", info.name)
    } else {
        "The application and its shortcut are removed. Your own documents are not touched.".to_string()
    };

    return <div style={{ display: "flex", flexDirection: "column", position: "fixed", top: "0", left: "0", right: "0", bottom: "0", zIndex: "2147483000", fontFamily: "inherit", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        <main style={{ flex: "1", padding: "32px 36px 0" }}>
            <h1 style={{ fontSize: "22px", margin: "0 0 4px" }}>{title}</h1>
            <p style={{ color: "var(--rx-muted)", margin: "0 0 20px" }}>{line}</p>
            <div style={{ padding: "8px 12px", border: "1px solid var(--rx-border)", borderRadius: "8px", wordBreak: "break-all" }}>{info.destination.clone()}</div>
        </main>
        <footer style={{ display: "flex", justifyContent: "flex-end", gap: "8px", padding: "16px 36px 24px" }}>
            <button style={button} onClick={move |_| { rustx::setup::quit(); }}>{if removed { "Close" } else { "Cancel" }}</button>
            <button style={danger} disabled={removed} onClick={move |_| {
                task(async move {
                    match rustx::setup::uninstall() {
                        Ok(()) => removed = true,
                        Err(why) => failed = why,
                    }
                });
            }}>Uninstall</button>
        </footer>
    </div>;
}
