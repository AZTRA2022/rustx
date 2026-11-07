//! What a window is, besides a page: its icon and what it says it is.
//!
//! An application that ships has a logo in its title bar, a logo in the
//! taskbar, and a name and a version somebody can read. None of that is
//! the page's business and all of it is the window's, so it lives here.
//!
//! The icon is set on the window Chromium made, through the platform's
//! own call. The metadata is remembered and handed to the page, because
//! a page that shows its own version should not have to be edited when
//! the version changes. Writing the version into the *binary* — what
//! Windows shows in a file's properties — needs a resource linked at
//! build time, which is the command line's work, not a library's.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// What an application says it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct About {
    /// The window's title, which is also the application's name.
    pub name: String,
    /// `"0.1.0"`, as the application's own crate declares it.
    pub version: String,
    /// One line, for a window that shows one.
    pub description: String,
    /// Who publishes it.
    pub publisher: String,
}

fn state() -> &'static Mutex<(Option<PathBuf>, About)> {
    static STATE: OnceLock<Mutex<(Option<PathBuf>, About)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((None, About::default())))
}

/// Said by the facade before the window opens.
pub fn declare(icon: Option<PathBuf>, about: About) {
    let mut held = state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *held = (icon, about);
}

/// What the application said it is.
pub fn about() -> About {
    state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .1
        .clone()
}

/// The icon the application declared, if it declared one.
pub fn icon() -> Option<PathBuf> {
    state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .0
        .clone()
}

// ─────────────────────────── Une deuxième fenêtre ───────────────────────────
//
// Une fenêtre ne peut être créée que par le processus navigateur, et
// `open` est appelé là où la page est écrite. Le pont porte donc la
// demande, et ce fichier tient les deux bouts : `open` dans le
// processus de rendu, `asked` dans celui de la fenêtre.
//
// **Mesuré avant d'être écrit** (`RUSTX_TRACE=1`, deux fenêtres sur le
// même document) :
//
// 1. le fragment de l'URL arrive intact dans `frame.url()` du processus
//    de rendu — `…/index.html#/device/7`. C'est par là que la route
//    voyage, et une URL qui dit la vérité se lit dans les traces ;
// 2. les deux fenêtres ont **chacune leur processus de rendu** (9268 et
//    1116 sur la mesure). Le plan annonçait le contraire — « elles
//    partagent le magasin » — et c'est faux : deux fenêtres sont deux
//    états. Ce qu'elles partagent, elles le partageront par le pont, et
//    c'est un autre travail.

/// Ce que le pont porte quand une page demande une fenêtre.
pub const OPEN: &str = "open-window";
/// Et quand elle demande qu'on en ferme une.
pub const CLOSE: &str = "close-window";
/// Et quand elle commande la sienne : le cadre, la taille, le rang, le
/// déplacement.
pub const CONTROL: &str = "control-window";

// ───────────────────────── Commander sa fenêtre ─────────────────────────
//
// Beaucoup de logiciels n'ont pas la fenêtre que le système dessine.
// Enlever le cadre ne suffit pas : il faut rendre à la page ce que le
// cadre faisait, et c'est ce que ces fonctions sont.
//
// Elles partent du processus de rendu, où la page est écrite, vers
// celui de la fenêtre, qui est le seul à tenir la poignée du système.
// La fenêtre visée est celle d'où le message arrive : une page ne
// commande que la sienne, et n'a donc rien à nommer.

/// Donner un ordre à la fenêtre où cette page est dessinée.
pub fn command(order: rustx_os::window::Order) {
    crate::bridge::ask(
        CONTROL,
        &[crate::value::Value::String(order.word().to_string())],
    );
}

/// Enlever le cadre du système : la page dessine tout.
///
/// Ce qu'il faut savoir avant de le faire : plus de barre de titre,
/// donc plus rien pour déplacer la fenêtre. C'est [`drag`] qui le
/// rend, appelé depuis la barre que la page a dessinée.
pub fn frameless() {
    command(rustx_os::window::Order::Dress(
        rustx_os::window::Dressing::None,
    ));
}

/// Et le remettre.
pub fn framed() {
    command(rustx_os::window::Order::Dress(
        rustx_os::window::Dressing::System,
    ));
}

/// La réduire dans la barre des tâches.
pub fn minimize() {
    command(rustx_os::window::Order::Spread(
        rustx_os::window::Spread::Minimized,
    ));
}

/// L'agrandir à toute la place que le bureau laisse.
pub fn maximize() {
    command(rustx_os::window::Order::Spread(
        rustx_os::window::Spread::Maximized,
    ));
}

