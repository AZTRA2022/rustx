// FOOTAGE — what came back from the floor.
//
// The library is the production's own. The three controls over it are
// not: the filter, the sort and the selection are real state, and
// "Copy the shot list" puts the chosen reels on the real clipboard.

let mut note: String = String::new();
let mut clip_filter: String = String::new();
let mut graded_only: bool = false;
let mut picked: Vec<String> = Vec::new();

fn Page() -> Element {
    let needle = clip_filter.to_lowercase();
    let clips: Vec<Clip> = footage()
        .into_iter()
        .filter(|clip| !graded_only || clip.graded)
        .filter(|clip| {
            needle.is_empty()
                || clip.scene.to_lowercase().contains(&needle)
                || clip.reel.to_lowercase().contains(&needle)
                || clip.department.to_lowercase().contains(&needle)
        })
        .collect();
    let all = footage();

    return <div key="footage" className="px-5 py-4">
        <div className="flex items-start">
            <div>
                <h1 className="text-[13px] font-semibold text-foreground">Footage</h1>
                <p className="mt-0.5 text-[11px] text-muted">
                    {format!("{} clips, {} graded. {} selected.", all.len(), all.iter().filter(|clip| clip.graded).count(), picked.len())}
                </p>
            </div>
            <div className="ml-auto flex items-center gap-2">
                <span className="flex items-center gap-2 rounded-md border border-border bg-[#11151b] px-2.5 py-[5px]">
                    <Search size={11} color="#5a6572" />
                    <input
                        id="clip-filter"
                        className="w-40 bg-transparent text-[11px] outline-none placeholder:text-faint"
                        placeholder="Scene, reel or unit"
                        value={clip_filter.clone()}
                        onInput={move |event| clip_filter = event.value}
                    />
                </span>
                <button
                    className="rounded-md border border-border bg-[#11151b] px-2.5 py-[6px] text-[11px] text-foreground hover:border-faint"
                    onClick={move |_| graded_only = !graded_only}
                >
                    {if graded_only { "Graded only" } else { "Everything" }}
                </button>
                <button
                    className="rounded-md bg-blue px-2.5 py-[6px] text-[11px] text-white hover:brightness-110 disabled:opacity-50"
                    disabled={picked.is_empty()}
                    onClick={move |_| copy_shot_list()}
                >
                    Copy the shot list
                </button>
            </div>
        </div>

        <div id="clips" className="mt-4 grid grid-cols-4 gap-2.5">
            {clips.iter().map(|clip| Card(clip.clone())).collect::<Vec<_>>()}
        </div>

        {if clips.is_empty() {
            <p className="mt-4 text-[11px] text-faint">No clip matches that.</p>
        } else {
            <span></span>
        }}
    </div>;
}

fn Card(clip: Clip) -> Element {
    let chosen = picked.contains(&clip.reel);
    let classes = format!(
        "rounded-md border p-3 text-left {}",
        if chosen { "border-blue bg-[#111a2b]" } else { "border-border bg-[#0f131a] hover:border-faint" }
    );
    // A strip of colour standing in for a thumbnail: the scene's own
    // hue, so two scenes never look alike and nothing pretends to be
    // a frame that does not exist.
    let seed: u32 = clip.scene.bytes().map(|byte| byte as u32).sum();
    let strip = format!(
        "height:52px;border-radius:4px;background:linear-gradient(120deg, hsl({} 38% 32%), hsl({} 38% 18%))",
        seed % 360,
        (seed + 50) % 360
    );
    let reel = clip.reel.clone();

    return <button key={clip.reel.clone()} className={classes} onClick={move |_| pick(reel.clone())}>
        <span className="block" style={strip}></span>
        <span className="mt-2 flex items-baseline">
            <span className="text-[12px] text-foreground">{format!("sc {}", clip.scene)}</span>
            <span className="ml-auto text-[10px] text-faint">{format!("tk {}", clip.take)}</span>
        </span>
        <span className="mt-1 flex items-baseline">
            <span className="text-[10px] text-muted">{clip.reel.clone()}</span>
            <span className="ml-auto text-[10px] tabular-nums text-muted">{clip.length.clone()}</span>
        </span>
        <span className="mt-1.5 flex items-center gap-1.5">
            <span className="text-[9px] text-faint">{clip.department.clone()}</span>
            {if clip.graded {
                <span className="ml-auto rounded-[3px] bg-[#11291d] px-1.5 py-[1px] text-[9px] text-[#4ec98a]">GRADED</span>
            } else {
                <span className="ml-auto rounded-[3px] bg-[#1a1f27] px-1.5 py-[1px] text-[9px] text-[#9aa5b1]">RUSH</span>
            }}
        </span>
    </button>;
}

fn pick(reel: String) {
    let mut all = picked.clone();
    if let Some(at) = all.iter().position(|held| held == &reel) {
        all.remove(at);
    } else {
        all.push(reel);
    }
    note = format!("{} clip(s) selected", all.len());
    picked = all;
}

fn copy_shot_list() {
    let chosen = picked.clone();
    let text = footage()
        .into_iter()
        .filter(|clip| chosen.contains(&clip.reel))
        .map(|clip| format!("{} · sc {} tk {} · {}", clip.reel, clip.scene, clip.take, clip.length))
        .collect::<Vec<_>>()
        .join("\n");
    match rustx::clipboard::set_text(&text) {
        Ok(()) => note = format!("{} clip(s) on the clipboard", chosen.len()),
        Err(error) => note = format!("the clipboard refused: {error}"),
    }
}
