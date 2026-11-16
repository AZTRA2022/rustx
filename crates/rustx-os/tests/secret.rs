//! Un secret, rangé pour de vrai dans le Gestionnaire d'identification,
//! relu, remplacé, puis retiré.

#![cfg(windows)]

use rustx_os::secret;

#[test]
fn a_secret_is_stored_read_back_replaced_and_forgotten() {
    let service = format!("rustx-test-{}", std::process::id());
    let account = "someone@example.org";
    secret::forget(&service, account).expect("a clean slate");
    assert_eq!(secret::fetch(&service, account).expect("readable"), None);

    secret::store(&service, account, "première — clé").expect("stored");
    assert_eq!(
        secret::fetch(&service, account)
            .expect("readable")
            .as_deref(),
        Some("première — clé"),
        "UTF-8 comes back as it went in"
    );

    secret::store(&service, account, "second").expect("replaced");
    assert_eq!(
        secret::fetch(&service, account)
            .expect("readable")
            .as_deref(),
        Some("second")
    );

    secret::forget(&service, account).expect("forgotten");
    assert_eq!(secret::fetch(&service, account).expect("readable"), None);
    secret::forget(&service, account).expect("forgetting twice is fine");
}
