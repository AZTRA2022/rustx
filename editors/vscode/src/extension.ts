// The wiring, and nothing else.
//
// The colours are a grammar and the underlines are a server; what is
// left for an extension to do is to find that server and talk to it. It
// is found on the `PATH`, or named by `rustx.server.path` for whoever
// built it themselves — which, until the server is published, is
// everyone.

import { existsSync } from "node:fs";

import * as vscode from "vscode";
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

// Plain JavaScript on purpose: these two are the only parts of the
// extension with a rule worth arguing about, and plain modules can be
// tested with `node --test` without a build or a running editor.
import { onlyIndentationChanged, reindent } from "./indent.mjs";
import { closingFor } from "./tags.mjs";

let client: LanguageClient | undefined;

/** Only a `.rsx`, and only one the editor is showing. */
const RSX = { scheme: "file", language: "rsx" } as const;

/**
 * Close a tag when its `>` is typed.
 *
 * VS Code pairs *characters*, and a closing tag is a *name*: there is
 * nothing to declare for this in a `language-configuration.json`, so
 * the rule is ours. The hard part is not the JSX, it is the Rust around
 * it — a `.rsx` ending in `Vec<u32>` answered with `</u32>` would be
 * worse than nothing — and that rule lives in `tags.mjs`, with its
 * tests.
 */
function closeTags(): vscode.Disposable {
  return vscode.workspace.onDidChangeTextDocument((event) => {
    if (event.document.languageId !== "rsx") {
      return;
    }
    // One change, and that change a `>`: anything else is a paste, an
    // undo, or a formatter, and none of them wants a tag closed.
    if (event.contentChanges.length !== 1) {
      return;
    }
    const change = event.contentChanges[0];
    if (change.text !== ">") {
      return;
    }
    const wanted = vscode.workspace
      .getConfiguration("rustx")
      .get<boolean>("closeTags", true);
    if (!wanted) {
      return;
    }
    const editor = vscode.window.activeTextEditor;
    if (editor === undefined || editor.document !== event.document) {
      return;
    }

    const after = change.range.start.translate(0, change.text.length);
    // Only what is near the cursor. This runs between the key going
    // down and the `>` appearing, and copying a megabyte of document
    // out of the editor on every one of those is a stutter the person
    // typing pays for. A tag's `<` is a few characters back — `tags.mjs`
    // will not look further than this either.
    const end = event.document.offsetAt(after);
    const start = event.document.positionAt(Math.max(0, end - 4096));
    let closing: string | undefined;
    try {
      closing = closingFor(event.document.getText(new vscode.Range(start, after)));
    } catch {
      // A rule that cannot decide closes nothing. This is a document
      // change handler: throwing here is an error on every keystroke.
      return;
    }
    if (closing === undefined) {
      return;
    }
    // `$0` leaves the caret between the two tags, which is where the
    // next thing typed belongs.
    void editor.insertSnippet(
      new vscode.SnippetString(`$0${closing}`),
      after,
      { undoStopBefore: false, undoStopAfter: true },
    );
  });
}

/**
 * What Format Document does to a `.rsx`.
 *
 * Reindenting and nothing else — see `indent.mjs` for why a `.rsx` can
 * be neither rustfmt's nor Prettier's.
 */
function formatting(): vscode.Disposable {
  return vscode.languages.registerDocumentFormattingEditProvider(RSX, {
    provideDocumentFormattingEdits(document, options) {
      const text = document.getText();
      let formatted: string;
      try {
        formatted = reindent(text, {
          tabSize: options.tabSize,
          insertSpaces: options.insertSpaces,
        });
      } catch {
        // Format Document must never fail with a stack trace out of
        // the extension. A file this cannot indent is a file left
        // exactly as it was, which is the worst that should happen to
        // somebody's source.
        return [];
      }
      // And never an edit that loses text: an indenter changes leading
      // whitespace and nothing else, so if anything else moved, the
      // result is thrown away rather than written over the document.
      if (!onlyIndentationChanged(text, formatted)) {
        return [];
      }
      if (formatted === text) {
        return [];
      }
      const whole = new vscode.Range(
        document.positionAt(0),
        document.positionAt(text.length),
      );
      return [vscode.TextEdit.replace(whole, formatted)];
    },
  });
}

/** Where the server is, as this workspace says. */
function server(): string {
  const named = vscode.workspace
    .getConfiguration("rustx")
    .get<string>("server.path");
  return named && named.trim() !== "" ? named.trim() : "rustx-lsp";
}

