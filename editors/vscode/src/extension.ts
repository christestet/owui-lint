import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext): void {
  client = createClient();
  context.subscriptions.push(
    vscode.commands.registerCommand("owui-lint.restartServer", async () => {
      await restart(context);
    }),
  );
  client.start();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function createClient(): LanguageClient {
  const config = vscode.workspace.getConfiguration("owui-lint");
  const command = config.get<string>("path", "owui-lint");

  // owui-lint complements Pylance/Pyright; it only adds Open WebUI-specific
  // diagnostics, hovers, quick-fixes and scaffolding snippets.
  const serverOptions: ServerOptions = {
    run: { command, args: ["server"], transport: TransportKind.stdio },
    debug: { command, args: ["server"], transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "python" }],
  };

  return new LanguageClient(
    "owui-lint",
    "owui-lint",
    serverOptions,
    clientOptions,
  );
}

async function restart(context: vscode.ExtensionContext): Promise<void> {
  if (client) {
    await client.stop();
  }
  client = createClient();
  context.subscriptions.push(client);
  await client.start();
}
