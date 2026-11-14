//! The frame around every LSP message: a `Content-Length`, a blank
//! line, and that many bytes of JSON.
//!
//! The reading is where a server goes wrong. A pipe hands over whatever
//! it has: a header can arrive in two pieces, a body can be waiting
//! before the reader asks, and `Content-Length` counts **bytes** while a
//! `.rsx` is full of accented text. A server that reads a line and hopes
//! desynchronises on the first fast typist, and then every message after
//! it is garbage.
//!
//! Three things beyond that, each of which was a way to lose a session:
//!
//! **A bad frame is not the end of the stream.** The reader used to
//! answer `None` to a missing `Content-Length`, a length that was not a
//! number, or a body that was not UTF-8 — and `None` is what the end of
//! the stream says, so the server stopped. One malformed message and
//! every underline in the window was gone until the editor was
//! reloaded. The two are told apart now, and only one of them ends
//! anything.
//!
//! **A length is not a promise.** `Content-Length: 99999999999` used to
//! allocate that many bytes before reading one of them, which is an
//! abort rather than a failure: no `catch_unwind` recovers from an
//! allocation the operating system refused.
//!
//! **What is already in hand is worth knowing about.** Somebody typing
//! quickly puts twenty `didChange` notifications into the pipe while the
//! first is still being compiled. Reading them into a buffer of our own
//! — rather than one syscall at a time inside the standard library's —
//! is what lets the server see that the first nineteen have already been
//! replaced.

use std::io::{BufRead, Write};

/// The most a single message may claim to be.
///
/// Sixty-four megabytes, which is two orders of magnitude more than the
/// largest `.rsx` anybody has written and still small enough that a
/// nonsense header is refused rather than allocated.
pub const LONGEST: usize = 64 * 1024 * 1024;

/// What came off the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// One message's body.
    Body(String),
    /// Something that was not a message. The stream is still open and
    /// the next frame may be perfectly good, so a server reads on.
    Broken(&'static str),
    /// The editor closed the pipe. This one, and only this one, is the
    /// end of a session.
    Over,
}

/// The frames of a stream, over a buffer of this module's own.
///
/// The buffer is the point: it is what makes "is there another message
/// already waiting?" a question that can be answered without blocking,
/// and blocking is the one thing a reader must not do when the answer
/// decides whether to compile a page nobody is looking at any more.
pub struct Frames<'a, R: BufRead + ?Sized> {
    input: &'a mut R,
    held: Vec<u8>,
    /// How much of `held` has been handed out already.
    from: usize,
    /// The header of the frame at `from`, once it has been read.
    ///
    /// Cached because a long message arrives in many pieces, and
    /// searching the whole buffer for the blank line again on each of
    /// them makes receiving one document quadratic in its size.
    head: Option<Head>,
}

/// What the header block of the frame at the front says.
#[derive(Debug, Clone, Copy)]
struct Head {
    /// Where the body begins, counted from `from`.
    body_at: usize,
    /// How long the body is, when the header said so.
    length: Option<usize>,
    /// Why this frame is not a frame, when it is not one.
    broken: Option<&'static str>,
}

impl<'a, R: BufRead + ?Sized> Frames<'a, R> {
    pub fn over(input: &'a mut R) -> Self {
        Self {
            input,
            held: Vec::new(),
            from: 0,
            head: None,
        }
    }

