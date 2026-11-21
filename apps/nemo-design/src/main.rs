//! Nemo Design: a node editor, a 3D viewport and a timeline, in one window.
//!
//! The editor itself is `components/NemoApp.tsx` (React, three.js): the graph of cards,
//! the viewport, the timeline, undo and redo, the music made in the page. What the page
//! cannot do alone is here, as natives it calls like functions:
//!
//! | native | what it does |
//! |---|---|
//! | `nemo_pick_image()` | asks for an image file, answers `{ name, url }` with the picture as a `data:` URL |
//! | `nemo_save(name, base64, extension)` | asks where to write, writes the bytes, answers the path |
//! | `nemo_open_text(extension)` | asks for a file and answers its text |
//! | `nemo_copy(text)` | puts text on the clipboard |
//!
//! Nothing here calls a generation model: the AI MODELS card is a choice that is kept, not a
//! request that is sent.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustx::os::dialog::{self, Ask, DialogError};
use rustx::{App, Config, NativeError, Value, register_native};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Nemo Design")
            .size(1280, 720)
            .min_size(720, 520)
            .frameless()
            .remember_window()
            .description("Cards, a 3D viewport and a timeline: design a character and export it.")
            .publisher("rustx")
            .on_context_created(natives),
    )?;

    app.run()
}

fn natives() {
    // The window has no frame: the page draws a bar and three marks, and these are what they do.
    register_native("nemo_window", |args| {
        match args.first() {
            Some(Value::String(order)) => match order.as_str() {
                "drag" => rustx::window::drag(),
                "minimize" => rustx::window::minimize(),
                "maximize" => rustx::window::toggle_maximize(),
                "close" => rustx::window::shut(),
                other => return Err(NativeError::new(format!("no window order `{other}`"))),
            },
            _ => return Err(NativeError::new("nemo_window takes an order")),
        }
        Ok(Value::Null)
    });

    register_native("nemo_pick_image", |_| {
        let ask =
            Ask::new("Choose an image").filter("Images", ["png", "jpg", "jpeg", "webp", "gif"]);
        let path = match dialog::open(ask) {
            Ok(mut paths) if !paths.is_empty() => paths.remove(0),
            Ok(_) | Err(DialogError::Cancelled) => return Ok(Value::Null),
            Err(why) => return Err(NativeError::new(why.to_string())),
        };
        let bytes = std::fs::read(&path)
            .map_err(|e| NativeError::new(format!("{}: {e}", path.display())))?;
        if bytes.len() > 12 * 1024 * 1024 {
            return Err(NativeError::new("That image is larger than 12 MB."));
        }
        let mime = mime_of(&path.to_string_lossy());
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok(rustx::record([
            ("name", Value::String(name)),
            (
                "url",
                Value::String(format!("data:{mime};base64,{}", base64(&bytes))),
            ),
        ]))
    });

    register_native("nemo_save", |args| {
        let (Some(Value::String(name)), Some(Value::String(data)), Some(Value::String(extension))) =
            (args.first(), args.get(1), args.get(2))
        else {
            return Err(NativeError::new(
                "nemo_save takes a name, the bytes in base64 and an extension",
            ));
        };
        let ask = Ask::new("Save")
            .named(format!("{name}.{extension}"))
            .filter("File", [extension.clone()]);
        let path = match dialog::save(ask) {
            Ok(path) => path,
            Err(DialogError::Cancelled) => return Ok(Value::Null),
            Err(why) => return Err(NativeError::new(why.to_string())),
        };
        let bytes = unbase64(data).ok_or_else(|| NativeError::new("the data is not base64"))?;
        std::fs::write(&path, bytes)
            .map_err(|e| NativeError::new(format!("{}: {e}", path.display())))?;
        Ok(Value::String(path.display().to_string()))
    });

    register_native("nemo_open_text", |args| {
        let extension = match args.first() {
            Some(Value::String(e)) => e.clone(),
            _ => "nemo".to_string(),
        };
        let path = match dialog::open(Ask::new("Open").filter("File", [extension])) {
            Ok(mut paths) if !paths.is_empty() => paths.remove(0),
            Ok(_) | Err(DialogError::Cancelled) => return Ok(Value::Null),
            Err(why) => return Err(NativeError::new(why.to_string())),
        };
        let text = std::fs::read_to_string(&path)
            .map_err(|e| NativeError::new(format!("{}: {e}", path.display())))?;
        Ok(Value::String(text))
    });

    register_native("nemo_copy", |args| {
        if let Some(Value::String(text)) = args.first() {
            rustx::os::clipboard::set_text(text).map_err(|e| NativeError::new(e.to_string()))?;
        }
        Ok(Value::Null)
    });
}

fn mime_of(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut bits, mut have) = (0_u32, 0);
    for byte in text.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = ALPHABET.iter().position(|&a| a == byte)? as u32;
        bits = (bits << 6) | value;
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have) as u8);
            bits &= (1 << have) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_every_length() {
        for n in 0..40_usize {
            let data: Vec<u8> = (0..n as u8).map(|b| b.wrapping_mul(37)).collect();
            assert_eq!(unbase64(&base64(&data)).unwrap(), data, "length {n}");
        }
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert!(unbase64("***").is_none());
    }

    #[test]
    fn an_image_is_named_by_its_extension() {
        assert_eq!(mime_of("a/b/c.JPG"), "image/jpeg");
        assert_eq!(mime_of("noext"), "application/octet-stream");
    }
}
