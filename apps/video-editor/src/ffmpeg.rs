//! What this editor is a user interface over: `ffmpeg` and `ffprobe`.
//!
//! The application decodes nothing itself. It reads a file's real
//! duration and geometry with `ffprobe`, draws one real decoded frame
//! with `ffmpeg` for the preview, and hands the whole timeline to one
//! `ffmpeg` process that writes the output file. So every interesting
//! thing here is a command line, and a command line is a value: the
//! functions that build one are pure and have tests, and the two
//! functions that actually start a process are the thin part on top.
//!
//! # ffmpeg is a runtime dependency and never a build one
//!
//! Nothing in this file links against ffmpeg, bundles it, or needs it
//! to exist for `cargo check`, `cargo clippy`, `cargo test` or the
//! window to open. [`tools`] looks for the two programs when the
//! application first asks, and when they are not there it answers with
//! the sentence the interface shows — the same shape as
//! `rustx-os`'s refusal when `notify-send` is missing.
//!
//! # Subtitles are `drawtext`, not a generated `.srt`
//!
//! Both were on the table. `.srt` + the `subtitles` filter wins when
//! there are many cues with their own timings, because that is what the
//! format is for; it also needs libass in the ffmpeg build and a file
//! per clip whose timings would restate the trim we already applied.
//! Here a clip carries one line for its whole length, so `drawtext` is
//! the whole of what is needed — and it is in every ffmpeg build with
//! freetype, which is all of them in practice.
//!
//! The text still goes through a file, by `textfile=`, with
//! `expansion=none`. That pair is what makes the user's own text safe:
//! a subtitle typed as `C'est l'été: 50% off` carries a quote, a colon
//! and a percent sign, which are three different escaping problems in
//! a filter graph — measured here, `text=` with the default expansion
//! fails outright on the `%`. Through a file with expansion off,
//! nothing in the text is syntax.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

/// A program run for the window must not open a console of its own, which a
/// windowed application on Windows would otherwise show for every call.
fn quiet(mut command: std::process::Command) -> std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    command
}

fn quiet_async(mut command: rustx::tokio::process::Command) -> rustx::tokio::process::Command {
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    command
}

/// The two programs, where this machine has them.
#[derive(Debug, Clone)]
pub struct Tools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

/// `ffmpeg` and `ffprobe`, found once.
///
/// Once, deliberately: asking is starting two processes, and the
/// interface asks on every render of the window. What that costs is a
/// restart for somebody who installs ffmpeg while the window is open,
/// which the refusal below says out loud.
pub fn tools() -> &'static Result<Tools, String> {
    static FOUND: OnceLock<Result<Tools, String>> = OnceLock::new();
    FOUND.get_or_init(|| match (found("ffmpeg"), found("ffprobe")) {
        (Some(ffmpeg), Some(ffprobe)) => Ok(Tools { ffmpeg, ffprobe }),
        _ => Err(MISSING.to_string()),
    })
}

/// What the window says when they are not there.
pub const MISSING: &str = "\
    `ffmpeg` and `ffprobe` could not be run. They are what reads a file's duration \
    and what writes a render — this editor is an interface over them and decodes \
    nothing by itself. Install them with `winget install Gyan.FFmpeg` on Windows, \
    `brew install ffmpeg` on macOS, or your distribution's `ffmpeg` package, and \
    reopen this window.";

/// One program: on the PATH, or where an installer puts it.
///
/// shortcut: only the PATH branch has run on a machine here (winget
/// puts a shim on the PATH), so the directory list below is written and
/// not exercised. Upgrade it when somebody reports an ffmpeg this does
/// not find.
fn found(program: &str) -> Option<PathBuf> {
    // Not whether it worked: `-version` exits zero, but the only
    // question that matters is whether the process started at all.
    if quiet(std::process::Command::new(program))
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Some(PathBuf::from(program));
    }

    let mut roots: Vec<PathBuf> = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(variable) {
            roots.push(PathBuf::from(&root).join("ffmpeg").join("bin"));
        }
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        // Where `winget install Gyan.FFmpeg` puts its shims.
        roots.push(
            PathBuf::from(local)
                .join("Microsoft")
                .join("WinGet")
                .join("Links"),
        );
    }
    if let Some(data) = std::env::var_os("ProgramData") {
        roots.push(PathBuf::from(data).join("chocolatey").join("bin"));
    }
    roots.push(PathBuf::from("C:\\ffmpeg\\bin"));
    roots.push(PathBuf::from("/usr/local/bin"));
    roots.push(PathBuf::from("/opt/homebrew/bin"));

    for root in roots {
        for name in [format!("{program}.exe"), program.to_string()] {
            let guess = root.join(name);
            if guess.is_file() {
                return Some(guess);
            }
        }
    }
    None
}

/// What `ffprobe` said about a file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Media {
    pub path: PathBuf,
    /// Seconds, as the container reports them.
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    /// Whether there is an audio stream at all — which decides whether
    /// the render can carry sound. See [`plan`].
    pub audio: bool,
}

