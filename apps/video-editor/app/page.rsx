// Altlassian Raw.
//
// Real files on the timeline, real numbers from `ffprobe`, the real
// file playing in the monitor with its sound, and a render that is one
// `ffmpeg` process writing a playable file. Everything that builds a
// command line is in `src/ffmpeg.rs`, with its tests; the three natives
// the transport talks through are in `src/main.rs`; this file is the
// editor.
//
// What is real and what is not, said once, here:
//
//   - the CEF build this window runs on has no H.264 and no AAC, so
//     an mp4 does not play in it and a WebM does. Rather than refuse
//     most footage, ffmpeg builds a preview proxy of such a file once
//     and the monitor plays that; the render always uses the original;
//   - the monitor is a `<video>` playing the file off disk. Play,
//     pause, stop, seek, frame stepping, in and out points, volume and
//     the keyboard are all that element, driven from Rust;
//   - playing past a clip's out-point moves to the next clip and keeps
//     going, so the sequence plays as the cut — but the hand-off is a
//     reload of the element, so there is a gap of a frame or two at
//     each cut that the rendered file does not have;
//   - J cannot be reverse play. Chromium refuses a negative
//     `playbackRate`, so J steps a second back and pauses, and the
//     shortcut line says so;
//   - the colour filter is previewed only where the preview is honest.
//     `bw` is `grayscale(1)`, which is what `format=gray` does. Warm
//     and cool are `colortemperature` in ffmpeg and CSS has no such
//     thing, so the panel says they are applied at render instead of
//     showing an approximation that is not the result;
//   - there are no waveforms and no thumbnails along the track. Both
//     are real work and neither is faked here.
//
// The still that `ffmpeg` can extract is kept for one job: the monitor
// when Chromium cannot decode the file. That is a real fallback rather
// than a second preview.

let mut clips: Vec<ffmpeg::Clip> = Vec::new();
let mut selected: usize = 0;
let mut typed: String = String::new();
let mut status: String = String::new();

// Written by the natives in `src/main.rs`, from the element itself:
// where the video is, whether it is moving, whether it decoded at all,
// and the last key the window saw.
let mut played: f64 = 0.0;
let mut paused: bool = true;
let mut decodes: bool = true;
let mut pressed: String = String::new();
let mut presses: u64 = 0;
// let mut var = rustx


// What the element is told to do next, read by the transport script.
// The counter is in the string so that the same command twice over is
// still two commands.
let mut command: String = String::new();
let mut issued: u64 = 0;

let mut volume: f64 = 1.0;
let mut muted: bool = false;
let mut rate: f64 = 1.0;

// The fallback still, and the two counters that drive it. A number
// asked for and a number drawn, rather than a boolean: ffmpeg is a
// process, and a boolean would have lost what arrived while it ran.
// Whether a preview proxy is being built right now, and for which
// file — see `Picture`.
let mut building: bool = false;

let mut still: String = String::new();
let mut want_still: u64 = 0;
let mut drawn_still: u64 = 0;

// The render, plus where it goes — the destination is chosen in the
// click, because a file dialog belongs to the gesture that asked for
// it.
let mut want_render: u64 = 0;
let mut done_render: u64 = 0;
let mut target: String = String::new();
let mut rendering: bool = false;
let mut progress: f64 = 0.0;
let mut output: String = String::new();

