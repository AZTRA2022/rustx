//! What this machine says it has. Run it with `cargo run -p rustx-os
//! --example what_is_plugged_in`.

fn main() {
    rustx_os::devices::explain();
    println!("{:#?}", rustx_os::devices::devices());
}