/// One clip of the timeline: a file, a span of it, a look, a line.
#[derive(Debug, Clone)]
pub struct Clip {
    pub media: Media,
    /// The file's own name, which is what the timeline shows.
    pub name: String,
    /// Seconds into the file, both of them.
    pub start: f64,
    pub end: f64,
    /// A key of [`look_filter`].
    pub look: String,
    pub subtitle: String,
    /// The grade: `eq` in ffmpeg, `brightness()`, `contrast()` and
    /// `saturate()` in the monitor. Unchanged is 0, 1 and 1.
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    /// A clip that is switched off stays on the timeline and is left out
    /// of the render.
    pub enabled: bool,
}

impl Clip {
    pub fn new(media: Media) -> Self {
        let name = media
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| media.path.display().to_string());
        let end = media.duration;
        Self {
            media,
            name,
            start: 0.0,
            end,
            look: "none".to_string(),
            subtitle: String::new(),
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            enabled: true,
        }
    }

    /// ffmpeg's `eq` for this clip's grade, or `None` when it is untouched.
    pub fn grade_filter(&self) -> Option<String> {
        let unchanged = self.brightness.abs() < 1e-9
            && (self.contrast - 1.0).abs() < 1e-9
            && (self.saturation - 1.0).abs() < 1e-9;
        (!unchanged).then(|| {
            format!(
                "eq=brightness={:.2}:contrast={:.2}:saturation={:.2}",
                self.brightness, self.contrast, self.saturation
            )
        })
    }

    /// The same grade as the CSS of the monitor. `eq`'s brightness adds to
    /// the picture and CSS's multiplies it, so the two agree closely and not
    /// exactly: the monitor is a preview, the render is the result.
    pub fn grade_css(&self) -> String {
        format!(
            "brightness({:.2}) contrast({:.2}) saturate({:.2})",
            1.0 + self.brightness,
            self.contrast,
            self.saturation
        )
    }

    /// How long this clip is after its trim.
    pub fn length(&self) -> f64 {
        (self.end - self.start).max(0.0)
    }
}

/// The ffmpeg filter one colour choice means.
///
/// The two temperatures are the right way round because they were
/// measured rather than guessed: on a `smptebars` frame,
/// `temperature=3000` moves the chroma averages towards red and away
/// from blue, and `temperature=8000` does the opposite. `""` is the
/// honest answer for "none" and for a name nobody defined.
pub fn look_filter(look: &str) -> &'static str {
    match look {
        "warm" => "colortemperature=temperature=3000",
        "cool" => "colortemperature=temperature=8000",
        "bw" => "format=gray",
        _ => "",
    }
}

/// What `ffprobe` is asked, for one file.
pub fn probe_arguments(path: &Path) -> Vec<String> {
    vec![
        "-v".to_string(),
        "error".to_string(),
        "-show_entries".to_string(),
        "stream=codec_type,width,height,r_frame_rate:format=duration".to_string(),
        "-of".to_string(),
        "default=noprint_wrappers=1".to_string(),
        path.display().to_string(),
    ]
}

/// Read that answer.
///
/// The shape is `key=value` lines, stream by stream and then the
/// container. A file's audio stream also carries `r_frame_rate=0/0`, so
/// the geometry is taken from the first video stream and from nothing
/// else — which is the whole reason this is a function with a test
/// rather than three `lines().find(…)`.
pub fn read_probe(text: &str) -> Result<Media, String> {
    let mut media = Media::default();
    let mut in_video = false;
    let mut have_video = false;
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        match key {
            "codec_type" => {
                in_video = value == "video" && !have_video;
                if in_video {
                    have_video = true;
                }
                if value == "audio" {
                    media.audio = true;
                }
            }
            "width" if in_video => media.width = value.parse().unwrap_or(0),
            "height" if in_video => media.height = value.parse().unwrap_or(0),
            "r_frame_rate" if in_video => media.fps = ratio(value),
            "duration" => media.duration = value.parse().unwrap_or(0.0),
            _ => {}
        }
    }
    if !have_video {
        return Err("there is no video stream in this file.".to_string());
    }
    if media.duration <= 0.0 {
        return Err(
            "this file reports no duration, so it cannot be put on a timeline.".to_string(),
        );
    }
    Ok(media)
}

/// `"30000/1001"` — a frame rate as ffprobe writes it.
fn ratio(value: &str) -> f64 {
    match value.split_once('/') {
        Some((top, bottom)) => {
            let bottom: f64 = bottom.parse().unwrap_or(0.0);
            match bottom == 0.0 {
                true => 0.0,
                false => top.parse::<f64>().unwrap_or(0.0) / bottom,
            }
        }
        None => value.parse().unwrap_or(0.0),
    }
}

/// `ffprobe` on a file, and what it said.
pub fn probe(ffprobe: &Path, path: &Path) -> Result<Media, String> {
    let output = quiet(std::process::Command::new(ffprobe))
        .args(probe_arguments(path))
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("`{}` could not be run ({error}).", ffprobe.display()))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe refused {}: {}",
            path.display(),
            tail(&output.stderr)
        ));
    }
    let mut media = read_probe(&String::from_utf8_lossy(&output.stdout))?;
    media.path = path.to_path_buf();
    Ok(media)
}

