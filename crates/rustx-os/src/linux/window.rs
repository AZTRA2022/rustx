//! La fenêtre, sous Linux : X11, par le protocole lui-même.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! Ce fichier a été écrit sur une machine Windows, sans serveur
//! d'affichage. `cargo check --target x86_64-unknown-linux-gnu` est
//! tout ce qui l'a vérifié : il compile, et la logique pure d'en bas
//! (les indications Motif, les messages EWMH, la lecture de l'état, le
//! calcul du déplacement) a ses tests. Aucune fenêtre n'a été vue
//! perdre son cadre, se mettre en plein écran ou suivre la souris.
//! Avant de s'y fier en production, il faut le voir faire sur un vrai
//! bureau Linux — sous X11, et sous Wayland par XWayland — avec au moins
//! deux gestionnaires de fenêtres (Mutter et KWin, par exemple), parce
//! que tout ce qui suit est une *demande* au gestionnaire, et que chacun
//! l'honore à sa façon.
//!
//! # La poignée, et la connexion
//!
//! Sous Linux, `BrowserHost::window_handle()` rend un `XID` — un entier
//! qui désigne une fenêtre X11, la fenêtre de premier niveau que CEF a
//! faite. C'est ce que [`Pane`] porte, à la place d'un `HWND`.
//!
//! La connexion au serveur d'affichage n'est *pas* celle de Chromium :
//! CEF ne l'expose plus (`cef_get_xdisplay` a disparu de son API), et
//! il n'y a donc rien à faire passer depuis le moteur. Ce n'est pas un
//! pis-aller : un `XID` vaut pour tout le serveur, pas pour un client,
//! et c'est précisément ainsi qu'un gestionnaire de fenêtres ou
//! `xdotool` commandent des fenêtres qui ne sont pas les leurs. Ce
//! fichier ouvre donc la sienne, une fois, à la première demande.
//!
//! # Wayland
//!
//! Le `XID` est la seule poignée que CEF rend sous Linux : une fenêtre
//! CEF est une fenêtre X11, sous Wayland par XWayland. Une session
//! Wayland sans XWayland (`$WAYLAND_DISPLAY` sans `$DISPLAY`) n'a donc
//! pas de fenêtre que ce code puisse atteindre, et c'est refusé en le
//! disant. `xdg_toplevel` serait la voie Wayland, mais elle n'existe que
//! pour le client qui a créé la surface — Chromium, pas ce crate.
//!
//! # Ce qui est une demande, et ce qui ne l'est pas
//!
//! Sous X11, c'est le gestionnaire de fenêtres qui décide du cadre, de
//! la taille et du rang. On ne lui donne pas d'ordre : on pose une
//! propriété (`_MOTIF_WM_HINTS`) ou on lui envoie un message
//! (`_NET_WM_STATE`, `WM_CHANGE_STATE`) — exactement ce que GTK et Qt
//! font pour leurs propres fenêtres. Un gestionnaire qui ne les lit pas
//! (il y en a de très minimalistes) n'obéira pas, et rien ici ne peut
//! le savoir : ce qui est vérifié, c'est que le serveur a accepté la
//! demande.

use crate::error::OsError;
use crate::window::{Dressing, Pane, Spread};

pub const AVAILABLE: bool = true;

// Inutilisé sur une machine où ce fichier n'est compilé que pour être
// vérifié : `lib.rs` ne choisit ce répertoire que sous
// `cfg(target_os = "linux")`.
#[allow(unused_imports)]
pub use real::{
    close, demand_attention, drag, drag_out, dress, on_top, pointer, spread, spread_of,
};

// ─────────────── Ce qui se calcule sans serveur ───────────────
//
// Tout ce qui est construit ou lu avant de parler au serveur est ici,
// en entiers, et compilé partout : c'est la seule partie de ce fichier
// que des tests peuvent épingler sans écran.

/// `MWM_HINTS_DECORATIONS` : seul le champ des décorations est lu.
///
/// `MWM_HINTS_FUNCTIONS` n'est volontairement pas posé. C'est le même
/// piège que `WS_THICKFRAME` sous Windows : une fenêtre dont on
/// restreint les fonctions pour enlever son cadre est une fenêtre que
/// le gestionnaire refuse ensuite de retailler ou d'agrandir.
const MWM_HINTS_DECORATIONS: u32 = 1 << 1;
/// `MWM_DECOR_ALL` : toutes les décorations que le gestionnaire dessine.
const MWM_DECOR_ALL: u32 = 1;

