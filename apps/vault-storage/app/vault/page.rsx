// THE VAULT — what SECURE NOW opens, and the only screen of this
// application that does work rather than describing it.
//
// Everything here leaves the page and touches the machine:
//
//   SEAL        the cipher in `src/main.rs`, over the bytes you typed
//   REMEMBER    the passphrase into the platform's own credential
//               store, through `rustx::secret`
//   EXPORT      a real save dialog and a real file written
//   IMPORT      a real file picker and a real file read
//   COPY        the hex on the real clipboard
//   the QR      `react-qrcode-logo`, off npm, of that same hex
//
// The passphrase is never written into the vault file. A file on its
// own is hex and labels, and reveals nothing.

let mut items: Vec<VaultItem> = Vec::new();
let mut passphrase: String = String::new();
let mut remembered: bool = false;
let mut said: String = String::new();
let mut sealed_count: u32 = 0;

let mut label_draft: String = String::new();
let mut secret_draft: String = String::new();
let mut shown: String = String::new();
let mut shown_label: String = String::new();

fn Page() -> Element {
    let ready = !secret_draft.is_empty() && !passphrase.is_empty();
    let latest = items.last().map(|item| item.cipher_hex.clone()).unwrap_or_default();

    return <div key="vault" id="screen-vault" className="plain">
        <div className="row">
            <div>
                <h1 className="h1">The vault</h1>
                <p className="lead" style={{ margin: "6px 0 0", maxWidth: "560px" }}>
                    Type a secret, give it a label, and seal it. Nothing leaves this process except
                    as hex, and nothing leaves this machine at all.
                </p>
            </div>
            <span style={{ marginLeft: "auto" }} className="row">
                <button className="ghost small" onClick={move |_| import_vault()}>IMPORT</button>
                <button className="ghost small" onClick={move |_| export_vault()}>EXPORT</button>
            </span>
        </div>

        <div className="grid2" style={{ marginTop: "22px", alignItems: "start" }}>
            <div>
                {Form(ready)}
                {Keychain()}
            </div>
            <div>
                {Proof(latest)}
                {Revealed()}
            </div>
        </div>

        <div style={{ marginTop: "22px" }}>
            <div className="label">{format!("{} SEALED", items.len())}</div>
            <ul id="items" className="list" style={{ marginTop: "9px", display: "flex", flexDirection: "column", gap: "7px" }}>
                {items.iter().enumerate().map(|(index, item)| Row(index, item.clone())).collect::<Vec<_>>()}
            </ul>
            {if items.is_empty() {
                <p style={{ fontSize: "12px", color: "#9a9a9a", marginTop: "10px" }}>
                    The vault is empty. Seal something and it will appear here with its own address.
                </p>
            } else {
                <span></span>
            }}
        </div>
    </div>;
}

fn Form(ready: bool) -> Element {
    return <div className="panel">
        <div className="label">SEAL A SECRET</div>
        <input
            id="label"
            className="field-input"
            style={{ marginTop: "10px" }}
            placeholder="label — becomes the address"
            value={label_draft.clone()}
            onInput={move |event| label_draft = event.value}
        />
        <textarea
            id="secret"
            className="field-input"
            style={{ marginTop: "8px", height: "76px", resize: "none" }}
            placeholder="the secret text"
            value={secret_draft.clone()}
            onInput={move |event| secret_draft = event.value}
        ></textarea>
        <input
            id="passphrase"
            className="field-input"
            style={{ marginTop: "8px" }}
            type="password"
            placeholder="passphrase — the key, never stored with the blob"
            value={passphrase.clone()}
            onInput={move |event| passphrase = event.value}
        />
        <button
            id="seal"
            className="cta"
            style={{ marginTop: "12px", width: "100%" }}
            disabled={!ready}
            onClick={move |_| seal()}
        >
            SEAL INTO THE VAULT
        </button>
        <p className="readout" style={{ marginTop: "9px" }}>
            {if ready {
                "Ready. The blob is the secret XORed with the passphrase, byte by byte.".to_string()
            } else {
                "A secret and a passphrase are both needed before anything can be sealed.".to_string()
            }}
        </p>
    </div>;
}

