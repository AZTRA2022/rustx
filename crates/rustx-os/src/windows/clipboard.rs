use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
    OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::core::HSTRING;

use crate::clipboard::Image;
use crate::error::OsError;

/// The two formats this crate names. `CF_UNICODETEXT` and `CF_DIB`
/// are constants of the API, written here because the generated
/// bindings put them in a module nothing else in this file needs.
const CF_UNICODETEXT: u32 = 13;
const CF_DIB: u32 = 8;

/// Windows has one, and this file reaches it.
pub const AVAILABLE: bool = true;

/// How long the open is retried, and how long between tries: the
/// framework's own numbers, not Windows'.
use crate::clipboard::{BETWEEN, PATIENCE};

/// The clipboard, open. Closed when this is dropped, whatever
/// happened in between — a clipboard left open is every other
/// program's copy and paste broken until this process exits.
pub struct Held;

impl Drop for Held {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

/// Open it, retrying while somebody else has it.
///
/// The contention cannot be staged from inside one process — two
/// threads of the same process both get in — so what a test can
/// pin is the policy: `patience::retrying`, with these two numbers.
fn open() -> Result<Held, OsError> {
    crate::patience::retrying(
        || {
            // No window handle: this process owns the clipboard for
            // as long as the guard lives, which is one call.
            unsafe { OpenClipboard(None) }.map(|()| Held).map_err(|error| {
                OsError::Busy(format!(
                    "the clipboard (another application has it open: {})",
                    error.message()
                ))
            })
        },
        PATIENCE,
        BETWEEN,
    )
}

/// What the clipboard holds as text, or an empty string when it
/// holds none.
///
/// Empty rather than an error: a clipboard with nothing in it is the
/// normal state of a machine that has just booted, and a developer
/// who has to match on that writes the same `unwrap_or_default`
/// every time.
pub fn text() -> Result<String, OsError> {
    let _open = open()?;
    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }.is_err() {
        return Ok(String::new());
    }
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) }
        .map_err(|error| OsError::Platform(error.message()))?;
    let block = Block::locked(handle)?;
    let bytes = block.bytes();
    // UTF-16, and the terminating NUL is not part of the text.
    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect();
    Ok(String::from_utf16_lossy(&wide))
}

/// Put text on the clipboard, replacing whatever was there.
pub fn set_text(text: &str) -> Result<(), OsError> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let bytes: Vec<u8> = wide.iter().flat_map(|unit| unit.to_le_bytes()).collect();
    put(CF_UNICODETEXT, &bytes)
}

/// The image on the clipboard, if what is there is an image.
pub fn image() -> Result<Image, OsError> {
    let _open = open()?;
    if unsafe { IsClipboardFormatAvailable(CF_DIB) }.is_err() {
        return Err(OsError::NoSuchDevice(
            "an image on the clipboard — what is there is not one".to_string(),
        ));
    }
    let handle =
        unsafe { GetClipboardData(CF_DIB) }.map_err(|error| OsError::Platform(error.message()))?;
    let block = Block::locked(handle)?;
    read_dib(block.bytes())
}

/// Put an image on the clipboard, replacing whatever was there.
pub fn set_image(image: &Image) -> Result<(), OsError> {
    put(CF_DIB, &written_dib(image)?)
}

/// Bytes under a name a project invents.
///
/// The name is registered with Windows, which hands back the same
/// number to every program that asks for the same name — that is
/// what makes two windows of one application able to pass their own
/// objects to each other, and what keeps anything else on the
/// machine from reading them by accident.
pub fn set_custom(format: &str, bytes: &[u8]) -> Result<(), OsError> {
    let registered = register(format)?;
    // The length is written in front of the bytes. `GlobalSize` is
    // allowed to answer with more than was asked for, and a format
    // whose length is "about this many" is a format that cannot
    // carry a project's own objects.
    let mut block = Vec::with_capacity(bytes.len() + 4);
    block.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    block.extend_from_slice(bytes);
    put(registered, &block)
}

/// Read back what `set_custom` wrote under that name.
pub fn custom(format: &str) -> Result<Vec<u8>, OsError> {
    let registered = register(format)?;
    let _open = open()?;
    if unsafe { IsClipboardFormatAvailable(registered) }.is_err() {
        return Err(OsError::NoSuchDevice(format!(
            "`{format}` on the clipboard — nothing has put that format there"
        )));
    }
    let handle = unsafe { GetClipboardData(registered) }
        .map_err(|error| OsError::Platform(error.message()))?;
    let block = Block::locked(handle)?;
    let bytes = block.bytes();
    let Some(header) = bytes.get(..4) else {
        return Err(OsError::Platform(format!(
            "`{format}` on the clipboard is too short to be anything rustx wrote"
        )));
    };
    let length = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let body = bytes.get(4..4 + length).ok_or_else(|| {
        OsError::Platform(format!(
            "`{format}` on the clipboard says it is {length} bytes and is not"
        ))
    })?;
    Ok(body.to_vec())
}

/// The number Windows gives this name.
fn register(format: &str) -> Result<u32, OsError> {
    let name = HSTRING::from(format);
    let registered = unsafe { RegisterClipboardFormatW(&name) };
    if registered == 0 {
        return Err(OsError::Platform(format!(
            "Windows would not register the clipboard format `{format}`"
        )));
    }
    Ok(registered)
}

