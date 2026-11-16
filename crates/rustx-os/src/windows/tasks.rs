//! La liste de raccourcis, sous Windows : `ICustomDestinationList`.
//!
//! Trois objets COM, dans cet ordre, et il n'y en a pas d'autre :
//! `BeginList` ouvre la transaction, une collection de `IShellLinkW`
//! porte les lignes, `AddUserTasks` les donne, `CommitList` les écrit.
//! Sans `CommitList`, rien n'apparaît et rien ne dit pourquoi — c'est
//! le piège de cette API.
//!
//! # Le libellé d'une ligne n'est pas sa description
//!
//! `IShellLinkW::SetDescription` pose l'info-bulle, pas le texte qu'on
//! lit. Le texte vient de `PKEY_Title`, écrit dans le magasin de
//! propriétés du raccourci : un raccourci sans lui s'affiche sous le
//! nom du fichier visé, donc *toutes* les lignes s'appelleraient comme
//! l'exécutable. C'est pour cela que ce fichier construit un
//! `PROPVARIANT` à la main, ce qui est la seule chose inhabituelle ici.
//!
//! # Où la liste apparaît, et quand elle n'apparaît pas
//!
//! Au clic droit sur l'icône de la barre des tâches ou du menu Démarrer.
//! Windows la rattache à l'exécutable (ou à l'`AppUserModelID` que le
//! processus déclare, s'il en déclare un). Un exécutable lancé depuis
//! un dossier de compilation reçoit la liste comme un autre, mais
//! l'utilisateur ne la verra que s'il garde cette icône — ce qui est la
//! même limite que les notifications décrivent dans `notify.rs`, et
//! elle n'empêche rien d'ici.

use std::os::windows::ffi::OsStrExt;

use windows::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantClear};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemAlloc,
};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::Common::{IObjectArray, IObjectCollection};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    DestinationList, EnumerableObjectCollection, ICustomDestinationList, IShellLinkW, ShellLink,
};
use windows::Win32::Storage::EnhancedStorage::PKEY_Title;
use windows::core::{Interface, PCWSTR};

use crate::error::OsError;
use crate::tasks::{Task, command_line};

pub const AVAILABLE: bool = true;

fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn refused(what: &str, error: windows::core::Error) -> OsError {
    OsError::Platform(format!("{what} was refused by the Shell: {error}"))
}

fn started() {
    // SAFETY: COM pour ce fil ; déjà initialisé, l'appel le dit et c'est
    // sans conséquence.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
}

/// Un `PROPVARIANT` de type chaîne, et ce qu'il faut savoir pour ne pas
/// se tromper : la chaîne doit être allouée par `CoTaskMemAlloc`, parce
/// que c'est `PropVariantClear` qui la libérera, et lui n'appelle que
/// `CoTaskMemFree`. Une chaîne Rust passée là serait libérée par le
/// mauvais allocateur.
struct Text(PROPVARIANT);

impl Text {
    fn new(text: &str) -> Result<Self, OsError> {
        let source = wide(text);
        let bytes = std::mem::size_of_val(source.as_slice());
        // SAFETY: une taille non nulle (la chaîne a au moins son zéro).
        let room = unsafe { CoTaskMemAlloc(bytes) }.cast::<u16>();
        if room.is_null() {
            return Err(OsError::Platform(
                "there was no memory for a jump list label".to_string(),
            ));
        }
        // SAFETY: `room` est une allocation de `bytes` octets, et la
        // source en fait exactement autant.
        unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), room, source.len()) };
        let mut variant = PROPVARIANT::default();
        // SAFETY: le champ est une union de types tous valides en
        // écriture ; on pose le discriminant `vt` et le membre qui lui
        // correspond, ce qui est le contrat de cette structure.
        unsafe {
            let inner = &mut variant.Anonymous.Anonymous;
            inner.vt = VT_LPWSTR;
            inner.Anonymous.pwszVal = windows::core::PWSTR(room);
        }
        Ok(Self(variant))
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        // SAFETY: un `PROPVARIANT` que ce fichier a rempli, libéré une
        // fois. `IPropertyStore::SetValue` copie la valeur, donc la
        // libérer après est correct.
        let _ = unsafe { PropVariantClear(&mut self.0) };
    }
}