/// La valeur de `_MOTIF_WM_HINTS` pour cet habit : cinq mots, `flags`,
/// `functions`, `decorations`, `input_mode`, `status`.
///
/// Remettre le cadre se dit `MWM_DECOR_ALL` plutôt qu'en retirant la
/// propriété : tous les gestionnaires relisent une valeur changée, pas
/// tous une propriété effacée.
pub(crate) fn motif_hints(dressing: Dressing) -> [u32; 5] {
    let decorations = match dressing {
        Dressing::None => 0,
        Dressing::System => MWM_DECOR_ALL,
    };
    [MWM_HINTS_DECORATIONS, 0, decorations, 0, 0]
}

/// Retirer ou ajouter, dans un message `_NET_WM_STATE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    Remove = 0,
    Add = 1,
}

/// Les cinq mots d'un message `_NET_WM_STATE` : l'action, une ou deux
/// propriétés, et la source.
///
/// La source est `1`, « une application normale » : un gestionnaire
/// qui voit `0` (« inconnue », ce que les très vieux clients envoient)
/// peut traiter la demande autrement, et certains l'ignorent.
pub(crate) fn state_change(change: Change, first: u32, second: u32) -> [u32; 5] {
    [change as u32, first, second, 1, 0]
}

/// `IconicState`, dans `WM_STATE` et dans `WM_CHANGE_STATE` (ICCCM).
pub(crate) const ICONIC: u32 = 3;

/// Les atomes dont la lecture de l'état a besoin, en entiers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StateNames {
    pub fullscreen: u32,
    pub hidden: u32,
    pub maximized_vert: u32,
    pub maximized_horz: u32,
}

/// Ce que la fenêtre occupe, d'après `_NET_WM_STATE` et `WM_STATE`.
///
/// Le plein écran d'abord, comme sous Windows : une fenêtre en plein
/// écran peut garder aussi ses deux drapeaux « agrandie », et c'est le
/// plein écran qu'on voit. Agrandie veut dire les deux sens : une
/// fenêtre agrandie à la verticale seulement (ce qu'un double-clic sur
/// le bord du haut fait chez certains) n'est pas « agrandie ».
pub(crate) fn spread_from(states: &[u32], names: StateNames, iconic: bool) -> Spread {
    let has = |atom: u32| atom != 0 && states.contains(&atom);
    if has(names.fullscreen) {
        Spread::Fullscreen
    } else if iconic || has(names.hidden) {
        Spread::Minimized
    } else if has(names.maximized_vert) && has(names.maximized_horz) {
        Spread::Maximized
    } else {
        Spread::Normal
    }
}

/// Où mettre la fenêtre quand la souris est passée de `grabbed_at` à
/// `now`, la fenêtre étant en `origin` au moment de l'appui.
///
/// Un écart et non une position : la fenêtre suit la souris depuis
/// l'endroit où on l'a attrapée, elle ne saute pas sous le pointeur.
pub(crate) fn dragged_to(
    origin: (i32, i32),
    grabbed_at: (i32, i32),
    now: (i32, i32),
) -> (i32, i32) {
    (
        origin.0 + (now.0 - grabbed_at.0),
        origin.1 + (now.1 - grabbed_at.1),
    )
}

/// Le `XID` qu'une poignée porte, si c'en est un.
///
/// Zéro est la poignée d'avant la fenêtre ; et un `XID` tient sur 29
/// bits (le protocole garde les trois du haut à zéro), donc ce qui ne
/// tient pas sur 32 n'a pas été rendu par CEF.
pub(crate) fn xid(pane: Pane) -> Result<u32, OsError> {
    if !pane.exists() {
        return Err(OsError::Platform(
            "there is no window to command yet: the handle is still nothing".to_string(),
        ));
    }
    u32::try_from(pane.0).map_err(|_| {
        OsError::Platform(format!(
            "{:#x} is not an X11 window id — it does not fit in 32 bits",
            pane.0
        ))
    })
}

/// S'il y a un serveur X à joindre, d'après l'environnement.
///
/// Séparé de la lecture de l'environnement pour qu'un test le dise
/// sans toucher aux variables du processus.
pub(crate) fn reachable(display: Option<&str>, wayland: Option<&str>) -> Result<(), OsError> {
    let set = |value: Option<&str>| value.is_some_and(|value| !value.is_empty());
    if set(display) {
        return Ok(());
    }
    if set(wayland) {
        return Err(OsError::Unsupported(
            "window control on a Wayland session without XWayland ($WAYLAND_DISPLAY is set, \
             $DISPLAY is not)",
        ));
    }
    Err(OsError::Platform(
        "there is no display to talk to: $DISPLAY is not set".to_string(),
    ))
}