/// One decoded frame of a clip, with the clip's look on it.
///
/// `-ss` before `-i` so that ffmpeg seeks instead of decoding from the
/// start, and `-frames:v 1` so that it stops at the frame we asked
/// for. JPEG rather than PNG: the preview is shown by an `<img>` in a
/// `file://` page and a 100 KB file redraws immediately where a 2 MB
/// one does not.
pub fn frame_arguments(clip: &Clip, at: f64, out: &Path) -> Vec<String> {
    let mut arguments = vec![
        "-hide_banner".to_string(),
        "-nostdin".to_string(),
        "-y".to_string(),
        "-ss".to_string(),
        format!("{at:.3}"),
        "-i".to_string(),
        clip.media.path.display().to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
    ];
    let chain: Vec<String> = [Some(look_filter(&clip.look).to_string()).filter(|look| !look.is_empty()), clip.grade_filter()]
        .into_iter()
        .flatten()
        .collect();
    if !chain.is_empty() {
        arguments.push("-vf".to_string());
        arguments.push(chain.join(","));
    }
    arguments.push("-q:v".to_string());
    arguments.push("4".to_string());
    arguments.push(out.display().to_string());
    arguments
}

/// A preview proxy, as a command line.
///
/// The CEF build this window runs on has no H.264 and no AAC — a WebM
/// of VP8 and Vorbis plays, an mp4 does not, and most footage is an
/// mp4. Measured here rather than assumed: the monitor refused
/// `a.mp4` with `DEMUXER_ERROR`, and played the same picture as
/// `a.webm`.
///
/// An editor's answer to a file it cannot play is a proxy, and this is
/// one: the whole file, once, into something the window can decode, at
/// 640 wide and in a hurry. Its timestamps are the source's, so the
/// playhead means the same thing in both, and the render never touches
/// it — it is a preview and nothing else.
pub fn proxy_plan(media: &Media, out: &Path) -> Plan {
    let mut arguments: Vec<String> = ["-hide_banner", "-nostdin", "-y", "-i"]
        .iter()
        .map(|flag| flag.to_string())
        .collect();
    arguments.push(media.path.display().to_string());
    for flag in [
        "-vf",
        "scale=640:-2",
        "-c:v",
        "libvpx",
        // `realtime` and `cpu-used 8`: this is a preview of a file the
        // person is waiting to look at, not an encode anybody keeps.
        "-deadline",
        "realtime",
        "-cpu-used",
        "8",
        "-b:v",
        "900k",
    ] {
        arguments.push(flag.to_string());
    }
    match media.audio {
        true => {
            for flag in ["-c:a", "libvorbis", "-b:a", "96k"] {
                arguments.push(flag.to_string());
            }
        }
        false => arguments.push("-an".to_string()),
    }
    for flag in ["-nostats", "-progress", "pipe:1"] {
        arguments.push(flag.to_string());
    }
    arguments.push(out.display().to_string());

    Plan {
        arguments,
        seconds: media.duration,
        audio: media.audio,
        subtitles: Vec::new(),
    }
}

/// What a file's proxy is called.
///
/// The path, hashed, so that the same file gets the same proxy and a
/// second session does not build it again.
///
/// shortcut: the name is the path and not the bytes, so a file
/// replaced at the same path keeps the old proxy. Delete the cache, or
/// hash the size and the date too, the day that matters.
pub fn proxy_name(path: &Path) -> String {
    // FNV-1a, which is eight lines nobody has to maintain rather than
    // a dependency for one filename.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.display().to_string().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("proxy-{hash:016x}.webm")
}

/// A render, as a command line and what the caller must write first.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub arguments: Vec<String>,
    /// The output's length in seconds — what the progress bar divides
    /// by.
    pub seconds: f64,
    /// Whether the output carries sound.
    pub audio: bool,
    /// The subtitle files the graph names, with their text. Writing
    /// them is [`run`]'s first job; they are listed rather than written
    /// here so that this function stays pure and testable.
    pub subtitles: Vec<(PathBuf, String)>,
}

