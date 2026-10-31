//! Du processus de la fenêtre à celui de la page.
//!
//! Une poignée de choses n'existent que dans le processus navigateur,
//! parce qu'elles *sont* la fenêtre : ce qu'on lâche dessus, par
//! exemple. Tout ce qu'un développeur écrit vit dans le processus de
//! rendu. Entre les deux, un message de Chromium — et c'est lui qui est
//! épinglé ici, entièrement, parce qu'une liste de valeurs et un
//! message se construisent sans que Chromium soit démarré.

use rustx_engine::Value;
use rustx_engine::bridge;

/// libcef lit la version de son API avant d'accepter la moindre
/// structure : sans cela, construire un message fait abandonner le
/// processus avec « called with invalid version -1 ». Dit une fois,
/// dans chaque test, parce qu'ils ne tournent pas dans un ordre connu.
fn speaking_cef() {
    rustx_engine::declare_api_version();
}

#[test]
fn the_values_cross_as_themselves() {
    speaking_cef();
    // La traduction est écrite deux fois — une par direction — et deux
    // écritures se désaccordent.
    let crossing = vec![
        Value::Null,
        Value::Bool(true),
        Value::Number(42.5),
        Value::String("C:/Users/aztra/Images/éléphant.png".to_string()),
    ];
    let list = bridge::written(&crossing).expect("une liste de Chromium");
    assert_eq!(bridge::read(&list), crossing);
}

#[test]
fn what_was_told_reaches_the_one_who_listens() {
    speaking_cef();
    // Bout à bout, sans les deux processus : le message est construit
    // comme le côté fenêtre le construit, et remis au côté page comme
    // Chromium le remet.
    let heard = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let keep = heard.clone();
    bridge::listen("test.un", move |values| keep.borrow_mut().push(values));

    let message = bridge::saying("test.un", &[Value::String("é".to_string())])
        .expect("un message de Chromium");
    assert!(bridge::told(&message), "le message était pour nous");
    assert_eq!(
        *heard.borrow(),
        vec![vec![Value::String("é".to_string())]],
        "ce qui a été dit n'est pas arrivé"
    );
}

#[test]
fn a_message_nobody_listens_to_is_not_ours_to_swallow() {
    speaking_cef();
    // Chromium lit la réponse comme « traité ». Un message qu'on dit
    // avoir traité et qu'on a jeté est un message perdu pour celui qui
    // l'attendait peut-être ailleurs.
    let message = bridge::saying("test.personne.n.ecoute", &[]).expect("un message");
    assert!(!bridge::told(&message));

    // Et un message qui n'est pas du pont ne l'est pas non plus.
    let other = bridge::message_named("une.autre.chose", &[]).expect("un message");
    assert!(!bridge::told(&other));
}
