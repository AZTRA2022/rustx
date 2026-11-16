// PRODUCT — what Cryostasis is, told against what the application
// in this window really does.

let mut items: Vec<VaultItem> = Vec::new();
let mut sealed_count: u32 = 0;

fn Page() -> Element {
    let held: usize = items.iter().map(|item| item.cipher_hex.len() / 2).sum();

    return <div key="product" id="screen-product" className="plain">
        <div className="label">PRODUCT</div>
        <h1 className="h1" style={{ marginTop: "8px" }}>A vault that does nothing clever</h1>
        <p className="lead" style={{ marginTop: "10px", maxWidth: "620px" }}>
            Cold storage is a discipline before it is a technology. Assets go in, the key stays
            out, and nothing in between is allowed to be interesting. This page is what the product
            does; the <span className="link">Security</span> page is what it refuses to claim.
        </p>

        <div className="grid3" style={{ marginTop: "26px" }}>
            {Feature(
                "SEAL".to_string(),
                "A secret and a passphrase go in; hex comes out. The passphrase is never written beside the blob, in memory or on disk.".to_string(),
            )}
            {Feature(
                "PROVE".to_string(),
                "Every sealed item gets an address, a fingerprint and a real QR code of its ciphertext, so custody can be shown without being broken.".to_string(),
            )}
            {Feature(
                "CARRY".to_string(),
                "A vault exports to one plain file and reads back from it. The file holds labels and hex; on its own it reveals nothing.".to_string(),
            )}
        </div>

        <div className="grid2" style={{ marginTop: "26px" }}>
            <div className="panel">
                <div className="label">WHAT IS IN THIS WINDOW RIGHT NOW</div>
                <div className="readout" style={{ marginTop: "10px", fontSize: "11px" }}>
                    <div>{format!("ITEMS SEALED: {}", items.len())}</div>
                    <div>{format!("CIPHERTEXT HELD: {held} bytes")}</div>
                    <div>{format!("SEALS THIS SESSION: {}", sealed_count)}</div>
                    <div>KEYCHAIN: the platform's own credential store</div>
                    <div>NETWORK: none, by construction</div>
                </div>
                <button className="cta" style={{ marginTop: "14px" }} onClick={move |_| rustx::router::navigate("/vault".to_string())}>
                    Open the vault
                </button>
            </div>

            <div className="panel">
                <div className="label">WHAT IT IS NOT</div>
                <p style={{ fontSize: "12px", lineHeight: "1.75", margin: "10px 0 0", color: "#2a2a2a" }}>
                    It is not a custodian. It does not hold anyone else's keys, it does not sign
                    anything on your behalf, and there is no server behind it to be compromised.
                    What runs here runs here. The page that explains the cipher is deliberately
                    blunt about how far that goes.
                </p>
                <button className="ghost" style={{ marginTop: "14px" }} onClick={move |_| rustx::router::navigate("/security".to_string())}>
                    Read the security page
                </button>
            </div>
        </div>

        <div className="grid4" style={{ marginTop: "26px" }}>
            {Step("01".to_string(), "Choose a passphrase".to_string())}
            {Step("02".to_string(), "Seal what matters".to_string())}
            {Step("03".to_string(), "Keep the proof".to_string())}
            {Step("04".to_string(), "Export and go cold".to_string())}
        </div>
    </div>;
}

fn Feature(name: String, text: String) -> Element {
    return <div key={name.clone()} className="panel">
        <div style={{ fontSize: "13px", letterSpacing: "0.08em" }}>{name}</div>
        <p style={{ fontSize: "12px", lineHeight: "1.7", margin: "9px 0 0", color: "#3a3a3a" }}>{text}</p>
    </div>;
}

fn Step(number: String, text: String) -> Element {
    return <div key={number.clone()} style={{ borderTop: "1px solid #0a0a0a", paddingTop: "9px" }}>
        <div style={{ fontSize: "10px", color: "#9a9a9a" }}>{number}</div>
        <div style={{ fontSize: "12px", marginTop: "4px" }}>{text}</div>
    </div>;
}
