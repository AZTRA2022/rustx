//! Vault-Storage Secure — "Cryostasis", institutional-grade cold
//! storage.
//!
//! The front of the application is the product: a landing page, what
//! it does, how it is secured, what it costs, the documentation, and
//! a way to get in touch. Every one of those is a real route with
//! real content, and the one call to action — SECURE NOW — leads into
//! the thing the product actually is, at `/vault`.
//!
//! And the vault is real work, not a mock-up of one. A secret typed
//! into it is run through the cipher in this file and only ever
//! leaves the process as hex; the QR code beside it is
//! `react-qrcode-logo`, off npm, encoding that hex for real; the
//! passphrase can be put in the machine's own keychain through
//! `rustx::secret`; and the vault can be written to and read back
//! from a real file on disk.
//!
//! What is *not* real, and is named rather than hidden: the cipher is
//! a reversible XOR stream, chosen because it is honest about what it
//! is — ten readable lines — rather than a badly-built AES that would
//! invite trust it has not earned. The `/security` page says so in
//! the same words.
//!
//! The headline of the landing page is drawn letter by letter from
//! the bitmap font at the foot of this file, because the typeface the
//! design calls for is a 5×7 pixel face and there is no such font on
//! the machine. Thirty-five cells a letter, laid out by the page.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rustx::{App, Config};

/// One sealed secret.
#[derive(Clone)]
pub struct VaultItem {
    /// The label as an address holds it.
    pub slug: String,
    pub label: String,
    pub cipher_hex: String,
    /// What the clock said when it was sealed.
    pub at: String,
}

/// A page of the documentation.
#[derive(Clone)]
pub struct Doc {
    pub slug: String,
    pub title: String,
    pub blurb: String,
    pub body: Vec<(String, String)>,
}

/// A plan on the pricing page.
#[derive(Clone)]
pub struct Plan {
    pub name: String,
    pub price: String,
    pub per: String,
    pub about: String,
    pub lines: Vec<String>,
    pub headline: bool,
}

#[rustx::main]
fn main() -> Result<(), rustx::Error> {
    let app = App::new(
        Config::new("Cryostasis")
            .size(1180, 780)
            .description("Institutional-grade cold storage, and the vault behind it.")
            .publisher("rustx"),
    )?;

    app.run()
}

// ---- the cipher -------------------------------------------------------

/// A reversible XOR stream cipher, on purpose the simplest thing that
/// still round-trips text through a passphrase: this application's
/// point is to show the vault's shape, not to be a cryptography
/// library, and the security page says exactly that.
pub fn xor_cipher(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return data.to_vec();
    }
    data.iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key[index % key.len()])
        .collect()
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{:02x}", byte)).collect()
}

pub fn from_hex(text: &str) -> Vec<u8> {
    let text = text.trim();
    let mut out = Vec::new();
    let mut index = 0;
    while index + 2 <= text.len() {
        if let Ok(byte) = u8::from_str_radix(&text[index..index + 2], 16) {
            out.push(byte);
        }
        index += 2;
    }
    out
}

/// A short, stable fingerprint of a blob, for the readouts that want
/// one. Not a cryptographic hash and not called one.
pub fn fingerprint(text: &str) -> String {
    let mut value: u32 = 0x4f9a;
    for byte in text.bytes() {
        value = value.rotate_left(5) ^ byte as u32;
    }
    format!("0X{:08X}", value)
}