// ─────────────────────── Ce qui parle au serveur ───────────────────────

#[cfg(target_os = "linux")]
mod real {
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        AtomEnum, ClientMessageEvent, ConfigureWindowAux, ConnectionExt as _, EventMask,
        KeyButMask, PropMode, Window,
    };
    use x11rb::rust_connection::RustConnection;
    use x11rb::wrapper::ConnectionExt as _;

    use super::{
        Change, ICONIC, StateNames, dragged_to, motif_hints, reachable, spread_from, state_change,
        xid,
    };
    use crate::error::OsError;
    use crate::window::{Dressing, Pane, Spread};

    x11rb::atom_manager! {
        Atoms: AtomsCookie {
            _MOTIF_WM_HINTS,
            _NET_WM_STATE,
            _NET_WM_STATE_FULLSCREEN,
            _NET_WM_STATE_MAXIMIZED_VERT,
            _NET_WM_STATE_MAXIMIZED_HORZ,
            _NET_WM_STATE_ABOVE,
            _NET_WM_STATE_HIDDEN,
            _NET_WM_STATE_DEMANDS_ATTENTION,
            _NET_FRAME_EXTENTS,
            WM_STATE,
            WM_CHANGE_STATE,
            WM_PROTOCOLS,
            WM_DELETE_WINDOW,
        }
    }

    /// La connexion de ce crate au serveur, et ce qu'on en a lu une
    /// fois.
    struct Display {
        link: RustConnection,
        root: Window,
        atoms: Atoms,
    }

    /// Ouverte à la première demande, et gardée : une connexion par
    /// ordre serait une poignée de main avec le serveur à chaque clic.
    /// Un échec est gardé aussi — un `$DISPLAY` absent ne réapparaît pas
    /// en cours de route.
    fn display() -> Result<&'static Display, OsError> {
        static ONE: OnceLock<Result<Display, OsError>> = OnceLock::new();
        ONE.get_or_init(open).as_ref().map_err(Clone::clone)
    }

    fn open() -> Result<Display, OsError> {
        let display = std::env::var("DISPLAY").ok();
        let wayland = std::env::var("WAYLAND_DISPLAY").ok();
        reachable(display.as_deref(), wayland.as_deref())?;
        let (link, screen) = x11rb::connect(None).map_err(|error| {
            OsError::Platform(format!(
                "the X server at $DISPLAY={} could not be reached: {error}",
                display.as_deref().unwrap_or("")
            ))
        })?;
        let root = link
            .setup()
            .roots
            .get(screen)
            .map(|screen| screen.root)
            .ok_or_else(|| {
                OsError::Platform(format!("the X server has no screen number {screen}"))
            })?;
        let atoms = Atoms::new(&link)
            .map_err(complaint("asking the X server for its atoms"))?
            .reply()
            .map_err(complaint("asking the X server for its atoms"))?;
        Ok(Display { link, root, atoms })
    }

    /// Ce que le serveur dit quand un appel échoue.
    fn complaint<E: std::fmt::Display>(about: &'static str) -> impl FnOnce(E) -> OsError {
        move |error| OsError::Platform(format!("{about} was refused by the X server: {error}"))
    }

    /// La fenêtre que le gestionnaire gère, depuis celle que CEF a
    /// rendue.
    ///
    /// C'est celle qui porte `WM_STATE` — le gestionnaire le pose sur
    /// chaque fenêtre qu'il prend en charge — et c'est à elle que les
    /// propriétés et les messages doivent être adressés, pas à un enfant
    /// ni au cadre que le gestionnaire a mis autour. CEF rend déjà la
    /// fenêtre de premier niveau, d'après son code ; on la cherche quand
    /// même en remontant, parce que c'est un détail de CEF et non une
    /// promesse de son API.
    ///
    /// Sans `WM_STATE` nulle part — une fenêtre pas encore montrée, ce
    /// qui est le cas quand on l'habille à sa création — c'est l'ancêtre
    /// le plus haut sous la racine : avant d'être montrée, une fenêtre
    /// n'a pas encore été placée dans un cadre, et cet ancêtre est elle.
    fn client(display: &Display, given: Window) -> Result<Window, OsError> {
        // Une demande sur un `XID` qui ne désigne plus rien revient en
        // `BadWindow` : c'est la question que `IsWindow` pose sous Windows,
        // posée ici en même temps que la première lecture.
        let gone = |_| {
            OsError::Platform(format!(
                "the handle {given:#x} is not a window any more — it was closed"
            ))
        };
        let mut at = given;
        let mut highest = given;
        // Une borne, pour qu'un arbre inattendu ne fasse pas tourner ceci
        // indéfiniment : aucune fenêtre réelle n'est à seize niveaux sous
        // la racine.
        for _ in 0..16 {
            let managed = display
                .link
                .get_property(false, at, display.atoms.WM_STATE, AtomEnum::ANY, 0, 0)
                .map_err(complaint("reading the window's state"))?
                .reply()
                .map_err(gone)?;
            if managed.type_ != u32::from(AtomEnum::NONE) {
                return Ok(at);
            }
            let tree = display
                .link
                .query_tree(at)
                .map_err(complaint("reading the window tree"))?
                .reply()
                .map_err(complaint("reading the window tree"))?;
            if tree.parent == tree.root || tree.parent == x11rb::NONE {
                return Ok(highest);
            }
            at = tree.parent;
            highest = at;
        }
        Ok(highest)
    }

    /// La connexion, et la fenêtre visée.
    ///
    /// La poignée est lue avant d'ouvrir quoi que ce soit : une poignée
    /// nulle est refusée pour ce qu'elle est, même sur une machine sans
    /// serveur X, comme sous Windows.
    fn target(pane: Pane) -> Result<(&'static Display, Window), OsError> {
        let given = xid(pane)?;
        let display = display()?;
        Ok((display, client(display, given)?))
    }

    /// Un message au gestionnaire, par la racine : c'est ainsi que l'EWMH
    /// et l'ICCCM veulent qu'on lui parle.
    fn tell(display: &Display, window: Window, kind: u32, data: [u32; 5]) -> Result<(), OsError> {
        let message = ClientMessageEvent::new(32, window, kind, data);
        display
            .link
            .send_event(
                false,
                display.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                message,
            )
            .map_err(complaint("sending a message to the window manager"))?
            .check()
            .map_err(complaint("sending a message to the window manager"))
    }

    fn state(
        display: &Display,
        window: Window,
        change: Change,
        first: u32,
        second: u32,
    ) -> Result<(), OsError> {
        tell(
            display,
            window,
            display.atoms._NET_WM_STATE,
            state_change(change, first, second),
        )
    }

    /// Les atomes de `_NET_WM_STATE`, et si `WM_STATE` dit « réduite ».
    fn read_state(display: &Display, window: Window) -> Result<(Vec<u32>, bool), OsError> {
        let states = display
            .link
            .get_property(
                false,
                window,
                display.atoms._NET_WM_STATE,
                AtomEnum::ATOM,
                0,
                64,
            )
            .map_err(complaint("reading the window's state"))?
            .reply()
            .map_err(complaint("reading the window's state"))?;
        let states: Vec<u32> = states.value32().map(Iterator::collect).unwrap_or_default();
        let wm_state = display
            .link
            .get_property(
                false,
                window,
                display.atoms.WM_STATE,
                display.atoms.WM_STATE,
                0,
                1,
            )
            .map_err(complaint("reading the window's state"))?
            .reply()
            .map_err(complaint("reading the window's state"))?;
        let iconic = wm_state
            .value32()
            .and_then(|mut words| words.next())
            .is_some_and(|word| word == ICONIC);
        Ok((states, iconic))
    }

    fn names(display: &Display) -> StateNames {
        StateNames {
            fullscreen: display.atoms._NET_WM_STATE_FULLSCREEN,
            hidden: display.atoms._NET_WM_STATE_HIDDEN,
            maximized_vert: display.atoms._NET_WM_STATE_MAXIMIZED_VERT,
            maximized_horz: display.atoms._NET_WM_STATE_MAXIMIZED_HORZ,
        }
    }

    fn flush(display: &Display) -> Result<(), OsError> {
        display
            .link
            .flush()
            .map_err(complaint("sending the request"))
    }

    pub fn dress(pane: Pane, dressing: Dressing) -> Result<(), OsError> {
        let (display, window) = target(pane)?;
        // Posée avant que la fenêtre soit montrée, la propriété est lue
        // au moment où le gestionnaire la prend : c'est le cas du moteur,
        // qui habille la fenêtre à sa création, et il n'y a alors jamais
        // de cadre. Changée sur une fenêtre déjà montrée, Mutter, KWin et
        // Xfwm la relisent ; un gestionnaire plus ancien peut attendre
        // que la fenêtre soit cachée puis remontrée, ce que ce fichier ne
        // fait pas exprès — Chromium voit ces allers-retours et les
        // prend pour une fenêtre qu'on ferme.
        display
            .link
            .change_property32(
                PropMode::REPLACE,
                window,
                display.atoms._MOTIF_WM_HINTS,
                display.atoms._MOTIF_WM_HINTS,
                &motif_hints(dressing),
            )
            .map_err(complaint("changing the window frame"))?
            .check()
            .map_err(complaint("changing the window frame"))?;
        flush(display)
    }

    pub fn spread(pane: Pane, how: Spread) -> Result<(), OsError> {
        let (display, window) = target(pane)?;
        let atoms = &display.atoms;
        // Contrairement à Windows, il n'y a ici rien à retenir : le
        // plein écran est un état que le gestionnaire tient, et c'est lui
        // qui sait de quelle taille revenir.
        if how == Spread::Fullscreen {
            state(
                display,
                window,
                Change::Add,
                atoms._NET_WM_STATE_FULLSCREEN,
                0,
            )?;
            return flush(display);
        }
        let (states, iconic) = read_state(display, window)?;
        // Tout autre ordre sort d'abord du plein écran, comme sous
        // Windows : une fenêtre qu'on réduit depuis le plein écran et
        // qu'on rouvre doit revenir à sa place, pas à l'écran entier —
        // et le gestionnaire, lui, garderait le plein écran sous la
        // réduction.
        if states.contains(&atoms._NET_WM_STATE_FULLSCREEN) {
            state(
                display,
                window,
                Change::Remove,
                atoms._NET_WM_STATE_FULLSCREEN,
                0,
            )?;
            if how == Spread::Normal {
                // Revenue d'où elle venait — agrandie, si elle l'était :
                // c'est ce que « revenir » veut dire d'un plein écran.
                return flush(display);
            }
        }
        match how {
            Spread::Minimized => {
                // `WM_CHANGE_STATE` et non `_NET_WM_STATE_HIDDEN` : l'EWMH
                // dit que ce drapeau-là se lit, et que réduire se demande
                // par l'ICCCM.
                tell(
                    display,
                    window,
                    atoms.WM_CHANGE_STATE,
                    [super::ICONIC, 0, 0, 0, 0],
                )?;
            }
            Spread::Maximized | Spread::Normal => {
                if iconic {
                    // Montrer une fenêtre réduite est, pour l'ICCCM, la
                    // façon de la rendre : il n'y a pas de message pour
                    // cela.
                    display
                        .link
                        .map_window(window)
                        .map_err(complaint("bringing the window back"))?
                        .check()
                        .map_err(complaint("bringing the window back"))?;
                }
                let change = if how == Spread::Maximized {
                    Change::Add
                } else {
                    Change::Remove
                };
                state(
                    display,
                    window,
                    change,
                    atoms._NET_WM_STATE_MAXIMIZED_VERT,
                    atoms._NET_WM_STATE_MAXIMIZED_HORZ,
                )?;
            }
            // Traité plus haut ; la branche existe parce que le
            // compilateur demande toutes les variantes.
            Spread::Fullscreen => {}
        }
        flush(display)
    }

    pub fn spread_of(pane: Pane) -> Result<Spread, OsError> {
        let (display, window) = target(pane)?;
        let (states, iconic) = read_state(display, window)?;
        Ok(spread_from(&states, names(display), iconic))
    }

    pub fn on_top(pane: Pane, above: bool) -> Result<(), OsError> {
        let (display, window) = target(pane)?;
        let change = if above { Change::Add } else { Change::Remove };
        state(
            display,
            window,
            change,
            display.atoms._NET_WM_STATE_ABOVE,
            0,
        )?;
        flush(display)
    }

    /// Marquer la fenêtre comme demandant l'attention : le gestionnaire
    /// la signale (clignement, couleur dans la barre des tâches) et retire
    /// la marque de lui-même quand elle prend le focus.
    pub fn demand_attention(pane: Pane) -> Result<(), OsError> {
        let (display, window) = target(pane)?;
        state(
            display,
            window,
            Change::Add,
            display.atoms._NET_WM_STATE_DEMANDS_ATTENTION,
            0,
        )?;
        flush(display)
    }

    /// Si un déplacement est en cours : un seul à la fois, parce qu'un
    /// deuxième appui pendant le premier ne peut pas exister, et que deux
    /// fils qui déplacent la même fenêtre se la disputeraient.
    static DRAGGING: AtomicBool = AtomicBool::new(false);

    /// Commencer un déplacement : la fenêtre suit la souris jusqu'au
    /// relâchement.
    ///
    /// # Pourquoi pas `_NET_WM_MOVERESIZE`
    ///
    /// C'est le message que l'EWMH prévoit pour cela, et ce qu'une
    /// fenêtre GTK envoie quand on attrape sa barre. Mais l'EWMH dit
    /// aussi que le client *doit* avoir rendu toutes ses prises de
    /// souris avant de l'envoyer — le gestionnaire prend alors la souris
    /// lui-même, et n'y arrive pas si quelqu'un d'autre la tient. Ici,
    /// quelqu'un la tient : l'appui a eu lieu dans la fenêtre de
    /// Chromium, et le serveur a donné à *la connexion de Chromium* la
    /// prise implicite qui suit tout appui. Une prise ne se rend que
    /// par la connexion qui la tient, et ce crate n'a pas celle-là. Le
    /// message partirait, et le gestionnaire ne pourrait rien en faire.
    ///
    /// Ce qui reste, et qui ne dépend d'aucune prise : lire où est la
    /// souris (`QueryPointer` répond à tout client, prise ou pas) et
    /// déplacer la fenêtre de l'écart, jusqu'à ce que le bouton soit
    /// relâché. C'est ce que fait `xdotool`, et ce que ce fichier fait.
    /// Ce que cela coûte : pas d'aimantation aux bords de l'écran ni de
    /// glisser-vers-le-haut-pour-agrandir, que seul le gestionnaire sait
    /// faire.
    ///
    /// Un fil, et non une boucle ici : contrairement à Windows, rien
    /// n'oblige à bloquer le thread de la fenêtre, et Chromium a besoin
    /// de lui pour redessiner pendant qu'on la traîne.
    pub fn drag(pane: Pane) -> Result<(), OsError> {
        let (display, window) = target(pane)?;
        let pressed = display
            .link
            .query_pointer(display.root)
            .map_err(complaint("reading the pointer"))?
            .reply()
            .map_err(complaint("reading the pointer"))?;
        if !pressed.mask.contains(KeyButMask::BUTTON1) {
            // Le bouton est déjà relâché quand l'ordre arrive : c'était un
            // clic, pas un déplacement, et il n'y a rien à suivre.
            return Ok(());
        }
        if DRAGGING.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let origin = match origin(display, window) {
            Ok(origin) => origin,
            Err(refused) => {
                DRAGGING.store(false, Ordering::SeqCst);
                return Err(refused);
            }
        };
        let grabbed_at = (i32::from(pressed.root_x), i32::from(pressed.root_y));
        std::thread::spawn(move || {
            follow(display, window, origin, grabbed_at);
            DRAGGING.store(false, Ordering::SeqCst);
        });
        Ok(())
    }

    /// Où la fenêtre est, dans les coordonnées où on la redemande.
    ///
    /// Pas tout à fait celles qu'elle a à l'écran : pour l'ICCCM, la
    /// position qu'un client demande est celle du coin du *cadre* du
    /// gestionnaire, pas de la fenêtre dedans. Sans ôter l'épaisseur du
    /// cadre (`_NET_FRAME_EXTENTS`), une fenêtre encadrée sauterait de
    /// la hauteur de sa barre au premier mouvement. Une fenêtre sans
    /// cadre a des épaisseurs nulles.
    fn origin(display: &Display, window: Window) -> Result<(i32, i32), OsError> {
        let at = display
            .link
            .translate_coordinates(window, display.root, 0, 0)
            .map_err(complaint("reading the window's place"))?
            .reply()
            .map_err(complaint("reading the window's place"))?;
        let extents: Vec<u32> = display
            .link
            .get_property(
                false,
                window,
                display.atoms._NET_FRAME_EXTENTS,
                AtomEnum::CARDINAL,
                0,
                4,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .and_then(|reply| reply.value32().map(Iterator::collect))
            .unwrap_or_default();
        let (left, top) = match extents.as_slice() {
            [left, _right, top, _bottom] => (*left as i32, *top as i32),
            _ => (0, 0),
        };
        Ok((i32::from(at.dst_x) - left, i32::from(at.dst_y) - top))
    }

    /// Suivre la souris jusqu'au relâchement.
    ///
    /// Une erreur arrête le déplacement sans rien dire : la seule qui
    /// arrive en pratique est la fenêtre fermée en cours de route, et la
    /// fin du déplacement est alors exactement ce qu'il faut.
    fn follow(display: &Display, window: Window, origin: (i32, i32), grabbed_at: (i32, i32)) {
        let mut last = origin;
        loop {
            // Soixante fois par seconde : ce que l'écran montre, pas plus.
            std::thread::sleep(Duration::from_millis(16));
            let Ok(Ok(now)) = display
                .link
                .query_pointer(display.root)
                .map(|cookie| cookie.reply())
            else {
                return;
            };
            if !now.mask.contains(KeyButMask::BUTTON1) {
                return;
            }
            let to = dragged_to(
                origin,
                grabbed_at,
                (i32::from(now.root_x), i32::from(now.root_y)),
            );
            if to == last {
                continue;
            }
            last = to;
            let moved = display
                .link
                .configure_window(window, &ConfigureWindowAux::new().x(to.0).y(to.1))
                .is_ok()
                && display.link.flush().is_ok();
            if !moved {
                return;
            }
        }
    }

    /// Refusé : XDND fait du client qui glisse le propriétaire de la
    /// sélection `XdndSelection` et exige qu'il tienne la prise de
    /// souris — celle que le serveur a donnée à la connexion de
    /// Chromium au moment de l'appui, et qu'une autre connexion ne peut
    /// ni emprunter ni rendre. C'est le même mur que `drag`, mais sans
    /// contournement possible : on peut suivre la souris soi-même,
    /// on ne peut pas tenir un dialogue XDND à la place de quelqu'un
    /// d'autre. Voir `window::drag_out`.
    pub fn drag_out(_pane: Pane, _paths: &[std::path::PathBuf]) -> Result<(), OsError> {
        Err(OsError::Unsupported(
            "dragging files out of the window on X11 (XDND requires owning XdndSelection and              holding the pointer grab, which belongs to Chromium's connection)",
        ))
    }

    pub fn close(pane: Pane) -> Result<(), OsError> {
        let (display, window) = target(pane)?;
        let atoms = &display.atoms;
        // `WM_DELETE_WINDOW` et non `DestroyWindow` ni `KillClient` : la
        // fenêtre est celle de Chromium, c'est lui qui doit la défaire, et
        // ce message est celui qu'il écoute — le même que la croix du
        // gestionnaire envoie. `KillClient` fermerait toute sa connexion,
        // donc toutes ses fenêtres d'un coup.
        let protocols = display
            .link
            .get_property(false, window, atoms.WM_PROTOCOLS, AtomEnum::ATOM, 0, 32)
            .map_err(complaint("reading what the window listens to"))?
            .reply()
            .map_err(complaint("reading what the window listens to"))?;
        let listens = protocols
            .value32()
            .is_some_and(|mut each| each.any(|atom| atom == atoms.WM_DELETE_WINDOW));
        if !listens {
            return Err(OsError::Platform(format!(
                "the window {window:#x} does not listen for WM_DELETE_WINDOW, and destroying \
                 it from outside would pull it from under Chromium"
            )));
        }
        let message = ClientMessageEvent::new(
            32,
            window,
            atoms.WM_PROTOCOLS,
            [atoms.WM_DELETE_WINDOW, x11rb::CURRENT_TIME, 0, 0, 0],
        );
        display
            .link
            .send_event(false, window, EventMask::NO_EVENT, message)
            .map_err(complaint("asking the window to close"))?
            .check()
            .map_err(complaint("asking the window to close"))?;
        flush(display)
    }

    /// Là où la souris est, en coordonnées d'écran — celles de la
    /// racine, qui couvre tous les moniteurs d'un même bureau.
    pub fn pointer() -> Result<(i32, i32), OsError> {
        let display = display()?;
        let at = display
            .link
            .query_pointer(display.root)
            .map_err(complaint("reading the pointer"))?
            .reply()
            .map_err(complaint("reading the pointer"))?;
        Ok((i32::from(at.root_x), i32::from(at.root_y)))
    }
}

