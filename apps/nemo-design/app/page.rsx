// Nemo Design.
//
// `Scene` is `components/Scene.tsx`, a real three.js viewport — not a
// placeholder picture. The controls below are plain rustx state, and
// cross into the viewport as props. "Ask an agent for a palette" is a
// stub, labelled as one: it picks from a short list and says why,
// standing in for the AI-assisted designing the showcase copy
// describes.

let mut color = "#8b5cf6".to_string();
let mut wireframe = false;
let mut spin: f64 = 0.6;
let mut agent_note = String::new();

fn Page() -> Element {
    return <div style={{ fontFamily: "Segoe UI, system-ui, sans-serif", padding: "24px", backgroundColor: "#0a0a12", color: "#e8e8f5", minHeight: "100vh" }}>
        <h1 style={{ margin: "0 0 4px", fontSize: "22px" }}>Nemo Design</h1>
        <p style={{ margin: "0 0 18px", fontSize: "13px", color: "#9490c0" }}>a real three.js viewport, with a stubbed design agent.</p>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 280px", gap: "20px" }}>
            <Scene color={color.clone()} wireframe={wireframe} spin={spin} />
            {Controls()}
        </div>
    </div>;
}

fn Controls() -> Element {
    return <div style={{ border: "1px solid #241f3d", borderRadius: "10px", padding: "16px", backgroundColor: "#121026", display: "flex", flexDirection: "column", gap: "14px" }}>
        <div>
            <label style={{ display: "flex", alignItems: "center", gap: "6px", fontSize: "12px", color: "#a99bf0" }}>
                <lucideReact::Palette size={14} /> color
            </label>
            <div style={{ display: "flex", gap: "6px", marginTop: "6px" }}>
                {ColorDot("#8b5cf6".to_string())}
                {ColorDot("#34d399".to_string())}
                {ColorDot("#f472b6".to_string())}
                {ColorDot("#60a5fa".to_string())}
                {ColorDot("#fbbf24".to_string())}
            </div>
        </div>

        <label style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "13px" }}>
            <input id="wireframe" type="checkbox" checked={wireframe} onChange={move |_| wireframe = !wireframe} />
            <lucideReact::Box size={14} /> wireframe
        </label>

        <label style={{ fontSize: "12px", color: "#a99bf0" }}>
            <span style={{ display: "inline-flex", alignItems: "center", gap: "6px" }}><lucideReact::RotateCw size={14} /> spin speed ({format!("{:.1}", spin)})</span>
            <input type="range" min="0" max="3" step="0.1" value={spin.to_string()}
                onInput={move |event| spin = event.number().unwrap_or(0.6)}
                style={{ width: "100%" }} />
        </label>

        <button id="ask-agent" onClick={move |_| {
            let (picked, why) = suggest_palette();
            color = picked;
            agent_note = why;
        }} style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: "6px", padding: "8px 12px", backgroundColor: "#7c5cff", color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer" }}>
            <lucideReact::Sparkles size={14} /> ask an agent for a palette
        </button>
        {if agent_note.is_empty() {
            <span></span>
        } else {
            <p id="agent-note" style={{ fontSize: "11px", color: "#6f69a0", margin: "0" }}>{agent_note.clone()}</p>
        }}
    </div>;
}

fn ColorDot(value: String) -> Element {
    let active = color == value;
    let pick = value.clone();
    // The swatch *is* the colour, so the declaration is computed: a
    // `style={{ … }}` object is read at compile time and takes only
    // literal strings.
    let dot = format!(
        "width: 26px; height: 26px; border-radius: 50%; cursor: pointer; \
         background-color: {}; border: 2px solid {}",
        value,
        if active { "#ffffff" } else { "transparent" }
    );

    return <button onClick={move |_| color = pick.clone()} style={dot}></button>;
}

/// A stubbed design agent: deterministic, and honest about being one.
fn suggest_palette() -> (String, String) {
    let options = [
        ("#8b5cf6", "violet reads well against the dark viewport background"),
        ("#34d399", "green separates cleanly from the ambient light"),
        ("#60a5fa", "blue keeps the wireframe edges legible"),
    ];
    let pick = options[(spin * 10.0) as usize % options.len()];
    (pick.0.to_string(), format!("agent note: {} (stub — no model was called)", pick.1))
}
