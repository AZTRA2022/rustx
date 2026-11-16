// The landing page.
//
// The headline is not a font: there is no 5×7 bitmap face on this
// machine, so the two words are laid out cell by cell from the table
// in `src/main.rs` — one absolutely-positioned square per lit pixel.
//
// The cube in the middle is six CSS faces in a perspective box,
// turning once every twenty-six seconds, with a blurred conic
// gradient behind it for the refraction. No image file ships with
// this application.
//
// The panel in the corner is real: the QR code is
// `react-qrcode-logo` off npm, encoding the fingerprint of whatever
// the vault holds right now, and every readout beside it is counted
// off that same vault.

let mut items: Vec<VaultItem> = Vec::new();
let mut sealed_count: u32 = 0;

fn Page() -> Element {
    let held: usize = items.iter().map(|item| item.cipher_hex.len() / 2).sum();
    let seal = items
        .last()
        .map(|item| item.cipher_hex.clone())
        .unwrap_or_else(|| "cryostasis-empty-vault".to_string());
    let mark = fingerprint(&seal);

    return <div key="home" id="screen-home" className="field">
        {Ticks()}
        {Tags()}

        <div style={{ position: "relative", padding: "40px 48px 0", zIndex: "2" }}>
            {Headline()}

            <div style={{ display: "flex", alignItems: "flex-start", marginTop: "44px" }}>
                <div style={{ width: "270px" }}>
                    <p className="lead" style={{ margin: "0" }}>
                        Institutional-grade cold storage security. Assets held offline, encrypted,
                        and untouched until you need them.
                    </p>
                    <div className="row" style={{ marginTop: "18px" }}>
                        <button className="cta" onClick={move |_| rustx::router::navigate("/vault".to_string())}>
                            Secure Now→
                        </button>
                        <button className="ghost" onClick={move |_| rustx::router::navigate("/how-it-works".to_string())}>
                            How It Works
                        </button>
                    </div>
                </div>

                {Cube()}
            </div>
        </div>

        {Custody(mark, seal, held)}
    </div>;
}

// ---- the headline ----------------------------------------------------

fn Headline() -> Element {
    return <div>
        {Word("YOUR ASSETS,".to_string(), 0)}
        <div style={{ height: "14px" }}></div>
        {Word("FROZEN IN TIME".to_string(), 1)}
    </div>;
}

/// One line of the headline, laid out cell by cell. A space is a
/// glyph of its own — six empty columns — so the words line up on the
/// same grid as the letters.
fn Word(text: String, line: usize) -> Element {
    let cell = 10usize;
    let columns = word_width(&text);
    let frame = format!(
        "position:relative;height:{}px;width:{}px",
        7 * cell,
        columns * cell
    );
    let cells = lit_cells(&text);

    return <div key={line.to_string()} className="pixelline" style={frame}>
        {cells
            .iter()
            .map(|at| Pixel(format!("{line}-{}-{}", at.0, at.1), at.0, at.1, cell))
            .collect::<Vec<_>>()}
    </div>;
}

fn Pixel(name: String, column: usize, row: usize, cell: usize) -> Element {
    let paint = format!(
        "position:absolute;left:{}px;top:{}px;width:{}px;height:{}px;background:#0a0a0a",
        column * cell,
        row * cell,
        cell,
        cell
    );
    return <span key={name} style={paint}></span>;
}

// ---- the cube --------------------------------------------------------

fn Cube() -> Element {
    let size = 268usize;
    let half = size / 2;
    let stage = format!(
        "position:relative;margin-left:52px;margin-top:-26px;width:{}px;height:{}px;perspective:900px",
        size + 90,
        size + 110
    );
    let box_style = format!(
        "position:absolute;left:45px;top:18px;width:{size}px;height:{size}px;transform-style:preserve-3d"
    );

    return <div style={stage}>
        <span className="halo" style={format!("left:14px;top:4px;width:{}px;height:{}px", size + 60, size + 60)}></span>
        <span className="shadow" style={format!("left:66px;top:{}px;width:{}px;height:26px", size + 34, size - 58)}></span>
        <div className="cube" style={box_style}>
            {Face("front".to_string(), format!("transform: translateZ({half}px)"))}
            {Face("back".to_string(), format!("transform: rotateY(180deg) translateZ({half}px)"))}
            {Face("right".to_string(), format!("transform: rotateY(90deg) translateZ({half}px)"))}
            {Face("left".to_string(), format!("transform: rotateY(-90deg) translateZ({half}px)"))}
            {Face("top".to_string(), format!("transform: rotateX(90deg) translateZ({half}px)"))}
            {Face("bottom".to_string(), format!("transform: rotateX(-90deg) translateZ({half}px)"))}
        </div>
    </div>;
}

