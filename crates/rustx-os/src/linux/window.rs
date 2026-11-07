//! La fenêtre, sous Linux.
//!
//! Pas encore, et elle le dit. Ce que ce fichier sera : sous X11, les
//! propriétés `_MOTIF_WM_HINTS` pour enlever le cadre et
//! `_NET_WM_MOVERESIZE` pour commencer un déplacement — c'est ce que
//! fait tout logiciel à barre de titre propre ; sous Wayland, la même
//! chose passe par `xdg_toplevel` (`set_fullscreen`, `set_maximized`,
//! `move`), qui est la façon officielle et la seule.
//!
//! Les deux demandent une connexion au serveur d'affichage, que le
//! moteur a et que ce crate n'atteint pas encore. Le travail est donc
//! ce fichier, et l'API au-dessus ne change pas d'une ligne.
//!
//! Refuser en nommant la plateforme est voulu : un programme qui se
//! construit sous Linux et dont les boutons de fenêtre ne font rien en
//! silence est pire qu'un programme qui dit où il en est.

use crate::error::OsError;
use crate::window::{Dressing, Pane, Spread};

pub const AVAILABLE: bool = false;

const WHAT: &str = "window control";

pub fn dress(_pane: Pane, _dressing: Dressing) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn spread(_pane: Pane, _how: Spread) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn spread_of(_pane: Pane) -> Result<Spread, OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn on_top(_pane: Pane, _above: bool) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn drag(_pane: Pane) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn close(_pane: Pane) -> Result<(), OsError> {
    Err(OsError::Unsupported(WHAT))
}

pub fn pointer() -> Result<(i32, i32), OsError> {
    Err(OsError::Unsupported(WHAT))
}