/// Un raccourci qui relance ce programme avec ces arguments, portant ce
/// libellé.
fn link(task: &Task) -> Result<IShellLinkW, OsError> {
    // SAFETY: la classe documentée du raccourci du Shell.
    let link: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) }
        .map_err(|error| refused("a jump list entry", error))?;
    let program = wide(&task.program.to_string_lossy());
    let arguments = wide(&command_line(&task.arguments));
    // SAFETY: l'interface est vivante, et les chaînes vivent jusqu'au
    // retour de cette fonction ; le Shell les copie.
    unsafe {
        link.SetPath(PCWSTR(program.as_ptr()))
            .map_err(|error| refused("the entry's program", error))?;
        link.SetArguments(PCWSTR(arguments.as_ptr()))
            .map_err(|error| refused("the entry's arguments", error))?;
        // L'icône est celle du programme lui-même : un raccourci sans
        // icône s'affiche avec une page blanche.
        link.SetIconLocation(PCWSTR(program.as_ptr()), 0)
            .map_err(|error| refused("the entry's icon", error))?;
        if !task.tooltip.trim().is_empty() {
            let tooltip = wide(&task.tooltip);
            link.SetDescription(PCWSTR(tooltip.as_ptr()))
                .map_err(|error| refused("the entry's tooltip", error))?;
        }
    }
    // Le texte lu, qui n'est pas la description : voir le commentaire du
    // module.
    let title = Text::new(&task.label)?;
    // SAFETY: le raccourci implémente `IPropertyStore` ; `SetValue`
    // copie la valeur, et `Commit` l'écrit dans l'objet.
    unsafe {
        let store: IPropertyStore = link
            .cast()
            .map_err(|error| refused("the entry's property store", error))?;
        store
            .SetValue(&PKEY_Title, &title.0)
            .map_err(|error| refused("the entry's label", error))?;
        store
            .Commit()
            .map_err(|error| refused("the entry's label", error))?;
    }
    Ok(link)
}

pub fn offer(tasks: &[Task]) -> Result<(), OsError> {
    started();
    // SAFETY: les deux classes documentées de cette API.
    let (list, collection) = unsafe {
        let list: ICustomDestinationList =
            CoCreateInstance(&DestinationList, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| refused("the jump list", error))?;
        let collection: IObjectCollection =
            CoCreateInstance(&EnumerableObjectCollection, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| refused("the jump list", error))?;
        (list, collection)
    };
    // `BeginList` rend le nombre de lignes que l'utilisateur a enlevées
    // de la liste à la main. On ne s'en sert pas — on ne propose que des
    // tâches, pas des documents récents, et une tâche enlevée ne revient
    // pas — mais l'argument est obligatoire.
    let mut removed = 0u32;
    // SAFETY: les interfaces sont vivantes ; `BeginList` rend l'ensemble
    // des éléments retirés, dont on n'a que faire, d'où le type.
    unsafe {
        let _: IObjectArray = list
            .BeginList(&mut removed)
            .map_err(|error| refused("opening the jump list", error))?;
        for task in tasks {
            let entry = link(task)?;
            collection
                .AddObject(&entry)
                .map_err(|error| refused("adding a jump list entry", error))?;
        }
        let array: IObjectArray = collection
            .cast()
            .map_err(|error| refused("the jump list entries", error))?;
        list.AddUserTasks(&array)
            .map_err(|error| refused("the jump list entries", error))?;
        // Sans ceci, tout ce qui précède est perdu en silence.
        list.CommitList()
            .map_err(|error| refused("committing the jump list", error))?;
    }
    Ok(())
}

pub fn clear() -> Result<(), OsError> {
    started();
    // SAFETY: la classe documentée de cette API.
    let list: ICustomDestinationList =
        unsafe { CoCreateInstance(&DestinationList, None, CLSCTX_INPROC_SERVER) }
            .map_err(|error| refused("the jump list", error))?;
    // Une liste ouverte puis validée sans tâche est une liste vide : il
    // n'y a pas d'appel « enlever » pour sa propre application
    // (`DeleteList` prend un `AppUserModelID`, qu'un programme non
    // enregistré n'a pas).
    let mut removed = 0u32;
    // SAFETY: l'interface est vivante.
    unsafe {
        let _: IObjectArray = list
            .BeginList(&mut removed)
            .map_err(|error| refused("opening the jump list", error))?;
        list.CommitList()
            .map_err(|error| refused("clearing the jump list", error))?;
    }
    Ok(())
}
