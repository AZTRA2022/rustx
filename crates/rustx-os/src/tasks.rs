//! Les actions que le système offre sur l'icône de l'application, sans
//! passer par sa fenêtre : la liste de raccourcis de Windows (clic
//! droit sur l'icône de la barre des tâches), le menu du Dock de macOS.
//!
//! Ce qu'on y met : « Nouveau document », « Ouvrir le dernier »,
//! « Démarrer l'enregistrement ». Ce sont des façons de *lancer*
//! l'application en lui disant déjà quoi faire — l'utilisateur clique
//! avant que la fenêtre existe.
//!
//! # Chaque action relance le programme
//!
//! C'est la différence avec un menu de barre système
//! ([`crate::tray`]), et elle décide de tout : une action de cette
//! liste n'est pas un message envoyé à l'application qui tourne, c'est
//! une ligne de commande que le système exécute. Une application qui
//! tourne déjà la reçoit par son instance unique
//! ([`crate::instance`]), qui est faite pour cela ; une application
//! arrêtée démarre avec ces arguments.
//!
//! Conséquence : une action porte des arguments, pas un identifiant à
//! écouter. Il n'y a rien à lire en retour, et donc aucune fonction
//! pour le faire.

use std::path::PathBuf;

use crate::error::OsError;

/// Une ligne de la liste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Ce que l'utilisateur lit.
    pub label: String,
    /// L'info-bulle, si on en veut une.
    pub tooltip: String,
    /// Le programme à lancer. Celui de ce processus, le plus souvent —
    /// [`own_program`] le donne.
    pub program: PathBuf,
    /// Les arguments qu'il recevra. Chacun est passé tel quel ; ce qui
    /// contient une espace est entouré de guillemets au passage.
    pub arguments: Vec<String>,
}

impl Task {
    /// Une action qui relance ce programme avec ces arguments.
    pub fn new(label: impl Into<String>, arguments: &[&str]) -> Result<Self, OsError> {
        Ok(Self {
            label: label.into(),
            tooltip: String::new(),
            program: own_program()?,
            arguments: arguments.iter().map(|each| each.to_string()).collect(),
        })
    }

    /// La même, avec son info-bulle.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = text.into();
        self
    }
}

/// Le programme de ce processus.
pub fn own_program() -> Result<PathBuf, OsError> {
    std::env::current_exe()
        .map_err(|error| OsError::Platform(format!("this program's own path is unknown: {error}")))
}

/// Si cette plateforme a une telle liste et que ce code l'atteint.
pub fn supported() -> bool {
    crate::platform::tasks::AVAILABLE
}

/// Poser la liste, en remplaçant celle d'avant.
pub fn offer(tasks: &[Task]) -> Result<(), OsError> {
    if tasks.is_empty() {
        return clear();
    }
    if let Some(nameless) = tasks.iter().find(|task| task.label.trim().is_empty()) {
        return Err(OsError::Platform(format!(
            "a task with no label is a blank line in the menu (its program is {})",
            nameless.program.display()
        )));
    }
    crate::platform::tasks::offer(tasks)
}

/// L'enlever.
pub fn clear() -> Result<(), OsError> {
    crate::platform::tasks::clear()
}

/// Les arguments en une ligne de commande, chacun entre guillemets s'il
/// en a besoin.
///
/// Ce qui en a besoin : une espace, une tabulation, ou un guillemet. Un
/// chemin avec une espace passé sans guillemets devient deux arguments,
/// et c'est le défaut classique de cette API.
pub(crate) fn command_line(arguments: &[String]) -> String {
    arguments
        .iter()
        .map(|argument| {
            if argument.is_empty() {
                "\"\"".to_string()
            } else if argument.contains([' ', '\t', '"']) {
                format!("\"{}\"", argument.replace('"', "\\\""))
            } else {
                argument.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_argument_with_a_space_stays_one_argument() {
        let arguments = vec!["--open".to_string(), r"C:\My Files\a.rx".to_string()];
        assert_eq!(command_line(&arguments), r#"--open "C:\My Files\a.rx""#);
    }

    #[test]
    fn a_quote_inside_an_argument_is_escaped_and_an_empty_one_survives() {
        let arguments = vec![r#"say "hi""#.to_string(), String::new()];
        assert_eq!(command_line(&arguments), r#""say \"hi\"" """#);
        assert_eq!(command_line(&[]), "");
    }

    #[test]
    fn a_task_without_a_label_is_refused_before_the_list_is_touched() {
        let blank = Task {
            label: "   ".to_string(),
            tooltip: String::new(),
            program: "x".into(),
            arguments: Vec::new(),
        };
        let said = offer(&[blank]).expect_err("refused");
        assert!(said.to_string().contains("blank line"), "got {said}");
    }

    #[test]
    fn a_task_knows_this_programs_own_path() {
        let task = Task::new("Nouveau", &["--new"]).expect("a task");
        assert_eq!(task.program, own_program().expect("a path"));
        assert_eq!(task.arguments, vec!["--new".to_string()]);
    }
}
