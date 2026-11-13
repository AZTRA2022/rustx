//! Vault-Storage Secure — the showcase's "Hyper Secret data storage".
//!
//! A real cipher (AES, as the marketing copy implies) and a real
//! threat model are weeks of work and easy to get wrong; building that
//! badly and calling it secure would be worse than not building it.
//! What this app shows honestly: a small vault of labelled secrets,
//! each one run through a simple, clearly-labelled reversible cipher,
//! and a *real* QR code — `react-qrcode-logo`, off npm, not drawn by
//! hand — of whatever the vault currently holds.

use rustx::{App, Config};

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Vault-Storage Secure")
            .size(820, 640)
            .description("A small vault, a toy cipher, and a real QR code.")
            .publisher("rustx"),
    )?;

    app.run()
}
