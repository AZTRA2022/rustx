// ONE SEALED ITEM — its own proof-of-custody page, at the address
// the label became when it was sealed.
//
// Everything on it is computed from the blob: the fingerprint, the
// byte count, the QR code. The only thing that can open it is the
// passphrase, which lives in the layout above and in the machine's
// credential store if you asked for that.

let mut items: Vec<VaultItem> = Vec::new();
let mut passphrase: String = String::new();
let mut said: String = String::new();

let mut opened: String = String::new();

fn Page(label: String) -> Element {
    let found = items.iter().find(|item| item.slug == label).cloned();

    return <div key={format!("item-{label}")} id="screen-item" className="plain">
        <div className="row" style={{ fontSize: "11px", color: "#767676" }}>
            <button className="nav-item" onClick={move |_| rustx::router::navigate("/vault".to_string())}>
                The vault
            </button>
            <span style={{ color: "#c4c4c4" }}>/</span>
            <span style={{ color: "#0a0a0a" }}>
                {found.as_ref().map(|item| item.label.clone()).unwrap_or_else(|| "not in this vault".to_string())}
            </span>
        </div>

        {match found {
            Some(item) => Custody(item),
            None => Missing(label.clone()),
        }}
    </div>;
}

fn Custody(item: VaultItem) -> Element {
    let bytes = item.cipher_hex.len() / 2;
    let mark = fingerprint(&item.cipher_hex);
    let hex = item.cipher_hex.clone();
    let hex_to_copy = item.cipher_hex.clone();
    let label = item.label.clone();

    return <div key={item.slug.clone()} style={{ marginTop: "18px" }}>
        <h1 className="h1">{item.label.clone()}</h1>
        <p className="lead" style={{ margin: "6px 0 0" }}>
            {format!("Sealed at {} · {bytes} bytes · {mark}", item.at)}
        </p>

        <div className="grid2" style={{ marginTop: "20px", alignItems: "start" }}>
            <div className="panel-dark">
                <div style={{ fontSize: "10px", letterSpacing: "0.1em" }}>[PROOF OF CUSTODY]</div>
                <div className="row" style={{ marginTop: "12px", alignItems: "flex-start", gap: "14px" }}>
                    <span style={{ backgroundColor: "#ffffff", padding: "8px", display: "inline-flex" }}>
                        <reactQrcodeLogo::QRCode value={hex.clone()} size={140} fgColor="#0a0a0a" bgColor="#ffffff" ecLevel="M" quietZone={2} />
                    </span>
                    <span style={{ fontSize: "10px", lineHeight: "1.9", letterSpacing: "0.06em", color: "#c9c9c9" }}>
                        <span style={{ display: "block" }}>{mark.clone()}</span>
                        <span style={{ display: "block" }}>{format!("BYTES: {bytes}")}</span>
                        <span style={{ display: "block" }}>{format!("SEALED: {}", item.at)}</span>
                        <span style={{ display: "block" }}>CIPHER: XOR-STREAM</span>
                        <span style={{ display: "block" }}>STATE: IMMUTABLE</span>
                        <span style={{ display: "block" }}>OFFLINE STATUS: OK</span>
                    </span>
                </div>
            </div>

            <div>
                <div className="panel">
                    <div className="label">THE SEALED BLOB</div>
                    <p className="blob" style={{ marginTop: "9px" }}>{item.cipher_hex.clone()}</p>
                    <div className="row" style={{ marginTop: "10px" }}>
                        <button className="ghost small" onClick={move |_| copy_hex(hex_to_copy.clone())}>COPY THE HEX</button>
                        <button className="cta small" onClick={move |_| open_it(hex.clone(), label.clone())}>OPEN IT</button>
                    </div>
                </div>

                <div className="panel" style={{ marginTop: "14px" }}>
                    <div className="label">OPENED</div>
                    {if opened.is_empty() {
                        <p style={{ fontSize: "12px", color: "#9a9a9a", margin: "9px 0 0" }}>
                            {if passphrase.is_empty() {
                                "There is no passphrase in this window. Set one in the vault first.".to_string()
                            } else {
                                "Press OPEN IT to run the blob back through the cipher.".to_string()
                            }}
                        </p>
                    } else {
                        <p id="opened" className="blob" style={{ marginTop: "9px" }}>{opened.clone()}</p>
                    }}
                </div>
            </div>
        </div>

        <p className="readout" style={{ marginTop: "16px" }}>
            This page is addressable: the label became the slug when the item was sealed, so this
            proof can be gone back to, reloaded, and linked.
        </p>
    </div>;
}

fn Missing(label: String) -> Element {
    return <div style={{ marginTop: "18px" }}>
        <h1 className="h1">Nothing is sealed under that name</h1>
        <p className="lead" style={{ margin: "8px 0 0", maxWidth: "520px" }}>
            {format!("This vault holds no item at “{label}”. A vault lives in the window that opened it — if this was sealed in another session, import the file it was exported to.")}
        </p>
        <div className="row" style={{ marginTop: "16px" }}>
            <button className="cta" onClick={move |_| rustx::router::navigate("/vault".to_string())}>
                Back to the vault
            </button>
            <button className="ghost" onClick={move |_| rustx::router::navigate("/docs/quickstart".to_string())}>
                Read the quickstart
            </button>
        </div>
    </div>;
}

fn copy_hex(hex: String) {
    match rustx::clipboard::set_text(&hex) {
        Ok(()) => said = "the sealed blob is on the clipboard".to_string(),
        Err(error) => said = format!("the clipboard refused: {error}"),
    }
}

fn open_it(hex: String, label: String) {
    if passphrase.is_empty() {
        said = "there is no passphrase in this window".to_string();
        return;
    }
    let plain = xor_cipher(&from_hex(&hex), passphrase.as_bytes());
    opened = String::from_utf8_lossy(&plain).to_string();
    said = format!("{label} opened");
}