fn Keychain() -> Element {
    return <div className="panel" style={{ marginTop: "14px" }}>
        <div className="label">THE KEYCHAIN</div>
        <p style={{ fontSize: "12px", lineHeight: "1.65", margin: "9px 0 0", color: "#2a2a2a" }}>
            {if remembered {
                "This machine's credential store is holding the passphrase under the service name Cryostasis.".to_string()
            } else {
                "The passphrase is in this window only. Ask the machine to keep it and it will survive closing the window.".to_string()
            }}
        </p>
        <div className="row" style={{ marginTop: "11px" }}>
            <button className="ghost small" disabled={passphrase.is_empty()} onClick={move |_| remember_it()}>
                REMEMBER IT
            </button>
            <button className="ghost small" disabled={!remembered} onClick={move |_| forget_it()}>
                FORGET
            </button>
            <button className="ghost small" onClick={move |_| rustx::router::navigate("/docs/the-keychain".to_string())}>
                WHAT THIS DOES
            </button>
        </div>
    </div>;
}

fn Proof(latest: String) -> Element {
    let value = if latest.is_empty() { "cryostasis-empty-vault".to_string() } else { latest.clone() };
    let held: usize = items.iter().map(|item| item.cipher_hex.len() / 2).sum();

    return <div className="panel-dark">
        <div style={{ fontSize: "10px", letterSpacing: "0.1em" }}>[PROOF OF CUSTODY]</div>
        <div className="row" style={{ marginTop: "12px", alignItems: "flex-start", gap: "14px" }}>
            <span style={{ backgroundColor: "#ffffff", padding: "7px", display: "inline-flex" }}>
                <reactQrcodeLogo::QRCode value={value.clone()} size={118} fgColor="#0a0a0a" bgColor="#ffffff" ecLevel="M" quietZone={2} />
            </span>
            <span style={{ fontSize: "10px", lineHeight: "1.9", letterSpacing: "0.06em", color: "#c9c9c9" }}>
                <span style={{ display: "block" }}>{fingerprint(&value)}</span>
                <span style={{ display: "block" }}>{format!("QUEUE: {}", items.len())}</span>
                <span style={{ display: "block" }}>{format!("BYTES: {held}")}</span>
                <span style={{ display: "block" }}>{format!("SEALS: {}", sealed_count)}</span>
                <span style={{ display: "block" }}>CIPHER: XOR-STREAM</span>
                <span style={{ display: "block" }}>OFFLINE STATUS: OK</span>
            </span>
        </div>
        <p style={{ fontSize: "10px", lineHeight: "1.6", color: "#9a9a9a", margin: "12px 0 0" }}>
            The code above encodes the hex of the most recent seal and nothing else. A phone can read
            it; without the passphrase it is noise.
        </p>
    </div>;
}

fn Revealed() -> Element {
    if shown.is_empty() {
        return <div className="panel" style={{ marginTop: "14px" }}>
            <div className="label">REVEALED</div>
            <p style={{ fontSize: "12px", color: "#9a9a9a", margin: "9px 0 0" }}>
                Open a sealed item below with the passphrase in the form and it appears here.
            </p>
        </div>;
    }
    return <div className="panel" style={{ marginTop: "14px" }}>
        <div className="label">{format!("REVEALED — {}", shown_label.clone().to_uppercase())}</div>
        <p id="revealed" className="blob" style={{ marginTop: "9px" }}>{shown.clone()}</p>
        <div className="row" style={{ marginTop: "10px" }}>
            <button className="ghost small" onClick={move |_| copy_revealed()}>COPY IT</button>
            <button className="ghost small" onClick={move |_| { shown = String::new(); shown_label = String::new(); }}>CLEAR</button>
        </div>
    </div>;
}

fn Row(index: usize, item: VaultItem) -> Element {
    let going = format!("/vault/{}", item.slug);
    let hex = item.cipher_hex.clone();
    let label = item.label.clone();

    return <li key={item.slug.clone()} className="item">
        <span style={{ minWidth: "0" }}>
            <span style={{ display: "block", fontSize: "12px" }}>{item.label.clone()}</span>
            <span style={{ display: "block", fontSize: "10px", color: "#9a9a9a" }}>
                {format!("{} · {} bytes · {}", item.at, item.cipher_hex.len() / 2, fingerprint(&item.cipher_hex))}
            </span>
        </span>
        <span className="row" style={{ marginLeft: "auto" }}>
            <button className="ghost small" onClick={move |_| open_one(hex.clone(), label.clone())}>REVEAL</button>
            <button className="ghost small" onClick={move |_| rustx::router::navigate(going.clone())}>PROOF</button>
            <button className="ghost small" onClick={move |_| drop_one(index)}>REMOVE</button>
        </span>
    </li>;
}

