//! Les écrans, sous Linux : RandR, par la connexion X11.
//!
//! # Vérifié à la compilation, jamais lancé
//!
//! `cargo check --target x86_64-unknown-linux-gnu` seulement ; le
//! calcul de la zone de travail depuis `_NET_WORKAREA` a son test, qui
//! tourne partout. Aucun écran réel n'a été listé par ce code.
//!
//! # Ce que RandR donne, et ce qu'il ne donne pas
//!
//! Les sorties branchées, leur nom (`DP-2`, `eDP-1`) et leur place sur
//! le bureau : c'est ce que `xrandr` affiche, et la même requête. Ce
//! que RandR ne connaît pas : les barres du bureau. La zone de travail
//! est `_NET_WORKAREA`, une propriété de la racine — qui n'en donne
//! qu'*une*, pour tout le bureau, et non une par écran. Elle est donc
//! rendue à l'écran qui la contient, et les autres ont leur rectangle
//! entier comme zone de travail : c'est faux pour un deuxième moniteur
//! qui porterait un panneau, et c'est tout ce que X11 dit.
//!
//! L'échelle vient de la même source que `system::scale` — X11 n'a pas
//! d'échelle par écran, et les bureaux qui en ont une la tiennent
//! ailleurs.

use crate::error::OsError;
use crate::screen::{Area, Screen};

/// La zone de travail d'un écran : l'intersection de `_NET_WORKAREA`
/// avec lui, quand elle le touche, et lui-même sinon.
pub(crate) fn work_for(bounds: Area, workarea: Option<Area>) -> Area {
    let Some(work) = workarea else {
        return bounds;
    };
    let left = bounds.x.max(work.x);
    let top = bounds.y.max(work.y);
    let right = (bounds.x + bounds.width).min(work.x + work.width);
    let bottom = (bounds.y + bounds.height).min(work.y + work.height);
    if right <= left || bottom <= top {
        // La zone de travail ne touche pas cet écran : elle est celle du
        // bureau, et cet écran n'est pas celui qui porte les barres.
        return bounds;
    }
    Area::between(left, top, right, bottom)
}

#[cfg(target_os = "linux")]
pub fn screens() -> Result<Vec<Screen>, OsError> {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};

    let (link, number) = x11rb::connect(None).map_err(|error| {
        OsError::Platform(format!("the X server could not be reached: {error}"))
    })?;
    let screen = &link.setup().roots[number];
    let root = screen.root;
    let refused = |error: Box<dyn std::error::Error + Send + Sync>| {
        OsError::Platform(format!("RandR refused: {error}"))
    };

    // `_NET_WORKAREA` : quatre entiers par bureau virtuel, et c'est le
    // premier qui est le bureau courant.
    let workarea = link
        .intern_atom(true, b"_NET_WORKAREA")
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|reply| reply.atom)
        .filter(|atom| *atom != x11rb::NONE)
        .and_then(|atom| {
            let reply = link
                .get_property(false, root, atom, AtomEnum::CARDINAL, 0, 4)
                .ok()?
                .reply()
                .ok()?;
            let words: Vec<u32> = reply.value32()?.collect();
            match words.as_slice() {
                [x, y, width, height, ..] => Some(Area {
                    x: *x as i32,
                    y: *y as i32,
                    width: *width as i32,
                    height: *height as i32,
                }),
                _ => None,
            }
        });

    let resources = link
        .randr_get_screen_resources_current(root)
        .map_err(|error| refused(error.into()))?
        .reply()
        .map_err(|error| refused(error.into()))?;
    let primary = link
        .randr_get_output_primary(root)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|reply| reply.output);
    let scale = super::system::scale().unwrap_or(1.0);

    let mut found = Vec::new();
    for output in resources.outputs {
        let Ok(Ok(information)) = link
            .randr_get_output_info(output, resources.config_timestamp)
            .map(|cookie| cookie.reply())
        else {
            continue;
        };
        // `crtc` nul : la sortie existe mais ne dessine rien — un port
        // sans écran branché, ou un écran éteint.
        if information.crtc == x11rb::NONE {
            continue;
        }
        let Ok(Ok(crtc)) = link
            .randr_get_crtc_info(information.crtc, resources.config_timestamp)
            .map(|cookie| cookie.reply())
        else {
            continue;
        };
        let bounds = Area {
            x: i32::from(crtc.x),
            y: i32::from(crtc.y),
            width: i32::from(crtc.width),
            height: i32::from(crtc.height),
        };
        found.push(Screen {
            name: String::from_utf8_lossy(&information.name).to_string(),
            bounds,
            work: work_for(bounds, workarea),
            scale,
            primary: primary == Some(output),
        });
    }
    if found.is_empty() {
        return Err(OsError::Platform(
            "RandR reported no connected output".to_string(),
        ));
    }
    Ok(found)
}

/// Jamais la plateforme choisie hors de Linux — voir `lib.rs`.
#[cfg(not(target_os = "linux"))]
pub fn screens() -> Result<Vec<Screen>, OsError> {
    unreachable!("linux/screen.rs is never the platform module outside Linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEFT: Area = Area {
        x: -1920,
        y: 0,
        width: 1920,
        height: 1080,
    };
    const MAIN: Area = Area {
        x: 0,
        y: 0,
        width: 2560,
        height: 1440,
    };

    #[test]
    fn the_work_area_is_cut_out_of_the_screen_that_carries_the_bars() {
        // Une barre de 48 pixels en bas de l'écran principal, et le
        // bureau s'étend de -1920 à 2560.
        let desktop = Area::between(-1920, 0, 2560, 1392);
        assert_eq!(
            work_for(MAIN, Some(desktop)),
            Area::between(0, 0, 2560, 1392),
            "the primary screen loses the bar"
        );
    }

    #[test]
    fn a_screen_the_work_area_misses_keeps_its_whole_self() {
        // Une zone de travail qui ne couvre que l'écran principal.
        let only_main = Area::between(0, 0, 2560, 1392);
        assert_eq!(work_for(LEFT, Some(only_main)), LEFT);
        assert_eq!(work_for(LEFT, None), LEFT);
    }
}