/// Les mêmes noms, pour que ce fichier se vérifie sur une machine qui
/// n'est pas Linux — voir `lib.rs`. Jamais la plateforme choisie hors
/// de Linux.
#[cfg(not(target_os = "linux"))]
mod real {
    use crate::error::OsError;
    use crate::window::{Dressing, Pane, Spread};

    const ELSEWHERE: &str = "linux/window.rs is compiled for its own sake on a non-Linux \
                             machine, and is never the platform module chosen there";

    pub fn dress(_pane: Pane, _dressing: Dressing) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn spread(_pane: Pane, _how: Spread) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn spread_of(_pane: Pane) -> Result<Spread, OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn on_top(_pane: Pane, _above: bool) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn drag(_pane: Pane) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn drag_out(_pane: Pane, _paths: &[std::path::PathBuf]) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn close(_pane: Pane) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn demand_attention(_pane: Pane) -> Result<(), OsError> {
        unreachable!("{ELSEWHERE}")
    }
    pub fn pointer() -> Result<(i32, i32), OsError> {
        unreachable!("{ELSEWHERE}")
    }
}

#[cfg(test)]
mod tests {
    // Compilés et lancés partout, Windows compris : cette logique est en
    // entiers, et c'est la seule partie de ce fichier qu'une machine sans
    // serveur X peut faire tourner.
    use super::*;

