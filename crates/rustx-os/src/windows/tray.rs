//! L'icône de barre système de Windows : `Shell_NotifyIconW`.
//!
//! # Un fil, une fenêtre invisible, une boucle
//!
//! Le système parle à une icône de barre par messages, et des messages
//! vont à une fenêtre. L'icône a donc sa fenêtre : `HWND_MESSAGE`, une
//! fenêtre qui n'est jamais dessinée et qui n'existe que pour recevoir.
//! Elle vit sur un fil à elle, avec sa boucle, ce qui est ce qui permet
//! d'obtenir une icône depuis n'importe quel fil et n'importe quel
//! processus — y compris celui où la page est écrite.
//!
//! Tout ce qui touche la fenêtre se fait **sur ce fil** : Windows
//! l'exige pour `TrackPopupMenu`, et un menu ouvert depuis un autre fil
//! est un menu qui ne se referme jamais. Ce qui vient du dehors — un
//! menu à remplacer, une notification à montrer, l'ordre de s'arrêter —
//! arrive par un message posté à la fenêtre, et c'est le fil qui agit.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, HICON, HWND_MESSAGE, IDI_APPLICATION,
    IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, LoadIconW, LoadImageW, MF_CHECKED, MF_SEPARATOR,
    MF_STRING, MF_UNCHECKED, MSG, PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_RIGHTALIGN, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW,
};
use windows::core::PCWSTR;

use crate::error::OsError;
use crate::notify::What;
use crate::tray::{CLICK, Chosen, Item};

/// Windows en a une, et ce fichier la pose.
pub const AVAILABLE: bool = true;

/// Le message que le système envoie à notre fenêtre quand on clique sur
/// l'icône. `WM_APP` et au-delà appartiennent à l'application.
const ICON_MESSAGE: u32 = WM_APP + 1;
/// Et ceux que nous nous envoyons à nous-mêmes.
const SET_MENU: u32 = WM_APP + 2;
const SHOW_BALLOON: u32 = WM_APP + 3;

/// Combien d'icônes ce processus a posées et pas encore retirées.
static SHOWN: AtomicUsize = AtomicUsize::new(0);

pub fn shown() -> usize {
    SHOWN.load(Ordering::Acquire)
}

/// Ce que le fil de l'icône et le reste du programme partagent.
struct Shared {
    /// Les clics, en attente de lecture.
    chosen: Chosen,
    /// Ce que le menu doit contenir la prochaine fois qu'on l'ouvre.
    menu: Mutex<Vec<Item>>,
    /// Ce qu'une notification doit dire, posé avant le message.
    balloon: Mutex<Option<What>>,
}

