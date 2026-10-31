//! Du processus de la fenêtre à celui de la page.
//!
//! Tout ce qu'un développeur écrit vit dans le processus de rendu :
//! l'arbre, l'état, les gestionnaires d'événements. C'est ce qui fait
//! qu'un `onClick` est un appel et non un message — même processus,
//! même thread, même pile.
//!
//! Mais une poignée de choses n'existent que dans le processus
//! navigateur, parce qu'elles *sont* la fenêtre. Ce qu'on lâche dessus
//! en est une : seul le gestionnaire natif de Chromium connaît le
//! chemin réel d'un fichier déposé, et il vit là-bas.
//!
//! D'où ce passage. Un nom, des valeurs, un message de Chromium, et un
//! crochet qui l'attend de l'autre côté :
//!
//! ```ignore
//! // côté fenêtre
//! bridge::tell(frame, "rustx.dropped", &valeurs);
//!
//! // côté page, une fois au démarrage
//! bridge::listen("rustx.dropped", |valeurs| { … });
//! ```
//!
//! # Une seule direction, pour l'instant
//!
//! De la fenêtre vers la page, sans réponse. L'autre direction — une
//! question posée depuis la page, une réponse qui revient sous le même
//! numéro — n'a qu'un seul usage à venir, une deuxième fenêtre, et
//! s'écrira avec elle. Un registre de questions en attente que
//! personne ne pose serait du code qu'on garde sans le mesurer.

use std::cell::RefCell;
use std::collections::HashMap;

use cef::{
    CefString, Frame, ImplFrame, ImplListValue, ImplProcessMessage, ListValue, ProcessId,
    ProcessMessage,
};

use crate::value::Value;

/// Ce que porte un message du pont, avant le nom de ce qui est dit.
///
/// Préfixé, pour que `told` reconnaisse ce qui lui appartient sans
/// avoir à connaître tous les noms que le moteur se dit à lui-même.
pub const PREFIX: &str = "rustx.";

thread_local! {
    /// Ce que le processus de rendu écoute, par nom.
    static LISTENING: RefCell<HashMap<String, Vec<Box<dyn Fn(Vec<Value>)>>>> =
        RefCell::new(HashMap::new());
}

/// Dire quelque chose à la page. Appelé dans le processus navigateur.
pub(crate) fn tell(frame: &Frame, what: &str, values: &[Value]) {
    let Some(mut message) = saying(what, values) else {
        return;
    };
    frame.send_process_message(ProcessId::RENDERER, Some(&mut message));
}

/// Écouter ce nom. Appelé dans le processus de rendu.
///
/// Plusieurs crochets sur un même nom, dans l'ordre où ils ont été
/// posés : une page peut vouloir savoir ce qu'une autre partie de
/// l'application sait déjà.
pub fn listen(what: &str, hook: impl Fn(Vec<Value>) + 'static) {
    LISTENING.with(|listening| {
        listening
            .borrow_mut()
            .entry(plain(what).to_string())
            .or_default()
            .push(Box::new(hook));
    });
}

/// Un message arrivé dans le processus de rendu.
///
/// Publique pour que le test puisse la remettre à la place de
/// Chromium, qui est la seule façon de prouver le passage sans deux
/// vrais processus.
///
/// Rend `true` quand quelqu'un l'écoutait, ce que Chromium lit comme
/// « traité ». Un message que personne n'écoute n'est pas à nous, et le
/// dire traité serait le perdre pour celui qui l'attendait ailleurs.
pub fn told(message: &ProcessMessage) -> bool {
    let wire = CefString::from(&message.name()).to_string();
    if !wire.starts_with(PREFIX) {
        return false;
    }
    // Les crochets sont posés sous le nom qu'un développeur écrit ; le
    // préfixe n'appartient qu'au fil.
    let name = plain(&wire).to_string();
    let values = message
        .argument_list()
        .map(|list| read(&list))
        .unwrap_or_default();

    // Les crochets sont sortis de la table le temps de l'appel : l'un
    // d'eux a le droit d'en poser un autre sans bloquer sur l'emprunt.
    let hooks = LISTENING.with(|listening| listening.borrow_mut().remove(&name));
    let Some(hooks) = hooks else {
        return false;
    };
    for hook in &hooks {
        hook(values.clone());
    }
    LISTENING.with(|listening| {
        let mut listening = listening.borrow_mut();
        // Ce qu'un crochet a posé pendant l'appel vient après.
        let posted = listening.remove(&name).unwrap_or_default();
        let mut all = hooks;
        all.extend(posted);
        listening.insert(name, all);
    });
    true
}