    const NAMES: StateNames = StateNames {
        fullscreen: 101,
        hidden: 102,
        maximized_vert: 103,
        maximized_horz: 104,
    };

    #[test]
    fn frameless_asks_for_no_decorations_and_leaves_the_functions_alone() {
        let hints = motif_hints(Dressing::None);
        assert_eq!(hints[0], MWM_HINTS_DECORATIONS, "only decorations are said");
        assert_eq!(
            hints[0] & 1,
            0,
            "MWM_HINTS_FUNCTIONS would take resizing away"
        );
        assert_eq!(hints[2], 0, "no decorations at all");
    }

    #[test]
    fn framed_asks_for_every_decoration_rather_than_deleting_the_hint() {
        let hints = motif_hints(Dressing::System);
        assert_eq!(hints, [MWM_HINTS_DECORATIONS, 0, MWM_DECOR_ALL, 0, 0]);
    }

    #[test]
    fn a_state_message_says_its_action_its_atoms_and_that_it_is_an_application() {
        assert_eq!(state_change(Change::Add, 7, 8), [1, 7, 8, 1, 0]);
        assert_eq!(state_change(Change::Remove, 7, 0), [0, 7, 0, 1, 0]);
    }

    #[test]
    fn fullscreen_is_read_before_anything_else() {
        let all = [103, 104, 101];
        assert_eq!(spread_from(&all, NAMES, false), Spread::Fullscreen);
        assert_eq!(spread_from(&[101], NAMES, true), Spread::Fullscreen);
    }