fn Page() -> Element {
    // One task: the proxy, the still and the render — the three
    // things that are an ffmpeg process. It belongs to the page, so it
    // stops when the page goes.
    //
    // Everything in here reads the page's values directly and calls no
    // function of this file. That is not a style: this runs on the
    // runtime's own thread, and a plain function that names a value by
    // its name looks it up in the store of the thread it is on — which
    // is not the one that draws, so it finds an empty one. Reading
    // `clips` here is a handle the component captured, and that one
    // goes to where the value really is. It cost an afternoon.
    async move {
        loop {
            rustx::time::sleep(std::time::Duration::from_millis(50)).await;

            let held = clips.clone();
            let here = selected.min(held.len().saturating_sub(1));

            let Ok(tools) = ffmpeg::tools() else {
                continue;
            };

            // Something to look at straight away, and the whole of the
            // monitor until the proxy below is written: one frame,
            // drawn by ffmpeg at the playhead. It goes first because
            // building the proxy is awaited, and a minute of black is
            // not an interface.
            if !decodes && drawn_still != want_still {
                let wanted = want_still;
                match held.get(here) {
                    None => still = String::new(),
                    Some(clip) => {
                        let file = beside(&format!("still-{wanted}.jpg"));
                        match ffmpeg::extract(&tools.ffmpeg, clip, played, &file).await {
                            Ok(()) => {
                                let _ = std::fs::remove_file(beside(&format!(
                                    "still-{drawn_still}.jpg"
                                )));
                                still = as_url(&file);
                            }
                            Err(why) => {
                                still = String::new();
                                status = why;
                            }
                        }
                    }
                }
                drawn_still = wanted;
            }

            // The window refused this file and there is no proxy for
            // it yet: build one. It is the whole file through ffmpeg,
            // so it takes a while on a long clip and says so while it
            // runs.
            let wanting = held.get(here).filter(|_| !decodes && !building);
            if let Some(clip) = wanting {
                let proxy = beside(&ffmpeg::proxy_name(&clip.media.path));
                if !proxy.is_file() {
                    building = true;
                    progress = 0.0;
                    status = format!(
                        "{} cannot be played by this window. ffmpeg is building a preview proxy of it; the render uses the original file.",
                        clip.name
                    );
                    let plan = ffmpeg::proxy_plan(&clip.media, &proxy);
                    match ffmpeg::run(&tools.ffmpeg, &plan, |done| progress = done).await {
                        Ok(()) => status = String::new(),
                        Err(why) => status = why,
                    }
                    progress = 0.0;
                    building = false;
                }
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
                            false => "The render is silent: one clip has no audio stream, so \
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

    return <div className="flex h-screen flex-col overflow-hidden bg-chrome text-text">
        {Keys()}
        {Toolbar()}
        {Tooling()}
        <div className="flex min-h-0 flex-1">
            <div className="flex min-w-0 flex-1 flex-col">
                {Monitor()}
                {Sequence()}
            </div>
            {Controls()}
        </div>
        {StatusBar()}
    </div>;
}

// The keyboard, consumed where it can be acted on.
//
// The native in `src/main.rs` writes the key's name and bumps a
// counter, and nothing more: deciding what a key means needs the
// clips, the trim and the frame rate, and those are reachable by name
// only from the thread that draws. So a component does it — this one,
// which draws nothing. Writing state while rendering asks for one more
// render, which is exactly what a key press should cause anyway.
fn Keys() -> Element {
    effect(presses, move || act(pressed.clone()));
    // Twenty times a second at most, and only while something plays:
    // the number it watches is the playhead, rounded.
    effect((played * 20.0) as u64, move || carry_on());

    return <span></span>;
}

// Playing past a clip's out-point: on to the next one, or stop. This
// is what makes the timeline play as a cut rather than as one file.
//
// It is an effect and not part of the task for the reason the task's
// own comment gives: this needs `clips` by name, and a name only finds
// the page's values on the thread that draws.
fn carry_on() {
    if paused {
        return;
    }
    let held = clips.clone();
    let here = selected.min(held.len().saturating_sub(1));
    let Some(clip) = held.get(here) else {
        return;
    };
    if played < clip.end - 0.05 {
        return;
    }
    match held.get(here + 1) {
        Some(next) => {
            let at = next.start;
            selected = here + 1;
            played = at;
            tell(format!("start:{at:.3}"));
        }
        None => tell("pause".to_string()),
    }
}

// ---- the icons --------------------------------------------------------

// lucide-react, written as a tag.
//
// `lucide-react` is in this application's `package.json`, so the
// framework bundles it and a page writes its components by the name
// the entry point gives the package — the same path `recharts::` takes
// in `apps/hybrid`. No `.tsx` of ours is involved: an icon is one tag.
//
// They go through one function because what the interface asks for is
// a word ("the icon for play"), and because the size is then decided
// once here rather than on thirty tags.
fn Icon(name: String) -> Element {
    let size = "h-3.5 w-3.5 shrink-0";

    return match name.as_str() {
        "film" => <lucideReact::Film className={size} />,
        "monitor" => <lucideReact::MonitorPlay className={size} />,
        "sliders" => <lucideReact::SlidersHorizontal className={size} />,
        "download" => <lucideReact::Download className={size} />,
        "folder-open" => <lucideReact::FolderOpen className={size} />,
        "plus" => <lucideReact::Plus className={size} />,
        "chevron-left" => <lucideReact::ChevronLeft className={size} />,
        "chevron-right" => <lucideReact::ChevronRight className={size} />,
        "trash" => <lucideReact::Trash2 className={size} />,
        "play" => <lucideReact::Play className={size} />,
        "pause" => <lucideReact::Pause className={size} />,
        "stop" => <lucideReact::Square className={size} />,
        "skip-back" => <lucideReact::SkipBack className={size} />,
        "skip-forward" => <lucideReact::SkipForward className={size} />,
        "volume" => <lucideReact::Volume2 className={size} />,
        _ => <lucideReact::VolumeX className={size} />,
    };
}

// ---- the frame around everything --------------------------------------

// A panel's title: an icon, ten pixels of upper case, and a hairline
// under it. Four panels wear it, which is why it is a function.
fn PanelTitle(icon: String, name: String, aside: String) -> Element {
    return <div className="flex h-7 shrink-0 items-center justify-between gap-2 border-b border-line px-3">
        <span className="flex items-center gap-1.5 text-muted">
            {Icon(icon)}
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em]">{name}</span>
        </span>
        <span className="truncate text-[10px] tabular-nums text-faint">{aside}</span>
    </div>;
}

fn Toolbar() -> Element {
    // A shadcn `Button`, in a suite. The component brings the shape,
    // the focus ring and the disabled state; the classes bring the
    // twenty-four pixels of height and the eleven-pixel type that a
    // dense toolbar is made of. That is the division all the way
    // down: the components are the material, this file is the look.
    let button = "h-6 gap-1.5 border-line bg-track px-2 text-[11px] text-text \
                  hover:border-faint hover:bg-track hover:text-white";

    return <div className="flex h-10 shrink-0 items-center gap-2 border-b border-line bg-panel px-3">
        <span className="flex items-center gap-2 pr-1 text-white">
            {Icon("film".to_string())}
            <span className="text-[12px] font-semibold tracking-tight">Altlassian Raw</span>
        </span>
        <Separator orientation="vertical" className="h-4 bg-line" />
        <Button id="open" variant="outline" size="sm" className={button.to_string()}
            onClick={move |_| open_files()}>
            {Icon("folder-open".to_string())}
            <span>Import media…</span>
        </Button>
        // A tag the engine mounts, in a row of React ones, and
        // deliberately: a handler on a component is handed React's own
        // event through the bridge, and `event.value` — what a page
        // writes everywhere else — is what the engine's own tags give.
        // `apps/hybrid` keeps a plain input in its card for the same
        // reason. shadcn's `Input` is a styled input and nothing more,
        // so its class list is the whole of what it was for.
        <input id="typed" placeholder="or paste a file path"
            value={typed.clone()}
            onInput={move |event| typed = event.value}
            className="h-6 w-72 border border-line bg-chrome px-2 text-[11px] text-text placeholder:text-faint focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-mark" />
        <Button id="add-typed" variant="outline" size="sm" className={button.to_string()}
            onClick={move |_| add_typed()}>
            {Icon("plus".to_string())}
            <span>Add</span>
        </Button>
        <span className="flex-1"></span>
        <span className="text-[10px] uppercase tracking-[0.14em] text-faint">{sequence_label()}</span>
    </div>;
}

// Whether the two programs are even here, and what to do when they are
// not. The same answer the rest of this repository gives about a tool
// it does not ship: name it, say what installs it.
fn Tooling() -> Element {
    let Err(why) = ffmpeg::tools() else {
        return <span></span>;
    };

    return <div id="no-ffmpeg" className="shrink-0 border-b border-danger/40 bg-danger/10 px-3 py-2 text-[11px] leading-relaxed text-danger">
        {why.clone()}
    </div>;
}

// ---- the monitor ------------------------------------------------------

fn Monitor() -> Element {
    return <div className="flex min-h-0 flex-1 flex-col bg-panel">
        {PanelTitle("monitor".to_string(), "Program monitor".to_string(), monitor_clock())}
        <div className="flex min-h-0 flex-1 items-center justify-center bg-black p-3">
            {Picture()}
        </div>
        {Scrub()}
        {Transport()}
    </div>;
}

fn Picture() -> Element {
    let Some(clip) = clip_at(selected) else {
        return <div id="preview" className="max-w-md text-center text-[11px] leading-relaxed text-faint">
            {"No media. Import a file and it lands on the timeline — the monitor then plays that file, with its sound, off your disk."}
        </div>;
    };

    // A proxy, once there is one: it stays the source from then on,
    // because switching back to a file the window refuses would start
    // the same refusal again on every frame.
    let proxy = beside(&ffmpeg::proxy_name(&clip.media.path));
    if proxy.is_file() {
        let paint = css_look(clip.look.clone());
        return <video id="monitor" src={as_url(&proxy)} data-cmd={command.clone()}
            preload="auto" className="max-h-full max-w-full object-contain" style={paint}>
        </video>;
    }

    // Refused, and no proxy yet: a frame ffmpeg drew, and a line
    // saying what is happening rather than a black rectangle.
    if !decodes {
        return <div className="flex h-full w-full flex-col items-center justify-center gap-2">
            {if still.is_empty() {
                <span className="text-[11px] text-faint">{"Drawing a frame with ffmpeg…"}</span>
            } else {
                <img id="preview" src={still.clone()} className="max-h-full max-w-full object-contain" />
            }}
            <span className="max-w-md text-center text-[10px] leading-relaxed text-danger">
                {"This window cannot decode that file — it has no H.264. ffmpeg is making a preview proxy of it; until that is done the monitor is one frame at a time. The render uses the original."}
            </span>
        </div>;
    }

    let paint = css_look(clip.look.clone());
    return <video id="monitor" src={as_url(&clip.media.path)} data-cmd={command.clone()}
        preload="auto" className="max-h-full max-w-full object-contain" style={paint}>
    </video>;
}

// The look, where CSS can say the same thing ffmpeg will do. See the
// note at the top of this file for the two it cannot.
fn css_look(look: String) -> String {
    match look.as_str() {
        "bw" => "filter: grayscale(1)".to_string(),
        _ => String::new(),
    }
}

// Where the playhead is inside the selected clip's cut. Dragging it
// seeks the file, which is the same thing clicking the ruler does.
fn Scrub() -> Element {
    let Some(clip) = clip_at(selected) else {
        return <div className="h-9 shrink-0 border-t border-line"></div>;
    };

    return <div className="flex h-9 shrink-0 items-center gap-3 border-t border-line px-3">
        <span className="shrink-0 text-[11px] tabular-nums text-faint">{clock(clip.start)}</span>
        <input id="playhead" type="range" min={clip.start.to_string()} max={clip.end.to_string()}
            step="0.01" value={played.clamp(clip.start, clip.end).to_string()}
            onInput={move |event| seek(event.number().unwrap_or(0.0))} />
        <span className="shrink-0 text-[11px] tabular-nums text-faint">{clock(clip.end)}</span>
    </div>;
}

fn Transport() -> Element {
    let has = !clips.is_empty();
    let times = rate;
    let speed = match times == 1.0 {
        true => String::new(),
        false => format!("{times:.0}×"),
    };

    return <div className="flex h-10 shrink-0 items-center gap-1 border-t border-line px-3">
        {Key("skip-back".to_string(), "Go to in point (Home)".to_string(), "to-in".to_string(), has, "home".to_string())}
        {Key("chevron-left".to_string(), "Previous frame (left arrow)".to_string(), "frame-back".to_string(), has, "left".to_string())}
        {PlayKey(has)}
        {Key("chevron-right".to_string(), "Next frame (right arrow)".to_string(), "frame-on".to_string(), has, "right".to_string())}
        {Key("skip-forward".to_string(), "Go to out point (End)".to_string(), "to-out".to_string(), has, "end".to_string())}
        {Key("stop".to_string(), "Stop and return to the in point".to_string(), "stop".to_string(), has, "stop".to_string())}

        <span className="w-8 pl-1 text-[10px] tabular-nums text-mark">{speed}</span>
        <span className="flex-1"></span>

        <span className="text-[11px] tabular-nums text-white">{clock(played)}</span>
        <span className="pl-1 text-[11px] tabular-nums text-faint">{format!("/ {}", clock(out_of(selected)))}</span>

        <Separator orientation="vertical" className="mx-2 h-4 bg-line" />
        {Key(
            if muted { "muted".to_string() } else { "volume".to_string() },
            "Mute (M)".to_string(),
            "mute".to_string(),
            has,
            "m".to_string(),
        )}
        <input id="volume" type="range" min="0" max="1" step="0.05" value={volume.to_string()}
            onInput={move |event| set_volume(event.number().unwrap_or(1.0))}
            className="w-20" />
    </div>;
}

// One transport button. Every one of them is an icon with no text, so
// every one of them carries its name for a screen reader and for the
// tooltip — which is also where the keyboard shortcut is written.
fn Key(icon: String, name: String, id: String, enabled: bool, does: String) -> Element {
    return <Button id={id} variant="outline" size="icon"
        className="h-7 w-8 border-line bg-track text-text hover:border-faint hover:bg-track hover:text-white"
        title={name.clone()} aria-label={name} disabled={!enabled}
        onClick={move |_| act(does.clone())}>
        {Icon(icon)}
    </Button>;
}

// Play and pause are one button, because they are one thing.
fn PlayKey(enabled: bool) -> Element {
    // The one place the accent is allowed on a control: the button
    // that starts the thing the window is for.
    let style = format!(
        "h-7 w-10 border text-white {}",
        match paused {
            true => "border-mark bg-mark hover:bg-mark",
            false => "border-mark bg-mark-soft hover:bg-mark-soft",
        }
    );
    let name = match paused {
        true => "Play (space)".to_string(),
        false => "Pause (space)".to_string(),
    };

    return <Button id="play" size="icon" className={style} title={name.clone()} aria-label={name}
        disabled={!enabled} onClick={move |_| act("space".to_string())}>
        {Icon(if paused { "play".to_string() } else { "pause".to_string() })}
    </Button>;
}

// ---- the timeline -----------------------------------------------------

fn Sequence() -> Element {
    return <div className="flex h-[186px] shrink-0 flex-col border-t border-line bg-panel">
        {PanelTitle("film".to_string(), "Timeline".to_string(), sequence_label())}
        <div className="min-h-0 flex-1 overflow-hidden px-3 pb-3">
            {Ruler()}
            {Track()}
        </div>
    </div>;
}

// Eleven marks across the sequence, labelled in timecode, and each one
// seeks when it is clicked. Computed from the real total rather than
// drawn as a decoration: the numbers under the clips are where those
// clips are.
fn Ruler() -> Element {
    let total = sequence_seconds();
    let marks: Vec<Element> = (0..11)
        .map(|step| {
            let at = total * f64::from(step) / 10.0;
            Mark(step, clock(at), at)
        })
        .collect();

    return <div className="flex h-5 items-end border-b border-line">{marks}</div>;
}

fn Mark(step: i32, label: String, at: f64) -> Element {
    let last = step == 10;
    let place = match last {
        true => "flex-1 pr-0 text-right",
        false => "flex-1 border-l border-line pl-1",
    };
    let style = format!("{place} cursor-pointer text-[9px] tabular-nums text-faint hover:text-text");

    return <div key={step.to_string()} className={style} onClick={move |_| seek_sequence(at)}>{label}</div>;
}

fn Track() -> Element {
    if clips.is_empty() {
        return <div className="mt-2 flex h-20 items-center justify-center border border-dashed border-line bg-track text-[11px] text-faint">
            {"V1 — empty. Imported files land here in the order they arrive."}
        </div>;
    }

    let total = sequence_seconds();
    let blocks: Vec<Element> = clips
        .iter()
        .enumerate()
        .map(|(index, clip)| ClipBlock(index, clip.clone(), total))
        .collect();

    return <div className="relative mt-2 flex h-20 gap-px bg-track">
        {blocks}
        {Playhead()}
    </div>;
}

fn ClipBlock(index: usize, clip: ffmpeg::Clip, total: f64) -> Element {
    let active = selected == index;
    // A clip's width is its real trimmed share of the sequence, so the
    // declaration is computed rather than written as a class.
    let box_style = format!("width: {:.3}%", (clip.length() / total) * 100.0);
    let skin = match active {
        true => "border border-mark bg-mark-soft",
        false => "border border-line bg-[#1d1c24] hover:border-faint",
    };
    let frame_style = format!(
        "relative flex min-w-[46px] cursor-pointer flex-col justify-between overflow-hidden px-2 py-1.5 {skin}"
    );
    // The left accent is the clip's colour filter, which is the one
    // thing about a clip you want to read at a glance on a track.
    let accent = format!("absolute left-0 top-0 h-full w-[3px] {}", match clip.look.as_str() {
        "warm" => "bg-[#e0a46c]",
        "cool" => "bg-[#6ca8e0]",
        "bw" => "bg-[#9aa0ab]",
        _ => "bg-[#3a3844]",
    });
    let start = clip.start;

    return <div key={index.to_string()} className={frame_style} style={box_style}
        onClick={move |_| {
            selected = index;
            played = start;
            want_still += 1;
            tell(format!("seek:{start:.3}"));
        }}>
        <span className={accent}></span>
        <span className="truncate pl-2 text-[11px] text-text">{clip.name.clone()}</span>
        <span className="truncate pl-2 text-[10px] tabular-nums text-faint">
            {format!("{} – {}", clock(clip.start), clock(clip.end))}
        </span>
    </div>;
}

// Where the playhead is across the whole sequence: the cuts before the
// selected clip, plus how far into it we are.
fn Playhead() -> Element {
    let at = (sequence_position() / sequence_seconds() * 100.0).clamp(0.0, 100.0);

    return <span className="pointer-events-none absolute top-0 h-full w-px bg-mark" style={format!("left: {at:.3}%")}></span>;
}

// ---- the right-hand column --------------------------------------------

fn Controls() -> Element {
    return <div className="flex w-[300px] shrink-0 flex-col border-l border-line bg-panel">
        {Effects()}
        {Export()}
    </div>;
}

fn Effects() -> Element {
    let Some(clip) = clip_at(selected) else {
        return <Card className="flex min-h-0 flex-1 flex-col border-0 bg-panel">
            {PanelTitle("sliders".to_string(), "Effect controls".to_string(), String::new())}
            <p className="p-3 text-[11px] leading-relaxed text-faint">
                {"Nothing is selected. Import a file, then trim it here and give it a colour and a subtitle."}
            </p>
        </Card>;
    };
    let index = selected.min(clips.len() - 1);

    // A shadcn `Card` with its corners and its shadow taken off by
    // the theme, which is what a panel in a suite is: a frame with a
    // hairline round it.
    return <Card className="flex min-h-0 flex-1 flex-col overflow-y-auto border-0 bg-panel">
        {PanelTitle("sliders".to_string(), "Effect controls".to_string(), format!("clip {}", index + 1))}

        <div className="border-b border-line px-3 py-2">
            <div id="clip-name" className="truncate text-[12px] text-white">{clip.name.clone()}</div>
            <div id="clip-media" className="truncate pt-0.5 text-[10px] tabular-nums text-faint">
                {format!(
                    "{}×{} · {:.2} fps · {} · {}",
                    clip.media.width,
                    clip.media.height,
                    clip.media.fps,
                    clock(clip.media.duration),
                    if clip.media.audio { "audio" } else { "no audio" }
                )}
            </div>
        </div>

        <div className="border-b border-line px-3 py-2">
            {Trim("In".to_string(), clock(clip.start), clip.media.duration, clip.start, "in".to_string(), index)}
            {Trim("Out".to_string(), clock(clip.end), clip.media.duration, clip.end, "out".to_string(), index)}
            <div className="flex items-baseline justify-between pt-1">
                <span className="text-[10px] uppercase tracking-[0.14em] text-faint">Duration</span>
                <span className="text-[11px] tabular-nums text-muted">{clock(clip.length())}</span>
            </div>
            <div className="flex gap-1 pt-2">
                {Mark_("Set in (I)".to_string(), "set-in".to_string(), "i".to_string())}
                {Mark_("Set out (O)".to_string(), "set-out".to_string(), "o".to_string())}
            </div>
        </div>

        <div className="border-b border-line px-3 py-2">
            <div className="pb-1.5 text-[10px] uppercase tracking-[0.14em] text-faint">Colour</div>
            <div className="flex gap-1">
                {FilterButton(index, "none".to_string(), clip.look.clone())}
                {FilterButton(index, "warm".to_string(), clip.look.clone())}
                {FilterButton(index, "cool".to_string(), clip.look.clone())}
                {FilterButton(index, "bw".to_string(), clip.look.clone())}
            </div>
            <div className="truncate pt-1.5 font-mono text-[10px] text-faint">
                {match ffmpeg::look_filter(&clip.look) {
                    "" => "no filter".to_string(),
                    filter => filter.to_string(),
                }}
            </div>
            {LookNote(clip.look.clone())}
        </div>

        <div className="border-b border-line px-3 py-2">
            <div className="pb-1.5 text-[10px] uppercase tracking-[0.14em] text-faint">Subtitle</div>
            <input id="subtitle" placeholder="burned in by ffmpeg" value={clip.subtitle.clone()}
                onInput={move |event| set_subtitle(index, event.value.clone())}
                className="h-6 w-full border border-line bg-chrome px-2 text-[11px] text-text placeholder:text-faint focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-mark" />
        </div>

        <div className="flex gap-1 px-3 py-2">
            {OrderButton("chevron-left".to_string(), "Move left".to_string(), "move-left".to_string(), index > 0, -1)}
            {OrderButton("chevron-right".to_string(), "Move right".to_string(), "move-right".to_string(), index + 1 < clips.len(), 1)}
            {OrderButton("trash".to_string(), "Remove".to_string(), "remove".to_string(), true, 0)}
        </div>
    </Card>;
}

// Warm and cool are `colortemperature`, and CSS has nothing that does
// that. Rather than show an approximation and let somebody believe it,
// the panel says where the colour will appear.
fn LookNote(look: String) -> Element {
    if look != "warm" && look != "cool" {
        return <span></span>;
    }

    return <p className="pt-1 text-[10px] leading-relaxed text-faint">
        {"The monitor does not show this one: it is ffmpeg's colortemperature, and CSS has no equivalent. It is applied when you render."}
    </p>;
}

// A trim control: the name on the left, the timecode on the right, the
// slider under both.
fn Trim(
    name: String,
    value: String,
    duration: f64,
    at: f64,
    which: String,
    index: usize,
) -> Element {
    return <label className="block pb-1.5">
        <span className="flex items-baseline justify-between">
            <span className="text-[10px] uppercase tracking-[0.14em] text-faint">{name}</span>
            <span className="text-[11px] tabular-nums text-muted">{value}</span>
        </span>
        <input type="range" min="0" max={duration.to_string()} step="0.04" value={at.to_string()}
            onInput={move |event| {
                let moved = event.number().unwrap_or(0.0);
                edit(index, |clip| match which.as_str() {
                    "in" => clip.start = moved.min(clip.end - 0.04).max(0.0),
                    _ => clip.end = moved.max(clip.start + 0.04).min(clip.media.duration),
                });
            }} />
    </label>;
}

// Set in / set out, from wherever the playhead is.
fn Mark_(label: String, id: String, does: String) -> Element {
    return <Button id={id} variant="outline" size="sm" onClick={move |_| act(does.clone())}
        className="h-6 flex-1 border-line bg-track text-muted hover:border-faint hover:bg-track hover:text-text text-[11px]">
        {label}
    </Button>;
}

fn FilterButton(index: usize, name: String, active_name: String) -> Element {
    let active = name == active_name;
    let apply = name.clone();
    let chip = format!(
        "h-6 flex-1 px-0 text-[11px] capitalize {}",
        match active {
            true => "border-mark bg-mark-soft text-white hover:bg-mark-soft hover:text-white",
            false => "border-line bg-track text-muted hover:border-faint hover:bg-track hover:text-text",
        }
    );

    return <Button variant="outline" size="sm" className={chip} onClick={move |_| {
        edit(index, |clip| clip.look = apply.clone())
    }}>{name}</Button>;
}

// The three that change the order of the sequence. `by` is where the
// clip goes: one place left, one place right, or nowhere, which is the
// one that removes it.
fn OrderButton(icon: String, label: String, id: String, enabled: bool, by: i64) -> Element {
    return <Button id={id} variant="outline" size="sm" disabled={!enabled}
        className="h-6 flex-1 gap-1 px-1 border-line bg-track text-muted hover:border-faint hover:bg-track hover:text-text text-[11px]"
        title={label.clone()} aria-label={label.clone()}
        onClick={move |_| match by {
            0 => remove_clip(),
            by => shift(by),
        }}>
        {Icon(icon)}
        <span>{label}</span>
    </Button>;
}

fn Export() -> Element {
    let filled = format!("width: {:.1}%", progress);
    let render = match rendering {
        true => "h-7 w-full gap-1.5 bg-mark-soft text-[11px] text-white hover:bg-mark-soft",
        false => "h-7 w-full gap-1.5 bg-mark text-[11px] font-semibold text-white hover:brightness-110",
    };

    return <Card className="shrink-0 rounded-none border-0 border-t border-line bg-panel">
        {PanelTitle("download".to_string(), "Export".to_string(), String::new())}
        <div className="px-3 py-2">
            <Button id="render" className={render.to_string()} onClick={move |_| start_render()}>
                {Icon("download".to_string())}
                <span>{if rendering { "Rendering…" } else { "Render sequence" }}</span>
            </Button>
            <div className="mt-2 h-[3px] bg-track">
                <div id="progress" className="h-full bg-mark" style={filled}></div>
            </div>
            {Written()}
        </div>
    </Card>;
}

fn Written() -> Element {
    if output.is_empty() {
        return <p className="pt-2 text-[10px] leading-relaxed text-faint">
            {"The bar follows what ffmpeg itself reports as it writes, which is why it moves unevenly."}
        </p>;
    }

    return <div className="pt-2">
        <div id="output" className="truncate text-[10px] text-good">{output.clone()}</div>
        <Button id="reveal" variant="outline" size="sm" onClick={move |_| reveal_output()}
            className="mt-1.5 h-6 w-full gap-1.5 border-line bg-track text-muted hover:border-faint hover:bg-track hover:text-text text-[11px]">
            {Icon("folder-open".to_string())}
            <span>Show in folder</span>
        </Button>
    </div>;
}

// The last refusal, and the keys — which is where somebody finds out
// that there are any.
fn StatusBar() -> Element {
    let tone = match status.is_empty() {
        true => "text-faint",
        false => "text-danger",
    };
    let line = format!("flex h-6 shrink-0 items-center gap-3 border-t border-line bg-panel px-3 text-[10px] {tone}");

    return <div className={line}>
        <span id="status" className="truncate">{said()}</span>
        <span className="flex-1"></span>
        <span className="shrink-0 text-faint">
            {"Space play · K pause · L faster · J a second back (Chromium cannot play in reverse) · ←/→ frame · shift+←/→ second · I/O in and out · Home/End · M mute"}
        </span>
    </div>;
}

// ---- what a key, or a transport button, means ------------------------

// One place for both, because a transport button is a key with an icon
// on it. Everything here is ordinary state; what reaches the element is
// the command `tell` writes.
fn act(what: String) {
    let Some(clip) = clip_at(selected) else {
        return;
    };
    let frame = match clip.media.fps > 1.0 {
        true => 1.0 / clip.media.fps,
        false => 1.0 / 25.0,
    };

    match what.as_str() {
        "space" => match paused {
            true => {
                // From inside the cut: a playhead left outside it by a
                // trim would otherwise play the part that was cut.
                let at = played.clamp(clip.start, clip.end);
                played = at;
                tell(format!("start:{at:.3}"));
            }
            false => tell("pause".to_string()),
        },
        "k" => tell("pause".to_string()),
        "l" => {
            rate = match rate {
                speed if speed >= 4.0 => 1.0,
                speed => speed * 2.0,
            };
            let speed = rate;
            tell(format!("rate:{speed:.2}"));
            if paused {
                tell(format!("start:{:.3}", played.clamp(clip.start, clip.end)));
            }
        }
        // Not reverse play: Chromium refuses a negative playbackRate,
        // so this is the honest half of J — stop, and a second back.
        "j" => {
            tell("pause".to_string());
            seek(played - 1.0);
        }
        "left" => seek(played - frame),
        "right" => seek(played + frame),
        "shift-left" => seek(played - 1.0),
        "shift-right" => seek(played + 1.0),
        "home" => seek(clip.start),
        "end" => seek(clip.end),
        "stop" => {
            tell("pause".to_string());
            seek(clip.start);
        }
        "i" => edit(selected, |clip| {
            clip.start = played.min(clip.end - 0.04).max(0.0)
        }),
        "o" => edit(selected, |clip| {
            clip.end = played.max(clip.start + 0.04).min(clip.media.duration)
        }),
        "m" => {
            muted = !muted;
            tell(format!("volume:{:.2}", if muted { 0.0 } else { volume }));
        }
        _ => {}
    }
}

// Move the playhead, inside the cut. The element is told, and the page
// writes the value too rather than waiting for the report — a slider
// that lags its own drag feels broken.
fn seek(to: f64) {
    let Some(clip) = clip_at(selected) else {
        return;
    };
    let at = to.clamp(clip.start, clip.end);
    played = at;
    want_still += 1;
    tell(format!("seek:{at:.3}"));
}

// A click on the ruler: a position in the whole sequence, which is a
// clip and a time inside it.
fn seek_sequence(at: f64) {
    let mut passed = 0.0;
    let held = clips.clone();
    for (index, clip) in held.iter().enumerate() {
        if at <= passed + clip.length() || index + 1 == held.len() {
            let inside = clip.start + (at - passed).clamp(0.0, clip.length());
            selected = index;
            played = inside;
            want_still += 1;
            tell(format!("seek:{inside:.3}"));
            return;
        }
        passed += clip.length();
    }
}

fn set_volume(level: f64) {
    volume = level.clamp(0.0, 1.0);
    muted = volume == 0.0;
    let level = volume;
    tell(format!("volume:{level:.2}"));
}

// What the element is to do next. The counter is in the string so that
// two identical commands are two commands.
fn tell(what: String) {
    issued += 1;
    let count = issued;
    command = format!("{count}|{what}");
}

// ---- what the buttons do ---------------------------------------------

fn open_files() {
    let ask = rustx::dialog::Ask::new("Import media")
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
            played = 0.0;
            decodes = true;
            want_still += 1;
            status = String::new();
        }
    }
}

