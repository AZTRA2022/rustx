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

/// Oui : la spécification de freedesktop.org porte des actions, et
/// `notify-send --action` les expose depuis libnotify 0.8 (2022). Une
/// distribution plus ancienne refuse cette option, et [`ask`] le dit
/// alors en nommant la version.
pub const CAN_ASK: bool = true;

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

/// Les arguments de `notify-send` pour une notification à boutons.
///
/// `--action=ID=Libellé` une fois par bouton : `notify-send` écrit sur
/// sa sortie standard l'`ID` de celui qui a été choisi, et rien du tout
/// si la notification a été fermée ou a expiré. `--wait` est
/// indispensable — sans lui, `notify-send` rend la main aussitôt et il
/// n'y a rien à lire.
pub fn asking(what: &What, actions: &[crate::notify::Action]) -> Vec<String> {
    // `asked`, et non `arguments` : une variable de ce nom-là cacherait
    // la fonction `arguments` qu'on appelle deux lignes plus bas.
    let mut asked = vec!["--wait".to_string()];
    for action in actions {
        // Pas de `--` ici : ce sont des options, et leur valeur est
        // collée avec un `=` pour qu'un libellé commençant par un tiret
        // reste un libellé.
        asked.push(format!("--action={}={}", action.id, action.label));
    }
    asked.extend(arguments(what));
    asked
}

pub fn ask(
    what: &What,
    actions: &[crate::notify::Action],
) -> Result<Option<String>, OsError> {
    let out = Command::new("notify-send")
        .args(asking(what, actions))
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            OsError::Platform(format!(
                "`notify-send` could not be run ({error}) — it is in the `libnotify-bin`                  package"
            ))
        })?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        // Une libnotify d'avant 0.8 ne connaît ni `--action` ni
        // `--wait`, et le dit comme toute option inconnue.
        if said.contains("--action") || said.contains("--wait") || said.contains("Unknown option")
        {
            return Err(OsError::Unsupported(
                "buttons in a notification with this `notify-send` (--action and --wait came                  with libnotify 0.8)",
            ));
        }
        return Err(OsError::Platform(format!(
            "`notify-send` failed: {}",
            said.trim()
        )));
    }
    let chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // Rien sur la sortie : fermée ou expirée, ce qui est une réponse.
    Ok((!chosen.is_empty()).then_some(chosen))
}

#[cfg(test)]
mod asking_tests {
    use super::*;
    use crate::notify::Action;

    #[test]
    fn every_button_becomes_an_action_option_and_the_call_waits() {
        let what = What::new("Sauvegarde", "Elle est prête.");
        let actions = [Action::new("open", "Ouvrir"), Action::new("later", "Plus tard")];
        let arguments = asking(&what, &actions);
        assert_eq!(arguments[0], "--wait", "without it there is nothing to read");
        assert!(arguments.contains(&"--action=open=Ouvrir".to_string()), "{arguments:?}");
        assert!(arguments.contains(&"--action=later=Plus tard".to_string()), "{arguments:?}");
        // Le titre et le corps restent derrière `--`.
        let separator = arguments.iter().position(|each| each == "--").expect("a --");
        assert_eq!(arguments[separator + 1], "Sauvegarde");
        assert_eq!(arguments[separator + 2], "Elle est prête.");
    }

    #[test]
    fn a_label_that_looks_like_an_option_stays_a_label() {
        let actions = [Action::new("rm", "--force")];
        let arguments = asking(&What::new("t", "b"), &actions);
        assert!(arguments.contains(&"--action=rm=--force".to_string()), "{arguments:?}");
    }
}
