//! The frame around every LSP message: a `Content-Length`, a blank
//! line, and that many bytes of JSON.
//!
//! The reading is where a server goes wrong. A pipe hands over whatever
//! it has: a header can arrive in two pieces, a body can be waiting
//! before the reader asks, and `Content-Length` counts **bytes** while a
//! `.rsx` is full of accented text. A server that reads a line and hopes
//! desynchronises on the first fast typist, and then every message after
//! it is garbage.

use std::io::{BufRead, Write};

/// One message's body, or `None` when the stream is over.
pub fn read(input: &mut impl BufRead) -> Option<String> {
    let mut length: Option<usize> = None;

    // The header, line by line, until the blank one.
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(said) = line
            .split_once(':')
            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .map(|(_, value)| value.trim())
        {
            length = said.parse().ok();
        }
    }

    // And the body, to the byte.
    let length = length?;
    let mut body = vec![0u8; length];
    input.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

/// Send one message's body, with the header a client expects.
pub fn write(output: &mut impl Write, body: &str) {
    let _ = output.write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    let _ = output.write_all(body.as_bytes());
    let _ = output.flush();
}
