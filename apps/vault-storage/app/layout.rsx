// The frame every screen sits in: the product's navigation at the
// top and its footer at the foot. The vault's own state lives here
// too, because it must survive moving between the vault, an item's
// proof page, and the marketing pages — a value a page declares is
// kept by its name for the whole application, so the pages below
// redeclare what they read and get these same cells.

let mut items: Vec<VaultItem> = Vec::new();
let mut passphrase: String = String::new();
let mut remembered: bool = false;
let mut said: String = String::new();
let mut sealed_count: u32 = 0;

fn Layout(children: Element) -> Element {
    effect((), || {
        // A passphrase the machine was asked to keep comes back here,
        // from the platform's own credential store.
        if let Ok(Some(kept)) = rustx::secret::fetch("Cryostasis", "passphrase") {
            passphrase = kept;
            remembered = true;
        }
    });

    let here = rustx::router::route();

    return <div className="shell">
        <header className="topbar">
            <button className="mark" onClick={move |_| rustx::router::navigate("/".to_string())}>
                {Mark()}
                <span style={{ fontSize: "13px", letterSpacing: "0.06em" }}>CRYOSTASIS</span>
            </button>
            <nav className="nav">
                {NavItem("Product".to_string(), "/product".to_string(), here.clone())}
                {NavItem("Security".to_string(), "/security".to_string(), here.clone())}
                {NavItem("Pricing".to_string(), "/pricing".to_string(), here.clone())}
                {NavItem("Docs".to_string(), "/docs".to_string(), here.clone())}
                {NavItem("Vault".to_string(), "/vault".to_string(), here.clone())}
            </nav>
            <button className="cta" onClick={move |_| rustx::router::navigate("/contact".to_string())}>
                Let's Talk
            </button>
        </header>

        {children}

        <footer className="footer">
            <span>CRYOSTASIS · COLD STORAGE</span>
            <button onClick={move |_| rustx::router::navigate("/how-it-works".to_string())}>How it works</button>
            <button onClick={move |_| rustx::router::navigate("/docs".to_string())}>Docs</button>
            <button onClick={move |_| rustx::router::navigate("/security".to_string())}>Security</button>
            <button onClick={move |_| rustx::router::navigate("/contact".to_string())}>Contact</button>
            <span id="said" style={{ marginLeft: "12px", color: "#767676" }}>{said.clone()}</span>
            <span style={{ marginLeft: "auto" }}>
                {format!("{} sealed · {} this session", items.len(), sealed_count)}
            </span>
        </footer>
    </div>;
}

fn NavItem(label: String, route: String, here: String) -> Element {
    let active = here == route || here.starts_with(&format!("{route}/"));
    let classes = if active { "nav-item on".to_string() } else { "nav-item".to_string() };
    let going = route.clone();
    return <button key={label.clone()} className={classes} onClick={move |_| rustx::router::navigate(going.clone())}>
        {label}
    </button>;
}

/// The logo: three stacked chevrons inside a rounded square, drawn
/// rather than loaded, so the application ships no image file.
fn Mark() -> Element {
    return <span style={{ display: "inline-flex", alignItems: "center", justifyContent: "center", width: "22px", height: "22px", borderRadius: "5px", backgroundColor: "#0a0a0a" }}>
        <span style={{ display: "flex", flexDirection: "column", gap: "2px" }}>
            <span style={{ width: "11px", height: "2px", backgroundColor: "#ffffff" }}></span>
            <span style={{ width: "11px", height: "2px", backgroundColor: "#ffffff", opacity: "0.72" }}></span>
            <span style={{ width: "11px", height: "2px", backgroundColor: "#ffffff", opacity: "0.44" }}></span>
        </span>
    </span>;
}