/// Lui rendre sa taille.
pub fn restore() {
    command(rustx_os::window::Order::Spread(
        rustx_os::window::Spread::Normal,
    ));
}

/// Tout l'écran, cadre compris.
pub fn fullscreen() {
    command(rustx_os::window::Order::Spread(
        rustx_os::window::Spread::Fullscreen,
    ));
}

/// La garder devant les autres, ou lui rendre son rang.
pub fn on_top(above: bool) {
    command(rustx_os::window::Order::OnTop(above));
}

/// Commencer un déplacement à la souris.
///
/// Appelé depuis `onMouseDown` de la barre de titre que la page a
/// dessinée : le système prend la suite et déplace la fenêtre jusqu'au
/// relâchement, exactement comme sur sa propre barre.
pub fn drag() {
    command(rustx_os::window::Order::Drag);
}

/// Fermer la fenêtre où cette page est dessinée.
pub fn shut() {
    command(rustx_os::window::Order::Close);
}

/// Ce qu'une fenêtre est, avant d'exister.
///
/// Un nom, parce que c'est par son nom qu'on la referme : la page
/// nomme la fenêtre qu'elle ouvre, et n'a donc pas besoin qu'on lui
/// réponde un identifiant à travers deux processus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct What {
    pub name: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// La route sur laquelle elle s'ouvre.
    pub at: String,
    /// Le cadre du système autour d'elle, ou pas.
    pub dressing: rustx_os::window::Dressing,
}

impl What {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            title: name.clone(),
            name,
            width: 900,
            height: 640,
            at: "/".to_string(),
            dressing: rustx_os::window::Dressing::System,
        }
    }

    pub fn titled(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// La route de l'application sur laquelle cette fenêtre s'ouvre.
    pub fn at(mut self, route: impl Into<String>) -> Self {
        self.at = route.into();
        self
    }

    /// Sans le cadre du système : la page dessine tout, et c'est à elle
    /// de donner un endroit où attraper la fenêtre — `window::drag()`.
    pub fn frameless(mut self) -> Self {
        self.dressing = rustx_os::window::Dressing::None;
        self
    }
}

/// Une fenêtre ouverte, désignée par son nom.
///
/// Son `Drop` ne la ferme pas, et c'est voulu : une page ouvre une
/// fenêtre depuis un gestionnaire d'événement, et le jeton meurt à la
/// fin du gestionnaire. Une fenêtre qui se refermerait avec lui ne
/// serait jamais vue. Elle se ferme quand on le dit — `close()` — ou
/// quand l'utilisateur la ferme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    name: String,
}

impl Window {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Fermer cette fenêtre. Une deuxième notification, sous le même nom.
    pub fn close(&self) {
        crate::bridge::ask(CLOSE, &[crate::value::Value::String(self.name.clone())]);
    }
}

/// Ouvrir une fenêtre, depuis la page.
///
/// La demande part vers le processus navigateur et n'attend rien :
/// `Ok` dit que la demande est partie, et la fenêtre apparaît au
/// prochain tour de sa boucle.
pub fn open(what: What) -> Result<Window, crate::error::EngineError> {
    use crate::value::Value;

    let name = what.name.clone();
    let asked = crate::bridge::ask(
        OPEN,
        &[
            Value::String(name.clone()),
            Value::String(what.title),
            Value::Number(f64::from(what.width)),
            Value::Number(f64::from(what.height)),
            Value::String(route(&what.at)),
            Value::String(
                rustx_os::window::Order::Dress(what.dressing)
                    .word()
                    .to_string(),
            ),
        ],
    );
    if !asked {
        return Err(crate::error::EngineError::NoPage);
    }
    Ok(Window { name })
}

/// Une route comme le routeur les écrit, quoi qu'on ait tapé.
fn route(asked: &str) -> String {
    let trimmed = asked.trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }
    if trimmed.starts_with('/') {
        return trimmed.to_string();
    }
    format!("/{trimmed}")
}

/// La route qu'une URL porte dans son fragment, s'il y en a une.
///
/// C'est ainsi qu'une deuxième fenêtre apprend sur quelle route elle
/// s'ouvre : elle reçoit le même document que la première, et le
/// fragment est ce que le document ne dit pas.
pub fn page_in(url: &str) -> Option<String> {
    let (_, fragment) = url.split_once('#')?;
    let fragment = fragment.trim();
    if fragment.is_empty() {
        return None;
    }
    Some(route(fragment))
}

