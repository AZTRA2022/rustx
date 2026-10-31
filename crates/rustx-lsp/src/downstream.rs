//! A language server of our own, downstream of this one.
//!
//! `rustx-lsp` answers what it knows — where the compiler refused a page
//! — and asks `rust-analyzer` for the rest. That makes it a client too,
//! and the failures of a client are its own: an answer that belongs to
//! another question, a notification in the middle of one, a child that
//! was never installed, a child that died while the editor was open.
//!
//! Three rules hold it together. Every question carries an id of this
//! client's own, so an answer is matched rather than assumed. Reading
//! happens on a thread, so a question has a deadline — a keystroke must
//! never wait on a server that stopped answering. And nothing here is an
//! error the editor hears about: no server in front means no completion,
//! which is exactly what a fresh machine has.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::{json, rpc};

/// How long a question waits before it is answered with nothing.
const PATIENCE: Duration = Duration::from_millis(1500);

/// A language server this one talks to.
pub struct Server {
    answers: mpsc::Receiver<String>,
    output: Box<dyn Write + Send>,
    child: Option<Child>,
    command: Option<(String, Vec<String>)>,
    next_id: i64,
    /// Everything written to it, which is what a test reads back and
    /// what makes a session legible when one goes wrong.
    sent: String,
}

impl Server {
    /// A server over two streams, whatever they are.
    ///
    /// Reading is done on a thread of its own: the point of a deadline
    /// is that the thread which waits is not the one the editor is
    /// talking to.
    pub fn over(input: Box<dyn BufRead + Send>, output: Box<dyn Write + Send>) -> Self {
        let (sender, answers) = mpsc::channel();
        std::thread::spawn(move || {
            let mut input = input;
            while let Some(message) = rpc::read(&mut input) {
                if sender.send(message).is_err() {
                    return;
                }
            }
        });
        Self {
            answers,
            output,
            child: None,
            command: None,
            next_id: 0,
            sent: String::new(),
        }
    }

    /// Start `command` and hold it, or `None` if there is nothing to
    /// hold.
    ///
    /// `None` covers both "not installed" and "installed but did not
    /// answer", because the editor does the same thing in either case.
    pub fn start(command: &str) -> Option<Self> {
        Self::start_with(command, &[])
    }

    /// The same, for a server that is started with arguments.
    pub fn start_with(command: &str, arguments: &[&str]) -> Option<Self> {
        let mut child = Command::new(command)
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let input = BufReader::new(child.stdout.take()?);
        let output = child.stdin.take()?;
        let mut server = Self::over(Box::new(input), Box::new(output));
        server.command = Some((
            command.to_string(),
            arguments
                .iter()
                .map(|argument| argument.to_string())
                .collect(),
        ));

        let answer = server.ask(
            "initialize",
            "{\"processId\":null,\"rootUri\":null,\"capabilities\":{}}",
        );
        answer.as_ref()?;
        server.tell("initialized", "{}");
        server.child = Some(child);
        Some(server)
    }

    /// Ask, and wait for the answer to that question.
    ///
    /// An answer to another question is dropped: this client keeps one
    /// question in flight at a time, so a stray id belongs to a question
    /// whose asker has already given up.
    pub fn ask(&mut self, method: &str, params: &str) -> Option<String> {
        self.next_id += 1;
        let id = self.next_id;
        self.write(&format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{}\",\"params\":{params}}}",
            json::escape(method)
        ));

        loop {
            let message = self.answers.recv_timeout(PATIENCE).ok()?;
            if json::number(&message, "id") == Some(id) {
                return Some(message);
            }
        }
    }

    /// Say something that has no answer.
    pub fn tell(&mut self, method: &str, params: &str) {
        self.write(&format!(
            "{{\"jsonrpc\":\"2.0\",\"method\":\"{}\",\"params\":{params}}}",
            json::escape(method)
        ));
    }

    /// Start the child again, after it died.
    ///
    /// A server over plain streams cannot be restarted — there is no
    /// command to run — and says so by answering `false`.
    pub fn restart(&mut self) -> bool {
        let Some((command, arguments)) = self.command.clone() else {
            return false;
        };
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        match Self::start_with(&command, &arguments) {
            Some(fresh) => {
                *self = fresh;
                true
            }
            None => false,
        }
    }

    /// Everything written to the server so far.
    pub fn written(&self) -> &str {
        &self.sent
    }

    fn write(&mut self, body: &str) {
        self.sent
            .push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
        rpc::write(&mut self.output, body);
        let _ = self.output.flush();
    }
}

impl Drop for Server {
    /// A server per window, and a child per server: one left behind is a
    /// `rust-analyzer` indexing a project nobody has open.
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
