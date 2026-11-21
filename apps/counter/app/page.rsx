// The home page of a new application: the one create-next-app writes, with
// RustX where it says Next.js. Edit app/page.rsx and save: the window follows.

fn Logo() -> Element {
    // The wordmark takes the place of the Next.js logo: a mark, then the name.
    return <div style={{ display: "flex", alignItems: "center", gap: "10px", color: "var(--rx-fg)" }}>
        <div style={{ width: "38px", height: "38px", borderRadius: "10px", background: "var(--rx-fg)", color: "var(--rx-bg)", display: "flex", alignItems: "center", justifyContent: "center", fontWeight: "800", fontSize: "22px" }}>R</div>
        <span style={{ fontSize: "30px", fontWeight: "700", letterSpacing: "-0.04em" }}>RustX</span>
    </div>;
}

fn Icon(path: String) -> Element {
    return <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d={path} />
    </svg>;
}

fn FooterLink(label: String, path: String, url: String) -> Element {
    return <a style={{ display: "flex", alignItems: "center", gap: "8px", cursor: "pointer", color: "var(--rx-fg)", textDecoration: "none", fontSize: "14px" }} onClick={move |_| { let _ = rustx::os::shell::open_url(&url); }}>
        {Icon(path)}
        {label}
    </a>;
}

fn WindowControls() -> Element {
    // The window has no frame: this strip moves it, and the three marks do what the frame's did.
    let mark = "width:40px;height:28px;border:0;background:transparent;color:var(--rx-muted);cursor:pointer;font-size:13px;";
    return <div style={{ position: "fixed", top: "0", left: "0", right: "0", height: "32px", display: "flex", alignItems: "center", zIndex: "10" }}>
        <div style={{ flex: "1", height: "32px" }} onMouseDown={move |_| { rustx::window::drag(); }}></div>
        <button aria-label="Minimize" style={mark} onClick={move |_| { rustx::window::minimize(); }}>{"\u{2013}"}</button>
        <button aria-label="Maximize" style={mark} onClick={move |_| { rustx::window::toggle_maximize(); }}>{"\u{25A1}"}</button>
        <button aria-label="Close" style={mark} onClick={move |_| { rustx::window::shut(); }}>{"\u{2715}"}</button>
    </div>;
}

fn SystemButton(name: String) -> Element {
    let target = name.clone();
    return <button style={{ padding: "2px 8px", borderRadius: "999px", border: "1px solid var(--rx-border)", background: "transparent", color: "var(--rx-muted)", cursor: "pointer", fontSize: "12px" }} onClick={move |_| { rustx::setup::open(&target); }}>{name}</button>;
}

fn Page() -> Element {
    let names = ["about", "settings", "permissions", "license", "install", "update", "uninstall"];
    let system: Vec<Element> = names.iter().map(|name| SystemButton(name.to_string())).collect();
    let primary = "display:flex;align-items:center;justify-content:center;gap:8px;height:48px;padding:0 20px;border-radius:999px;border:1px solid transparent;background:var(--rx-fg);color:var(--rx-bg);font-weight:500;font-size:16px;cursor:pointer;";
    let secondary = "display:flex;align-items:center;justify-content:center;height:48px;min-width:180px;padding:0 20px;border-radius:999px;border:1px solid var(--rx-border);background:transparent;color:var(--rx-fg);font-weight:500;font-size:16px;cursor:pointer;";
    return <div style={{ minHeight: "100vh", boxSizing: "border-box", padding: "80px 80px 80px", display: "grid", gridTemplateRows: "20px 1fr 20px", alignItems: "center", justifyItems: "center", gap: "64px", background: "var(--rx-bg)", color: "var(--rx-fg)" }}>
        {WindowControls()}
        <main style={{ display: "flex", flexDirection: "column", gap: "32px", gridRowStart: "2", alignItems: "flex-start" }}>
            {Logo()}
            <ol style={{ margin: "0", paddingLeft: "20px", fontSize: "14px", lineHeight: "24px", fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace", letterSpacing: "-0.01em" }}>
                <li style={{ marginBottom: "8px" }}>
                    {"Get started by editing "}
                    <code style={{ background: "rgba(127,127,127,0.14)", padding: "2px 4px", borderRadius: "4px", fontWeight: "600" }}>app/page.rsx</code>
                    {"."}
                </li>
                <li>Save and see your changes instantly.</li>
            </ol>
            <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
                <button style={primary} onClick={move |_| { let _ = rustx::os::shell::open_url("https://rustx.dev/docs/tooling/packaging"); }}>
                    <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true"><path d="M10 3 L18 17 H2 Z" /></svg>
                    Deploy now
                </button>
                <button style={secondary} onClick={move |_| { let _ = rustx::os::shell::open_url("https://rustx.dev/docs"); }}>Read our docs</button>
            </div>
            <div style={{ display: "flex", gap: "8px", flexWrap: "wrap", alignItems: "center" }}>
                <span style={{ color: "var(--rx-muted)", fontSize: "12px" }}>System screens</span>
                {system}
                <button style={{ padding: "2px 8px", borderRadius: "999px", border: "1px solid var(--rx-border)", background: "transparent", color: "var(--rx-muted)", cursor: "pointer", fontSize: "12px" }} onClick={move |_| { rustx::router::navigate("/counter"); }}>counter</button>
            </div>
        </main>
        <footer style={{ gridRowStart: "3", display: "flex", gap: "24px", flexWrap: "wrap", alignItems: "center", justifyContent: "center" }}>
            {FooterLink("Learn".to_string(), "M2.5 1.5 h8 l3 3 v10 h-11 Z M10.5 1.5 v3 h3".to_string(), "https://rustx.dev/docs/getting-started/installation".to_string())}
            {FooterLink("Examples".to_string(), "M1.5 2.5 h13 v11 h-13 Z M1.5 5.5 h13".to_string(), "https://github.com/AZTRA2022/rustx/tree/main/apps".to_string())}
            {FooterLink("Go to rustx.dev \u{2192}".to_string(), "M8 1.5 a6.5 6.5 0 1 0 0.01 0 Z M1.5 8 h13 M8 1.5 c-3 3.5 -3 9.5 0 13 M8 1.5 c3 3.5 3 9.5 0 13".to_string(), "https://rustx.dev".to_string())}
        </footer>
    </div>;
}