/// The whole sequence: trim, look, subtitle, concat, in one command.
///
/// One ffmpeg process and one `concat` filter, not a `concat` demuxer
/// over intermediate files: the filter re-encodes anyway (every clip
/// has its own filter chain), and the demuxer would have meant writing
/// one temporary file per clip and reading it back.
///
/// The concat filter demands that every input agree on size, sample
/// aspect and frame rate, so each clip is scaled and padded to the
/// first clip's geometry. That is the editor's sequence setting, chosen
/// for the user rather than asked about.
///
/// shortcut: sound is carried only when *every* clip has an audio
/// stream, because `concat` with `a=1` wants an audio input from each
/// one. A mixed timeline renders silent and the interface says so.
/// Upgrade by giving the silent clips an `anullsrc` input.
pub fn plan(
    clips: &[Clip],
    out: &Path,
    subtitle_dir: &Path,
    font: Option<&Path>,
) -> Result<Plan, String> {
    let on: Vec<Clip> = clips.iter().filter(|clip| clip.enabled).cloned().collect();
    let clips = &on[..];
    let Some(first) = clips.first() else {
        return Err("there is nothing on the timeline to render.".to_string());
    };
    let width = match first.media.width {
        0 => 1280,
        width => width,
    };
    let height = match first.media.height {
        0 => 720,
        height => height,
    };
    let fps = match first.media.fps > 0.0 {
        true => first.media.fps,
        false => 30.0,
    };
    let audio = clips.iter().all(|clip| clip.media.audio);

    let mut arguments: Vec<String> = ["-hide_banner", "-nostdin", "-y"]
        .iter()
        .map(|flag| flag.to_string())
        .collect();
    let mut video = String::new();
    let mut sound = String::new();
    let mut subtitles: Vec<(PathBuf, String)> = Vec::new();
    let mut seconds = 0.0;

    for (index, clip) in clips.iter().enumerate() {
        let length = clip.length();
        if length < 0.05 {
            return Err(format!("{} is trimmed down to nothing.", clip.name));
        }
        seconds += length;

        // `-ss` and `-t` before `-i`: the trim is done by the demuxer,
        // which never decodes the part nobody asked for. `-t` rather
        // than `-to`, because a duration after the seek is unambiguous
        // and `-to` on an input is read against the file's own
        // timeline.
        arguments.push("-ss".to_string());
        arguments.push(format!("{:.3}", clip.start));
        arguments.push("-t".to_string());
        arguments.push(format!("{length:.3}"));
        arguments.push("-i".to_string());
        arguments.push(clip.media.path.display().to_string());

        video.push_str(&format!(
            "[{index}:v]scale={width}:{height}:force_original_aspect_ratio=decrease,\
             pad={width}:{height}:-1:-1,setsar=1,fps={fps:.4}"
        ));
        let look = look_filter(&clip.look);
        if !look.is_empty() {
            video.push(',');
            video.push_str(look);
        }
        if let Some(grade) = clip.grade_filter() {
            video.push(',');
            video.push_str(&grade);
        }
        // After the look, not before: `format=gray` is how black and
        // white is done, and the concat filter wants every input in the
        // same pixel format as the encoder.
        video.push_str(",format=yuv420p");

        let said = clip.subtitle.trim();
        if !said.is_empty() {
            let Some(font) = font else {
                return Err(format!(
                    "no font file was found on this machine, so the subtitle on {} cannot be \
                     burned in. Clear it, or install a font and reopen this window.",
                    clip.name
                ));
            };
            let file = subtitle_dir.join(format!("subtitle-{index}.txt"));
            video.push_str(&format!(
                ",drawtext=fontfile='{}':textfile='{}':expansion=none:fontsize={}:\
                 fontcolor=white:borderw=2:bordercolor=black@0.7:x=(w-text_w)/2:y=h-text_h-{}",
                in_filter(font),
                in_filter(&file),
                (height / 22).max(12),
                height / 12
            ));
            subtitles.push((file, said.to_string()));
        }
        video.push_str(&format!("[v{index}];"));

        if audio {
            sound.push_str(&format!(
                "[{index}:a]aresample=48000,\
                 aformat=sample_fmts=fltp:channel_layouts=stereo[a{index}];"
            ));
        }
    }

    let mut graph = video;
    graph.push_str(&sound);
    for index in 0..clips.len() {
        graph.push_str(&format!("[v{index}]"));
        if audio {
            graph.push_str(&format!("[a{index}]"));
        }
    }
    graph.push_str(&format!(
        "concat=n={}:v=1:a={}[v]",
        clips.len(),
        u8::from(audio)
    ));
    if audio {
        graph.push_str("[a]");
    }

    arguments.push("-filter_complex".to_string());
    arguments.push(graph);
    arguments.push("-map".to_string());
    arguments.push("[v]".to_string());
    if audio {
        arguments.push("-map".to_string());
        arguments.push("[a]".to_string());
    }
    for flag in ["-c:v", "libx264", "-preset", "veryfast", "-crf", "20"] {
        arguments.push(flag.to_string());
    }
    if audio {
        for flag in ["-c:a", "aac", "-b:a", "160k"] {
            arguments.push(flag.to_string());
        }
    }
    // `-progress pipe:1` is the machine-readable one, on stdout, which
    // leaves stderr to be nothing but what went wrong. `-nostats` so
    // the human-readable line is not written twice.
    for flag in ["-nostats", "-progress", "pipe:1"] {
        arguments.push(flag.to_string());
    }
    arguments.push(out.display().to_string());

    Ok(Plan {
        arguments,
        seconds,
        audio,
        subtitles,
    })
}

/// A path as a filter graph reads one.
///
/// Three things in one small function, and all three have bitten:
/// backslashes become slashes (ffmpeg accepts them on Windows and the
/// filter parser does not treat them as text), a drive colon is escaped
/// because `:` separates a filter's options even inside quotes, and a
/// quote in somebody's folder name closes the quoted string — so it is
/// closed, escaped and reopened, which is the only way ffmpeg offers.
fn in_filter(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "'\\''")
}