    /// The next frame, reading from the stream when the buffer has no
    /// whole one.
    ///
    /// Named `next` and not an `Iterator`: the end of this stream is a
    /// `Frame::Over` the server has to answer (it shuts down, and takes
    /// `rust-analyzer` with it), not a `None` that a `for` loop would
    /// swallow. An iterator would hide exactly the case that matters.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Frame {
        loop {
            match self.take() {
                Taken::Frame(frame) => return frame,
                Taken::Nothing => {
                    if !self.fill() {
                        // The pipe closed. Anything left in the buffer is
                        // half a message, which is not a message.
                        return Frame::Over;
                    }
                }
            }
        }
    }

    /// The next readable message, skipping anything that was not one.
    ///
    /// `None` only at the end of the stream, which is the distinction a
    /// server's loop turns on: a broken frame must not be allowed to
    /// look like a closed pipe.
    pub fn message(&mut self) -> Option<String> {
        loop {
            match self.next() {
                Frame::Body(body) => return Some(body),
                Frame::Broken(why) => {
                    // Said out loud: a client whose frames this server
                    // cannot read has no other way to find out.
                    eprintln!("rustx-lsp: a message was skipped ({why}).");
                    continue;
                }
                Frame::Over => return None,
            }
        }
    }

    /// Whether a whole frame is already in the buffer — no syscall, no
    /// waiting, no reading.
    pub fn waiting(&mut self) -> bool {
        matches!(self.look(), Taken::Frame(_))
    }

    /// Read more bytes, `false` when there are no more to be had.
    fn fill(&mut self) -> bool {
        // What has been handed out is dropped first, so a long session
        // does not keep every message it ever read.
        if self.from > 0 {
            self.held.drain(..self.from);
            self.from = 0;
        }
        let mut chunk = [0u8; 8192];
        match self.input.read(&mut chunk) {
            Ok(0) | Err(_) => false,
            Ok(read) => {
                self.held.extend_from_slice(&chunk[..read]);
                true
            }
        }
    }

    /// Take a frame out of the buffer, if there is a whole one.
    fn take(&mut self) -> Taken {
        let Some((past, frame)) = self.measured() else {
            return Taken::Nothing;
        };
        self.from += past;
        self.head = None;
        Taken::Frame(frame)
    }

    /// The same, without consuming anything.
    fn look(&mut self) -> Taken {
        match self.measured() {
            Some((_, frame)) => Taken::Frame(frame),
            None => Taken::Nothing,
        }
    }

    /// The header block of the frame at the front, read once.
    fn head(&mut self) -> Option<Head> {
        if let Some(head) = self.head {
            return Some(head);
        }
        let rest = &self.held[self.from..];
        // Whichever blank line comes first. The specification says
        // `\r\n\r\n`; more than one small client sends `\n\n`, and a
        // body is allowed to hold either.
        let crlf = find(rest, b"\r\n\r\n");
        let bare = find(rest, b"\n\n");
        let (end, separator) = match (crlf, bare) {
            (Some(crlf), Some(bare)) if bare < crlf => (bare, 2),
            (Some(crlf), _) => (crlf, 4),
            (None, Some(bare)) => (bare, 2),
            (None, None) => return None,
        };
        let length = std::str::from_utf8(&rest[..end]).ok().and_then(|head| {
            head.split(['\r', '\n'])
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        });
        let head = Head {
            body_at: end + separator,
            length,
            broken: match length {
                // No length, so where the body ends is unknowable. The
                // header block is thrown away and reading resumes after
                // it: the body, whatever it was, will not look like a
                // header and will be thrown away in its turn. A session
                // survives this; a session that stopped reading did not.
                None => Some("no Content-Length"),
                Some(length) if length > LONGEST => Some("Content-Length past all reason"),
                Some(_) => None,
            },
        };
        self.head = Some(head);
        Some(head)
    }

    /// How many bytes the frame at the front of the buffer occupies, and
    /// what it holds — `None` while there is not a whole one yet.
    fn measured(&mut self) -> Option<(usize, Frame)> {
        let head = self.head()?;
        if let Some(why) = head.broken {
            return Some((head.body_at, Frame::Broken(why)));
        }
        let length = head.length.unwrap_or(0);
        let rest = &self.held[self.from..];
        if rest.len() < head.body_at + length {
            // The body is still arriving.
            return None;
        }
        let body = &rest[head.body_at..head.body_at + length];
        let past = head.body_at + length;
        match std::str::from_utf8(body) {
            Ok(text) => Some((past, Frame::Body(text.to_string()))),
            // The length is known, so this one is skipped exactly and
            // the stream stays in step.
            Err(_) => Some((past, Frame::Broken("a body that is not UTF-8"))),
        }
    }
}

enum Taken {
    Frame(Frame),
    Nothing,
}

/// Where `needle` first appears in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Send one message's body, with the header a client expects.
pub fn write(output: &mut impl Write, body: &str) {
    let _ = output.write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    let _ = output.write_all(body.as_bytes());
    let _ = output.flush();
}

#[cfg(test)]
mod tests {
    use super::{Frame, Frames};

    fn framed(bodies: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        for body in bodies {
            out.extend_from_slice(
                format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes(),
            );
        }
        out
    }

