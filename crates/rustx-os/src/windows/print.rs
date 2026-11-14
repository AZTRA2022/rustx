//! L'impression, sous Windows : les verbes du Shell, et
//! `EnumPrintersW`.
//!
//! # Deux verbes, et pourquoi pas une API d'impression
//!
//! `ShellExecuteW` avec le verbe `"print"` demande à l'application
//! associée au fichier de l'imprimer — c'est ce que fait le clic droit
//! « Imprimer » de l'Explorateur, et c'est le seul moyen d'imprimer un
//! PDF ou un `.docx` sans savoir les lire. `"printto"` fait la même
//! chose vers une imprimante nommée, et c'est ainsi qu'on envoie sans
//! dialogue.
//!
//! L'API d'impression de Windows (`StartDocPrinter`, `StartPage`) n'est
//! pas celle-ci : elle attend qu'on *dessine* le document page par
//! page, ce qui suppose de savoir lire le format. Ce fichier imprime un
//! fichier, pas des pixels.
//!
//! # Ce que « sans dialogue » vaut vraiment
//!
//! `"printto"` est un verbe que l'application associée *peut* ne pas
//! déclarer — le lecteur de PDF intégré à Edge, par exemple, le
//! déclare, d'autres non. Quand il manque, le Shell rend
//! `ERROR_NO_ASSOCIATION`, et c'est dit en nommant le verbe plutôt que
//! retombé en silence sur un dialogue que personne n'attendait.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::ERROR_NO_ASSOCIATION;
use windows::Win32::Graphics::Printing::{
    EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
    PRINTER_INFO_4W,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::PCWSTR;

use crate::error::OsError;
use crate::print::Printer;

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// L'imprimante par défaut, si la machine en a une.
pub(crate) fn default_printer() -> Option<String> {
    let mut length = 0u32;
    // Un premier appel pour la longueur : il échoue exprès, en disant
    // combien de caractères il faut.
    // SAFETY: aucun tampon, et la longueur demandée en retour.
    let _ = unsafe { GetDefaultPrinterW(None, &mut length) };
    if length == 0 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize];
    // SAFETY: un tampon de la longueur que Windows vient de dire.
    let got =
        unsafe { GetDefaultPrinterW(Some(windows::core::PWSTR(buffer.as_mut_ptr())), &mut length) };
    if !got.as_bool() {
        return None;
    }
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let name = String::from_utf16_lossy(&buffer[..end]);
    (!name.is_empty()).then_some(name)
}

pub fn printers() -> Result<Vec<Printer>, OsError> {
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    // Niveau 4 : le nom et le serveur, sans ouvrir chaque imprimante —
    // le niveau que Microsoft recommande pour une simple liste, et le
    // seul qui ne réveille pas une imprimante réseau endormie.
    const LEVEL: u32 = 4;
    let mut needed = 0u32;
    let mut count = 0u32;
    // SAFETY: aucun tampon ; l'appel échoue en disant la taille voulue.
    let _ = unsafe { EnumPrintersW(flags, None, LEVEL, None, &mut needed, &mut count) };
    if needed == 0 {
        return Ok(Vec::new());
    }
    let mut buffer = vec![0u8; needed as usize];
    // SAFETY: un tampon de la taille que Windows vient de dire.
    unsafe {
        EnumPrintersW(
            flags,
            None,
            LEVEL,
            Some(&mut buffer),
            &mut needed,
            &mut count,
        )
    }
    .map_err(|error| OsError::Platform(format!("the printers could not be listed: {error}")))?;
    let default = default_printer();
    let mut all = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        // SAFETY: Windows a écrit `count` structures `PRINTER_INFO_4W`
        // au début de ce tampon, et le tampon vit jusqu'à la fin de la
        // fonction. Les noms sont des chaînes terminées par zéro qui
        // vivent dedans.
        let name = unsafe {
            let at = buffer.as_ptr().cast::<PRINTER_INFO_4W>().add(index);
            let found = &*at;
            if found.pPrinterName.is_null() {
                continue;
            }
            found.pPrinterName.to_string().unwrap_or_default()
        };
        if name.is_empty() {
            continue;
        }
        all.push(Printer {
            default: default.as_deref() == Some(name.as_str()),
            name,
        });
    }
    Ok(all)
}

pub fn print(path: &Path, ask: bool) -> Result<(), OsError> {
    let file = wide(&path.to_string_lossy());
    let verb = wide(if ask { "print" } else { "printto" });
    // `printto` prend l'imprimante entre guillemets en paramètre : un
    // nom d'imprimante avec une espace — « Microsoft Print to PDF » en a
    // deux — en vaudrait trois sans eux.
    let printer = if ask {
        None
    } else {
        let name = default_printer().ok_or_else(|| {
            OsError::Platform("there is no default printer to print to without asking".to_string())
        })?;
        Some(wide(&format!("\"{name}\"")))
    };
    // SAFETY: des chaînes terminées par zéro qui vivent jusqu'au retour.
    // `SW_HIDE` : l'application associée imprime et ne se montre pas.
    let answer = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            printer
                .as_ref()
                .map(|printer| PCWSTR(printer.as_ptr()))
                .unwrap_or(PCWSTR::null()),
            PCWSTR::null(),
            SW_HIDE,
        )
    };
    // `ShellExecuteW` rend une valeur supérieure à 32 quand il a réussi
    // — un reste de Windows 16 bits, où c'était une poignée d'instance.
    let code = answer.0 as usize;
    if code > 32 {
        return Ok(());
    }
    if code == ERROR_NO_ASSOCIATION.0 as usize {
        return Err(OsError::Unsupported(if ask {
            "printing this kind of file (no installed application declares a `print` verb \
             for it)"
        } else {
            "printing this kind of file without a dialog (no installed application declares \
             a `printto` verb for it)"
        }));
    }
    Err(OsError::Platform(format!(
        "the Shell refused to print {}: code {code}",
        path.display()
    )))
}