/// A font to burn subtitles with.
///
/// `drawtext` can ask fontconfig for a family name, but Windows has no
/// fontconfig, so a file is the portable answer.
pub fn font_file() -> Option<PathBuf> {
    let mut guesses: Vec<PathBuf> = Vec::new();
    if let Some(root) = std::env::var_os("SystemRoot") {
        let fonts = PathBuf::from(root).join("Fonts");
        guesses.push(fonts.join("arial.ttf"));
        guesses.push(fonts.join("segoeui.ttf"));
        guesses.push(fonts.join("tahoma.ttf"));
    }
    guesses.push(PathBuf::from(
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ));
    guesses.push(PathBuf::from("/Library/Fonts/Arial.ttf"));
    guesses.push(PathBuf::from("/System/Library/Fonts/Helvetica.ttc"));
    guesses.into_iter().find(|guess| guess.is_file())
}

/// A line of `-progress pipe:1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Progress {
    /// How far into the output ffmpeg has written, in seconds.
    At(f64),
    Done,
}

/// Read one such line, or `None` for the dozen others it writes.
///
/// `out_time_us=N/A` is a real line: ffmpeg writes it before the first
/// frame comes out of the filter graph, and `"N/A".parse()` would have
/// made the bar jump to zero.
pub fn read_progress(line: &str) -> Option<Progress> {
    let (key, value) = line.trim().split_once('=')?;
    match key {
        "out_time_us" => value.parse::<f64>().ok().map(|us| Progress::At(us / 1e6)),
        "progress" if value == "end" => Some(Progress::Done),
        _ => None,
    }
}

/// The last words of a failed process, which is what a person needs.
///
/// Four lines: ffmpeg's real complaint is at the end, and the thousand
/// lines of stream configuration above it are not a message.
pub fn tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let from = lines.len().saturating_sub(4);
    match lines[from..].join(" / ") {
        said if said.is_empty() => "it failed and said nothing.".to_string(),
        said => said,
    }
}

/// Draw one frame of a clip into `out`.
pub async fn extract(ffmpeg: &Path, clip: &Clip, at: f64, out: &Path) -> Result<(), String> {
    let output = quiet_async(rustx::tokio::process::Command::new(ffmpeg))
        .args(frame_arguments(clip, at, out))
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| format!("`{}` could not be run ({error}).", ffmpeg.display()))?;
    match output.status.success() {
        true => Ok(()),
        false => Err(format!("ffmpeg drew no frame: {}", tail(&output.stderr))),
    }
}

