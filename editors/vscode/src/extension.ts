// The wiring, and nothing else.
//
// The colours are a grammar and the underlines are a server; what is
// left for an extension to do is to find that server and talk to it. It
// is found on the `PATH`, or named by `rustx.server.path` for whoever
// built it themselves — which, until the server is published, is
// everyone.

import * as vscode from "vscode";
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

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
  const analyzer = vscode.workspace
    .getConfiguration("rustx")
    .get<string>("rustAnalyzer.path");
  if (!analyzer || analyzer.trim() === "") {
    return undefined;
  }
  return { ...process.env, RUSTX_RUST_ANALYZER: analyzer.trim() } as Record<
    string,
    string
  >;
}

export function activate(context: vscode.ExtensionContext): void {
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

  context.subscriptions.push({
    dispose: () => {
      void client?.stop();
    },
  });
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
