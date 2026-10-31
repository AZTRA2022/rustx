//! Les raccourcis globaux de Windows : `RegisterHotKey`.
//!
//! # Un fil, et pas même une fenêtre
//!
//! `RegisterHotKey(None, …)` envoie `WM_HOTKEY` à la *file du fil* qui
//! a enregistré, et non à une fenêtre. Un fil avec une boucle suffit
//! donc — pas de fenêtre invisible, contrairement à l'icône de barre
//! système. Et comme pour elle, c'est ce fil à lui qui fait qu'un
//! raccourci s'obtient d'où la page est écrite.
//!
//! L'enregistrement doit se faire **sur le fil qui écoute** : Windows
//! lie la combinaison au fil, et un enregistrement fait ailleurs
//! enverrait les appuis à une file que personne ne lit.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PostThreadMessageW, WM_HOTKEY, WM_QUIT,
};

use crate::error::OsError;

/// Windows en donne, et ce fichier les demande.
pub const AVAILABLE: bool = true;

/// `MOD_NOREPEAT` : un appui tenu est un appui, et non quarante.
const NO_REPEAT: u32 = 0x4000;

/// La combinaison tenue par un fil.
pub struct Shortcut {
    pressed: Arc<AtomicU64>,
    thread_id: u32,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Shortcut {
    pub fn listen(modifiers: u32, key: u32, keys: &str) -> Result<Self, OsError> {
        let pressed = Arc::new(AtomicU64::new(0));
        let theirs = pressed.clone();
        let written = keys.to_string();

        let (say, heard) = std::sync::mpsc::channel::<Result<u32, OsError>>();
        let thread = std::thread::Builder::new()
            .name(format!("rustx-shortcut-{keys}"))
            .spawn(move || live(modifiers, key, written, theirs, say))
            .map_err(|error| {
                OsError::Platform(format!("le fil du raccourci n'a pas démarré : {error}"))
            })?;

        let thread_id = heard
            .recv()
            .map_err(|_| OsError::Platform("le fil du raccourci s'est arrêté".to_string()))??;

        Ok(Self {
            pressed,
            thread_id,
            thread: Some(thread),
        })
    }

    pub fn pressed(&self) -> u64 {
        self.pressed.swap(0, Ordering::AcqRel)
    }
}

impl Drop for Shortcut {
    fn drop(&mut self) {
        // `WM_QUIT` posté au fil : sa boucle s'arrête, et il rend la
        // combinaison en partant. Attendu, parce qu'un `Drop` qui rend
        // la main avant que les touches soient libres est un `Drop`
        // après lequel un second `listen` échoue.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// La vie du fil : l'enregistrement, la boucle, et les touches rendues.
fn live(
    modifiers: u32,
    key: u32,
    keys: String,
    pressed: Arc<AtomicU64>,
    say: std::sync::mpsc::Sender<Result<u32, OsError>>,
) {
    let id = 1;
    let asked = unsafe {
        RegisterHotKey(
            None,
            id,
            HOT_KEY_MODIFIERS(modifiers | NO_REPEAT),
            key,
        )
    };
    if let Err(error) = asked {
        // La raison la plus fréquente, et de loin : quelqu'un d'autre
        // l'a. Le dire ainsi évite à un utilisateur de chercher dans
        // son propre logiciel ce qui n'y est pas.
        let _ = say.send(Err(OsError::Busy(format!(
            "{keys} ({})",
            error.message()
        ))));
        return;
    }

    let thread_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    if say.send(Ok(thread_id)).is_err() {
        // Personne n'écoute : le `listen` a été abandonné entre-temps.
        unsafe {
            let _ = UnregisterHotKey(None, id);
        }
        return;
    }

    let mut message = MSG::default();
    // `GetMessageW` rend 0 sur `WM_QUIT`, ce que le `Drop` poste.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        if message.message == WM_HOTKEY {
            pressed.fetch_add(1, Ordering::AcqRel);
        }
    }

    unsafe {
        let _ = UnregisterHotKey(None, id);
    }
}
