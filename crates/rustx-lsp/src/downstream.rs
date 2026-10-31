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
///
/// Three seconds, which sounds long for a keystroke and is not: an
/// editor asks and draws when the answer comes, and `rust-analyzer`
/// takes its time on the first questions of a session. At a second and
/// a half, measured, the first hover of a page was lost every time.
const PATIENCE: Duration = Duration::from_millis(3000);

/// How long the handshake waits. Nobody is typing through it, and a
/// server that has just started has a manifest to read.
const HANDSHAKE: Duration = Duration::from_secs(20);

/// A language server this one talks to.
pub struct Server {
    answers: mpsc::Receiver<String>,
    output: Box<dyn Write + Send>,
    child: Option<Child>,
    command: Option<(String, Vec<String>)>,
    next_id: i64,
    /// The project it was started in, if it was told one.
    root: Option<String>,
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
            root: None,
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

    /// The same, in the project the editor said it had opened.
    ///
    /// `rust-analyzer` analyses a workspace, and without one it analyses
    /// nothing: a server started with no root answers every question
    /// with silence, which looks exactly like a server that is not
    /// installed. The editor names the root in its own `initialize`, so
    /// that is the one to pass on.
    pub fn start_in(
        command: &str,
        root: Option<&str>,
        linked: &[std::path::PathBuf],
    ) -> Option<Self> {
        Self::started(command, &[], root, linked)
    }

    /// The same, for a server that is started with arguments.
    pub fn start_with(command: &str, arguments: &[&str]) -> Option<Self> {
        Self::started(command, arguments, None, &[])
    }

    fn started(
        command: &str,
        arguments: &[&str],
        root: Option<&str>,
        linked: &[std::path::PathBuf],
    ) -> Option<Self> {
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

        server.root = root.map(str::to_string);
        // The handshake is the one question worth waiting for: a server
        // that has just been started has a process to create and a
        // manifest to read, and a keystroke is not waiting on this one.
        let answer = server.wait_longer_for("initialize", &handshake(root, linked));
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
        self.asked(method, params, PATIENCE)
    }

    /// The same, for the handshake, which nobody is typing through.
    fn wait_longer_for(&mut self, method: &str, params: &str) -> Option<String> {
        self.asked(method, params, HANDSHAKE)
    }

    fn asked(&mut self, method: &str, params: &str, patience: Duration) -> Option<String> {
        self.next_id += 1;
        let id = self.next_id;
        self.write(&format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{}\",\"params\":{params}}}",
            json::escape(method)
        ));

        loop {
            let message = match self.answers.recv_timeout(patience) {
                Ok(message) => message,
                Err(why) => {
                    traced(&format!("{method} #{id} got nothing ({why})"));
                    return None;
                }
            };
            // A message carrying a method is the child asking us
            // something, and its ids are numbered from one as well: read
            // as an answer, the child's first request would be taken for
            // the answer to our first question.
            if json::field(&message, "method").is_some() {
                continue;
            }
            if json::number(&message, "id") == Some(id) {
                traced(&format!("{method} #{id} → {message}"));
                return Some(message);
            }
        }
    }

    /// The project this server was started in, if it was told one.
    pub fn root(&self) -> Option<&str> {
        self.root.as_deref()
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
        traced(&format!("→ {body}"));
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

/// The `initialize` a client sends, naming the project it has open.
///
/// Its own function because it is the one message whose contents decide
/// whether anything downstream works at all: `rust-analyzer` analyses a
/// workspace, and a handshake that names none leaves it analysing
/// nothing — which is indistinguishable, from the editor, from a server
/// that was never installed.
pub fn handshake(root: Option<&str>, linked: &[std::path::PathBuf]) -> String {
    // The projects to load, named outright. `rust-analyzer` would
    // otherwise look for them itself and find the application, whose
    // pages are included from inside a macro expansion and therefore
    // belong to no file it can answer about.
    let projects: Vec<String> = linked
        .iter()
        .map(|manifest| {
            let path = manifest.to_string_lossy().replace('\\', "/");
            format!("\"{}\"", json::escape(&path))
        })
        .collect();
    let options = format!(
        ",\"initializationOptions\":{{\"linkedProjects\":[{}]}}",
        projects.join(",")
    );
    match root {
        Some(root) => format!(
            "{{\"processId\":null,\"rootUri\":\"{}\",\"workspaceFolders\":\r
             [{{\"uri\":\"{}\",\"name\":\"rustx\"}}],\"capabilities\":{{}}{options}}}",
            json::escape(root),
            json::escape(root)
        ),
        None => format!("{{\"processId\":null,\"rootUri\":null,\"capabilities\":{{}}{options}}}"),
    }
}

/// Say what passed, when asked to.
///
/// `RUSTX_LSP_TRACE=1` and the whole conversation with the downstream
/// server goes to stderr, which the editor shows on its output channel.
/// Written because a session that answered nothing could not be told
/// apart from a session that was never asked, and the difference took an
/// afternoon of probing to find.
fn traced(what: &str) {
    if std::env::var("RUSTX_LSP_TRACE").is_ok() {
        let what: String = what.chars().take(400).collect();
        eprintln!("rustx-lsp: {what}");
    }
}
