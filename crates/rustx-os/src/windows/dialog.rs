//! Le dialogue de fichiers de Windows : `IFileOpenDialog`.
//!
//! Celui de Vista et d'après — la barre de navigation, les
//! emplacements, la recherche — et non le `GetOpenFileNameW` de
//! Windows 95, qui demande trois fois moins de code et donne un
//! dialogue qu'un utilisateur reconnaît comme vieux.
//!
//! # À qui le dialogue appartient
//!
//! Un dialogue modal a un propriétaire : la fenêtre qu'il bloque, et
//! devant laquelle il reste. La fenêtre de l'application est faite par
//! le processus navigateur, et ce code tourne dans celui de rendu : son
//! `HWND` n'est pas ici. Ce qui l'est, c'est la fenêtre de premier plan
//! — et au moment où quelqu'un clique sur « Ouvrir… », la fenêtre de
//! premier plan *est* la nôtre. C'est donc elle, et c'est exact autant
//! qu'un clic est un clic.

use std::path::PathBuf;

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PICKFOLDERS,
    FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog, IShellItem,
    SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::core::{HSTRING, Interface, PCWSTR};

use crate::dialog::{Ask, DialogError, patterns_for};

/// Windows en a un, et ce fichier l'ouvre.
pub const AVAILABLE: bool = true;

/// Ce que `Show` rend quand l'utilisateur ferme le dialogue.
///
/// `HRESULT_FROM_WIN32(ERROR_CANCELLED)`, mesuré en fermant un vrai
/// dialogue : la documentation dit « E_ABORT ou ERROR_CANCELLED » selon
/// la page qu'on lit, et une annulation prise pour une panne, c'est une
/// application qui journalise une erreur chaque fois que quelqu'un
/// change d'avis. Les deux sont donc acceptés.
const CANCELLED: i32 = 0x8007_04C7_u32 as i32;
const ABORTED: i32 = 0x8000_4004_u32 as i32;

pub fn open(ask: &Ask) -> Result<Vec<PathBuf>, DialogError> {
    let _com = Apartment::entered()?;
    let dialog: IFileOpenDialog = made(FileOpenDialog)?;
    let mut options = FOS_FORCEFILESYSTEM;
    if ask.many {
        options |= FOS_ALLOWMULTISELECT;
    }
    dressed(&dialog, ask, options)?;

    shown(&dialog)?;
    let chosen = unsafe { dialog.GetResults() }.map_err(refused)?;
    let how_many = unsafe { chosen.GetCount() }.map_err(refused)?;
    let mut paths = Vec::with_capacity(how_many as usize);
    for at in 0..how_many {
        let item = unsafe { chosen.GetItemAt(at) }.map_err(refused)?;
        paths.push(path_of(&item)?);
    }
    if paths.is_empty() {
        return Err(DialogError::Cancelled);
    }
    Ok(paths)
}

pub fn save(ask: &Ask) -> Result<PathBuf, DialogError> {
    let _com = Apartment::entered()?;
    let dialog: IFileDialog = made(FileSaveDialog)?;
    dressed(&dialog, ask, FOS_FORCEFILESYSTEM | FOS_OVERWRITEPROMPT)?;
    if let Some(name) = &ask.named {
        unsafe { dialog.SetFileName(&HSTRING::from(name.as_str())) }.map_err(refused)?;
    }

    shown(&dialog)?;
    let item = unsafe { dialog.GetResult() }.map_err(refused)?;
    path_of(&item)
}

pub fn folder(ask: &Ask) -> Result<PathBuf, DialogError> {
    let _com = Apartment::entered()?;
    let dialog: IFileDialog = made(FileOpenDialog)?;
    // Un dossier n'a pas de type de fichier : les filtres sont ignorés
    // plutôt que posés, parce que Windows en met une liste déroulante
    // vide dans un dialogue de dossier.
    let mut without = ask.clone();
    without.filters.clear();
    dressed(&dialog, &without, FOS_FORCEFILESYSTEM | FOS_PICKFOLDERS)?;

    shown(&dialog)?;
    let item = unsafe { dialog.GetResult() }.map_err(refused)?;
    path_of(&item)
}