/// What the server needs from this machine.
///
/// Completion, hovers and definitions come from `rust-analyzer`, which
/// the server starts itself. Someone whose copy is not on the `PATH` —
/// a rustup component in a toolchain directory, a build of their own —
/// says where it is, and it reaches the server through its environment.
function surroundings(): Record<string, string> | undefined {
  const named = vscode.workspace
    .getConfiguration("rustx")
    .get<string>("rustAnalyzer.path");
  const analyzer =
    named && named.trim() !== "" ? named.trim() : analyzerOfTheEditor();
  if (!analyzer) {
    return undefined;
  }
  return { ...process.env, RUSTX_RUST_ANALYZER: analyzer } as Record<
    string,
    string
  >;
}

/// The `rust-analyzer` VS Code already has, if it has one.
///
/// Anyone writing Rust in this editor has the `rust-analyzer` extension,
/// and it ships the binary — in a directory whose name carries its
/// version, which is why this asks the editor rather than guessing. It
/// saves the one setup step nobody should have to discover: the
/// `rust-analyzer` on a `PATH` is often a rustup shim for a component
/// that was never installed, and that shim fails with a message the
/// developer never sees.
function analyzerOfTheEditor(): string | undefined {
  const theirs = vscode.extensions.getExtension("rust-lang.rust-analyzer");
  if (!theirs) {
    return undefined;
  }
  const binary = vscode.Uri.joinPath(
    theirs.extensionUri,
    "server",
    process.platform === "win32" ? "rust-analyzer.exe" : "rust-analyzer",
  );
  return existsSync(binary.fsPath) ? binary.fsPath : undefined;
}

/**
 * Start the server, or say why it did not start.
 *
 * Separate from `activate` because restarting it is a command: a server
 * that has got out of step with the compiler beside it — an older build
 * still underlining what the compiler now accepts — is fixed by this
 * and not by reloading the window.
 */
function start(): void {
  // A server that cannot even be constructed — a `rustx.server.path`
  // holding something that is not a command, an environment this
  // cannot read — must not take the extension down with it. The
  // grammar, the tag closing and the formatter do not need a server,
  // and an editor that lost all three because a binary is missing is
  // a worse day than one that lost the underlines.
  try {
    startOrThrow();
  } catch (reason: unknown) {
    void vscode.window.showWarningMessage(
      `rustx: the language server could not be started (${said(reason)}). ` +
        "Colours, tag closing and formatting still work.",
    );
  }
}

function startOrThrow(): void {
  const command = server();
  const options: ServerOptions = {
    run: {
      command,
      transport: TransportKind.stdio,
      options: { env: surroundings() },
    },
    debug: {
      command,
      transport: TransportKind.stdio,
      options: { env: surroundings() },
    },
  };

  const client_options: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "rsx" }],
    // The server reads the text the editor is holding, so nothing on
    // disk needs watching: a `.rsx` that changes outside the editor is
    // a `.rsx` the editor will reopen.
    outputChannelName: "rustx",
  };

  client = new LanguageClient("rustx", "rustx", options, client_options);

  // A server that is not there is an ordinary thing — nobody has built
  // it on a fresh checkout — and it must not be an error dialog on every
  // `.rsx` that is opened.
  client.start().catch((reason: unknown) => {
    void vscode.window.showWarningMessage(
      `rustx: the language server (${command}) did not start. ` +
        "Build it with `cargo build -p rustx-lsp`, or set `rustx.server.path`. " +
        `(${said(reason)})`,
    );
  });
}

/** Whatever was thrown, as a sentence. */
function said(reason: unknown): string {
  if (reason instanceof Error) {
    return reason.message;
  }
  try {
    return String(reason);
  } catch {
    // A thrown object whose `toString` throws. It happens.
    return "no reason given";
  }
}

/**
 * Stop the client, and do not care how that goes.
 *
 * A server that has already died rejects its own `stop()`, and a
 * rejection here is an error dialog on a command that did what the
 * user asked. Restarting a dead server is the normal reason to restart
 * one.
 */
async function stop(): Promise<void> {
  const going = client;
  client = undefined;
  if (going === undefined) {
    return;
  }
  try {
    await going.stop();
  } catch {
    // Already gone, which is where we wanted it.
  }
}

/** True while a restart is in flight, so that two cannot overlap. */
let restarting = false;

export function activate(context: vscode.ExtensionContext): void {
  start();

  context.subscriptions.push(
    closeTags(),
    formatting(),
    vscode.commands.registerCommand("rustx.restartServer", async () => {
      // Two restarts at once would leave one client running with
      // nothing holding it: the second `start()` overwrites `client`
      // and the first server is never stopped.
      if (restarting) {
        return;
      }
      restarting = true;
      try {
        await stop();
        start();
        void vscode.window.setStatusBarMessage("rustx: server restarted", 2000);
      } finally {
        restarting = false;
      }
    }),
    {
      dispose: () => {
        void stop();
      },
    },
  );
}

export function deactivate(): Thenable<void> | undefined {
  // Not `client.stop()` directly: a rejection on the way out is an
  // error VS Code reports about an editor that is already closing.
  return stop();
}