thread_local! {
    /// L'URL de la page de ce processus de rendu.
    static LOADED_AT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Dit par le processus de rendu quand une page reçoit son contexte.
pub(crate) fn loaded_at(url: &str) {
    LOADED_AT.with(|held| *held.borrow_mut() = url.to_string());
}

/// La route que cette fenêtre-ci a reçue, s'il y en a une.
///
/// Lue par la façade avant de démarrer le routeur : la première fenêtre
/// n'a pas de fragment et part de sa route de départ, une deuxième en a
/// un et part de là.
pub fn page_asked_for() -> Option<String> {
    LOADED_AT.with(|held| page_in(&held.borrow()))
}

/// Put the declared icon on a window that has just been created.
///
/// Called with the handle Chromium gives for its own window. A failure
/// is said once and is not fatal: an application with no icon is an
/// application with the system's icon, which is a cosmetic loss and not
/// a reason to refuse to start.
pub fn dress(handle: *mut std::ffi::c_void) {
    let Some(path) = icon() else {
        return;
    };
    if !path.exists() {
        eprintln!(
            "rustx: the window's icon was declared as {} and there is no such file",
            path.display()
        );
        return;
    }
    match load(&path) {
        Ok(()) => {}
        Err(complaint) => eprintln!("rustx: {complaint}"),
    }
    set(handle, &path);
}

/// Is this a file the platform can load as an icon?
///
/// Read rather than guessed: an `.ico` begins with a reserved zero, the
/// type `1`, and how many images it holds. A `.png` renamed to `.ico` is
/// the mistake this catches, and it catches it with a sentence instead
/// of an icon that silently does not appear.
pub fn load(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|error| {
        format!(
            "the window's icon at {} could not be read: {error}",
            path.display()
        )
    })?;
    if bytes.len() < 6 {
        return Err(format!("{} is too short to be an icon", path.display()));
    }
    let reserved = u16::from_le_bytes([bytes[0], bytes[1]]);
    let kind = u16::from_le_bytes([bytes[2], bytes[3]]);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    if reserved != 0 || kind != 1 || count == 0 {
        return Err(format!(
            "{} is not an icon file. A window's icon is an `.ico`; a `.png` renamed \
             to `.ico` is not one.",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    const IMAGE_ICON: u32 = 1;
    const LR_LOADFROMFILE: u32 = 0x0010;
    const LR_DEFAULTSIZE: u32 = 0x0040;
    const WM_SETICON: u32 = 0x0080;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;

    unsafe extern "system" {
        fn LoadImageW(
            instance: *mut c_void,
            name: *const u16,
            kind: u32,
            width: i32,
            height: i32,
            load: u32,
        ) -> *mut c_void;
        fn SendMessageW(window: *mut c_void, message: u32, high: usize, low: isize) -> isize;
    }

    /// Both sizes: the title bar reads the small one and the taskbar the
    /// big one, and a window given only one of them has a blurred logo
    /// in the other place.
    pub(super) fn set(window: *mut c_void, path: &Path) {
        if window.is_null() {
            return;
        }
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);

        for (which, size) in [(ICON_SMALL, 16), (ICON_BIG, 32)] {
            let icon = unsafe {
                LoadImageW(
                    std::ptr::null_mut(),
                    wide.as_ptr(),
                    IMAGE_ICON,
                    size,
                    size,
                    LR_LOADFROMFILE | LR_DEFAULTSIZE,
                )
            };
            if icon.is_null() {
                eprintln!(
                    "rustx: the window's icon at {} could not be loaded by the system",
                    path.display()
                );
                return;
            }
            unsafe { SendMessageW(window, WM_SETICON, which, icon as isize) };
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use std::ffi::c_void;
    use std::path::Path;

    /// The other platforms' windows take their icon from the bundle or
    /// from the desktop entry, which the command line writes.
    pub(super) fn set(_window: *mut c_void, _path: &Path) {}
}

fn set(handle: *mut std::ffi::c_void, path: &Path) {
    platform::set(handle, path);
}

// ───────────── Dans le processus qui possède les fenêtres ─────────────

/// L'URL du document que l'application a chargé, sans fragment.
///
/// Une deuxième fenêtre charge le même document, avec un fragment qui
/// dit sa route. Retenue plutôt que recalculée : `App::run` l'a déjà
/// construite, et la reconstruire ailleurs serait deux vérités.
fn index() -> &'static Mutex<String> {
    static INDEX: OnceLock<Mutex<String>> = OnceLock::new();
    INDEX.get_or_init(|| Mutex::new(String::new()))
}

/// Dit par `App::run`, avant que la première fenêtre s'ouvre.
pub(crate) fn opens_from(url: &str) {
    *index()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = url.to_string();
}

/// Le document dont les fenêtres de cette application partent.
pub(crate) fn document() -> String {
    index()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

std::thread_local! {
    /// Les fenêtres ouvertes, par nom, sur le thread de l'interface du
    /// processus navigateur — le seul endroit où un `Browser` de CEF
    /// peut être touché.
    static OPEN_WINDOWS: std::cell::RefCell<Vec<(String, i32, cef::Browser)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Les noms des fenêtres demandées dont la création n'est pas encore
    /// revenue. Une file, parce que les créations sont séquentielles sur
    /// ce thread : `on_after_created` arrive dans l'ordre où on a
    /// demandé.
    static EXPECTED: std::cell::RefCell<
        std::collections::VecDeque<(String, rustx_os::window::Dressing)>,
    > = const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
}

/// Le nom de la fenêtre qu'on est en train de créer, et l'habit qu'elle
/// portera dès qu'elle existera.
pub(crate) fn expecting(name: &str, dressing: rustx_os::window::Dressing) {
    EXPECTED.with(|names| names.borrow_mut().push_back((name.to_string(), dressing)));
}

/// Une fenêtre vient d'exister : elle prend le nom qui l'attendait.
pub(crate) fn created(browser: &cef::Browser) {
    use cef::ImplBrowser;

    let (name, dressing) = EXPECTED
        .with(|names| names.borrow_mut().pop_front())
        .unwrap_or_else(|| ("main".to_string(), rustx_os::window::Dressing::System));
    OPEN_WINDOWS.with(|open| {
        open.borrow_mut()
            .push((name, browser.identifier(), browser.clone()))
    });
    // L'habit tout de suite : une application sans cadre n'en a jamais,
    // pas même le temps d'un éclair au démarrage.
    if let Some(handle) = {
        use cef::ImplBrowserHost;
        browser.host().map(|host| host.window_handle())
    } {
        wear(handle.0.cast(), dressing);
    }
}

/// Une fenêtre s'en va. `true` si c'était la dernière.
///
/// Ce que la boucle de messages lit : une application à deux fenêtres
/// dont on ferme la première ne s'arrête pas, et c'est le seul endroit
/// qui le sait.
pub(crate) fn gone(browser: Option<&cef::Browser>) -> bool {
    use cef::ImplBrowser;

    let identifier = browser.map(|browser| browser.identifier());
    OPEN_WINDOWS.with(|open| {
        let mut open = open.borrow_mut();
        match identifier {
            Some(identifier) => open.retain(|(_, each, _)| *each != identifier),
            // Sans identifiant, on ne sait pas laquelle est partie : la
            // plus ancienne, puisque c'est la seule supposition qui ne
            // garde pas une fenêtre morte dans la liste.
            None => {
                if !open.is_empty() {
                    open.remove(0);
                }
            }
        }
        open.is_empty()
    })
}

/// Combien de fenêtres cette application a ouvertes.
pub fn how_many_are_open() -> usize {
    OPEN_WINDOWS.with(|open| open.borrow().len())
}

/// Ce que la page a demandé, lu dans le processus de la fenêtre.
///
/// Rend `true` quand c'était pour nous, ce que Chromium lit comme
/// « traité ».
pub(crate) fn asked(message: &cef::ProcessMessage, from: Option<&cef::Browser>) -> bool {
    use cef::{CefString, ImplProcessMessage};

    let wire = CefString::from(&message.name()).to_string();
    let what = wire.strip_prefix(crate::bridge::PREFIX).unwrap_or(&wire);
    if what != OPEN && what != CLOSE && what != CONTROL {
        return false;
    }
    let values = message
        .argument_list()
        .map(|list| crate::bridge::read(&list))
        .unwrap_or_default();
    let text = |at: usize| match values.get(at) {
        Some(crate::value::Value::String(text)) => text.clone(),
        _ => String::new(),
    };
    let number = |at: usize, fallback: u32| match values.get(at) {
        Some(crate::value::Value::Number(number)) if *number > 0.0 => *number as u32,
        _ => fallback,
    };

    if what == CONTROL {
        obey(&text(0), from);
        return true;
    }

    if what == CLOSE {
        close_named(&text(0));
        return true;
    }

    let name = text(0);
    if name.is_empty() {
        eprintln!("rustx: une fenêtre a été demandée sans nom, et un nom est ce qui la referme");
        return true;
    }
    if let Some((_, _, open)) = OPEN_WINDOWS.with(|open| {
        open.borrow()
            .iter()
            .find(|(each, _, _)| *each == name)
            .cloned()
    }) {
        // Déjà ouverte : on la ramène devant plutôt que d'en ouvrir une
        // deuxième sous le même nom, que `close` ne saurait plus
        // désigner.
        use cef::{ImplBrowser, ImplBrowserHost};
        if let Some(host) = open.host() {
            host.set_focus(1);
        }
        return true;
    }
    let document = document();
    if document.is_empty() {
        eprintln!(
            "rustx: une fenêtre a été demandée avant que l'application ait chargé son document"
        );
        return true;
    }
    let at = route(&text(4));
    crate::cef_app::open_window(&crate::cef_app::WindowSpec {
        name,
        title: text(1),
        width: number(2, 900),
        height: number(3, 640),
        url: format!("{document}#{at}"),
        dressing: match rustx_os::window::Order::from_word(&text(5)) {
            Some(rustx_os::window::Order::Dress(dressing)) => dressing,
            // Rien, ou un mot qu'on ne connaît pas : le cadre du
            // système, qui est ce qu'une fenêtre a quand personne n'a
            // rien demandé.
            _ => rustx_os::window::Dressing::System,
        },
    });
    true
}

/// Fermer la fenêtre qui porte ce nom.
fn close_named(name: &str) {
    use cef::{ImplBrowser, ImplBrowserHost};

    let found = OPEN_WINDOWS.with(|open| {
        open.borrow()
            .iter()
            .find(|(each, _, _)| each == name)
            .map(|(_, _, browser)| browser.clone())
    });
    let Some(browser) = found else {
        eprintln!("rustx: aucune fenêtre ne s'appelle « {name} »");
        return;
    };
    if let Some(host) = browser.host() {
        // Sans forcer : la fenêtre a le droit de demander à son tour.
        host.close_browser(0);
    }
}

/// Exécuter un ordre sur la fenêtre d'où il vient.
///
/// Rien n'est nommé : le message arrive d'un `Browser`, et ce
/// `Browser` est une fenêtre. Une page ne commande donc que la sienne,
/// ce qui est la seule chose qu'elle puisse vouloir dire par
/// « réduis-toi ».
fn obey(word: &str, from: Option<&cef::Browser>) {
    use cef::{ImplBrowser, ImplBrowserHost};

    let Some(order) = rustx_os::window::Order::from_word(word) else {
        eprintln!("rustx: « {word} » n'est pas un ordre de fenêtre");
        return;
    };
    let Some(handle) = from
        .and_then(|browser| browser.host())
        .map(|host| host.window_handle())
    else {
        eprintln!(
            "rustx: « {word} » est arrivé sans fenêtre — la demande vient d'un processus \
             qui n'en a pas"
        );
        return;
    };
    let pane = rustx_os::window::Pane::from_handle(handle.0.cast());
    if let Err(refused) = order.given(pane) {
        eprintln!("rustx: « {word} » a été refusé : {refused}");
    }
}

/// Mettre la fenêtre dans l'habit qu'on lui a demandé en l'ouvrant.
///
/// Appelé à la création, parce que c'est Chromium qui fait la fenêtre
/// et que la poignée est la première chose qu'il rend. Une application
/// qui se veut sans cadre n'a ainsi jamais de cadre, pas même le temps
/// d'un éclair au démarrage.
pub(crate) fn wear(handle: *mut std::ffi::c_void, dressing: rustx_os::window::Dressing) {
    if dressing == rustx_os::window::Dressing::System {
        return;
    }
    let pane = rustx_os::window::Pane::from_handle(handle);
    if let Err(refused) = rustx_os::window::dress(pane, dressing) {
        eprintln!("rustx: la fenêtre n'a pas pu être déshabillée : {refused}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ico_is_recognised() {
        let path = std::env::temp_dir().join("rustx-window-icon-good.ico");
        // The header of a one-image icon, which is all `load` reads.
        std::fs::write(&path, [0u8, 0, 1, 0, 1, 0, 32, 32]).expect("a file");
        assert_eq!(load(&path), Ok(()));
    }

    #[test]
    fn a_png_named_ico_is_refused_by_name() {
        let path = std::env::temp_dir().join("rustx-window-icon-bad.ico");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n").expect("a file");
        let said = load(&path).expect_err("should be refused");
        assert!(said.contains("not an icon file"), "got {said}");
        assert!(said.contains(".png"), "got {said}");
    }

    #[test]
    fn a_missing_icon_says_which_file() {
        let said = load(Path::new("nowhere/at/all.ico")).expect_err("should be refused");
        assert!(said.contains("could not be read"), "got {said}");
    }
}
