//! Une seule instance, sous Windows : un mutex nommé et une fenêtre
//! invisible.
//!
//! Le mutex nommé répond à « suis-je le premier ? » : le système ne
//! laisse qu'un processus créer un nom donné, et le rend quand ce
//! processus s'arrête — même s'il s'arrête mal, ce qui est la raison de
//! le préférer à un fichier verrou.
//!
//! La fenêtre invisible répond à « que me passe la deuxième ? ». Elle
//! porte le nom de l'application comme titre, ce qui permet à l'autre
//! de la trouver par `FindWindowW` sans rien partager d'avance, et elle
//! reçoit `WM_COPYDATA` — le seul message de Windows qui porte des
//! octets d'un processus à un autre sans tuyau à ouvrir.

use std::os::windows::ffi::OsStrExt;
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GetMessageW,
    HWND_MESSAGE, MSG, PostMessageW, PostQuitMessage, RegisterClassW, SendMessageW,
    TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COPYDATA, WM_DESTROY, WNDCLASSW,
};
use windows::core::PCWSTR;

use crate::error::OsError;
use crate::instance::BETWEEN;

/// Windows sait tenir un nom, et ce fichier le tient.
pub const AVAILABLE: bool = true;

/// Ce que `WM_COPYDATA` porte, pour que personne d'autre ne soit pris
/// pour nous : un programme quelconque peut envoyer ce message à une
/// fenêtre, et celle-ci lirait n'importe quoi.
const OURS: usize = 0x7275_7374;

/// Le nom de la classe de la fenêtre invisible.
const CLASS: &str = "rustx-instance-window";