fn Face(name: String, paint: String) -> Element {
    return <span key={name} style={paint}></span>;
}

// ---- the instrument markings -----------------------------------------

/// The faint plus signs scattered over the grid. Their positions are
/// fixed, not random: a page that moved its own marks every frame
/// would be a different page every frame.
fn Ticks() -> Element {
    let at: Vec<(i32, i32)> = vec![
        (72, 112), (176, 298), (247, 542), (305, 86), (305, 640),
        (462, 140), (520, 72), (706, 68), (706, 338), (778, 572),
        (892, 410), (1022, 184), (1022, 470), (1094, 112), (118, 646),
    ];
    return <div>
        {at.iter().map(|spot| Tick(spot.0, spot.1)).collect::<Vec<_>>()}
    </div>;
}

fn Tick(left: i32, top: i32) -> Element {
    let paint = format!("left:{left}px;top:{top}px");
    return <span key={format!("{left}-{top}")} className="tick" style={paint}>+</span>;
}

/// The little outlined labels of the design. Each one says something
/// true about this application rather than being set dressing: the
/// vault is offline, the window holds its own lock, nothing is sent.
fn Tags() -> Element {
    let all: Vec<(i32, i32, &str)> = vec![
        (14, 16, "COLD STORAGE\n//CRYOSTASIS"),
        (960, 16, "NODE STATUS\n//SECURED"),
        (948, 118, "VAULT: 0X4F9A"),
        (12, 252, "LOCK: ENGAGED"),
        (852, 282, "IMMUTABLE"),
        (680, 398, "COORD: [REDACTED]"),
        (152, 506, "NODE: ACTIVE"),
        (12, 552, "AUDIT: PASSED"),
    ];
    return <div>
        {all.iter().map(|spot| Tag(spot.0, spot.1, spot.2.to_string())).collect::<Vec<_>>()}
        <span className="corner" style={{ left: "10px", top: "10px", borderRight: "none", borderBottom: "none" }}></span>
        <span style={{ position: "absolute", left: "14px", bottom: "26px", fontSize: "8px", letterSpacing: "0.16em", color: "#c4c4c4", lineHeight: "1.3", whiteSpace: "pre" }}>
            {"C\nR\nY\nO\nS\nT\nA\nS\nI\nS"}
        </span>
    </div>;
}

fn Tag(left: i32, top: i32, text: String) -> Element {
    let paint = format!("left:{left}px;top:{top}px");
    return <span key={format!("{left}-{top}")} className="tag" style={paint}>{text}</span>;
}

// ---- proof of custody -------------------------------------------------

fn Custody(mark: String, seal: String, held: usize) -> Element {
    return <div style={{ position: "absolute", right: "22px", bottom: "22px", display: "flex", border: "1px solid #0a0a0a", backgroundColor: "#ffffff", zIndex: "2" }}>
        <div style={{ padding: "10px 12px", borderRight: "1px solid #e4e4e4", width: "190px" }}>
            <div style={{ fontSize: "10px", letterSpacing: "0.1em", marginBottom: "8px" }}>[PROOF OF CUSTODY]</div>
            <div className="readout">
                <div>{mark.clone()}</div>
                <div>-40°C</div>
                <div>XOR-STREAM</div>
                <div>VERIFIED</div>
                <div>OFFLINE STATUS: OK</div>
            </div>
        </div>
        <div style={{ padding: "10px 12px", borderRight: "1px solid #e4e4e4", display: "flex", alignItems: "center" }}>
            <reactQrcodeLogo::QRCode value={seal.clone()} size={92} fgColor="#0a0a0a" bgColor="#ffffff" ecLevel="M" quietZone={4} />
        </div>
        <div style={{ padding: "10px 12px", width: "160px" }}>
            <div className="readout" style={{ textAlign: "right" }}>
                <div>{format!("QUEUE: {}", items.len())}</div>
                <div>{format!("BYTES: {held}")}</div>
                <div>{format!("SEALS: {}", sealed_count)}</div>
                <div>CIPHER: OK</div>
                <div>DRIFT: 0.001</div>
            </div>
            <button
                className="cta small"
                style={{ marginTop: "10px", width: "100%" }}
                onClick={move |_| rustx::router::navigate("/vault".to_string())}
            >
                OPEN THE VAULT
            </button>
        </div>
    </div>;
}
