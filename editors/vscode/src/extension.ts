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
import { reindent } from "./indent.mjs";
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
    const upToHere = event.document.getText(
      new vscode.Range(new vscode.Position(0, 0), after),
    );
    const closing = closingFor(upToHere);
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
      const formatted = reindent(text, {
        tabSize: options.tabSize,
        insertSpaces: options.insertSpaces,
      });
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
        `(${reason instanceof Error ? reason.message : String(reason)})`,
    );
  });

}

export function activate(context: vscode.ExtensionContext): void {
  start();

  context.subscriptions.push(
    closeTags(),
    formatting(),
    vscode.commands.registerCommand("rustx.restartServer", async () => {
      await client?.stop();
      client = undefined;
      start();
      void vscode.window.setStatusBarMessage("rustx: server restarted", 2000);
    }),
    {
      dispose: () => {
        void client?.stop();
      },
    },
  );
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