/// Le message qui porte ce que la fenêtre dit.
///
/// Publique parce que le test du pont le construit : c'est la seule
/// façon de prouver le passage sans deux vrais processus.
pub fn saying(what: &str, values: &[Value]) -> Option<ProcessMessage> {
    message_named(&format!("{PREFIX}{}", what.trim_start_matches(PREFIX)), values)
}

/// Un message, sous le nom exact qu'on lui donne.
pub fn message_named(name: &str, values: &[Value]) -> Option<ProcessMessage> {
    let message = cef::process_message_create(Some(&CefString::from(name)))?;
    let list = message.argument_list()?;
    list.set_size(values.len());
    for (at, value) in values.iter().enumerate() {
        set(&list, at, value);
    }
    Some(message)
}

/// Les valeurs, écrites dans une liste de Chromium.
///
/// Créer une liste ne demande pas que Chromium soit démarré — c'est une
/// structure de la bibliothèque, pas du navigateur — donc l'aller-retour
/// se vérifie dans un test ordinaire.
pub fn written(values: &[Value]) -> Option<ListValue> {
    let list = cef::list_value_create()?;
    list.set_size(values.len());
    for (at, value) in values.iter().enumerate() {
        set(&list, at, value);
    }
    Some(list)
}

/// Et relues.
pub fn read(list: &ListValue) -> Vec<Value> {
    (0..list.size()).map(|at| get(list, at)).collect()
}

/// Une valeur, posée dans la liste.
fn set(list: &ListValue, at: usize, value: &Value) {
    match value {
        Value::Null => {
            list.set_null(at);
        }
        Value::Bool(yes) => {
            list.set_bool(at, i32::from(*yes));
        }
        Value::Number(number) => {
            list.set_double(at, *number);
        }
        Value::String(text) => {
            list.set_string(at, Some(&CefString::from(text.as_str())));
        }
        // Une liste ou un enregistrement ne traversent pas : ce que le
        // pont porte est ce qu'une capacité de la fenêtre a à dire —
        // des chemins, des nombres, des drapeaux. Écrits comme du texte
        // plutôt que jetés en silence.
        other => {
            list.set_string(at, Some(&CefString::from(format!("{other:?}").as_str())));
        }
    };
}

/// Et relue.
fn get(list: &ListValue, at: usize) -> Value {
    use cef::ValueType;
    match list.get_type(at) {
        ValueType::BOOL => Value::Bool(list.bool(at) != 0),
        ValueType::INT => Value::Number(f64::from(list.int(at))),
        ValueType::DOUBLE => Value::Number(list.double(at)),
        ValueType::STRING => Value::String(CefString::from(&list.string(at)).to_string()),
        _ => Value::Null,
    }
}

/// Le nom sans le préfixe du pont.
///
/// Un crochet est posé sous le nom qu'un développeur écrit —
/// `"dropped"` — et le message voyage sous `"rustx.dropped"` : le
/// préfixe sert à reconnaître ce qui nous appartient dans la file de
/// Chromium, et n'a rien à faire dans ce qu'on écrit.
fn plain(name: &str) -> &str {
    name.strip_prefix(PREFIX).unwrap_or(name)
}