/// Open the clipboard, empty it, and hand it one block under one
/// format.
///
/// The block is Windows' afterwards: `SetClipboardData` takes it,
/// and freeing it here would be freeing somebody else's memory. It
/// is freed on the way out only when the hand-over failed.
fn put(format: u32, bytes: &[u8]) -> Result<(), OsError> {
    let _open = open()?;
    unsafe { EmptyClipboard() }.map_err(|error| OsError::Platform(error.message()))?;

    let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }
        .map_err(|error| OsError::Platform(error.message()))?;
    unsafe {
        let at = GlobalLock(block);
        if at.is_null() {
            let _ = GlobalFree(Some(block));
            return Err(OsError::Platform(
                "Windows would not lock the block the clipboard was to be given".to_string(),
            ));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len());
        let _ = GlobalUnlock(block);
    }

    let given = unsafe { SetClipboardData(format, Some(HANDLE(block.0))) };
    if let Err(error) = given {
        unsafe {
            let _ = GlobalFree(Some(block));
        }
        return Err(OsError::Platform(error.message()));
    }
    Ok(())
}

/// A clipboard block, locked for as long as this lives.
struct Block {
    handle: HGLOBAL,
    at: *mut u8,
    size: usize,
}

impl Block {
    fn locked(handle: HANDLE) -> Result<Self, OsError> {
        let handle = HGLOBAL(handle.0);
        let at = unsafe { GlobalLock(handle) } as *mut u8;
        if at.is_null() {
            return Err(OsError::Platform(
                "Windows would not lock what the clipboard holds".to_string(),
            ));
        }
        let size = unsafe { GlobalSize(handle) };
        Ok(Self { handle, at, size })
    }

    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.at, self.size) }
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        unsafe {
            let _ = GlobalUnlock(self.handle);
        }
    }
}

/// An image as a `CF_DIB`: the header, then the rows.
///
/// Bottom-up, which is what a DIB is unless its height is negative,
/// and a negative height is a thing half the programs on the machine
/// read wrong. So the rows are turned over on the way in and on the
/// way out, and every other program sees an ordinary bitmap.
fn written_dib(image: &Image) -> Result<Vec<u8>, OsError> {
    let stride = image.width as usize * 4;
    let expected = stride * image.height as usize;
    if image.bgra.len() != expected {
        return Err(OsError::Platform(format!(
            "an image of {}×{} is {expected} bytes of BGRA, and this one is {}",
            image.width,
            image.height,
            image.bgra.len()
        )));
    }

    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: image.width as i32,
        biHeight: image.height as i32,
        biPlanes: 1,
        biBitCount: 32,
        // BI_RGB: no compression, four bytes a pixel, the fourth
        // ignored by Windows and carried through by everything.
        biCompression: 0,
        biSizeImage: expected as u32,
        ..Default::default()
    };

    let mut out = Vec::with_capacity(header.biSize as usize + expected);
    out.extend_from_slice(unsafe {
        std::slice::from_raw_parts(
            (&header as *const BITMAPINFOHEADER) as *const u8,
            header.biSize as usize,
        )
    });
    for row in image.bgra.chunks_exact(stride).rev() {
        out.extend_from_slice(row);
    }
    Ok(out)
}

/// The other way: a `CF_DIB` read back into rows top to bottom.
fn read_dib(bytes: &[u8]) -> Result<Image, OsError> {
    let head = std::mem::size_of::<BITMAPINFOHEADER>();
    if bytes.len() < head {
        return Err(OsError::Platform(
            "the image on the clipboard is shorter than its own header".to_string(),
        ));
    }
    // Read field by field rather than cast: the clipboard's header
    // may be a `BITMAPV5HEADER`, which starts with these same
    // fields and goes on for longer.
    let number = |at: usize| {
        i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    let size = number(0) as usize;
    let width = number(4);
    let height = number(8);
    let bits = u16::from_le_bytes([bytes[14], bytes[15]]);
    let compression = number(16);

    if bits != 32 || compression != 0 && compression != 3 {
        return Err(OsError::Platform(format!(
            "the image on the clipboard is {bits}-bit (compression {compression}); \
             rustx reads the 32-bit ones. Paste it into something that converts it, \
             or ask the application that copied it for a plain bitmap."
        )));
    }

    let upside_down = height > 0;
    let (width, height) = (width.unsigned_abs(), height.unsigned_abs());
    let stride = width as usize * 4;
    let wanted = stride * height as usize;
    // A `BI_BITFIELDS` header is followed by three masks; a
    // palette, which a 32-bit image does not have, would be here
    // too. The rows are at the end, which is where they are read
    // from.
    let from = bytes.len().checked_sub(wanted).ok_or_else(|| {
        OsError::Platform(format!(
            "the image on the clipboard says it is {width}×{height} and carries \
             {} bytes, which is not enough",
            bytes.len().saturating_sub(size)
        ))
    })?;
    let rows = &bytes[from..];

    let mut bgra = Vec::with_capacity(wanted);
    if upside_down {
        for row in rows.chunks_exact(stride).rev() {
            bgra.extend_from_slice(row);
        }
    } else {
        bgra.extend_from_slice(rows);
    }
    Ok(Image {
        width,
        height,
        bgra,
    })
}