    #[test]
    fn minimized_is_read_from_either_icccm_or_ewmh() {
        assert_eq!(spread_from(&[], NAMES, true), Spread::Minimized);
        assert_eq!(spread_from(&[102], NAMES, false), Spread::Minimized);
        assert_eq!(spread_from(&[103, 104], NAMES, true), Spread::Minimized);
    }

    #[test]
    fn maximized_means_both_directions() {
        assert_eq!(spread_from(&[104, 103], NAMES, false), Spread::Maximized);
        assert_eq!(spread_from(&[103], NAMES, false), Spread::Normal);
        assert_eq!(spread_from(&[104], NAMES, false), Spread::Normal);
        assert_eq!(spread_from(&[], NAMES, false), Spread::Normal);
    }

    #[test]
    fn an_atom_the_server_does_not_have_is_never_matched() {
        // Un atome jamais créé vaut zéro ; une liste qui contiendrait un
        // zéro ne doit pas passer pour « plein écran ».
        let none = StateNames {
            fullscreen: 0,
            ..NAMES
        };
        assert_eq!(spread_from(&[0], none, false), Spread::Normal);
    }

    #[test]
    fn a_dragged_window_follows_the_pointer_by_the_distance_it_moved() {
        assert_eq!(dragged_to((100, 50), (130, 60), (130, 60)), (100, 50));
        assert_eq!(dragged_to((100, 50), (130, 60), (150, 40)), (120, 30));
        // Vers un moniteur à gauche du principal : des coordonnées
        // négatives sont des coordonnées.
        assert_eq!(dragged_to((10, 10), (20, 20), (-200, 25)), (-210, 15));
    }

    #[test]
    fn a_handle_is_an_xid_only_if_it_is_one() {
        assert!(xid(Pane(0)).is_err());
        assert_eq!(xid(Pane(0x0420_0007)).expect("an xid"), 0x0420_0007);
        assert!(xid(Pane(-1)).is_err());
        assert!(xid(Pane(isize::MAX)).is_err() || usize::BITS == 32);
    }

    #[test]
    fn a_wayland_session_without_xwayland_is_refused_by_name() {
        assert_eq!(reachable(Some(":0"), None), Ok(()));
        assert_eq!(reachable(Some(":1"), Some("wayland-0")), Ok(()));
        let refused = reachable(None, Some("wayland-0")).expect_err("no X server");
        assert!(matches!(refused, OsError::Unsupported(what) if what.contains("Wayland")));
        let refused = reachable(Some(""), None).expect_err("no display at all");
        assert!(refused.to_string().contains("$DISPLAY"), "got {refused}");
    }
}
