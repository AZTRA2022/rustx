// Altlassian Raw.
//
// Real files on the timeline, real numbers from `ffprobe`, a real
// decoded frame for the preview, and a render that is one `ffmpeg`
// process writing a playable file. Everything that builds a command
// line is in `src/ffmpeg.rs`, with its tests; this file is the editor.
//
// What the preview is: a still. One frame, decoded again when the
// playhead, the trim or the colour of the selected clip changes.
// Playing the sequence inside the window would mean a decoder in the
// render process, which is a different piece of software — so there is
// no play button pretending otherwise.

let mut clips: Vec<ffmpeg::Clip> = Vec::new();
let mut selected: usize = 0;
// Seconds after the selected clip's in-point.
let mut playhead: f64 = 0.0;
let mut typed: String = String::new();
let mut status: String = String::new();

// The preview, and the two counters that drive it. A number asked for
// and a number drawn, rather than a boolean: the frame is decoded by a
// task, and a boolean would have lost the clicks that arrived while it
// was decoding.
let mut frame: String = String::new();
let mut want_frame: u64 = 0;
let mut drawn_frame: u64 = 0;

// The same pair for the render, plus where it goes — the destination is
// chosen in the click, because a file dialog belongs to the gesture
// that asked for it.
let mut want_render: u64 = 0;
let mut done_render: u64 = 0;
let mut target: String = String::new();
let mut rendering: bool = false;
let mut progress: f64 = 0.0;
let mut output: String = String::new();

fn Page() -> Element {
    // One task, two programs. It belongs to the page, so it stops when
    // the page goes; it waits rather than polls hard, because the work
    // it does is starting processes.
    async move {
        loop {
            rustx::time::sleep(std::time::Duration::from_millis(90)).await;

            let Ok(tools) = ffmpeg::tools() else {
                continue;
            };

            if drawn_frame != want_frame {
                let wanted = want_frame;
                let held = clips.clone();
                match held.get(selected.min(held.len().saturating_sub(1))) {
                    None => frame = String::new(),
                    Some(clip) => {
                        let at = (clip.start + playhead).min(clip.end).max(clip.start);
                        let file = beside(&format!("frame-{wanted}.jpg"));
                        match ffmpeg::extract(&tools.ffmpeg, clip, at, &file).await {
                            Ok(()) => {
                                let _ = std::fs::remove_file(beside(&format!(
                                    "frame-{drawn_frame}.jpg"
                                )));
                                frame = shown(&file);
                            }
                            Err(why) => {
                                frame = String::new();
                                status = why;
                            }
                        }
                    }
                }
                drawn_frame = wanted;
            }

            if done_render != want_render {
                let wanted = want_render;
                let held = clips.clone();
                let out = std::path::PathBuf::from(target.clone());
                match ffmpeg::plan(&held, &out, &beside(""), ffmpeg::font_file().as_deref()) {
                    Err(why) => status = why,
                    Ok(plan) => {
                        // The named shortcut of `plan`, said while it
                        // happens and still said afterwards rather than
                        // discovered when the file is played.
                        let silent = match plan.audio {
                            true => String::new(),
                            false => "the output is silent: one clip has no audio stream, so \
                                      the sequence cannot carry sound."
                                .to_string(),
                        };
                        status = silent.clone();
                        match ffmpeg::run(&tools.ffmpeg, &plan, |done| progress = done).await {
                            Ok(()) => {
                                output = out.display().to_string();
                                status = silent;
                            }
                            Err(why) => {
                                status = why;
                                progress = 0.0;
                            }
                        }
                    }
                }
                rendering = false;
                done_render = wanted;
            }
        }
    };

    return <div style={{ fontFamily: "Segoe UI, system-ui, sans-serif", padding: "24px", backgroundColor: "#0a0a0f", color: "#eceef5", minHeight: "100vh" }}>
        <h1 style={{ margin: "0 0 4px", fontSize: "22px" }}>Altlassian Raw</h1>
        <p style={{ margin: "0 0 18px", fontSize: "13px", color: "#8f93ad" }}>
            cut, trim, colour, subtitle — over ffmpeg, on your own files.
        </p>

        {Tooling()}
        {Open()}
        {Timeline()}
        {Preview()}
        {Editor()}
        {Render()}
        {Said()}
    </div>;
}

