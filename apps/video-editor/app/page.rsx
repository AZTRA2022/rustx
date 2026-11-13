// Altlassian Raw.
//
// Three mock clips, a timeline sized by their duration, trim handles,
// a color filter, a subtitle field — the shape of a cut, not an
// actual decode/encode pipeline. "Render" below runs a real
// background loop that advances a real progress value; it does not
// invoke ffmpeg, and says so once it finishes.

#[derive(Clone)]
struct Clip {
    name: String,
    duration: f64,
    trim_start: f64,
    trim_end: f64,
    filter: String,
    subtitle: String,
}

let mut clips: Vec<Clip> = vec![
    Clip { name: "intro.mp4".to_string(), duration: 12.0, trim_start: 0.0, trim_end: 12.0, filter: "none".to_string(), subtitle: String::new() },
    Clip { name: "demo.mp4".to_string(), duration: 28.0, trim_start: 2.0, trim_end: 24.0, filter: "warm".to_string(), subtitle: "Watch this part".to_string() },
    Clip { name: "outro.mp4".to_string(), duration: 6.0, trim_start: 0.0, trim_end: 6.0, filter: "none".to_string(), subtitle: String::new() },
];
let mut selected: usize = 0;
let mut rendering = false;
let mut progress: f64 = 0.0;
let mut rendered = false;

fn Page() -> Element {
    async move {
        loop {
            if !rendering {
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                continue;
            }
            progress += 4.0;
            if progress >= 100.0 {
                progress = 100.0;
                rendering = false;
                rendered = true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(90)).await;
        }
    };

    return <div style={{ fontFamily: "Segoe UI, system-ui, sans-serif", padding: "24px", backgroundColor: "#0a0a0f", color: "#eceef5", minHeight: "100vh" }}>
        <h1 style={{ margin: "0 0 4px", fontSize: "22px" }}>Altlassian Raw</h1>
        <p style={{ margin: "0 0 18px", fontSize: "13px", color: "#8f93ad" }}>cut, trim, color, subtitle — a shell, with no ffmpeg process behind it.</p>

        {Timeline()}
        {Editor()}
        {Render()}
    </div>;
}

fn Timeline() -> Element {
    let total: f64 = clips.iter().map(|clip| clip.duration).sum::<f64>().max(1.0);
    return <div id="timeline" style={{ display: "flex", gap: "2px", height: "56px", marginBottom: "18px", borderRadius: "8px", overflow: "hidden" }}>
        {clips.iter().enumerate().map(|(index, clip)| ClipBlock(index, clip.clone(), total)).collect::<Vec<_>>()}
    </div>;
}

fn ClipBlock(index: usize, clip: Clip, total: f64) -> Element {
    let active = selected == index;
    // A clip's width on the timeline is its share of the sequence, so
    // the declaration is computed rather than written as an object of
    // literals.
    let block = format!(
        "width: {:.3}%; color: #eceef5; display: flex; align-items: center; \
         justify-content: center; font-size: 11px; cursor: pointer; overflow: hidden; \
         background-color: {}; border: 2px solid {}",
        (clip.duration / total) * 100.0,
        if active { "#4338ca" } else { "#1c2030" },
        if active { "#818cf8" } else { "transparent" }
    );

    return <div key={index.to_string()} onClick={move |_| selected = index} style={block}>{clip.name.clone()}</div>;
}

fn Editor() -> Element {
    if clips.is_empty() {
        return <p>no clips.</p>;
    }
    let index = selected.min(clips.len() - 1);
    let clip = clips[index].clone();

    return <div style={{ border: "1px solid #1c2030", borderRadius: "10px", padding: "16px", backgroundColor: "#11131c", marginBottom: "18px" }}>
        <h2 style={{ margin: "0 0 12px", fontSize: "13px", color: "#9499b8" }}>{clip.name.clone()} — {format!("{:.0}s", clip.duration)}</h2>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
            <label style={{ fontSize: "12px", color: "#8f93ad" }}>
                trim start ({format!("{:.1}s", clip.trim_start)})
                <input type="range" min="0" max={clip.duration.to_string()} step="0.5" value={clip.trim_start.to_string()}
                    onInput={move |event| {
                        if let Some(clip) = clips.get_mut(index) {
                            clip.trim_start = event.number().unwrap_or(0.0).min(clip.trim_end);
                        }
                    }}
                    style={{ width: "100%" }} />
            </label>
            <label style={{ fontSize: "12px", color: "#8f93ad" }}>
                trim end ({format!("{:.1}s", clip.trim_end)})
                <input type="range" min="0" max={clip.duration.to_string()} step="0.5" value={clip.trim_end.to_string()}
                    onInput={move |event| {
                        if let Some(clip) = clips.get_mut(index) {
                            clip.trim_end = event.number().unwrap_or(clip.duration).max(clip.trim_start);
                        }
                    }}
                    style={{ width: "100%" }} />
            </label>
        </div>

        <div style={{ display: "flex", gap: "10px", marginTop: "14px", alignItems: "center" }}>
            <span style={{ fontSize: "12px", color: "#8f93ad" }}>color</span>
            {FilterButton(index, "none".to_string(), clip.filter.clone())}
            {FilterButton(index, "warm".to_string(), clip.filter.clone())}
            {FilterButton(index, "cool".to_string(), clip.filter.clone())}
            {FilterButton(index, "bw".to_string(), clip.filter.clone())}
        </div>

        <input id="subtitle" placeholder="subtitle for this clip" value={clip.subtitle.clone()}
            onInput={move |event| {
                if let Some(clip) = clips.get_mut(index) { clip.subtitle = event.value; }
            }}
            style={{ width: "100%", marginTop: "12px", padding: "7px", backgroundColor: "#0a0a0f", border: "1px solid #1c2030", color: "#eceef5", borderRadius: "6px" }} />
    </div>;
}

fn FilterButton(index: usize, name: String, active_name: String) -> Element {
    let active = name == active_name;
    let apply = name.clone();
    let chip = format!(
        "padding: 4px 10px; font-size: 11px; border-radius: 6px; color: #eceef5; \
         cursor: pointer; border: 1px solid {}; background-color: {}",
        if active { "#818cf8" } else { "#1c2030" },
        if active { "#1a1f36" } else { "transparent" }
    );

    return <button onClick={move |_| {
        if let Some(clip) = clips.get_mut(index) { clip.filter = apply.clone(); }
    }} style={chip}>{name}</button>;
}

fn Render() -> Element {
    let filled = format!("height: 100%; width: {}%; background-color: #818cf8", progress as u32);

    return <div style={{ border: "1px solid #1c2030", borderRadius: "10px", padding: "16px", backgroundColor: "#11131c" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "10px" }}>
            <h2 style={{ margin: "0", fontSize: "13px", color: "#9499b8" }}>render</h2>
            <button id="render" onClick={move |_| {
                if !rendering {
                    rendering = true;
                    rendered = false;
                    progress = 0.0;
                }
            }} style={{ padding: "7px 14px", backgroundColor: "#4338ca", color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer" }}>
                {if rendering { "rendering…" } else { "render sequence" }}
            </button>
        </div>
        <div style={{ height: "10px", borderRadius: "5px", backgroundColor: "#1c2030", overflow: "hidden" }}>
            <div id="progress" style={filled}></div>
        </div>
        {if rendered {
            <p style={{ marginTop: "10px", fontSize: "12px", color: "#8f93ad" }}>
                done — simulated. No ffmpeg process ran; a real build would hand this timeline to one.
            </p>
        } else {
            <span></span>
        }}
    </div>;
}