/// Run a plan, reporting how far along it is as a percentage.
///
/// `report` rather than a page value written by name: a page's state
/// handles are reached from a future but are not `Sync`, and this
/// future is spawned with a `Send` bound. A closure the page hands in
/// is the one shape that works from here and reads as what it is.
pub async fn run(ffmpeg: &Path, plan: &Plan, mut report: impl FnMut(f64)) -> Result<(), String> {
    use rustx::tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

    for (file, text) in &plan.subtitles {
        if let Some(directory) = file.parent() {
            std::fs::create_dir_all(directory)
                .map_err(|error| format!("{} could not be made ({error}).", directory.display()))?;
        }
        // No trailing newline: drawtext would draw a second, empty line
        // and push the first one up.
        std::fs::write(file, text)
            .map_err(|error| format!("{} could not be written ({error}).", file.display()))?;
    }

    let mut child = quiet_async(rustx::tokio::process::Command::new(ffmpeg))
        .args(&plan.arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("`{}` could not be run ({error}).", ffmpeg.display()))?;

    let mut output = child.stdout.take();
    let mut errors = child.stderr.take();

    // The two pipes are read side by side rather than one after the
    // other. A render that goes wrong can write more than a pipe holds,
    // and a full stderr stops ffmpeg for ever while we wait on stdout.
    // `join!` and not a spawned task: these two futures belong to the
    // page's own, which is where the progress is written.
    let (_, said) = rustx::tokio::join!(
        async {
            let Some(pipe) = output.take() else { return };
            let mut lines = BufReader::new(pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                match read_progress(&line) {
                    // Never 100 from a timestamp: the file is not
                    // finished until the process has exited.
                    Some(Progress::At(at)) if plan.seconds > 0.0 => {
                        report((at / plan.seconds * 100.0).clamp(0.0, 99.0));
                    }
                    Some(Progress::Done) => report(99.0),
                    _ => {}
                }
            }
        },
        async {
            let mut text = Vec::new();
            if let Some(pipe) = errors.as_mut() {
                let _ = pipe.read_to_end(&mut text).await;
            }
            text
        }
    );

    let status = child
        .wait()
        .await
        .map_err(|error| format!("ffmpeg could not be waited for ({error})."))?;
    match status.success() {
        true => {
            report(100.0);
            Ok(())
        }
        false => Err(format!("ffmpeg refused the render: {}", tail(&said))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two streams and a container, as `ffprobe` really prints them.
    const PROBED: &str = "codec_type=video\n\
         width=640\n\
         height=360\n\
         r_frame_rate=25/1\n\
         codec_type=audio\n\
         r_frame_rate=0/0\n\
         duration=6.000000\n";

    fn clip(name: &str, width: u32, audio: bool) -> Clip {
        let mut clip = Clip::new(Media {
            path: PathBuf::from(format!("C:\\footage\\{name}")),
            duration: 10.0,
            width,
            height: 360,
            fps: 25.0,
            audio,
        });
        clip.end = 10.0;
        clip
    }

    /// The whole pipeline, on a machine that has ffmpeg.
    ///
    /// Everything else in this module is a pure function over sample
    /// text, which is what lets the suite pass with ffmpeg nowhere on
    /// the machine — and which also means nothing else here would
    /// notice if the command line were plausible and wrong. So this one
    /// makes its own footage with `testsrc`, probes it, renders two
    /// trimmed clips with a look and a subtitle, and probes the result;
    /// and when the two programs are not there it returns, because a
    /// test that needs a tool it says is optional is a broken gate.
    #[test]
    fn a_sequence_really_is_rendered_when_ffmpeg_is_on_this_machine() {
        let Ok(tools) = tools() else {
            return;
        };
        let folder =
            std::env::temp_dir().join(format!("rustx-video-editor-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder in the temporary directory");

        let mut made = Vec::new();
        for (name, source) in [
            ("one.mp4", "testsrc=size=640x360:rate=25:duration=6"),
            ("two.mp4", "testsrc2=size=320x240:rate=30:duration=4"),
        ] {
            let file = folder.join(name);
            let drawn = quiet(std::process::Command::new(&tools.ffmpeg))
                .args([
                    "-hide_banner",
                    "-v",
                    "error",
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    source,
                ])
                .args(["-f", "lavfi", "-i", "sine=frequency=440:duration=6"])
                .args([
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                    "-c:a",
                    "aac",
                    "-shortest",
                ])
                .arg(&file)
                .status()
                .expect("ffmpeg runs");
            assert!(drawn.success(), "the footage for this test was not made");
            made.push(file);
        }

        let mut clips: Vec<Clip> = made
            .iter()
            .map(|file| Clip::new(probe(&tools.ffprobe, file).expect("the footage probes")))
            .collect();
        assert_eq!(clips[0].media.width, 640);
        assert!(clips[0].media.audio);
        clips[0].start = 1.0;
        clips[0].end = 3.0;
        clips[0].look = "warm".to_string();
        clips[0].subtitle = "C'est l'été: 50% off".to_string();
        clips[1].end = 1.5;
        clips[1].look = "bw".to_string();

        let out = folder.join("render.mp4");
        let plan = plan(&clips, &out, &folder, font_file().as_deref()).expect("a renderable plan");
        let mut seen: Vec<f64> = Vec::new();
        let said = rustx::tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime")
            .block_on(run(&tools.ffmpeg, &plan, |done| seen.push(done)));
        assert_eq!(said, Ok(()), "the render failed");

        let written = probe(&tools.ffprobe, &out).expect("the render probes");
        assert!(
            (written.duration - 3.5).abs() < 0.2,
            "the output is {}s and the two cuts are 3.5s",
            written.duration
        );
        assert_eq!((written.width, written.height), (640, 360));
        assert!(written.audio, "the output lost its sound");
        // The bar moved and finished, which is ffmpeg's own `-progress`
        // read back rather than a number this application invented.
        assert!(seen.len() > 1, "progress was reported {} times", seen.len());
        assert_eq!(seen.last(), Some(&100.0));

        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_probe_is_read_from_the_video_stream_and_the_container() {
        let media = read_probe(PROBED).expect("the sample has a video stream");
        assert_eq!(media.width, 640);
        assert_eq!(media.height, 360);
        assert_eq!(media.fps, 25.0);
        assert_eq!(media.duration, 6.0);
        assert!(media.audio);
    }

    #[test]
    fn the_audio_streams_frame_rate_is_not_the_videos() {
        // `r_frame_rate=0/0` follows `codec_type=audio` in the real
        // output, and reading the last one seen gave a sequence at 0
        // fps — a render whose every clip was one frame long.
        let swapped = "codec_type=audio\nr_frame_rate=0/0\ncodec_type=video\n\
                       width=1920\nheight=1080\nr_frame_rate=30000/1001\nduration=3.5\n";
        let media = read_probe(swapped).expect("there is a video stream");
        assert!((media.fps - 29.97).abs() < 0.01, "fps was {}", media.fps);
        assert_eq!(media.width, 1920);
    }

    #[test]
    fn a_file_with_no_video_stream_is_refused_by_name() {
        let only_sound = "codec_type=audio\nr_frame_rate=0/0\nduration=12.0\n";
        assert_eq!(
            read_probe(only_sound),
            Err("there is no video stream in this file.".to_string())
        );
    }

    #[test]
    fn a_file_with_no_duration_is_refused_rather_than_put_on_the_timeline() {
        let streaming =
            "codec_type=video\nwidth=640\nheight=360\nr_frame_rate=25/1\nduration=N/A\n";
        assert!(read_probe(streaming).is_err());
    }

    #[test]
    fn a_proxy_is_the_whole_file_in_something_this_window_can_decode() {
        let one = clip("a.mp4", 1280, true);
        let plan = proxy_plan(&one.media, Path::new("C:/cache/proxy-1.webm"));
        let line = plan.arguments.join(" ");
        assert!(line.contains("-i C:\\footage\\a.mp4"), "{line}");
        assert!(line.contains("-c:v libvpx"), "{line}");
        assert!(line.contains("-c:a libvorbis"), "{line}");
        assert!(line.contains("scale=640:-2"), "{line}");
        assert!(line.ends_with("C:/cache/proxy-1.webm"), "{line}");
        // The whole file and not the cut: trimming is the render's job,
        // and a proxy the trim had been applied to would make every
        // timecode in the window wrong.
        assert_eq!(plan.seconds, 10.0);
        assert!(!line.contains("-ss "), "{line}");
    }

    #[test]
    fn a_silent_file_gets_a_silent_proxy_rather_than_a_refused_command() {
        let one = clip("mute.mp4", 1280, false);
        let line = proxy_plan(&one.media, Path::new("out.webm"))
            .arguments
            .join(" ");
        assert!(line.contains("-an"), "{line}");
        assert!(!line.contains("libvorbis"), "{line}");
    }

    #[test]
    fn a_proxy_is_named_after_its_file_and_always_the_same_way() {
        let one = proxy_name(Path::new("C:/footage/a.mp4"));
        assert_eq!(one, proxy_name(Path::new("C:/footage/a.mp4")));
        assert_ne!(one, proxy_name(Path::new("C:/footage/b.mp4")));
        assert!(one.starts_with("proxy-") && one.ends_with(".webm"), "{one}");
    }

    #[test]
    fn the_trim_is_an_input_option_so_nothing_outside_it_is_decoded() {
        let mut one = clip("intro.mp4", 1280, true);
        one.start = 2.5;
        one.end = 7.0;
        let plan = plan(&[one], Path::new("out.mp4"), Path::new("subs"), None)
            .expect("a trimmed clip is renderable");
        let line = plan.arguments.join(" ");
        assert!(
            line.contains("-ss 2.500 -t 4.500 -i C:\\footage\\intro.mp4"),
            "the trim is not in front of the input: {line}"
        );
        assert_eq!(plan.seconds, 4.5);
    }

    #[test]
    fn every_clip_is_scaled_to_the_first_ones_geometry_before_being_concatenated() {
        let clips = vec![clip("a.mp4", 1280, true), clip("b.mp4", 320, true)];
        let plan = plan(&clips, Path::new("out.mp4"), Path::new("subs"), None)
            .expect("two clips are renderable");
        let graph = plan
            .arguments
            .iter()
            .find(|argument| argument.contains("concat="))
            .expect("there is a filter graph");
        assert_eq!(graph.matches("scale=1280:360").count(), 2, "{graph}");
        assert!(
            graph.contains("[v0][a0][v1][a1]concat=n=2:v=1:a=1[v][a]"),
            "{graph}"
        );
        assert!(plan.audio);
    }

    #[test]
    fn a_clip_without_sound_makes_the_whole_sequence_silent() {
        // The named shortcut, asserted rather than described: `concat`
        // with `a=1` wants an audio input from every clip, so one clip
        // without one decides for the sequence.
        let clips = vec![clip("a.mp4", 1280, true), clip("mute.mp4", 1280, false)];
        let plan = plan(&clips, Path::new("out.mp4"), Path::new("subs"), None)
            .expect("two clips are renderable");
        assert!(!plan.audio);
        let line = plan.arguments.join(" ");
        assert!(line.contains("concat=n=2:v=1:a=0[v]"), "{line}");
        assert!(!line.contains("[a]"), "{line}");
        assert!(!line.contains("-c:a"), "{line}");
    }

    #[test]
    fn a_clip_that_is_switched_off_is_left_out_of_the_render() {
        let mut off = clip("off.mp4", 1280, true);
        off.enabled = false;
        let on = clip("on.mp4", 1280, true);
        let plan = plan(&[off.clone(), on], Path::new("out.mp4"), Path::new("subs"), None).expect("renderable");
        let line = plan.arguments.join(" ");
        assert!(line.contains("on.mp4") && !line.contains("off.mp4"), "{line}");
        assert!(super::plan(&[off], Path::new("out.mp4"), Path::new("subs"), None).is_err());
    }

    #[test]
    fn a_grade_is_in_the_graph_and_in_the_frame_and_absent_when_untouched() {
        let mut one = clip("a.mp4", 1280, true);
        assert_eq!(one.grade_filter(), None);
        let plain = plan(&[one.clone()], Path::new("out.mp4"), Path::new("subs"), None).expect("renderable");
        assert!(!plain.arguments.join(" ").contains("eq="));
        one.brightness = 0.25;
        one.saturation = 1.5;
        assert_eq!(one.grade_filter().as_deref(), Some("eq=brightness=0.25:contrast=1.00:saturation=1.50"));
        let graded = plan(&[one.clone()], Path::new("out.mp4"), Path::new("subs"), None).expect("renderable");
        assert!(graded.arguments.join(" ").contains("eq=brightness=0.25:contrast=1.00:saturation=1.50,format=yuv420p"));
        assert!(frame_arguments(&one, 1.0, Path::new("f.jpg")).join(" ").contains("-vf eq=brightness=0.25"));
        assert_eq!(one.grade_css(), "brightness(1.25) contrast(1.00) saturate(1.50)");
    }

    #[test]
    fn the_look_a_clip_carries_is_in_the_graph_and_before_the_pixel_format() {
        let mut one = clip("a.mp4", 1280, true);
        one.look = "bw".to_string();
        let plan = plan(&[one], Path::new("out.mp4"), Path::new("subs"), None).expect("renderable");
        let graph = plan.arguments.join(" ");
        assert!(
            graph.contains("format=gray,format=yuv420p"),
            "black and white must be converted back for the encoder: {graph}"
        );
    }

    #[test]
    fn a_subtitle_goes_through_a_file_so_that_nothing_in_it_is_syntax() {
        let mut one = clip("a.mp4", 1280, true);
        one.subtitle = "C'est l'été: 50% off".to_string();
        let plan = plan(
            &[one],
            Path::new("out.mp4"),
            Path::new("C:\\cache"),
            Some(Path::new("C:\\Windows\\Fonts\\arial.ttf")),
        )
        .expect("renderable");
        let graph = plan.arguments.join(" ");
        assert!(
            graph.contains("textfile='C\\:/cache/subtitle-0.txt':expansion=none"),
            "{graph}"
        );
        assert!(
            graph.contains("fontfile='C\\:/Windows/Fonts/arial.ttf'"),
            "{graph}"
        );
        // The text is nowhere in the command line, which is the point.
        assert!(!graph.contains("50%"), "{graph}");
        assert_eq!(
            plan.subtitles,
            vec![(
                PathBuf::from("C:\\cache\\subtitle-0.txt"),
                "C'est l'été: 50% off".to_string()
            )]
        );
    }

    #[test]
    fn a_subtitle_with_no_font_on_the_machine_is_a_named_refusal() {
        let mut one = clip("a.mp4", 1280, true);
        one.subtitle = "hello".to_string();
        let refused = plan(&[one], Path::new("out.mp4"), Path::new("subs"), None)
            .expect_err("there is no font");
        assert!(refused.contains("a.mp4"), "{refused}");
    }

    #[test]
    fn an_empty_timeline_and_a_clip_trimmed_to_nothing_are_both_refused() {
        assert!(plan(&[], Path::new("out.mp4"), Path::new("subs"), None).is_err());
        let mut one = clip("a.mp4", 1280, true);
        one.end = one.start;
        assert!(plan(&[one], Path::new("out.mp4"), Path::new("subs"), None).is_err());
    }

    #[test]
    fn a_quote_in_a_path_closes_the_quoted_string_and_is_reopened() {
        assert_eq!(
            in_filter(Path::new("C:\\Users\\O'Hara\\font.ttf")),
            "C\\:/Users/O'\\''Hara/font.ttf"
        );
    }

    #[test]
    fn a_frame_is_asked_for_by_seeking_first_and_carries_the_clips_look() {
        let mut one = clip("a.mp4", 1280, true);
        one.look = "warm".to_string();
        let line = frame_arguments(&one, 3.25, Path::new("frame.jpg")).join(" ");
        assert!(
            line.contains("-ss 3.250 -i C:\\footage\\a.mp4 -frames:v 1"),
            "{line}"
        );
        assert!(
            line.contains("-vf colortemperature=temperature=3000"),
            "{line}"
        );
    }

    #[test]
    fn progress_is_read_from_the_lines_that_carry_it_and_from_no_others() {
        assert_eq!(
            read_progress("out_time_us=3178667"),
            Some(Progress::At(3.178667))
        );
        assert_eq!(read_progress("progress=end"), Some(Progress::Done));
        assert_eq!(read_progress("progress=continue"), None);
        assert_eq!(read_progress("out_time_us=N/A"), None);
        assert_eq!(read_progress("bitrate=   0.0kbits/s"), None);
        assert_eq!(read_progress("frame=42"), None);
    }

    #[test]
    fn a_failure_is_reported_with_ffmpegs_own_last_words() {
        let stderr = b"ffmpeg version 9.0.2\n  configuration: --enable-gpl\n\
            libavutil 60. 13.100\n  libavcodec 62. 20.100\n  libavformat 62. 6.100\n\
            [in#0 @ 000] Error opening input: No such file or directory\n\
            Error opening input file C:\\gone.mp4.\n\n";
        let said = tail(stderr);
        assert!(said.contains("No such file or directory"), "{said}");
        assert!(!said.contains("--enable-gpl"), "{said}");
        assert_eq!(tail(b""), "it failed and said nothing.");
    }
}