// Change a clip in place.
fn edit(index: usize, change: impl FnOnce(&mut ffmpeg::Clip)) {
    let mut held = clips.clone();
    let Some(clip) = held.get_mut(index) else {
        return;
    };
    change(clip);
    let (start, end) = (clip.start, clip.end);
    clips = held;
    played = played.clamp(start, end);
    want_still += 1;
}

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
    still = String::new();
    played = clip_at(selected).map(|clip| clip.start).unwrap_or(0.0);
    tell("pause".to_string());
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
        status = "There is nothing on the timeline to render.".to_string();
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

// Where the render landed, in the file manager.
fn reveal_output() {
    let written = output.clone();
    match rustx::shell::reveal(std::path::Path::new(&written)) {
        Ok(()) => {}
        Err(error) => status = format!("The folder could not be opened: {error}"),
    }
}

// ---- what the interface reads ----------------------------------------

fn clip_at(index: usize) -> Option<ffmpeg::Clip> {
    clips.get(index.min(clips.len().saturating_sub(1))).cloned()
}

// The sequence's real length, floored so that nothing divides by zero
// before a file has been imported.
fn sequence_seconds() -> f64 {
    clips.iter().map(|clip| clip.length()).sum::<f64>().max(0.001)
}

// Where the playhead is in the whole sequence rather than in one file.
fn sequence_position() -> f64 {
    let before: f64 = clips
        .iter()
        .take(selected.min(clips.len()))
        .map(|clip| clip.length())
        .sum();
    match clip_at(selected) {
        None => 0.0,
        Some(clip) => before + (played - clip.start).clamp(0.0, clip.length()),
    }
}

