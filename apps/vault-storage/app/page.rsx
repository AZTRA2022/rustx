// Vault-Storage Secure.
//
// Everything stored here only ever leaves this process as a hex blob
// produced by `xor_cipher` below — a cipher chosen on purpose because
// it is honest about what it is: reversible, readable in ten lines,
// and not a claim of real security. The QR code is not a decoration:
// it is `react-qrcode-logo`, off npm, rendering the sealed blob for
// real — the same way a phone's camera would read it back.

#[derive(Clone)]
struct VaultItem {
    label: String,
    cipher_hex: String,
}

let mut label_draft = String::new();
let mut secret = String::new();
let mut passphrase = String::new();
let mut items: Vec<VaultItem> = Vec::new();
let mut reveal = String::new();
let mut qr_text = String::new();
let mut copied = false;

fn Page() -> Element {
    return <div style={{ fontFamily: "Segoe UI, system-ui, sans-serif", padding: "24px", backgroundColor: "#0a0a12", color: "#e8e8f0", minHeight: "100vh" }}>
        <h1 style={{ margin: "0 0 4px", fontSize: "22px", letterSpacing: "1px" }}>VAULT-STORAGE SECURE</h1>
        <p style={{ margin: "0 0 20px", color: "#9490b8", fontSize: "13px" }}>
            A toy cipher, labelled as a toy, and a real QR code of what it produces.
        </p>

        <div style={{ display: "grid", gridTemplateColumns: "1.1fr 0.9fr", gap: "24px" }}>
            <div>
                {Form()}
                {List()}
            </div>
            <div>
                {Reveal()}
                {QrPanel()}
            </div>
        </div>
    </div>;
}