/// L'icône : le fil, et de quoi lui parler.
pub struct Icon {
    shared: Arc<Shared>,
    window: Window,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Un `HWND` qu'on a le droit d'envoyer à un autre fil.
///
/// Poster un message à une fenêtre depuis n'importe quel fil est permis
/// — c'est même la façon prévue de lui parler. Ce qui ne l'est pas est
/// de *l'utiliser* ailleurs que sur son fil, et rien ici ne le fait.
#[derive(Clone, Copy)]
struct Window(HWND);

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Icon {
    pub fn new(icon: &Path, tooltip: &str) -> Result<Self, OsError> {
        let icon = wide(icon.as_os_str());
        let tooltip = tooltip.to_string();
        let shared = Arc::new(Shared {
            chosen: Chosen::new(),
            menu: Mutex::new(Vec::new()),
            balloon: Mutex::new(None),
        });

        // Le fil rend sa fenêtre dès qu'il l'a, ou l'erreur qui l'en a
        // empêché : `Icon::new` ne doit pas rendre une icône dont rien
        // ne dit si elle est là.
        let (say, heard) = std::sync::mpsc::channel::<Result<Window, OsError>>();
        let theirs = shared.clone();
        let thread = std::thread::Builder::new()
            .name("rustx-tray".to_string())
            .spawn(move || live(theirs, icon, tooltip, say))
            .map_err(|error| {
                OsError::Platform(format!("le fil de l'icône n'a pas démarré : {error}"))
            })?;

        let window = heard
            .recv()
            .map_err(|_| OsError::Platform("le fil de l'icône s'est arrêté aussitôt".to_string()))?
            ?;

        Ok(Self {
            shared,
            window,
            thread: Some(thread),
        })
    }

    /// Une icône sans icône et sans infobulle, pour les notifications
    /// d'une application qui n'en tient pas.
    pub fn hidden() -> Result<Self, OsError> {
        let shared = Arc::new(Shared {
            chosen: Chosen::new(),
            menu: Mutex::new(Vec::new()),
            balloon: Mutex::new(None),
        });
        let (say, heard) = std::sync::mpsc::channel::<Result<Window, OsError>>();
        let theirs = shared.clone();
        let thread = std::thread::Builder::new()
            .name("rustx-notify".to_string())
            .spawn(move || live(theirs, Vec::new(), String::new(), say))
            .map_err(|error| {
                OsError::Platform(format!("le fil de la notification n'a pas démarré : {error}"))
            })?;
        let window = heard
            .recv()
            .map_err(|_| OsError::Platform("le fil de la notification s'est arrêté".to_string()))??;
        Ok(Self {
            shared,
            window,
            thread: Some(thread),
        })
    }

    pub fn menu(&self, items: &[Item]) -> Result<(), OsError> {
        {
            let mut held = self
                .shared
                .menu
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *held = items.to_vec();
        }
        // Posté plutôt qu'appliqué ici : le menu se construit sur le
        // fil de la fenêtre, et seulement quand on l'ouvre.
        post(self.window, SET_MENU)
    }

    pub fn clicked(&self) -> Option<String> {
        self.shared.chosen.take()
    }

    pub fn notify(&self, what: &What) -> Result<(), OsError> {
        {
            let mut held = self
                .shared
                .balloon
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *held = Some(what.clone());
        }
        post(self.window, SHOW_BALLOON)
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        let _ = post(self.window, WM_CLOSE);
        if let Some(thread) = self.thread.take() {
            // Attendu : le fil retire l'icône de la barre en partant, et
            // un `Drop` qui rend la main avant que ce soit fait est un
            // `Drop` dont un test ne peut rien dire — et, pour un
            // utilisateur, une icône morte qui traîne une seconde.
            let _ = thread.join();
        }
    }
}

/// Poster un message à la fenêtre de l'icône.
fn post(window: Window, message: u32) -> Result<(), OsError> {
    unsafe { PostMessageW(Some(window.0), message, WPARAM(0), LPARAM(0)) }
        .map_err(|error| OsError::Platform(error.message()))
}

/// La vie du fil : la fenêtre, l'icône, la boucle, et le ménage.
fn live(
    shared: Arc<Shared>,
    icon: Vec<u16>,
    tooltip: String,
    say: std::sync::mpsc::Sender<Result<Window, OsError>>,
) {
    let made = made_window(&shared);
    let (window, handle) = match made {
        Ok(it) => it,
        Err(why) => {
            let _ = say.send(Err(why));
            return;
        }
    };

    let loaded = if icon.is_empty() {
        unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or_default()
    } else {
        match unsafe {
            LoadImageW(
                None,
                PCWSTR(icon.as_ptr()),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
        } {
            Ok(it) => HICON(it.0),
            Err(error) => {
                let _ = say.send(Err(OsError::Platform(format!(
                    "Windows n'a pas chargé l'icône : {}",
                    error.message()
                ))));
                unsafe { let _ = DestroyWindow(window); }
                return;
            }
        }
    };

    let data = described(window, loaded, &tooltip);
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        let _ = say.send(Err(OsError::Platform(
            "Windows a refusé de poser l'icône dans la barre système".to_string(),
        )));
        unsafe {
            let _ = DestroyWindow(window);
        }
        return;
    }
    SHOWN.fetch_add(1, Ordering::Release);
    let _ = say.send(Ok(Window(window)));

    // La boucle. Sans filtre de fenêtre : ce fil n'en a qu'une, et un
    // filtre qui nomme une fenêtre déjà détruite fait rendre **-1** à
    // `GetMessageW` — ce que `as_bool()` lit comme « un message », donc
    // une boucle qui tourne pour toujours sur rien. Mesuré : le test du
    // `Drop` restait bloqué plus d'une minute et un cœur tournait à
    // plein, ce qui faisait aussi échouer, à côté, le test qui mesure
    // qu'une notification ne bloque pas.
    //
    // D'où les trois cas, écrits séparément : un message, `WM_QUIT`, et
    // l'erreur.
    let mut message = MSG::default();
    loop {
        let got = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if got.0 <= 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    // Et le ménage, quoi qu'il soit arrivé dans la boucle : si elle
    // s'est arrêtée autrement que par un `WM_CLOSE` — une erreur, un
    // `WM_QUIT` venu d'ailleurs — l'icône est encore là. Un second
    // retrait d'une icône déjà retirée échoue sans rien compter.
    let _ = data;
    removed(window);
    drop(handle);
}

/// Retirer l'icône de la barre, et ne compter que ce que le système a
/// accepté.
fn removed(window: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: 1,
        ..Default::default()
    };
    if unsafe { Shell_NotifyIconW(NIM_DELETE, &data) }.as_bool() {
        SHOWN.fetch_sub(1, Ordering::Release);
    }
}

/// La fenêtre invisible, et ce que son `wndproc` lira.
fn made_window(shared: &Arc<Shared>) -> Result<(HWND, Held), OsError> {
    let class = wide(std::ffi::OsStr::new("rustx-tray-window"));
    let instance = unsafe { GetModuleHandleW(None) }
        .map_err(|error| OsError::Platform(error.message()))?;

    let description = WNDCLASSW {
        lpfnWndProc: Some(answered),
        hInstance: instance.into(),
        lpszClassName: PCWSTR(class.as_ptr()),
        ..Default::default()
    };
    // Enregistrer deux fois la même classe rend 0 et met
    // `ERROR_CLASS_ALREADY_EXISTS` : ce n'est pas une erreur, c'est une
    // deuxième icône dans le même processus.
    unsafe { RegisterClassW(&description) };

    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class.as_ptr()),
            PCWSTR(class.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|error| {
        OsError::Platform(format!(
            "la fenêtre invisible de l'icône n'a pas été créée : {}",
            error.message()
        ))
    })?;