/// A label as an address holds it.
pub fn slugify(label: &str) -> String {
    let slug: String = label
        .to_lowercase()
        .chars()
        .map(|letter| {
            if letter.is_ascii_alphanumeric() {
                letter
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

/// The wall clock, as a readout prints it.
pub fn clock() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    let day = seconds % 86_400;
    format!("{:02}:{:02}:{:02}", day / 3600, (day % 3600) / 60, day % 60)
}

// ---- what the product says about itself -------------------------------

pub fn docs() -> Vec<Doc> {
    vec![
        Doc {
            slug: "quickstart".to_string(),
            title: "Quickstart".to_string(),
            blurb: "Seal your first secret in under a minute.".to_string(),
            body: vec![
                ("ONE — OPEN THE VAULT".to_string(), "SECURE NOW, on the landing page or in the navigation, opens the vault at /vault. Nothing is sent anywhere when you do: the vault is this process and the disk you point it at.".to_string()),
                ("TWO — CHOOSE A PASSPHRASE".to_string(), "The passphrase is the key. It is never written into a sealed blob and never stored with one. If you ask, it is put in the machine's own keychain through the operating system's credential store, under the service name 'Cryostasis'.".to_string()),
                ("THREE — SEAL".to_string(), "Give the secret a label and press SEAL. The vault shows the hex the cipher produced and a QR code of exactly that hex, which a phone can read back. The label becomes an address: /vault/<label> is that item's own proof-of-custody page.".to_string()),
                ("FOUR — KEEP IT".to_string(), "EXPORT writes the whole vault to a real file through the system's save dialog; IMPORT reads one back. The passphrase is not in the file, so a file on its own reveals nothing.".to_string()),
            ],
        },
        Doc {
            slug: "the-cipher".to_string(),
            title: "The cipher".to_string(),
            blurb: "What seals a secret here, exactly, and what it is not.".to_string(),
            body: vec![
                ("WHAT IT IS".to_string(), "A reversible XOR stream: every byte of the secret is XORed with the byte of the passphrase at the same position, the passphrase repeating. Ten lines of Rust, in src/main.rs, named xor_cipher.".to_string()),
                ("WHAT IT IS NOT".to_string(), "It is not AES, and this product does not claim it is. A repeating-key XOR falls to anyone who knows the length of the key and has enough ciphertext. It is here because an honest toy is worth more than a badly-built cipher with a famous name on it.".to_string()),
                ("WHY SHOW IT".to_string(), "Because the shape of a vault — sealing, labelling, proving custody, exporting, reading back — is the part a demonstration can get right, and the part a reader can check.".to_string()),
            ],
        },
        Doc {
            slug: "proof-of-custody".to_string(),
            title: "Proof of custody".to_string(),
            blurb: "The panel in the corner, line by line.".to_string(),
            body: vec![
                ("THE FINGERPRINT".to_string(), "0X… is a short rotating checksum of the sealed blob, computed by fingerprint() in src/main.rs. It changes when the blob changes and is stable when it does not. It is not a cryptographic hash and is not called one.".to_string()),
                ("THE QR".to_string(), "A real QR code, rendered by react-qrcode-logo off npm, encoding the hex of the sealed blob and nothing else. Read it with a phone and you get the ciphertext, which is useless without the passphrase — that is the point.".to_string()),
                ("THE READOUTS".to_string(), "Queue, retry, seed, cipher and drift are counted from the vault as it stands: how many items it holds, how many bytes they are, and what the last seal produced. OFFLINE STATUS is the plain truth about this machine — nothing here has a network behind it.".to_string()),
            ],
        },
        Doc {
            slug: "the-keychain".to_string(),
            title: "The keychain".to_string(),
            blurb: "Where a passphrase goes when you ask the machine to keep it.".to_string(),
            body: vec![
                ("WHAT IS CALLED".to_string(), "rustx::secret::store writes to the platform's own credential store — the Windows Credential Manager, the macOS keychain, the Secret Service on Linux — under the service name 'Cryostasis' and the account name 'passphrase'.".to_string()),
                ("READING IT BACK".to_string(), "rustx::secret::fetch returns it when the vault opens, so the passphrase survives closing the window without ever being written into the vault file.".to_string()),
                ("FORGETTING IT".to_string(), "FORGET calls rustx::secret::forget, which removes the entry from the credential store for real. The vault keeps working; it just stops remembering.".to_string()),
            ],
        },
    ]
}

pub fn doc_named(slug: &str) -> Option<Doc> {
    docs().into_iter().find(|doc| doc.slug == slug)
}

pub fn plans() -> Vec<Plan> {
    vec![
        Plan {
            name: "SINGLE NODE".to_string(),
            price: "FREE".to_string(),
            per: "one machine".to_string(),
            about: "The vault, the cipher and the proof panel, on the machine in front of you.".to_string(),
            lines: vec![
                "Unlimited sealed items".to_string(),
                "Real QR proof of custody".to_string(),
                "Export and import a vault file".to_string(),
                "Passphrase in the system keychain".to_string(),
            ],
            headline: false,
        },
        Plan {
            name: "CUSTODY".to_string(),
            price: "ON REQUEST".to_string(),
            per: "per vault, per year".to_string(),
            about: "What an institution would buy: several vaults, an audit trail, and someone to call.".to_string(),
            lines: vec![
                "Everything in SINGLE NODE".to_string(),
                "Multiple vaults, one console".to_string(),
                "Signed audit trail per seal".to_string(),
                "Named custody officer".to_string(),
                "Quarterly proof-of-reserve report".to_string(),
            ],
            headline: true,
        },
        Plan {
            name: "COLD SITE".to_string(),
            price: "ON REQUEST".to_string(),
            per: "per site".to_string(),
            about: "Hardware in a room with a door, and the paperwork that comes with it.".to_string(),
            lines: vec![
                "Everything in CUSTODY".to_string(),
                "Air-gapped seal ceremony".to_string(),
                "Geographic redundancy".to_string(),
                "Insurance schedule".to_string(),
            ],
            headline: false,
        },
    ]
}

// ---- the 5×7 face the headline is drawn in ----------------------------

/// Seven rows of five cells, `#` where the pixel is lit.
///
/// Written out rather than loaded, because the typeface the design
/// calls for is a bitmap face and there is none on this machine — and
/// a headline set in Arial would be a different design.
pub fn glyph(letter: char) -> [&'static str; 7] {
    match letter.to_ascii_uppercase() {
        'A' => [
            ".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'B' => [
            "####.", "#...#", "#...#", "####.", "#...#", "#...#", "####.",
        ],
        'C' => [
            ".####", "#....", "#....", "#....", "#....", "#....", ".####",
        ],
        'D' => [
            "####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####.",
        ],
        'E' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#####",
        ],
        'F' => [
            "#####", "#....", "#....", "####.", "#....", "#....", "#....",
        ],
        'G' => [
            ".###.", "#...#", "#....", "#..##", "#...#", "#...#", ".###.",
        ],
        'H' => [
            "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#",
        ],
        'I' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####",
        ],
        'J' => [
            "..###", "...#.", "...#.", "...#.", "...#.", "#..#.", ".##..",
        ],
        'K' => [
            "#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#",
        ],
        'L' => [
            "#....", "#....", "#....", "#....", "#....", "#....", "#####",
        ],
        'M' => [
            "#...#", "##.##", "#.#.#", "#...#", "#...#", "#...#", "#...#",
        ],
        'N' => [
            "#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#",
        ],
        'O' => [
            ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'P' => [
            "####.", "#...#", "#...#", "####.", "#....", "#....", "#....",
        ],
        'Q' => [
            ".###.", "#...#", "#...#", "#...#", "#.#.#", "#..#.", ".##.#",
        ],
        'R' => [
            "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
        ],
        'S' => [
            ".####", "#....", "#....", ".###.", "....#", "....#", "####.",
        ],
        'T' => [
            "#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#..",
        ],
        'U' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
        ],
        'V' => [
            "#...#", "#...#", "#...#", "#...#", "#...#", ".#.#.", "..#..",
        ],
        'W' => [
            "#...#", "#...#", "#...#", "#...#", "#.#.#", "##.##", "#...#",
        ],
        'X' => [
            "#...#", "#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#",
        ],
        'Y' => [
            "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
        ],
        'Z' => [
            "#####", "....#", "...#.", "..#..", ".#...", "#....", "#####",
        ],
        '0' => [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
        '1' => [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
        '2' => [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
        '3' => [
            "####.", "....#", "....#", ".###.", "....#", "....#", "####.",
        ],
        '4' => [
            "#...#", "#...#", "#...#", "#####", "....#", "....#", "....#",
        ],
        '5' => [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
        '6' => [
            ".###.", "#....", "#....", "####.", "#...#", "#...#", ".###.",
        ],
        '7' => [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
        '8' => [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
        '9' => [
            ".###.", "#...#", "#...#", ".####", "....#", "....#", ".###.",
        ],
        ',' => [
            ".....", ".....", ".....", ".....", "..##.", "..#..", ".#...",
        ],
        '.' => [
            ".....", ".....", ".....", ".....", ".....", ".##..", ".##..",
        ],
        '!' => [
            "..#..", "..#..", "..#..", "..#..", "..#..", ".....", "..#..",
        ],
        '-' => [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
        _ => [
            ".....", ".....", ".....", ".....", ".....", ".....", ".....",
        ],
    }
}

/// Every lit cell of a word, as (column, row) pairs in the word's own
/// grid — six columns a letter, so the letters do not touch.
pub fn lit_cells(word: &str) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for (at, letter) in word.chars().enumerate() {
        let rows = glyph(letter);
        for (y, row) in rows.iter().enumerate() {
            for (x, cell) in row.chars().enumerate() {
                if cell == '#' {
                    cells.push((at * 6 + x, y));
                }
            }
        }
    }
    cells
}

/// How many cells wide a word is, at six columns a letter.
pub fn word_width(word: &str) -> usize {
    word.chars().count() * 6
}