    #[test]
    fn a_frame_without_a_length_does_not_end_the_stream() {
        let mut stream: Vec<u8> = b"Content-Type: nonsense\r\n\r\n".to_vec();
        stream.extend_from_slice(&framed(&["{\"a\":1}"]));
        let mut input = std::io::Cursor::new(stream);
        let mut frames = Frames::over(&mut input);
        assert!(matches!(frames.next(), Frame::Broken(_)));
        // The body of the broken frame — `{"a":1}` has a header of its
        // own in front of it, so the next whole frame is found.
        let mut seen = Vec::new();
        loop {
            match frames.next() {
                Frame::Body(body) => seen.push(body),
                Frame::Broken(_) => continue,
                Frame::Over => break,
            }
        }
        assert_eq!(seen, vec!["{\"a\":1}".to_string()]);
    }

    #[test]
    fn a_body_that_is_not_utf8_is_skipped_and_the_next_one_is_read() {
        let mut stream: Vec<u8> = b"Content-Length: 2\r\n\r\n".to_vec();
        stream.extend_from_slice(&[0xff, 0xfe]);
        stream.extend_from_slice(&framed(&["{\"ok\":1}"]));
        let mut input = std::io::Cursor::new(stream);
        let mut frames = Frames::over(&mut input);
        assert_eq!(frames.next(), Frame::Broken("a body that is not UTF-8"));
        assert_eq!(frames.next(), Frame::Body("{\"ok\":1}".to_string()));
        assert_eq!(frames.next(), Frame::Over);
    }

    #[test]
    fn a_length_past_all_reason_is_refused_rather_than_allocated() {
        let mut input = std::io::Cursor::new(b"Content-Length: 99999999999\r\n\r\n".to_vec());
        let mut frames = Frames::over(&mut input);
        assert_eq!(
            frames.next(),
            Frame::Broken("Content-Length past all reason")
        );
    }

    #[test]
    fn a_header_split_across_reads_is_still_one_frame() {
        // What a pipe actually does. `Cursor` hands over everything at
        // once, so the split is made by reading from a stream that
        // answers one byte at a time.
        struct Dribble(Vec<u8>, usize);
        impl std::io::Read for Dribble {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.1 >= self.0.len() || out.is_empty() {
                    return Ok(0);
                }
                out[0] = self.0[self.1];
                self.1 += 1;
                Ok(1)
            }
        }
        let stream = framed(&["{\"one\":1}", "{\"two\":2}"]);
        let mut input = std::io::BufReader::new(Dribble(stream, 0));
        let mut frames = Frames::over(&mut input);
        assert_eq!(frames.next(), Frame::Body("{\"one\":1}".to_string()));
        assert_eq!(frames.next(), Frame::Body("{\"two\":2}".to_string()));
        assert_eq!(frames.next(), Frame::Over);
    }

    #[test]
    fn a_length_in_bytes_and_not_in_characters() {
        let body = "{\"t\":\"héllo 😀\"}";
        let mut input = std::io::Cursor::new(framed(&[body]));
        let mut frames = Frames::over(&mut input);
        assert_eq!(frames.next(), Frame::Body(body.to_string()));
    }

    #[test]
    fn what_is_already_in_the_buffer_is_known_without_waiting() {
        let mut input = std::io::Cursor::new(framed(&["{\"a\":1}", "{\"b\":2}"]));
        let mut frames = Frames::over(&mut input);
        assert!(!frames.waiting(), "nothing has been read yet");
        assert_eq!(frames.next(), Frame::Body("{\"a\":1}".to_string()));
        assert!(frames.waiting(), "the second frame came in the same read");
        assert_eq!(frames.next(), Frame::Body("{\"b\":2}".to_string()));
        assert!(!frames.waiting());
    }

    #[test]
    fn a_half_arrived_body_is_not_a_frame() {
        let mut input = std::io::Cursor::new(b"Content-Length: 40\r\n\r\n{\"a\":1}".to_vec());
        let mut frames = Frames::over(&mut input);
        assert_eq!(frames.next(), Frame::Over);
    }

    #[test]
    fn a_blank_line_without_a_carriage_return_is_still_a_separator() {
        // Not what the specification says, and what more than one small
        // client sends.
        let body = "{\"a\":1}";
        let stream = format!("Content-Length: {}\n\n{body}", body.len());
        let mut input = std::io::Cursor::new(stream.into_bytes());
        let mut frames = Frames::over(&mut input);
        assert_eq!(frames.next(), Frame::Body(body.to_string()));
    }
}
