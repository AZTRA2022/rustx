//! Les notifications, sous Linux.
//!
//! # Par `notify-send`, et pourquoi
//!
//! La bonne façon est un appel D-Bus à
//! `org.freedesktop.Notifications.Notify` : c'est la spécification que
//! GNOME, KDE et les autres implémentent, et c'est elle que le portail
//! relaie en bac à sable. Mais parler D-Bus demande une dépendance
//! — `zbus` et son runtime, ou les vtables à la main — et ce crate n'a
//! aujourd'hui que les liaisons de Microsoft.
//!
//! `notify-send` est l'autre bout du même mécanisme : c'est
//! exactement cet appel D-Bus, dans un programme que `libnotify`
//! installe et que toute distribution de bureau a. Ce que cela coûte :
//! un processus par notification, et un refus nommé quand le programme
//! n'est pas là — ce que dit [`show`], avec le paquet à installer.
//!
//! # Écrit, pas éprouvé
//!
//! Ce fichier a été écrit sur une machine Windows et n'a jamais tourné
//! sous Linux. Il est là parce qu'un refus poli valait moins qu'un
//! chemin qui marche, et il est signalé ici pour que personne ne le
//! prenne pour du code mesuré.

use std::process::{Command, Stdio};

use crate::error::OsError;
use crate::notify::What;

pub const AVAILABLE: bool = true;

/// Les arguments de `notify-send` pour cette notification.
///
/// Séparé de l'exécution pour une raison simple : c'est la partie qui
/// peut être fausse, et c'est la seule qui peut être lue par un test
/// sur n'importe quelle machine.
pub fn arguments(what: &What) -> Vec<String> {
    let mut arguments = Vec::new();
    if let Some(icon) = &what.icon {
        arguments.push("--icon".to_string());
        arguments.push(icon.to_string_lossy().to_string());
    }
    // `--` avant le titre : un titre qui commence par un tiret est un
    // titre, pas une option. C'est le genre de chose qui ne se voit
    // qu'un jour où un utilisateur nomme son fichier « -r ».
    arguments.push("--".to_string());
    arguments.push(what.title.clone());
    // Le corps est passé même vide : `notify-send` accepte un titre
    // seul, mais alors un corps vide deviendrait le titre de la
    // prochaine version.
    arguments.push(what.body.clone());
    arguments
}

pub fn show(what: &What) -> Result<(), OsError> {
    // Elle ne bloque pas : on ne lit pas la sortie, on ne l'attend pas.
    // Une notification qui attendrait sa lecture attendrait pour
    // toujours.
    Command::new("notify-send")
        .args(arguments(what))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            OsError::Platform(format!(
                "`notify-send` could not be run ({error}). It is what carries a \
                 notification on Linux — install `libnotify-bin` (Debian, Ubuntu) or \
                 `libnotify` (Arch, Fedora)."
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_that_looks_like_an_option_is_still_a_title() {
        let what = What::new("-r", "le corps");
        let arguments = arguments(&what);
        let separator = arguments
            .iter()
            .position(|argument| argument == "--")
            .expect("the separator");
        assert_eq!(arguments[separator + 1], "-r");
        assert_eq!(arguments[separator + 2], "le corps");
    }

    #[test]
    fn an_icon_is_given_before_the_separator() {
        let what = What::new("titre", "corps").icon("/usr/share/icons/app.png");
        let arguments = arguments(&what);
        assert_eq!(arguments[0], "--icon");
        assert_eq!(arguments[1], "/usr/share/icons/app.png");
        assert_eq!(arguments[2], "--");
    }
}
