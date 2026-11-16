//! La capture d'écran, sous Windows : GDI, une section DIB, `BitBlt`.
//!
//! # Pourquoi une section DIB et pas un bitmap compatible
//!
//! `CreateCompatibleBitmap` donne un bitmap dont les octets sont au
//! format de la carte, qu'il faut ensuite redemander par `GetDIBits`.
//! `CreateDIBSection` donne le bitmap *et* le pointeur sur ses octets,
//! dans le format qu'on a demandé : 32 bits, `BI_RGB`, donc du BGRA —
//! exactement ce que [`Image`] porte, et ce que le presse-papiers de ce
//! crate échange déjà.
//!
//! # Le haut en bas
//!
//! Une hauteur **négative** dans `BITMAPINFOHEADER` demande un bitmap
//! « descendant » : la première ligne des octets est la ligne du haut
//! de l'écran. Sans ce signe, Windows rend l'image à l'envers, et c'est
//! le piège de cette API — il ne se voit qu'à l'œil, sur une capture
//! retournée.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SRCCOPY,
    SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowRect, IsWindow, PW_RENDERFULLCONTENT, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::capture::bytes_for;
use crate::clipboard::Image;
use crate::error::OsError;
use crate::screen::Area;
use crate::window::Pane;

/// Le bureau entier, tous écrans confondus : son coin et sa taille.
///
/// Le coin peut être négatif — un moniteur à gauche du principal — et
/// c'est pour cela qu'il est lu et non supposé nul.
fn virtual_screen() -> Area {
    // SAFETY: quatre lectures de métriques système, sans argument.
    unsafe {
        Area {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        }
    }
}

/// Un contexte d'affichage de l'écran, rendu à la sortie.
struct ScreenDc(HDC);

impl Drop for ScreenDc {
    fn drop(&mut self) {
        // SAFETY: le contexte que `GetDC(None)` a rendu, rendu une fois.
        // La valeur dit s'il l'était déjà, ce qui n'arrive pas ici.
        let _ = unsafe { ReleaseDC(None, self.0) };
    }
}

/// Un contexte en mémoire et son bitmap, défaits à la sortie.
struct Canvas {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    width: u32,
    height: u32,
}

impl Canvas {
    /// Un canevas de cette taille, compatible avec cet écran.
    fn new(screen: HDC, width: u32, height: u32) -> Result<Self, OsError> {
        let length = bytes_for(width, height)?;
        let described = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                // Négatif : la première ligne est celle du haut.
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        // SAFETY: un contexte d'écran vivant.
        let dc = unsafe { CreateCompatibleDC(Some(screen)) };
        if dc.is_invalid() {
            return Err(OsError::Platform(
                "GDI refused a memory device context".to_string(),
            ));
        }
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        // SAFETY: une description remplie, et un pointeur que GDI écrit.
        let bitmap =
            unsafe { CreateDIBSection(Some(dc), &described, DIB_RGB_COLORS, &mut bits, None, 0) };
        let Ok(bitmap) = bitmap else {
            // SAFETY: le contexte qu'on vient de créer.
            let _ = unsafe { DeleteDC(dc) };
            return Err(OsError::Platform(format!(
                "GDI refused a {width}×{height} bitmap: {}",
                windows::core::Error::from_thread()
            )));
        };
        if bits.is_null() {
            // SAFETY: les deux objets qu'on vient de créer.
            unsafe {
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(dc);
            }
            return Err(OsError::Platform(
                "GDI gave a bitmap with no pixels".to_string(),
            ));
        }
        // SAFETY: le bitmap et le contexte qu'on vient de créer ; l'objet
        // rendu est celui qu'il faudra remettre avant de défaire.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };
        debug_assert_eq!(length, (width * height * 4) as usize);
        Ok(Self {
            dc,
            bitmap,
            previous,
            bits: bits.cast(),
            width,
            height,
        })
    }

    /// Les octets, copiés hors du bitmap.
    fn image(&self) -> Result<Image, OsError> {
        let length = bytes_for(self.width, self.height)?;
        // SAFETY: `bits` pointe sur `width * height * 4` octets que GDI a
        // alloués pour ce bitmap et qui vivent tant que lui.
        let bgra = unsafe { std::slice::from_raw_parts(self.bits, length) }.to_vec();
        Ok(Image {
            width: self.width,
            height: self.height,
            bgra,
        })
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        // SAFETY: on remet l'objet d'origine avant de défaire le bitmap —
        // un bitmap encore sélectionné dans un contexte ne se supprime
        // pas — puis les deux objets de ce canevas, une fois chacun.
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

pub fn area(area: Option<Area>) -> Result<Image, OsError> {
    let wanted = area.unwrap_or_else(virtual_screen);
    let (width, height) = (
        u32::try_from(wanted.width).unwrap_or(0),
        u32::try_from(wanted.height).unwrap_or(0),
    );
    // SAFETY: le contexte de l'écran entier ; `ScreenDc` le rend.
    let screen = ScreenDc(unsafe { GetDC(None) });
    if screen.0.is_invalid() {
        return Err(OsError::Platform(
            "the screen's device context could not be had".to_string(),
        ));
    }
    let canvas = Canvas::new(screen.0, width, height)?;
    // SAFETY: deux contextes vivants, et un rectangle dans l'écran.
    unsafe {
        BitBlt(
            canvas.dc,
            0,
            0,
            width as i32,
            height as i32,
            Some(screen.0),
            wanted.x,
            wanted.y,
            SRCCOPY,
        )
    }
    .map_err(|error| OsError::Platform(format!("the screen could not be copied: {error}")))?;
    canvas.image()
}

pub fn window(pane: Pane) -> Result<Image, OsError> {
    let handle = HWND(pane.0 as *mut std::ffi::c_void);
    // SAFETY: lecture seule ; c'est la question posée.
    if !pane.exists() || !unsafe { IsWindow(Some(handle)) }.as_bool() {
        return Err(OsError::Platform(format!(
            "the handle {:#x} is not a window",
            pane.0
        )));
    }
    let mut rect = Default::default();
    // SAFETY: lecture seule sur une poignée vivante.
    unsafe { GetWindowRect(handle, &mut rect) }
        .map_err(|error| OsError::Platform(format!("the window's place is unreadable: {error}")))?;
    let width = u32::try_from(rect.right - rect.left).unwrap_or(0);
    let height = u32::try_from(rect.bottom - rect.top).unwrap_or(0);
    // SAFETY: le contexte de l'écran, pour une compatibilité de format.
    let screen = ScreenDc(unsafe { GetDC(None) });
    let canvas = Canvas::new(screen.0, width, height)?;
    // `PW_RENDERFULLCONTENT` : sans lui, une fenêtre accélérée — ce
    // qu'une fenêtre Chromium est — se copie en noir. C'est le piège de
    // cet appel, et il ne se voit qu'à l'image.
    // SAFETY: une poignée vivante et un contexte de la bonne taille.
    let drawn = unsafe { PrintWindow(handle, canvas.dc, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT)) };
    if !drawn.as_bool() {
        // La fenêtre a refusé de se dessiner : la copier depuis l'écran
        // reste possible, et donne la même image si rien ne la recouvre.
        return self::area(Some(Area::between(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
        )));
    }
    canvas.image()
}
