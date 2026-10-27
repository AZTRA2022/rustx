use rustx_engine::{App, AppConfig, EngineError};

fn config_with(index: &str) -> AppConfig {
    AppConfig {
        title: "t".into(),
        width: 800,
        height: 600,
        index: index.into(),
    }
}

#[test]
fn app_rejects_a_missing_index() {
    let config = config_with("does-not-exist.html");
    assert!(matches!(App::new(config), Err(EngineError::IndexNotFound(_))));
}

#[test]
fn app_rejects_a_remote_url() {
    let config = config_with("https://example.com");
    assert!(matches!(
        App::new(config),
        Err(EngineError::RemoteContentForbidden)
    ));
}

#[test]
fn the_index_becomes_an_absolute_file_url() {
    let config = config_with("assets/hello/index.html");
    let app = App::new(config).expect("the example index should exist");

    let url = app.index_url().expect("a readable path yields a url");

    assert!(url.starts_with("file:///"), "got {url}");
    assert!(url.ends_with("/index.html"), "got {url}");
    assert!(
        !url.contains('\\'),
        "backslashes are not valid in a url: {url}"
    );
}

fn config_titled(title: &str) -> AppConfig {
    AppConfig {
        title: title.into(),
        ..config_with("assets/hello/index.html")
    }
}

#[test]
fn each_application_gets_its_own_cache_directory() {
    let alpha = App::new(config_titled("Alpha Recorder")).expect("index exists");
    let beta = App::new(config_titled("Beta Recorder")).expect("index exists");

    let (a, b) = (alpha.cache_dir(), beta.cache_dir());

    assert!(a.is_absolute(), "got {}", a.display());
    assert_ne!(
        a, b,
        "two applications sharing a cache directory share Chromium's process singleton"
    );
    assert!(
        a.to_string_lossy().to_ascii_lowercase().contains("alpha"),
        "the directory should name its application: {}",
        a.display()
    );
}