fn Form() -> Element {
    return <div style={{ border: "1px solid #241f3d", borderRadius: "10px", padding: "16px", backgroundColor: "#121026" }}>
        <h2 style={{ margin: "0 0 10px", fontSize: "13px", color: "#a99bf0", textTransform: "uppercase" }}>seal a secret</h2>
        <input id="label" placeholder="label" value={label_draft.clone()}
            onInput={move |event| label_draft = event.value}
            style={{ width: "100%", marginBottom: "8px", padding: "7px", backgroundColor: "#0a0a12", border: "1px solid #241f3d", color: "#e8e8f0", borderRadius: "6px" }} />
        <input id="secret" placeholder="the secret text" value={secret.clone()}
            onInput={move |event| secret = event.value}
            style={{ width: "100%", marginBottom: "8px", padding: "7px", backgroundColor: "#0a0a12", border: "1px solid #241f3d", color: "#e8e8f0", borderRadius: "6px" }} />
        <input id="passphrase" type="password" placeholder="passphrase" value={passphrase.clone()}
            onInput={move |event| passphrase = event.value}
            style={{ width: "100%", marginBottom: "10px", padding: "7px", backgroundColor: "#0a0a12", border: "1px solid #241f3d", color: "#e8e8f0", borderRadius: "6px" }} />
        <button id="seal" onClick={move |_| {
            if secret.is_empty() || passphrase.is_empty() { return; }
            let cipher = xor_cipher(secret.as_bytes(), passphrase.as_bytes());
            let hex = to_hex(&cipher);
            qr_text = hex.clone();
            items.push(VaultItem { label: if label_draft.is_empty() { "untitled".to_string() } else { label_draft.clone() }, cipher_hex: hex });
            label_draft = String::new();
            secret = String::new();
            reveal = String::new();
        }} style={{ padding: "8px 16px", backgroundColor: "#7c5cff", color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer" }}>
            seal into the vault
        </button>
    </div>;
}

fn List() -> Element {
    return <div style={{ marginTop: "16px" }}>
        <h2 style={{ margin: "0 0 10px", fontSize: "13px", color: "#a99bf0", textTransform: "uppercase" }}>{items.len()} sealed</h2>
        <ul id="items" style={{ listStyle: "none", padding: "0", margin: "0", display: "flex", flexDirection: "column", gap: "8px" }}>
            {items.iter().enumerate().map(|(index, item)| ItemRow(index, item.clone())).collect::<Vec<_>>()}
        </ul>
    </div>;
}

fn ItemRow(index: usize, item: VaultItem) -> Element {
    return <li key={index.to_string()} style={{ display: "flex", alignItems: "center", justifyContent: "space-between", border: "1px solid #241f3d", borderRadius: "8px", padding: "8px 12px", backgroundColor: "#121026" }}>
        <div>
            <div style={{ fontWeight: "600" }}>{item.label.clone()}</div>
            <div style={{ fontSize: "11px", color: "#6f69a0", fontFamily: "Consolas, monospace" }}>{item.cipher_hex.chars().take(24).collect::<String>()}…</div>
        </div>
        <div style={{ display: "flex", gap: "6px" }}>
            <button onClick={move |_| {
                qr_text = item.cipher_hex.clone();
                let bytes = from_hex(&item.cipher_hex);
                let opened = xor_cipher(&bytes, passphrase.as_bytes());
                reveal = String::from_utf8_lossy(&opened).to_string();
                copied = false;
            }} style={{ padding: "5px 10px", fontSize: "12px", backgroundColor: "#1c1836", color: "#c9c3f5", border: "1px solid #302a55", borderRadius: "6px", cursor: "pointer" }}>reveal</button>
            <button onClick={move |_| items.remove(index)} style={{ padding: "5px 10px", fontSize: "12px", backgroundColor: "#2a1420", color: "#f5a3b8", border: "1px solid #4a1c2e", borderRadius: "6px", cursor: "pointer" }}>remove</button>
        </div>
    </li>;
}

fn Reveal() -> Element {
    if reveal.is_empty() {
        return <div style={{ color: "#6f69a0", fontSize: "13px" }}>pick a sealed secret to reveal it with the current passphrase.</div>;
    }
    return <div style={{ border: "1px solid #241f3d", borderRadius: "10px", padding: "16px", backgroundColor: "#121026", marginBottom: "16px" }}>
        <h2 style={{ margin: "0 0 8px", fontSize: "13px", color: "#a99bf0", textTransform: "uppercase" }}>revealed</h2>
        <p id="revealed" style={{ fontFamily: "Consolas, monospace", wordBreak: "break-all" }}>{reveal.clone()}</p>
        <button id="copy" onClick={move |_| {
            copied = rustx::clipboard::set_text(&reveal).is_ok();
        }} style={{ padding: "6px 12px", fontSize: "12px", backgroundColor: "#1c1836", color: "#c9c3f5", border: "1px solid #302a55", borderRadius: "6px", cursor: "pointer" }}>
            {if copied { "copied to clipboard" } else { "copy to clipboard" }}
        </button>
    </div>;
}

fn QrPanel() -> Element {
    if qr_text.is_empty() {
        return <div style={{ color: "#6f69a0", fontSize: "13px" }}>seal or pick a secret to see its QR code.</div>;
    }
    return <div style={{ border: "1px solid #241f3d", borderRadius: "10px", padding: "16px", backgroundColor: "#121026" }}>
        <h2 style={{ margin: "0 0 10px", fontSize: "13px", color: "#a99bf0", textTransform: "uppercase" }}>QR of the sealed blob</h2>
        <div id="qr" style={{ display: "inline-block", backgroundColor: "#ffffff", padding: "12px", borderRadius: "6px" }}>
            <reactQrcodeLogo::QRCode value={qr_text.clone()} size={180} fgColor="#0a0a12" bgColor="#ffffff" ecLevel="M" />
        </div>
    </div>;
}

/// A reversible XOR stream cipher, on purpose the simplest thing that
/// still rounds-trips text through a passphrase: this app's whole
/// point is to show the vault's shape, not to be a cryptography
/// library.
fn xor_cipher(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }
    data.iter().enumerate().map(|(index, byte)| byte ^ key[index % key.len()]).collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn from_hex(text: &str) -> Vec<u8> {
    let text = text.trim();
    let mut out = Vec::new();
    let mut index = 0;
    while index + 2 <= text.len() {
        if let Ok(byte) = u8::from_str_radix(&text[index..index + 2], 16) {
            out.push(byte);
        }
        index += 2;
    }
    out
}
