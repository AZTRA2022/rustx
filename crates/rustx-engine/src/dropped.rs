//! Ce qu'on lâche sur la fenêtre, avec son vrai chemin.
//!
//! Une page voit un dépôt : `ondrop`, un `DataTransfer`, et des objets
//! `File` qu'elle peut lire. Ce qu'elle ne voit pas, et que le
//! navigateur lui cache exprès, c'est **où** ces fichiers sont. Un
//! logiciel qui catalogue, convertit, surveille ou réenregistre a
//! besoin du chemin : pour rouvrir le fichier demain, pour écrire à
//! côté, pour le passer à une bibliothèque Rust.
//!
//! Le gestionnaire natif de Chromium le connaît. Il vit dans le
//! processus de la fenêtre ; une page vit dans celui de rendu. Les
//! chemins traversent donc par [`crate::bridge`], et arrivent ici.
//!
//! ```ignore
//! dropped::on_files(|paths| for path in paths { catalogue(&path); });
//! // ou, pour une page qui préfère lire à chaque battement :
//! for path in dropped::files() { catalogue(&path); }
//! ```
//!
//! # Au moment où ça entre, pas au moment où ça tombe
//!
//! Ce que CEF rapporte est l'**entrée** du curseur dans la fenêtre avec
//! quelque chose dans la main, et non le lâcher : son gestionnaire de
//! glisser-déposer n'a pas d'événement pour le lâcher. Les chemins sont
//! donc connus un instant avant, et c'est à ce moment-là qu'ils
//! arrivent. Une page qui veut la coïncidence exacte écoute son propre
//! `ondrop` et lit [`files`] à ce moment-là : le chemin est déjà là.
//!
//! Conséquence assumée : un glisser-déposer abandonné — entré dans la
//! fenêtre, puis ressorti sans lâcher — laisse des chemins en attente.
//! [`files`] les prend, et le prochain dépôt les remplace.

use std::cell::RefCell;
use std::path::PathBuf;

use crate::value::Value;

/// Le nom sous lequel les chemins traversent.
pub const WHAT: &str = "dropped";

thread_local! {
    /// Ce qui a été lâché et que personne n'a encore lu.
    static WAITING: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    /// Les crochets, dans l'ordre où ils ont été posés.
    static HOOKS: RefCell<Vec<Box<dyn Fn(Vec<PathBuf>)>>> = RefCell::new(Vec::new());
    /// Si le pont a déjà été branché sur ce fil.
    static LISTENING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Être appelé quand des fichiers arrivent.
///
/// Sur le fil de la page, comme un gestionnaire d'événement. Plusieurs
/// crochets sont appelés dans l'ordre où ils ont été posés, chacun avec
/// la liste entière.
pub fn on_files(hook: impl Fn(Vec<PathBuf>) + 'static) {
    listening();
    HOOKS.with(|hooks| hooks.borrow_mut().push(Box::new(hook)));
}

/// Ce qui a été lâché depuis la dernière lecture.
///
/// Pris : lu une fois. Sans cela, une page qui lit à chaque battement
/// rouvrirait le même fichier sans fin.
pub fn files() -> Vec<PathBuf> {
    listening();
    WAITING.with(|waiting| std::mem::take(&mut *waiting.borrow_mut()))
}

/// Brancher le pont, une fois par fil.
///
/// Appelé au démarrage du processus de rendu, et non seulement à la
/// première lecture : une page qui ne lit qu'à son premier battement
/// manquerait ce qui a été lâché avant lui.
pub fn ready() {
    listening();
}

fn listening() {
    if LISTENING.with(std::cell::Cell::get) {
        return;
    }
    LISTENING.with(|listening| listening.set(true));

    // Les chemins arrivent de la fenêtre et attendent. Rien n'est
    // annoncé ici : l'entrée du curseur dans la fenêtre n'est pas un
    // dépôt, et un fichier traîné *au-dessus* d'une application ne doit
    // pas déclencher ce qu'un lâcher déclenche.
    crate::bridge::listen(WHAT, |values| {
        let paths: Vec<PathBuf> = values
            .into_iter()
            .filter_map(|value| match value {
                Value::String(path) if !path.is_empty() => Some(PathBuf::from(path)),
                _ => None,
            })
            .collect();
        // Un dépôt sans fichier n'est pas un dépôt de fichiers : du
        // texte, un lien, une image d'une page web.
        if paths.is_empty() {
            return;
        }
        WAITING.with(|waiting| *waiting.borrow_mut() = paths);
    });
}

/// Le lâcher a eu lieu : appeler les crochets.
///
/// Séparé de l'arrivée des chemins parce que les deux moments sont
/// différents : la fenêtre connaît les chemins quand le curseur entre,
/// et la page sait quand on lâche. Appelé par le gestionnaire que le
/// moteur met dans chaque page ([`LISTENER`]), et par un test à sa
/// place.
pub fn landed() {
    let paths = WAITING.with(|waiting| waiting.borrow().clone());
    if paths.is_empty() {
        return;
    }
    // Les crochets sont sortis de la liste le temps de l'appel : l'un
    // d'eux a le droit d'en poser un autre.
    let hooks = HOOKS.with(|hooks| std::mem::take(&mut *hooks.borrow_mut()));
    for hook in &hooks {
        hook(paths.clone());
    }
    HOOKS.with(|held| {
        let mut held = held.borrow_mut();
        let posted = std::mem::take(&mut *held);
        *held = hooks;
        held.extend(posted);
    });
}

/// Ce que le moteur met dans chaque page pour que le lâcher soit à
/// l'application et non au navigateur.
///
/// Sans `preventDefault`, Chromium fait ce qu'un navigateur fait d'un
/// fichier lâché sur une page : il y navigue. Mesuré en lâchant un
/// `.rsx` sur une fenêtre — l'application a disparu, remplacée par le
/// texte du fichier. Pour un logiciel, c'est un plantage : sa fenêtre
/// n'est plus la sienne.
pub const LISTENER: &str = "\
addEventListener('dragover', (event) => { event.preventDefault(); });\n\
addEventListener('drop', (event) => {\n\
  event.preventDefault();\n\
  if (typeof __rustx_dropped === 'function') { __rustx_dropped(); }\n\
});\n";

/// Ce que le côté fenêtre met dans le message.
///
/// Les noms tels que Chromium les donne, et rien quand il n'en donne
/// aucun — ce qui est le cas d'un dépôt de texte ou de lien.
pub fn carried(names: &[String]) -> Vec<Value> {
    names
        .iter()
        .filter(|name| !name.is_empty())
        .map(|name| Value::String(name.clone()))
        .collect()
}