    // Ce que le `wndproc` doit atteindre. Il tourne sur ce fil-ci et
    // nulle part ailleurs, donc un `thread_local` suffit — et évite de
    // ranger un pointeur dans la fenêtre.
    STATE.with(|state| *state.borrow_mut() = Some(shared.clone()));
    Ok((window, Held))
}

/// Ce qui vide le `thread_local` quand le fil s'en va.
struct Held;

impl Drop for Held {
    fn drop(&mut self) {
        STATE.with(|state| *state.borrow_mut() = None);
    }
}

thread_local! {
    static STATE: std::cell::RefCell<Option<Arc<Shared>>> =
        const { std::cell::RefCell::new(None) };
    /// Le menu ouvert, gardé pour être détruit après.
    static MENU: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
}

/// Ce que la fenêtre fait de chaque message.
unsafe extern "system" fn answered(
    window: HWND,
    message: u32,
    which: WPARAM,
    more: LPARAM,
) -> LRESULT {
    match message {
        ICON_MESSAGE => {
            // Le clic est dans la partie basse de `more`.
            let what = (more.0 as u32) & 0xffff;
            match what {
                WM_LBUTTONUP => with_state(|shared| shared.chosen.push(CLICK)),
                WM_RBUTTONUP => opened_menu(window),
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let number = (which.0 as u32) & 0xffff;
            with_state(|shared| {
                let items = shared
                    .menu
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                // Les identifiants de menu commencent à 1 : zéro est ce
                // que Windows rend quand on referme sans choisir.
                if let Some(item) = items.get(number.wrapping_sub(1) as usize) {
                    if !item.id.is_empty() {
                        shared.chosen.push(item.id.clone());
                    }
                }
            });
            LRESULT(0)
        }
        SET_MENU => {
            // Rien à faire : le menu est lu dans la table partagée au
            // moment où on l'ouvre. Le message existe pour réveiller la
            // boucle, et pour que `menu()` échoue si la fenêtre est
            // partie.
            LRESULT(0)
        }
        SHOW_BALLOON => {
            with_state(|shared| {
                let what = shared
                    .balloon
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take();
                if let Some(what) = what {
                    shown_balloon(window, &what);
                }
            });
            LRESULT(0)
        }
        WM_CLOSE => {
            // L'icône s'en va **avant** la fenêtre. Le système
            // identifie une icône par la fenêtre à qui elle parle :
            // détruire la fenêtre d'abord, c'est demander ensuite le
            // retrait d'une icône qu'il ne reconnaît plus, et l'icône
            // reste dans la barre jusqu'à la fin du processus. Mesuré :
            // c'est exactement ce que faisait la première version, et
            // le test du `Drop` l'a dit.
            removed(window);
            unsafe {
                let _ = DestroyWindow(window);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, which, more) },
    }
}

/// Ce que le `wndproc` a le droit de toucher.
fn with_state(what: impl FnOnce(&Arc<Shared>)) {
    STATE.with(|state| {
        if let Some(shared) = state.borrow().as_ref() {
            what(shared);
        }
    });
}

/// Ouvrir le menu là où est le curseur.
fn opened_menu(window: HWND) {
    let items: Vec<Item> = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|shared| {
                shared
                    .menu
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone()
            })
            .unwrap_or_default()
    });
    if items.is_empty() {
        return;
    }

    let menu = match unsafe { CreatePopupMenu() } {
        Ok(menu) => menu,
        Err(_) => return,
    };
    for (at, item) in items.iter().enumerate() {
        let number = at + 1;
        if item.label.is_empty() {
            unsafe {
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            }
            continue;
        }
        let label = wide(std::ffi::OsStr::new(&item.label));
        let mut flags = MF_STRING;
        if let Some(checked) = item.checked {
            flags |= if checked { MF_CHECKED } else { MF_UNCHECKED };
        }
        unsafe {
            let _ = AppendMenuW(menu, flags, number, PCWSTR(label.as_ptr()));
        }
    }

    let mut where_ = Default::default();
    unsafe {
        let _ = GetCursorPos(&mut where_);
        // Sans cela, le menu ne se referme pas quand on clique
        // ailleurs : Windows demande que la fenêtre qui le porte soit
        // au premier plan, et la nôtre est invisible.
        let _ = SetForegroundWindow(window);
        let _ = TrackPopupMenu(
            menu,
            TPM_BOTTOMALIGN | TPM_RIGHTALIGN,
            where_.x,
            where_.y,
            Some(0),
            window,
            None,
        );
        let _ = DestroyMenu(menu);
    }
    MENU.with(|kept| kept.set(0));
}

/// Montrer la bulle, par l'icône déjà posée.
fn shown_balloon(window: HWND, what: &What) {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: 1,
        uFlags: NIF_INFO,
        dwInfoFlags: NIIF_INFO,
        ..Default::default()
    };
    put(&mut data.szInfoTitle, &what.title);
    put(&mut data.szInfo, &what.body);
    let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
}

/// L'icône, décrite comme le système la veut.
fn described(window: HWND, icon: HICON, tooltip: &str) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: window,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: ICON_MESSAGE,
        hIcon: icon,
        ..Default::default()
    };
    put(&mut data.szTip, tooltip);
    data
}

/// Du texte dans un tableau de taille fixe, coupé et terminé par zéro.
fn put(field: &mut [u16], text: &str) {
    let wide: Vec<u16> = text.encode_utf16().take(field.len() - 1).collect();
    field[..wide.len()].copy_from_slice(&wide);
    field[wide.len()] = 0;
}

/// Une chaîne Windows, terminée par zéro.
fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}