/// Le titre, les filtres, le dossier de départ, les options.
fn dressed(
    dialog: &IFileDialog,
    ask: &Ask,
    options: windows::Win32::UI::Shell::FILEOPENDIALOGOPTIONS,
) -> Result<(), DialogError> {
    unsafe { dialog.SetOptions(options) }.map_err(refused)?;

    if !ask.title.is_empty() {
        unsafe { dialog.SetTitle(&HSTRING::from(ask.title.as_str())) }.map_err(refused)?;
    }

    // Les chaînes doivent vivre jusqu'à `SetFileTypes`, donc elles sont
    // gardées ici : `COMDLG_FILTERSPEC` ne contient que des pointeurs.
    let patterns = patterns_for(&ask.filters);
    let kept: Vec<(HSTRING, HSTRING)> = patterns
        .iter()
        .map(|(name, pattern)| (HSTRING::from(name.as_str()), HSTRING::from(pattern.as_str())))
        .collect();
    let specs: Vec<COMDLG_FILTERSPEC> = kept
        .iter()
        .map(|(name, pattern)| COMDLG_FILTERSPEC {
            pszName: PCWSTR(name.as_ptr()),
            pszSpec: PCWSTR(pattern.as_ptr()),
        })
        .collect();
    if !specs.is_empty() {
        unsafe { dialog.SetFileTypes(&specs) }.map_err(refused)?;
    }

    if let Some(start_in) = &ask.start_in {
        let path = HSTRING::from(start_in.as_os_str());
        // Un dossier de départ qui n'existe pas n'est pas une raison de
        // refuser le dialogue : Windows l'ouvre là où il l'a laissé.
        if let Ok(folder) =
            unsafe { SHCreateItemFromParsingName::<_, _, IShellItem>(&path, None) }
        {
            let _ = unsafe { dialog.SetFolder(&folder) };
        }
    }
    Ok(())
}

/// Ouvrir le dialogue et attendre. `Cancelled` quand il est fermé.
fn shown(dialog: &IFileDialog) -> Result<(), DialogError> {
    let owner = unsafe { GetForegroundWindow() };
    let owner = if owner.is_invalid() { None } else { Some(owner) };
    match unsafe { dialog.Show(owner) } {
        Ok(()) => Ok(()),
        Err(error) if matches!(error.code().0, CANCELLED | ABORTED) => Err(DialogError::Cancelled),
        Err(error) => Err(refused(error)),
    }
}

/// Un objet COM du shell, par son identifiant de classe.
fn made<T: Interface>(class: windows::core::GUID) -> Result<T, DialogError> {
    unsafe { CoCreateInstance(&class, None, CLSCTX_INPROC_SERVER) }.map_err(refused)
}

/// Le chemin d'un élément choisi, sur le disque et non dans le shell.
///
/// `FOS_FORCEFILESYSTEM` garantit que c'en est un ; sans lui,
/// l'utilisateur pourrait choisir une bibliothèque ou un appareil
/// photo, qui n'ont pas de chemin et dont il n'y aurait rien à faire.
fn path_of(item: &IShellItem) -> Result<PathBuf, DialogError> {
    let wide = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.map_err(refused)?;
    let path = unsafe { wide.to_string() }
        .map_err(|_| DialogError::Platform("un chemin que Windows n'a pas écrit en UTF-16 valide".to_string()))?;
    unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(wide.0 as *const _)) };
    Ok(PathBuf::from(path))
}

/// Ce que le système a dit, dans ses mots.
fn refused(error: windows::core::Error) -> DialogError {
    DialogError::Platform(error.message())
}

/// COM, entré pour la durée du dialogue et laissé comme on l'a trouvé.
///
/// Le dialogue est un objet COM : sans appartement sur ce thread, la
/// création échoue. Le thread l'a souvent déjà — Chromium initialise
/// COM sur plusieurs des siens — et dans ce cas `CoInitializeEx` rend
/// `RPC_E_CHANGED_MODE` ou `S_FALSE`, qui ne sont pas des échecs : le
/// `CoUninitialize` est alors celui de trop, et il n'est pas appelé.
struct Apartment {
    ours: bool,
}

impl Apartment {
    fn entered() -> Result<Self, DialogError> {
        let entered = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if entered.is_ok() {
            return Ok(Self { ours: true });
        }
        // `S_FALSE` : déjà initialisé sur ce thread, dans le même mode.
        // `RPC_E_CHANGED_MODE` : déjà initialisé dans un autre mode, ce
        // qui n'empêche pas d'ouvrir un dialogue.
        Ok(Self { ours: false })
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        if self.ours {
            unsafe { CoUninitialize() };
        }
    }
}
