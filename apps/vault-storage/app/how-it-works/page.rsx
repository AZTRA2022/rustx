// HOW IT WORKS — the second button on the landing page.
//
// Four steps, told at length, each one ending where the reader can
// actually do it. The readouts on the right are counted off the vault
// as it stands, so the page is about this window and not about a
// product in general.

let mut items: Vec<VaultItem> = Vec::new();
let mut passphrase: String = String::new();
let mut remembered: bool = false;
let mut sealed_count: u32 = 0;

let mut step: usize = 0;

fn Page() -> Element {
    let all = steps();
    let shown = all.get(step.min(all.len() - 1)).cloned().unwrap_or_else(|| all[0].clone());

    return <div key="how" id="screen-how" className="plain">
        <div className="label">HOW IT WORKS</div>
        <h1 className="h1" style={{ marginTop: "8px" }}>Four steps, and nothing between them</h1>
        <p className="lead" style={{ marginTop: "10px", maxWidth: "600px" }}>
            Cold storage is a procedure, not a feature list. This is the procedure, and the right
            column says where this window currently stands in it.
        </p>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 290px", gap: "26px", marginTop: "24px", alignItems: "start" }}>
            <div>
                <div className="row" style={{ gap: "8px" }}>
                    {all.iter().enumerate().map(|(index, one)| Pip(index, one.0.clone())).collect::<Vec<_>>()}
                </div>

                <div className="panel" style={{ marginTop: "16px", borderColor: "#0a0a0a" }}>
                    <div className="label">{format!("STEP {} — {}", step + 1, shown.0.clone().to_uppercase())}</div>
                    <p style={{ fontSize: "13px", lineHeight: "1.85", margin: "10px 0 0" }}>{shown.1.clone()}</p>
                    <div className="row" style={{ marginTop: "16px" }}>
                        <button className="ghost small" disabled={step == 0} onClick={move |_| step = step.saturating_sub(1)}>
                            BACK
                        </button>
                        <button className="ghost small" disabled={step + 1 >= all.len()} onClick={move |_| step += 1}>
                            NEXT
                        </button>
                        <button className="cta small" onClick={move |_| rustx::router::navigate(shown.2.clone())}>
                            {shown.3.clone()}
                        </button>
                    </div>
                </div>
            </div>

            <div className="panel-dark">
                <div style={{ fontSize: "10px", letterSpacing: "0.1em" }}>[WHERE YOU STAND]</div>
                <div style={{ fontSize: "10px", lineHeight: "2", letterSpacing: "0.06em", color: "#c9c9c9", marginTop: "12px" }}>
                    <div>{format!("PASSPHRASE: {}", if passphrase.is_empty() { "not set" } else { "set in this window" })}</div>
                    <div>{format!("KEYCHAIN: {}", if remembered { "holding it" } else { "empty" })}</div>
                    <div>{format!("ITEMS SEALED: {}", items.len())}</div>
                    <div>{format!("SEALS THIS SESSION: {}", sealed_count)}</div>
                    <div>{format!("EXPORTED: {}", if items.is_empty() { "nothing to export" } else { "ready when you are" })}</div>
                </div>
                <button className="cta small" style={{ marginTop: "16px", width: "100%" }} onClick={move |_| rustx::router::navigate("/vault".to_string())}>
                    GO TO THE VAULT
                </button>
            </div>
        </div>
    </div>;
}

fn Pip(index: usize, name: String) -> Element {
    let here = step == index;
    let paint = format!(
        "border:1px solid {};background:{};color:{};padding:5px 11px;font-size:10px;letter-spacing:0.08em",
        if here { "#0a0a0a" } else { "#e4e4e4" },
        if here { "#0a0a0a" } else { "#ffffff" },
        if here { "#ffffff" } else { "#767676" }
    );
    return <button key={name.clone()} style={paint} onClick={move |_| step = index}>
        {format!("{} · {}", index + 1, name)}
    </button>;
}

/// Name, the whole explanation, where it leads, and what that link
/// is called.
fn steps() -> Vec<(String, String, String, String)> {
    vec![
        (
            "Choose a key".to_string(),
            "A passphrase is the only thing standing between a sealed blob and its contents, and it is never written beside one. Type it into the vault and it lives in this window; ask the machine to keep it and it goes into the platform's own credential store through rustx::secret, which is the Windows Credential Manager, the macOS keychain, or the Secret Service, depending on where you are standing. The security page measures what you typed and tells you, without flattery, what the cipher actually gets out of it.".to_string(),
            "/security".to_string(),
            "MEASURE A PASSPHRASE".to_string(),
        ),
        (
            "Seal".to_string(),
            "A secret and a label go in. The bytes of the secret are XORed with the bytes of the passphrase, the passphrase repeating, and what comes out is hex. The label becomes an address: /vault/<label> is that item's own page from then on, so a seal can be linked, reloaded and gone back to. Nothing is uploaded, because there is nothing in this application that could upload it.".to_string(),
            "/vault".to_string(),
            "SEAL SOMETHING".to_string(),
        ),
        (
            "Prove custody".to_string(),
            "Every sealed item gets a fingerprint — a short rotating checksum of its ciphertext — and a real QR code of that ciphertext, rendered by react-qrcode-logo off npm. A phone reads the code and gets hex; without the passphrase that hex is noise, which is exactly what makes it safe to show. That is what a proof of custody is: evidence that a thing is held, with no part of the thing in the evidence.".to_string(),
            "/docs/proof-of-custody".to_string(),
            "READ THE PROOF DOC".to_string(),
        ),
        (
            "Go cold".to_string(),
            "EXPORT writes the whole vault to one plain file through the system's own save dialog: one line per item, the label and the hex, separated by a tab, and a header saying the passphrase is not in it. Put that file somewhere offline. IMPORT reads it back into any window of this application. The key never travelled with it, so the file alone is a list of noise and a set of names.".to_string(),
            "/docs/quickstart".to_string(),
            "READ THE QUICKSTART".to_string(),
        ),
    ]
}
