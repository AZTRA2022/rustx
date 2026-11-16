//! Asking the machine again, while it is busy.
//!
//! Several things a program asks the machine for are held by one
//! program at a time — the clipboard above all. Asked once, they fail a
//! few times a day for no reason the developer can reproduce; asked
//! again for a fifth of a second, they work. So the retry is written
//! once, here, with the two numbers handed in, rather than open-coded
//! at every call and forgotten at one of them.

use std::time::{Duration, Instant};

/// Try until it works or `patience` runs out, sleeping `between` tries.
///
/// The last refusal is what comes back, because the first one is rarely
/// the interesting one and a developer reading the error wants to know
/// what the machine said when it had had every chance.
///
/// The attempt is always made once, however small `patience` is: a
/// patience of zero means "do not wait", not "do not ask".
pub fn retrying<T, E>(
    mut attempt: impl FnMut() -> Result<T, E>,
    patience: Duration,
    between: Duration,
) -> Result<T, E> {
    let until = Instant::now() + patience;
    loop {
        match attempt() {
            Ok(it) => return Ok(it),
            Err(refusal) => {
                if Instant::now() + between > until {
                    return Err(refusal);
                }
            }
        }
        std::thread::sleep(between);
    }
}
