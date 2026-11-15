//! SHA-256, by hand.
//!
//! Why it is here rather than taken from a crate: two different
//! programs have to agree on it. `rustx package` writes the digest of
//! every file it ships into `rustx-release.json`, and the updater
//! inside a running application checks a download against the digest it
//! was promised. One of those is a command line and the other is a
//! window; a dependency that only one of them had would be a dependency
//! that proves nothing.
//!
//! It is FIPS 180-4 and nothing else: no streaming API, no HMAC, no
//! other function. It is used on files, and a file is a `Vec<u8>` by
//! the time either of those two programs has it.

/// The digest of these bytes, as the thirty-two bytes it is.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // The message, the single `1` bit after it, zeroes, and its length
    // in bits as a big-endian u64 -- padded so the whole thing is a
    // whole number of 64-byte blocks.
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((bytes.len() as u64) * 8).to_be_bytes());

    let mut words = [0_u32; 64];
    for block in padded.as_chunks::<64>().0 {
        for (at, four) in block.as_chunks::<4>().0.iter().enumerate() {
            words[at] = u32::from_be_bytes(*four);
        }
        for at in 16..64 {
            let fifteen = words[at - 15];
            let two = words[at - 2];
            let s0 = fifteen.rotate_right(7) ^ fifteen.rotate_right(18) ^ (fifteen >> 3);
            let s1 = two.rotate_right(17) ^ two.rotate_right(19) ^ (two >> 10);
            words[at] = words[at - 16]
                .wrapping_add(s0)
                .wrapping_add(words[at - 7])
                .wrapping_add(s1);
        }

        let mut held = state;
        for at in 0..64 {
            let s1 = held[4].rotate_right(6) ^ held[4].rotate_right(11) ^ held[4].rotate_right(25);
            let choose = (held[4] & held[5]) ^ ((!held[4]) & held[6]);
            let first = held[7]
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(ROUND[at])
                .wrapping_add(words[at]);
            let s0 = held[0].rotate_right(2) ^ held[0].rotate_right(13) ^ held[0].rotate_right(22);
            let most = (held[0] & held[1]) ^ (held[0] & held[2]) ^ (held[1] & held[2]);
            let second = s0.wrapping_add(most);

            held[7] = held[6];
            held[6] = held[5];
            held[5] = held[4];
            held[4] = held[3].wrapping_add(first);
            held[3] = held[2];
            held[2] = held[1];
            held[1] = held[0];
            held[0] = first.wrapping_add(second);
        }
        for (at, piece) in held.iter().enumerate() {
            state[at] = state[at].wrapping_add(*piece);
        }
    }

    let mut out = [0_u8; 32];
    for (at, word) in state.iter().enumerate() {
        out[at * 4..at * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The same digest, written the way a manifest and a release page write
/// it: sixty-four lowercase hexadecimal characters.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(64);
    for byte in sha256(bytes) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// The round constants: the first thirty-two bits of the fractional
/// parts of the cube roots of the first sixty-four primes.
const ROUND: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The three vectors every implementation is checked against, and
    /// the reason this file can be trusted at all: they are published,
    /// so they are not this implementation agreeing with itself.
    #[test]
    fn the_published_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    /// A message exactly one bit short of needing a second block, and
    /// one exactly over it. The padding rule is where a hand-written
    /// SHA-256 goes wrong, and it goes wrong only at the boundary.
    #[test]
    fn the_padding_boundary() {
        assert_eq!(
            sha256_hex(&[b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn a_million_a_s() {
        // The fourth published vector, and the only one that exercises
        // more blocks than a page of text.
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn one_changed_byte_changes_the_digest() {
        assert_ne!(sha256_hex(b"release-1.0.0"), sha256_hex(b"release-1.0.1"));
    }
}
