//! Who may load in the window, who may talk to the host, what is served.

use rustx_webview::assets::serve;
use rustx_webview::policy::{Navigation, Policy, origin_of};

const APP: &str = "http://rustx.localhost";

fn policy() -> Policy {
    Policy::new(APP, &["https://docs.example.test"])
}

#[test]
fn the_applications_own_pages_and_the_allow_list_stay_in_the_window() {
    let policy = policy();
    assert_eq!(
        policy.navigation("http://rustx.localhost/page/2?x=1#top"),
        Navigation::Stay
    );
    assert_eq!(
        policy.navigation("HTTP://RUSTX.LOCALHOST/"),
        Navigation::Stay
    );
    assert_eq!(
        policy.navigation("https://docs.example.test/guide"),
        Navigation::Stay
    );
    assert_eq!(policy.navigation("about:blank"), Navigation::Stay);
}

#[test]
fn any_other_web_address_opens_in_the_system_browser() {
    let policy = policy();
    assert_eq!(
        policy.navigation("https://example.org/"),
        Navigation::OpenExternally
    );
    assert_eq!(
        policy.navigation("http://example.org:8080/a"),
        Navigation::OpenExternally
    );
    // The same host on another scheme or port is another origin.
    assert_eq!(
        policy.navigation("http://docs.example.test/"),
        Navigation::OpenExternally
    );
    assert_eq!(
        policy.navigation("https://docs.example.test:8443/"),
        Navigation::OpenExternally
    );
}

#[test]
fn what_is_not_a_web_address_is_refused_and_named() {
    let policy = policy();
    for url in [
        "file:///C:/Windows/win.ini",
        "javascript:alert(1)",
        "data:text/html,<b>x</b>",
        "ms-msdt:/id PCWDiagnostic",
        "not a url",
        "",
    ] {
        assert!(
            matches!(policy.navigation(url), Navigation::Refuse(_)),
            "{url:?} should be refused"
        );
    }
    let Navigation::Refuse(why) = policy.navigation("file:///C:/x") else {
        panic!("a file is refused");
    };
    assert!(
        why.contains("file://"),
        "the refusal names the origin: {why}"
    );
}

#[test]
fn an_address_that_only_looks_like_the_application_is_not_it() {
    let policy = policy();
    for url in [
        // The host is what follows the `@`.
        "http://rustx.localhost@evil.test/",
        "http://rustx.localhost:80@evil.test/",
        "http://rustx.localhost.evil.test/",
        "http://evilrustx.localhost/",
    ] {
        assert_eq!(policy.navigation(url), Navigation::OpenExternally, "{url}");
        assert!(!policy.ipc_allowed(url), "{url} must not reach the host");
    }
}

#[test]
fn a_backslash_ends_the_host_the_way_a_browser_ends_it() {
    // For http a browser reads `\` as `/`: this is a page of
    // rustx.localhost whose path is `/@evil.test/`, so it is the
    // application's, and the parser must agree with the browser rather
    // than with a reading that makes it look hostile.
    let url = "http://rustx.localhost\\@evil.test/";
    assert_eq!(policy().navigation(url), Navigation::Stay);
    assert!(policy().ipc_allowed(url));
    // And the reverse: the host before the backslash is the host.
    assert_eq!(
        policy().navigation("http://evil.test\\@rustx.localhost/"),
        Navigation::OpenExternally
    );
}

#[test]
fn only_the_applications_own_page_may_call_the_host() {
    let policy = policy();
    assert!(policy.ipc_allowed("http://rustx.localhost/index.html"));
    // Shown in the window, and still not trusted.
    assert!(!policy.ipc_allowed("https://docs.example.test/"));
    assert!(!policy.ipc_allowed("https://example.org/"));
    assert!(!policy.ipc_allowed("about:blank"));
    assert!(!policy.ipc_allowed(""));
}

#[test]
fn an_origin_is_scheme_host_and_port_in_lower_case() {
    assert_eq!(
        origin_of("HTTPS://Example.TEST:8443/a?b#c").as_deref(),
        Some("https://example.test:8443")
    );
    assert_eq!(
        origin_of("rustx://localhost/x").as_deref(),
        Some("rustx://localhost")
    );
    assert_eq!(origin_of("javascript:alert(1)"), None);
    assert_eq!(origin_of("://nothing"), None);
    assert_eq!(origin_of("http:///no-host"), None);
}

fn folder() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rustx-assets-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("app/deep")).expect("a folder");
    std::fs::write(dir.join("app/index.html"), "<p>home</p>").expect("a page");
    std::fs::write(dir.join("app/deep/a b.css"), "p{}").expect("a style");
    std::fs::write(dir.join("secret.txt"), "outside").expect("a secret");
    dir
}

#[test]
fn files_are_served_from_the_folder_with_their_type() {
    let dir = folder();
    let root = dir.join("app");
    let home = serve(&root, "/");
    assert_eq!((home.status, home.mime), (200, "text/html; charset=utf-8"));
    assert_eq!(home.body, b"<p>home</p>");
    assert_eq!(serve(&root, "/index.html?v=2#x").status, 200);
    let style = serve(&root, "/deep/a%20b.css");
    assert_eq!((style.status, style.mime), (200, "text/css; charset=utf-8"));
    assert_eq!(serve(&root, "/missing.js").status, 404);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn nothing_outside_the_folder_is_served() {
    let dir = folder();
    let root = dir.join("app");
    for path in [
        "/../secret.txt",
        "/%2e%2e/secret.txt",
        "/deep/../../secret.txt",
        "/..%5Csecret.txt",
        "/C:/Windows/win.ini",
        "//..//secret.txt",
        "/index.html%00.png",
    ] {
        let answer = serve(&root, path);
        assert_ne!(answer.status, 200, "{path} must not be served");
        assert_ne!(answer.body, b"outside", "{path} leaked the secret");
    }
    let _ = std::fs::remove_dir_all(dir);
}