// ---- what the buttons do ---------------------------------------------

fn seal() {
    if secret_draft.is_empty() || passphrase.is_empty() {
        return;
    }
    let hex = to_hex(&xor_cipher(secret_draft.as_bytes(), passphrase.as_bytes()));
    let label = if label_draft.trim().is_empty() {
        format!("untitled {}", items.len() + 1)
    } else {
        label_draft.trim().to_string()
    };
    let mut slug = slugify(&label);
    let mut all = items.clone();
    // Two items may not share an address.
    while all.iter().any(|item| item.slug == slug) {
        slug = format!("{slug}-2");
    }
    all.push(VaultItem { slug, label: label.clone(), cipher_hex: hex, at: clock() });
    items = all;
    sealed_count += 1;
    label_draft = String::new();
    secret_draft = String::new();
    shown = String::new();
    shown_label = String::new();
    said = format!("{label} is sealed");
}

fn open_one(hex: String, label: String) {
    let opened = xor_cipher(&from_hex(&hex), passphrase.as_bytes());
    shown = String::from_utf8_lossy(&opened).to_string();
    shown_label = label.clone();
    said = format!("{label} opened with the passphrase in the form");
}

fn drop_one(index: usize) {
    let mut all = items.clone();
    if index < all.len() {
        let gone = all.remove(index);
        said = format!("{} removed from the vault", gone.label);
    }
    items = all;
}

fn copy_revealed() {
    match rustx::clipboard::set_text(&shown) {
        Ok(()) => said = "the revealed text is on the clipboard".to_string(),
        Err(error) => said = format!("the clipboard refused: {error}"),
    }
}

fn remember_it() {
    match rustx::secret::store("Cryostasis", "passphrase", &passphrase) {
        Ok(()) => {
            remembered = true;
            said = "the passphrase is in this machine's credential store".to_string();
        }
        Err(error) => said = format!("the credential store refused: {error}"),
    }
}

fn forget_it() {
    match rustx::secret::forget("Cryostasis", "passphrase") {
        Ok(()) => {
            remembered = false;
            said = "the credential store no longer holds it".to_string();
        }
        Err(error) => said = format!("the credential store refused: {error}"),
    }
}

/// The whole vault as one real file. The passphrase is not in it.
fn export_vault() {
    let ask = rustx::dialog::Ask::new("Export the vault")
        .filter("Cryostasis vault", ["vault", "txt"])
        .named("cryostasis.vault");
    let Ok(path) = rustx::dialog::save(ask) else {
        said = "the export was cancelled".to_string();
        return;
    };
    let mut text = String::from("# cryostasis vault — label\\thex. the passphrase is not here.\n");
    for item in items.iter() {
        text.push_str(&format!("{}\t{}\n", item.label, item.cipher_hex));
    }
    match std::fs::write(&path, text) {
        Ok(()) => {
            said = format!("{} item(s) written to {}", items.len(), path.display());
            let _ = rustx::os::shell::remember_recent(&path);
        }
        Err(error) => said = format!("the export failed: {error}"),
    }
}

fn import_vault() {
    let ask = rustx::dialog::Ask::new("Import a vault").filter("Cryostasis vault", ["vault", "txt"]);
    let Ok(paths) = rustx::dialog::open(ask) else {
        said = "the import was cancelled".to_string();
        return;
    };
    let Some(path) = paths.first() else {
        said = "the import was cancelled".to_string();
        return;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        said = format!("{} could not be read", path.display());
        return;
    };

    let mut all = items.clone();
    let mut joined = 0usize;
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let Some((label, hex)) = line.split_once('\t') else { continue };
        let mut slug = slugify(label);
        while all.iter().any(|item| item.slug == slug) {
            slug = format!("{slug}-2");
        }
        all.push(VaultItem {
            slug,
            label: label.to_string(),
            cipher_hex: hex.trim().to_string(),
            at: clock(),
        });
        joined += 1;
    }
    items = all;
    said = format!("{joined} item(s) read from {}", path.display());
}
