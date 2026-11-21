// SECURITY — the page that refuses to oversell.
//
// A product page that says "institutional-grade" owes the reader a
// page that says exactly what the cipher is. This is that page, and
// it is deliberately the least flattering screen in the application.

let mut passphrase: String = String::new();
let mut remembered: bool = false;
let mut said: String = String::new();

fn Page() -> Element {
    let strength = judge(&passphrase);

    return <div key="security" id="screen-security" className="plain">
        <div className="label">SECURITY</div>
        <h1 className="h1" style={{ marginTop: "8px" }}>What is actually protecting this</h1>
        <p className="lead" style={{ marginTop: "10px", maxWidth: "640px" }}>
            The landing page says “encrypted”. Here is the whole truth behind that word, because a
            vault that is vague about its cipher is not a vault.
        </p>

        <div className="panel" style={{ marginTop: "22px", borderColor: "#0a0a0a" }}>
            <div className="label">THE CIPHER, IN ONE PARAGRAPH</div>
            <p style={{ fontSize: "13px", lineHeight: "1.8", margin: "10px 0 0" }}>
                Every byte of a secret is XORed with the byte of the passphrase at the same
                position, the passphrase repeating. That is the whole algorithm; it is ten lines in
                src/main.rs under the name xor_cipher. A repeating-key XOR falls to anyone who
                knows the key length and has enough ciphertext. It is not AES, and nothing in this
                application claims it is.
            </p>
        </div>

        <div className="grid2" style={{ marginTop: "20px" }}>
            <div className="panel">
                <div className="label">WHAT IS TRUE</div>
                <ul className="list" style={{ marginTop: "10px", display: "flex", flexDirection: "column", gap: "8px" }}>
                    {Point("The passphrase never leaves this process unless you ask the machine's credential store to keep it.".to_string())}
                    {Point("A sealed blob is hex and nothing else: no label, no length hint, no key material.".to_string())}
                    {Point("There is no network call anywhere in this application. The vault cannot be exfiltrated by it.".to_string())}
                    {Point("An exported vault file holds labels and ciphertext. Without the passphrase it is a list of noise.".to_string())}
                </ul>
            </div>

            <div className="panel">
                <div className="label">WHAT IS NOT</div>
                <ul className="list" style={{ marginTop: "10px", display: "flex", flexDirection: "column", gap: "8px" }}>
                    {Point("The cipher is not strong. Treat a sealed blob as obfuscated, not encrypted.".to_string())}
                    {Point("There is no key derivation: a short passphrase is a short key, byte for byte.".to_string())}
                    {Point("Nothing here is audited, and the AUDIT: PASSED marking on the landing page is a design element.".to_string())}
                    {Point("The temperature readout is set dressing. There is no cold room.".to_string())}
                </ul>
            </div>
        </div>

        <div className="panel" style={{ marginTop: "20px" }}>
            <div className="label">YOUR PASSPHRASE, MEASURED</div>
            <input
                className="field-input"
                style={{ marginTop: "10px", maxWidth: "420px" }}
                type="password"
                placeholder="type one to see what the cipher gets"
                value={passphrase.clone()}
                onInput={move |event| passphrase = event.value}
            />
            <div className="readout" style={{ marginTop: "10px", fontSize: "11px" }}>
                <div>{format!("LENGTH: {} bytes — the key repeats every {} bytes of plaintext", passphrase.len(), passphrase.len().max(1))}</div>
                <div>{format!("DISTINCT BYTES: {}", distinct(&passphrase))}</div>
                <div>{format!("VERDICT: {strength}")}</div>
                <div>{if remembered { "KEYCHAIN: this machine is holding it" } else { "KEYCHAIN: not stored" }}</div>
            </div>
            <div className="row" style={{ marginTop: "12px" }}>
                <button className="ghost small" disabled={passphrase.is_empty()} onClick={move |_| {
                    let kept = passphrase.clone();
                    task(async move {
                        match rustx::secret::store("Cryostasis", "passphrase", &kept) {
                            Ok(()) => {
                                remembered = true;
                                said = "the passphrase is in this machine's credential store".to_string();
                            }
                            Err(error) => said = format!("the credential store refused: {error}"),
                        }
                    });
                }}>
                    PUT IT IN THE KEYCHAIN
                </button>
                <button className="ghost small" disabled={!remembered} onClick={move |_| {
                    task(async move {
                        match rustx::secret::forget("Cryostasis", "passphrase") {
                            Ok(()) => {
                                remembered = false;
                                said = "the credential store no longer holds it".to_string();
                            }
                            Err(error) => said = format!("the credential store refused: {error}"),
                        }
                    });
                }}>
                    TAKE IT OUT
                </button>
                <button className="cta small" onClick={move |_| rustx::router::navigate("/docs/the-cipher".to_string())}>
                    READ THE CIPHER DOC
                </button>
            </div>
        </div>
    </div>;
}

fn Point(text: String) -> Element {
    return <li key={text.clone()} style={{ display: "flex", gap: "8px", fontSize: "12px", lineHeight: "1.65", color: "#2a2a2a" }}>
        <span style={{ color: "#9a9a9a" }}>—</span>
        <span>{text}</span>
    </li>;
}

fn distinct(text: &str) -> usize {
    let mut seen: Vec<u8> = Vec::new();
    for byte in text.bytes() {
        if !seen.contains(&byte) {
            seen.push(byte);
        }
    }
    seen.len()
}

/// A verdict on the key, honest about the fact that the cipher is the
/// weak part no matter what is typed.
fn judge(text: &str) -> String {
    if text.is_empty() {
        return "no key at all — a secret sealed with nothing comes out unchanged".to_string();
    }
    if text.len() < 8 {
        return "short — the key repeats often enough to be read off the ciphertext".to_string();
    }
    if distinct(text) < 6 {
        return "repetitive — few distinct bytes means few distinct masks".to_string();
    }
    "as good as this cipher can use, which is still not very good".to_string()
}