// Whether the two programs are even here, and what to do when they are
// not. The same answer the rest of this repository gives about a tool
// it does not ship: name it, say what installs it.
fn Tooling() -> Element {
    let Err(why) = ffmpeg::tools() else {
        return <span></span>;
    };

    return <div id="no-ffmpeg" style={{ border: "1px solid #7f1d1d", backgroundColor: "#1b0f12", borderRadius: "10px", padding: "14px", marginBottom: "18px", fontSize: "12px", lineHeight: "1.6", color: "#fca5a5" }}>
        {why.clone()}
    </div>;
}

fn Open() -> Element {
    let button = "padding: 7px 14px; background-color: #1c2030; color: #eceef5; \
                  border: 1px solid #2a3147; border-radius: 6px; cursor: pointer; font-size: 12px";

    return <div style={{ display: "flex", gap: "8px", marginBottom: "14px", alignItems: "center" }}>
        <button id="open" onClick={move |_| open_files()} style={button.to_string()}>open video files…</button>
        <input id="typed" placeholder="…or paste a path and press add"
            value={typed.clone()}
            onInput={move |event| typed = event.value}
            style={{ flex: "1", padding: "7px", backgroundColor: "#11131c", border: "1px solid #1c2030", color: "#eceef5", borderRadius: "6px", fontSize: "12px" }} />
        <button id="add-typed" onClick={move |_| add_typed()} style={button.to_string()}>add</button>
    </div>;
}

fn Timeline() -> Element {
    if clips.is_empty() {
        return <div id="timeline" style={{ border: "1px dashed #1c2030", borderRadius: "8px", padding: "18px", marginBottom: "18px", fontSize: "12px", color: "#5f6480" }}>
            nothing on the timeline yet.
        </div>;
    }

    let total: f64 = clips.iter().map(|clip| clip.length()).sum::<f64>().max(0.001);
    return <div id="timeline" style={{ display: "flex", gap: "2px", height: "56px", marginBottom: "14px", borderRadius: "8px", overflow: "hidden" }}>
        {clips.iter().enumerate().map(|(index, clip)| ClipBlock(index, clip.clone(), total)).collect::<Vec<_>>()}
    </div>;
}

fn ClipBlock(index: usize, clip: ffmpeg::Clip, total: f64) -> Element {
    let active = selected == index;
    // A clip's width is its real trimmed share of the sequence, so the
    // declaration is computed rather than written as an object of
    // literals.
    let block = format!(
        "width: {:.3}%; min-width: 48px; color: #eceef5; display: flex; flex-direction: column; \
         align-items: center; justify-content: center; font-size: 11px; cursor: pointer; \
         overflow: hidden; background-color: {}; border: 2px solid {}",
        (clip.length() / total) * 100.0,
        if active { "#4338ca" } else { "#1c2030" },
        if active { "#818cf8" } else { "transparent" }
    );

    return <div key={index.to_string()} className="clip" onClick={move |_| {
        selected = index;
        playhead = 0.0;
        want_frame += 1;
    }} style={block}>
        <span>{clip.name.clone()}</span>
        <span style={{ fontSize: "10px", color: "#b9bed6" }}>{format!("{:.1}s", clip.length())}</span>
    </div>;
}