fn out_of(index: usize) -> f64 {
    clip_at(index).map(|clip| clip.end).unwrap_or(0.0)
}

fn sequence_label() -> String {
    match clips.len() {
        0 => "no media".to_string(),
        1 => format!("1 clip · {}", clock(sequence_seconds())),
        many => format!("{many} clips · {}", clock(sequence_seconds())),
    }
}

// The sequence timecode, which is what a monitor shows above the
// picture.
fn monitor_clock() -> String {
    match clips.is_empty() {
        true => String::new(),
        false => format!(
            "{} / {}",
            clock(sequence_position()),
            clock(sequence_seconds())
        ),
    }
}

fn said() -> String {
    match status.is_empty() {
        false => status.clone(),
        true => match ffmpeg::tools() {
            Err(_) => "ffmpeg was not found.".to_string(),
            Ok(_) => "ffmpeg and ffprobe are on this machine.".to_string(),
        },
    }
}

// Timecode, to hundredths. Not frames: a sequence of files at different
// frame rates has no one frame number, and a wrong one would be worse
// than none.
fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    let whole = seconds as u64;
    format!(
        "{:02}:{:02}:{:02}.{:02}",
        whole / 3600,
        (whole / 60) % 60,
        whole % 60,
        ((seconds - whole as f64) * 100.0).round() as u64 % 100
    )
}

// ---- where the files this application writes itself go ----------------

// The fallback stills and the subtitle files ffmpeg reads. The cache
// directory, because every one of them can be deleted at any moment
// and nothing is lost.
fn beside(name: &str) -> std::path::PathBuf {
    let directory = rustx::data::cache().unwrap_or_else(|_| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&directory);
    directory.join(name)
}

// A path as the page can load it. `file:///C:/…`, with the separators
// a URL wants rather than the ones Windows writes.
fn as_url(path: &std::path::Path) -> String {
    rustx::files::url(path)
}
