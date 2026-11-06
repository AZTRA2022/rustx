//! What the facade says an application may write.
//!
//! The README promises `rustx::time::sleep`, and for the rest of the
//! runtime `rustx::tokio` — naming `select!` and a channel. A promise
//! that does not compile is a bug, and tokio puts both of those behind
//! cargo features that `rt-multi-thread` does not imply.

#[test]
fn the_runtime_the_readme_names_is_reachable() {
    // `rustx::time`, which is what a page awaits on.
    let _sleep = rustx::time::sleep;

    // And the rest of it, named in the README: a channel, a lock, and
    // `select!`. Referencing them is the whole test — this file does not
    // compile if the features are off.
    let (sender, mut receiver) = rustx::tokio::sync::mpsc::channel::<u8>(1);
    let _lock = rustx::tokio::sync::Mutex::new(0_u8);

    let runtime = rustx::tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("a runtime");
    let got = runtime.block_on(async move {
        sender.send(7).await.expect("the channel is open");
        rustx::tokio::select! {
            value = receiver.recv() => value,
            _ = rustx::time::sleep(std::time::Duration::from_secs(5)) => None,
        }
    });

    assert_eq!(got, Some(7), "select! did not take the value the channel had");
}