fn Preview() -> Element {
    let box_style = "height: 300px; display: flex; align-items: center; justify-content: center; \
                     background-color: #07070b; border: 1px solid #1c2030; border-radius: 10px; \
                     margin-bottom: 14px; overflow: hidden; color: #5f6480; font-size: 12px";

    if frame.is_empty() {
        return <div id="preview" style={box_style.to_string()}>
            {if clips.is_empty() { "the preview is one real frame, decoded by ffmpeg — open a file" } else { "decoding a frame…" }}
        </div>;
    }

    // A `file://` URL, which this page is allowed to load: the engine
    // runs with `allow-file-access-from-files`. The file name carries
    // the counter, so the browser never shows a cached older frame.
    return <div id="preview" style={box_style.to_string()}>
        <img src={frame.clone()} style={{ maxHeight: "100%", maxWidth: "100%" }} />
    </div>;
}

fn Editor() -> Element {
    if clips.is_empty() {
        return <span></span>;
    }
    let index = selected.min(clips.len() - 1);
    let clip = clips[index].clone();
    let span = clip.length();

    return <div style={{ border: "1px solid #1c2030", borderRadius: "10px", padding: "16px", backgroundColor: "#11131c", marginBottom: "18px" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline", marginBottom: "12px" }}>
            <h2 id="clip-name" style={{ margin: "0", fontSize: "13px", color: "#9499b8" }}>{clip.name.clone()}</h2>
            <span id="clip-media" style={{ fontSize: "11px", color: "#5f6480" }}>
                {format!(
                    "{:.2}s · {}×{} · {:.2} fps · {}",
                    clip.media.duration,
                    clip.media.width,
                    clip.media.height,
                    clip.media.fps,
                    if clip.media.audio { "sound" } else { "no sound" }
                )}
            </span>
        </div>

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
            <label style={{ fontSize: "12px", color: "#8f93ad" }}>
                trim in ({format!("{:.2}s", clip.start)})
                <input type="range" min="0" max={clip.media.duration.to_string()} step="0.04" value={clip.start.to_string()}
                    onInput={move |event| edit(index, |clip| {
                        clip.start = event.number().unwrap_or(0.0).min(clip.end - 0.04).max(0.0);
                    })}
                    style={{ width: "100%" }} />
            </label>
            <label style={{ fontSize: "12px", color: "#8f93ad" }}>
                trim out ({format!("{:.2}s", clip.end)})
                <input type="range" min="0" max={clip.media.duration.to_string()} step="0.04" value={clip.end.to_string()}
                    onInput={move |event| edit(index, |clip| {
                        let duration = clip.media.duration;
                        clip.end = event.number().unwrap_or(duration).max(clip.start + 0.04).min(duration);
                    })}
                    style={{ width: "100%" }} />
            </label>
        </div>

        <label style={{ fontSize: "12px", color: "#8f93ad", display: "block", marginTop: "14px" }}>
            {format!("playhead ({:.2}s into the cut)", playhead)}
            <input id="playhead" type="range" min="0" max={span.to_string()} step="0.04" value={playhead.to_string()}
                onInput={move |event| {
                    playhead = event.number().unwrap_or(0.0);
                    want_frame += 1;
                }}
                style={{ width: "100%" }} />
        </label>

        <div style={{ display: "flex", gap: "10px", marginTop: "14px", alignItems: "center" }}>
            <span style={{ fontSize: "12px", color: "#8f93ad" }}>colour</span>
            {FilterButton(index, "none".to_string(), clip.look.clone())}
            {FilterButton(index, "warm".to_string(), clip.look.clone())}
            {FilterButton(index, "cool".to_string(), clip.look.clone())}
            {FilterButton(index, "bw".to_string(), clip.look.clone())}
            <span style={{ fontSize: "10px", color: "#5f6480" }}>{ffmpeg::look_filter(&clip.look).to_string()}</span>
        </div>

        <input id="subtitle" placeholder="subtitle burned into this clip" value={clip.subtitle.clone()}
            onInput={move |event| set_subtitle(index, event.value.clone())}
            style={{ width: "100%", marginTop: "12px", padding: "7px", backgroundColor: "#0a0a0f", border: "1px solid #1c2030", color: "#eceef5", borderRadius: "6px" }} />

        <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
            {SmallButton("move left".to_string(), "move-left".to_string(), index > 0, -1)}
            {SmallButton("move right".to_string(), "move-right".to_string(), index + 1 < clips.len(), 1)}
            {SmallButton("remove".to_string(), "remove".to_string(), true, 0)}
        </div>
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

    return <button className="look" onClick={move |_| {
        edit(index, |clip| clip.look = apply.clone())
    }} style={chip}>{name}</button>;
}

// The three that change the order of the sequence. `by` is where the
// clip goes: one place left, one place right, or nowhere, which is the
// one that removes it.
fn SmallButton(label: String, id: String, enabled: bool, by: i64) -> Element {
    let style = format!(
        "padding: 5px 10px; font-size: 11px; border-radius: 6px; border: 1px solid #2a3147; \
         background-color: #171b28; color: {}; cursor: {}",
        if enabled { "#eceef5" } else { "#4a4f66" },
        if enabled { "pointer" } else { "default" }
    );
    return <button id={id} onClick={move |_| {
        if !enabled {
            return;
        }
        match by {
            0 => remove_clip(),
            by => shift(by),
        }
    }} style={style}>{label}</button>;
}

fn Render() -> Element {
    let filled = format!(
        "height: 100%; width: {:.1}%; background-color: #818cf8; transition: width 120ms linear",
        progress
    );

    return <div style={{ border: "1px solid #1c2030", borderRadius: "10px", padding: "16px", backgroundColor: "#11131c" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "10px" }}>
            <h2 style={{ margin: "0", fontSize: "13px", color: "#9499b8" }}>render</h2>
            <button id="render" onClick={move |_| start_render()} style={{ padding: "7px 14px", backgroundColor: "#4338ca", color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer" }}>
                {if rendering { "rendering…" } else { "render sequence" }}
            </button>
        </div>
        <div style={{ height: "10px", borderRadius: "5px", backgroundColor: "#1c2030", overflow: "hidden" }}>
            <div id="progress" style={filled}></div>
        </div>
        {if output.is_empty() {
            <p style={{ marginTop: "10px", fontSize: "11px", color: "#5f6480" }}>
                {"the bar follows what ffmpeg itself reports as it writes, which is why it moves unevenly."}
            </p>
        } else {
            <div style={{ marginTop: "10px", display: "flex", gap: "10px", alignItems: "center", fontSize: "12px" }}>
                <span id="output" style={{ color: "#86efac" }}>{output.clone()}</span>
                <button id="reveal" onClick={move |_| reveal_output()} style={{ padding: "4px 10px", fontSize: "11px", borderRadius: "6px", border: "1px solid #2a3147", backgroundColor: "#171b28", color: "#eceef5", cursor: "pointer" }}>show in folder</button>
            </div>
        }}
    </div>;
}

// Whatever went wrong last, in the words of whatever refused.
fn Said() -> Element {
    if status.is_empty() {
        return <span></span>;
    }

    return <p id="status" style={{ marginTop: "14px", fontSize: "12px", lineHeight: "1.6", color: "#fca5a5", whiteSpace: "pre-wrap" }}>
        {status.clone()}
    </p>;
}

// ---- what the buttons do ---------------------------------------------

fn open_files() {
    let ask = rustx::dialog::Ask::new("Open video files")
        .filter("Video", ["mp4", "mov", "mkv", "webm", "avi", "m4v"])
        .many();
    let Ok(paths) = rustx::dialog::open(ask) else {
        return;
    };
    for path in paths {
        add(&path);
    }
}

fn add_typed() {
    let written = typed.trim().trim_matches('"').to_string();
    if written.is_empty() {
        return;
    }
    add(std::path::Path::new(&written));
    typed = String::new();
}

// Read a file's real numbers, and put it at the end of the sequence.
//
// shortcut: `ffprobe` is run here, in the click, rather than in the
// task. It is one short process per file and the dialog that chose the
// file blocked anyway; a folder of a hundred clips would want it moved.
fn add(path: &std::path::Path) {
    let Ok(tools) = ffmpeg::tools() else {
        status = ffmpeg::MISSING.to_string();
        return;
    };
    if !path.is_file() {
        status = format!("{} is not a file.", path.display());
        return;
    }
    match ffmpeg::probe(&tools.ffprobe, path) {
        Err(why) => status = why,
        Ok(media) => {
            let mut held = clips.clone();
            held.push(ffmpeg::Clip::new(media));
            selected = held.len() - 1;
            clips = held;
            playhead = 0.0;
            want_frame += 1;
            status = String::new();
        }
    }
}

// Change a clip in place, and ask for the preview again — which is what
// the trim and the colour need and the subtitle does not.
fn edit(index: usize, change: impl FnOnce(&mut ffmpeg::Clip)) {
    let mut held = clips.clone();
    let Some(clip) = held.get_mut(index) else {
        return;
    };
    change(clip);
    let span = clip.length();
    clips = held;
    playhead = playhead.min(span);
    want_frame += 1;
}

// Not through `edit`, which asks for the preview again: the subtitle is
// burned in by the render and is not in the still, so a frame per
// keystroke would be an ffmpeg process per keystroke for nothing.
fn set_subtitle(index: usize, text: String) {
    let mut held = clips.clone();
    let Some(clip) = held.get_mut(index) else {
        return;
    };
    clip.subtitle = text;
    clips = held;
}

fn shift(by: i64) {
    let mut held = clips.clone();
    let from = selected.min(held.len().saturating_sub(1));
    let to = from as i64 + by;
    if to < 0 || to as usize >= held.len() {
        return;
    }
    let to = to as usize;
    held.swap(from, to);
    clips = held;
    selected = to;
}

fn remove_clip() {
    let mut held = clips.clone();
    if held.is_empty() {
        return;
    }
    let index = selected.min(held.len() - 1);
    held.remove(index);
    selected = index.min(held.len().saturating_sub(1));
    clips = held;
    frame = String::new();
    playhead = 0.0;
    want_frame += 1;
}

// Where the render landed, in the file manager.
fn reveal_output() {
    let written = output.clone();
    match rustx::shell::reveal(std::path::Path::new(&written)) {
        Ok(()) => {}
        Err(error) => status = format!("the folder could not be opened: {error}"),
    }
}

fn start_render() {
    if rendering {
        return;
    }
    if ffmpeg::tools().is_err() {
        status = ffmpeg::MISSING.to_string();
        return;
    }
    if clips.is_empty() {
        status = "there is nothing on the timeline to render.".to_string();
        return;
    }
    let ask = rustx::dialog::Ask::new("Render the sequence to")
        .filter("MP4 video", ["mp4"])
        .named("render.mp4");
    let Ok(path) = rustx::dialog::save(ask) else {
        return;
    };

    target = path.display().to_string();
    output = String::new();
    status = String::new();
    progress = 0.0;
    rendering = true;
    want_render += 1;
}

// ---- where the files this application writes itself go ----------------

// The extracted frames and the subtitle files ffmpeg reads. The cache
// directory, because every one of them can be deleted at any moment
// and nothing is lost.
fn beside(name: &str) -> std::path::PathBuf {
    let directory = rustx::data::cache().unwrap_or_else(|_| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&directory);
    directory.join(name)
}

// A path as an `<img src>`. `file:///C:/…`, with the separators the URL
// wants rather than the ones Windows writes.
fn shown(path: &std::path::Path) -> String {
    let slashed = path.display().to_string().replace('\\', "/");
    format!("file:///{}", slashed.trim_start_matches('/'))
}