thread_local! {
    /// La file de ce fil, pour le `wndproc`.
    static QUEUE: std::cell::RefCell<Option<Arc<Mutex<Vec<String>>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Ce qu'une deuxième instance nous a passé, par processus : la page
/// lit d'un fil, la fenêtre écrit d'un autre.
fn handed() -> &'static Mutex<Vec<String>> {
    static HANDED: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();
    HANDED.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn handed_over() -> Vec<String> {
    let mut held = handed()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::mem::take(&mut *held)
}

/// Le nom tenu : le mutex, la fenêtre, et le fil qui l'écoute.
pub struct Only {
    _mutex: Handle,
    window: Window,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Un `HANDLE` que le jeton garde pour la durée de l'instance.
struct Handle(HANDLE);

// SAFETY: a Win32 `HANDLE` belongs to the process and not to a thread.
// Any thread may close one, and `CloseHandle` is documented as such. The
// handle in here is the named mutex, which nothing else in this crate
// holds a copy of, so it is closed exactly once — by the `Drop` below.
unsafe impl Send for Handle {}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle `CreateMutexW` gave us, closed once:
        // `Handle` is not `Clone` and the only other path out of
        // `Only::one` drops this same value rather than copying it.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Un `HWND` qu'on a le droit de poster depuis un autre fil.
#[derive(Clone, Copy)]
struct Window(HWND);

// SAFETY: an `HWND` is a process-wide identifier, and the only thing
// done with this one off its own thread is `PostMessageW`, which
// Windows documents as callable from any thread (it queues the message
// rather than running the window procedure on the caller). Nothing here
// calls `SendMessageW` or any of the window-state functions that must
// run on the thread that created the window.
unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Only {
    pub fn one(name: &str) -> Result<Self, OsError> {
        let wide = wide(&format!("Local\\rustx-instance-{name}"));
        let mutex = unsafe { CreateMutexW(None, true, PCWSTR(wide.as_ptr())) }
            .map_err(|error| OsError::Platform(error.message()))?;
        // Créé *et* déjà pris par quelqu'un : l'autre instance tourne.
        // `CreateMutexW` rend quand même un handle valide, qu'il faut
        // refermer — d'où le `Handle` construit avant le refus.
        let held = Handle(mutex);
        if unsafe { windows::Win32::Foundation::GetLastError() } == ERROR_ALREADY_EXISTS {
            drop(held);
            return Err(OsError::Busy(format!(
                "{name} — une autre instance tourne déjà. \
                 `instance::hand_over` lui passe les arguments de celle-ci."
            )));
        }

        let title = name.to_string();
        let (say, heard) = std::sync::mpsc::channel::<Result<Window, OsError>>();
        let thread = std::thread::Builder::new()
            .name("rustx-instance".to_string())
            .spawn(move || live(title, say))
            .map_err(|error| {
                OsError::Platform(format!("the instance thread did not start: {error}"))
            })?;
        let window = heard
            .recv()
            .map_err(|_| OsError::Platform("the instance thread stopped".to_string()))??;

        Ok(Self {
            _mutex: held,
            window,
            thread: Some(thread),
        })
    }
}

impl Drop for Only {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(Some(self.window.0), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Passer des arguments à l'instance qui tourne.
pub fn hand_over(name: &str, arguments: &[String]) -> Result<(), OsError> {
    let class = wide(CLASS);
    let title = wide(name);
    let window =
        unsafe { FindWindowW(PCWSTR(class.as_ptr()), PCWSTR(title.as_ptr())) }.map_err(|_| {
            OsError::NoSuchDevice(format!(
                "{name} — no instance is running, there is nobody to hand over to"
            ))
        })?;

    let joined: String = arguments.join(&BETWEEN.to_string());
    let bytes: Vec<u16> = joined.encode_utf16().collect();
    let data = COPYDATASTRUCT {
        dwData: OURS,
        cbData: (bytes.len() * 2) as u32,
        lpData: if bytes.is_empty() {
            std::ptr::null_mut()
        } else {
            bytes.as_ptr() as *mut _
        },
    };
    // `SendMessageW` et non `PostMessageW` : `WM_COPYDATA` porte un
    // pointeur vers notre mémoire, et il faut que l'autre processus
    // l'ait lu avant que cette fonction rende la main.
    unsafe {
        SendMessageW(
            window,
            WM_COPYDATA,
            Some(WPARAM(0)),
            Some(LPARAM(&data as *const _ as isize)),
        );
    }
    Ok(())
}

/// La vie du fil : la fenêtre, la boucle, et le ménage.
fn live(title: String, say: std::sync::mpsc::Sender<Result<Window, OsError>>) {
    let class = wide(CLASS);
    let name = wide(&title);
    let instance = match unsafe { GetModuleHandleW(None) } {
        Ok(instance) => instance,
        Err(error) => {
            let _ = say.send(Err(OsError::Platform(error.message())));
            return;
        }
    };

    let description = WNDCLASSW {
        lpfnWndProc: Some(answered),
        hInstance: instance.into(),
        lpszClassName: PCWSTR(class.as_ptr()),
        ..Default::default()
    };
    unsafe { RegisterClassW(&description) };

    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class.as_ptr()),
            // Le titre est le nom de l'application : c'est par lui que
            // la deuxième instance nous trouve.
            PCWSTR(name.as_ptr()),
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
    };
    let window = match window {
        Ok(window) => window,
        Err(error) => {
            let _ = say.send(Err(OsError::Platform(format!(
                "the instance's invisible window was not created: {}",
                error.message()
            ))));
            return;
        }
    };

    QUEUE.with(|queue| *queue.borrow_mut() = Some(Arc::new(Mutex::new(Vec::new()))));
    let _ = say.send(Ok(Window(window)));

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
}

/// Ce que la fenêtre fait de chaque message.
unsafe extern "system" fn answered(
    window: HWND,
    message: u32,
    which: WPARAM,
    more: LPARAM,
) -> LRESULT {
    match message {
        WM_COPYDATA => {
            let data = more.0 as *const COPYDATASTRUCT;
            // SAFETY: for `WM_COPYDATA`, Windows marshals the structure
            // into this process and `lParam` points at it, properly
            // aligned, for the duration of this call. `as_ref` answers
            // `None` for a null pointer, which is the one other thing
            // `lParam` could be.
            let read = unsafe { data.as_ref() }.filter(|data| data.dwData == OURS);
            if let Some(data) = read {
                let size = data.cbData as usize;
                let text = if size < 2 || data.lpData.is_null() {
                    String::new()
                } else {
                    // Read as bytes and decoded by hand, not as
                    // `*const u16`. Any program on the machine can post
                    // this message — the window is found by class and
                    // title — so `lpData` is a pointer somebody else
                    // chose, and reading it through a `*const u16` that
                    // happens to be odd is undefined behaviour rather
                    // than a wrong answer. `u8` has no alignment to get
                    // wrong.
                    //
                    // SAFETY: `lpData` is non-null and Windows
                    // guarantees `cbData` readable bytes at it for the
                    // duration of this call; the slice is consumed here
                    // and does not escape.
                    let bytes =
                        unsafe { std::slice::from_raw_parts(data.lpData as *const u8, size) };
                    let units: Vec<u16> = bytes
                        .chunks_exact(2)
                        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                        .collect();
                    String::from_utf16_lossy(&units)
                };
                let arguments: Vec<String> = text
                    .split(BETWEEN)
                    .filter(|piece| !piece.is_empty())
                    .map(str::to_string)
                    .collect();
                let mut held = handed()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                held.extend(arguments);
            }
            // Non nul : lu.
            LRESULT(1)
        }
        WM_CLOSE => {
            // SAFETY: the window this procedure was called about, on
            // the thread that created it — which is where
            // `DestroyWindow` has to be called.
            unsafe {
                let _ = DestroyWindow(window);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: posts to this thread's own message queue, which
            // is what ends the loop in `live`.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: the arguments this procedure was given, handed
        // straight back to the default handler.
        _ => unsafe { DefWindowProcW(window, message, which, more) },
    }
}

/// Une chaîne Windows, terminée par zéro.
fn wide(text: &str) -> Vec<u16> {
    std::ffi::OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
