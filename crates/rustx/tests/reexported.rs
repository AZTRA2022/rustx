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

#[test]
fn the_tokio_a_developer_knows_is_all_there() {
    // "Use async as you would anywhere else." A re-export that carries
    // a hand-picked third of tokio is not that: `fs`, `io`, `process`
    // and `net` are what ordinary async Rust reaches for, and an
    // application that asked for them got "could not find `fs` in
    // `tokio`" about a crate it never named.
    let runtime = rustx::tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");

    let path = std::env::temp_dir().join(format!("rustx-reexport-{}.txt", std::process::id()));
    let read_back = runtime.block_on(async {
        rustx::tokio::fs::write(&path, b"ecrit sans bloquer").await?;
        rustx::tokio::fs::read_to_string(&path).await
    });
    let _ = std::fs::remove_file(&path);

    assert_eq!(read_back.expect("the file round-tripped"), "ecrit sans bloquer");

    // And the pieces an application composes with, named so this file
    // stops compiling if they ever go away again.
    fn exists<T>() {}
    exists::<rustx::tokio::fs::File>();
    exists::<rustx::tokio::net::TcpListener>();
    exists::<rustx::tokio::sync::Semaphore>();
    exists::<rustx::tokio::process::Command>();
    fn reads_lines<T: rustx::tokio::io::AsyncBufReadExt>() {}
    reads_lines::<rustx::tokio::io::BufReader<rustx::tokio::fs::File>>();

    // `spawn`, run for real: a task on the runtime, awaited.
    let spawned = runtime.block_on(async {
        rustx::tokio::spawn(async { 21 * 2 })
            .await
            .expect("the task finished")
    });
    assert_eq!(spawned, 42);
}
